//! Volume profile generation (POC/VAH/VAL indicators)
//!
//! Generates volume distribution histograms across price levels using adaptive
//! tick-based binning. Identifies Point of Control (POC), Value Area High/Low
//! (VAH/VAL - 70% volume boundaries), and liquidity vacuum zones.

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::str::FromStr;
use uuid::Uuid;

use crate::orderbook::analytics::{
    storage::snapshot::OrderBookSnapshot,
    trade_stream::AggTrade,
    types::{ImpactLevel, LiquidityVacuum, VolumeBin, VolumeProfile},
};

/// Generate volume profile histogram from aggregated trade data
///
/// # Arguments
/// * `symbol` - Trading pair (e.g., "ETHUSDT")
/// * `trades` - Aggregated trade events from @aggTrade stream
/// * `duration_hours` - Analysis time period (1-168 hours)
/// * `custom_tick_size` - Optional custom bin size (if None, auto-calculated)
///
/// # Returns
/// VolumeProfile with histogram, POC, VAH, VAL, and liquidity vacuums
///
/// # Errors
/// * `insufficient_historical_data` - Need at least 1000 trades for reliable profile
pub async fn generate_volume_profile(
    symbol: &str,
    trades: Vec<AggTrade>,
    duration_hours: u32,
    custom_tick_size: Option<Decimal>,
) -> Result<VolumeProfile> {
    anyhow::ensure!(
        trades.len() >= 1000,
        "insufficient_historical_data: Minimum 1000 trades required for {}h profile, only {} trades available",
        duration_hours,
        trades.len()
    );

    anyhow::ensure!(
        (1..=168).contains(&duration_hours),
        "duration_hours must be between 1 and 168"
    );

    // Find price range
    let (price_min, price_max) = find_price_range(&trades)?;

    // Calculate adaptive bin size
    let bin_size = custom_tick_size.unwrap_or_else(|| {
        adaptive_bin_size(price_min, price_max, Decimal::from_str("0.01").unwrap())
    });

    // Bin trades by price level
    let bins = bin_trades_by_price(&trades, price_min, price_max, bin_size)?;

    // Find POC, VAH, VAL
    let (poc, vah, val) = find_poc_vah_val(&bins)?;

    // Note: Liquidity vacuums are calculated separately via get_liquidity_vacuums tool

    // Calculate total volume
    let total_volume: Decimal = bins.iter().map(|b| b.volume).sum();

    // Build histogram sorted by price
    let mut histogram = bins.clone();
    histogram.sort_by(|a, b| a.price_level.cmp(&b.price_level));

    let now = Utc::now();
    let start = now - Duration::hours(duration_hours as i64);

    Ok(VolumeProfile {
        symbol: symbol.to_string(),
        time_period_start: start,
        time_period_end: now,
        price_range_low: price_min,
        price_range_high: price_max,
        bin_size,
        bin_count: histogram.len(),
        histogram,
        total_volume,
        point_of_control: poc,
        value_area_high: vah,
        value_area_low: val,
    })
}

/// Calculate adaptive bin size using formula: max(tick_size × 10, price_range / 100)
///
/// # Arguments
/// * `price_min` - Lowest price in data
/// * `price_max` - Highest price in data
/// * `exchange_tick_size` - Exchange's minimum price increment
///
/// # Returns
/// Adaptive bin size (Decimal)
fn adaptive_bin_size(price_min: Decimal, price_max: Decimal, exchange_tick_size: Decimal) -> Decimal {
    let price_range = price_max - price_min;
    let range_based = price_range / Decimal::from(100);
    let tick_based = exchange_tick_size * Decimal::from(10);

    std::cmp::max(range_based, tick_based)
}

/// Group trades into price bins
///
/// # Arguments
/// * `trades` - Aggregated trade events
/// * `price_min` - Lower price boundary
/// * `price_max` - Upper price boundary
/// * `bin_size` - Price increment per bin
///
/// # Returns
/// Vector of VolumeBin with aggregated volume and trade counts
fn bin_trades_by_price(
    trades: &[AggTrade],
    price_min: Decimal,
    price_max: Decimal,
    bin_size: Decimal,
) -> Result<Vec<VolumeBin>> {
    let mut bins_map: HashMap<Decimal, (Decimal, u64)> = HashMap::new();

    for trade in trades {
        let price = Decimal::from_str(&trade.price)
            .context("Failed to parse trade price")?;
        let quantity = Decimal::from_str(&trade.quantity)
            .context("Failed to parse trade quantity")?;

        // Calculate bin center
        let bin_index = ((price - price_min) / bin_size).floor();
        let bin_center = price_min + (bin_index * bin_size) + (bin_size / Decimal::from(2));

        // Aggregate volume and count
        let entry = bins_map.entry(bin_center).or_insert((Decimal::ZERO, 0));
        entry.0 += quantity;
        entry.1 += 1;
    }

    // Convert to VolumeBin vec
    let bins: Vec<VolumeBin> = bins_map
        .into_iter()
        .map(|(price_level, (volume, trade_count))| VolumeBin {
            price_level,
            volume,
            trade_count,
        })
        .collect();

    Ok(bins)
}

/// Find Point of Control (POC), Value Area High (VAH), and Value Area Low (VAL)
///
/// # POC
/// Price level with highest volume (max volume bin)
///
/// # VAH/VAL
/// Upper and lower boundaries containing 70% of total volume, centered around POC
///
/// # Returns
/// (POC price, VAH price, VAL price)
fn find_poc_vah_val(bins: &[VolumeBin]) -> Result<(Decimal, Decimal, Decimal)> {
    anyhow::ensure!(!bins.is_empty(), "Cannot calculate POC/VAH/VAL from empty bins");

    // Find POC (max volume bin)
    let poc_bin = bins
        .iter()
        .max_by_key(|b| b.volume)
        .context("No bins available")?;
    let poc = poc_bin.price_level;

    // Calculate total volume
    let total_volume: Decimal = bins.iter().map(|b| b.volume).sum();
    let target_volume = total_volume * Decimal::from_str("0.70").unwrap();

    // Sort bins by price
    let mut sorted_bins = bins.to_vec();
    sorted_bins.sort_by(|a, b| a.price_level.cmp(&b.price_level));

    // Find POC index in sorted bins
    let poc_idx = sorted_bins
        .iter()
        .position(|b| b.price_level == poc)
        .unwrap_or(sorted_bins.len() / 2);

    // Expand around POC until we reach 70% volume
    let mut accumulated_volume = poc_bin.volume;
    let mut low_idx = poc_idx;
    let mut high_idx = poc_idx;

    while accumulated_volume < target_volume && (low_idx > 0 || high_idx < sorted_bins.len() - 1) {
        let below_volume = if low_idx > 0 {
            sorted_bins[low_idx - 1].volume
        } else {
            Decimal::ZERO
        };

        let above_volume = if high_idx < sorted_bins.len() - 1 {
            sorted_bins[high_idx + 1].volume
        } else {
            Decimal::ZERO
        };

        if below_volume > above_volume && low_idx > 0 {
            low_idx -= 1;
            accumulated_volume += sorted_bins[low_idx].volume;
        } else if high_idx < sorted_bins.len() - 1 {
            high_idx += 1;
            accumulated_volume += sorted_bins[high_idx].volume;
        } else {
            break;
        }
    }

    let val = sorted_bins[low_idx].price_level;
    let vah = sorted_bins[high_idx].price_level;

    Ok((poc, vah, val))
}

/// Create liquidity vacuum with severity classification
fn create_vacuum(
    symbol: &str,
    low_bin: &VolumeBin,
    high_bin: &VolumeBin,
    median_volume: Decimal,
) -> LiquidityVacuum {
    let actual_volume = (low_bin.volume + high_bin.volume) / Decimal::from(2);
    let deficit = (median_volume - actual_volume) / median_volume * Decimal::from(100);
    let volume_deficit_pct = deficit.to_string().parse::<f64>().unwrap_or(0.0);

    let expected_impact = ImpactLevel::from_deficit_pct(volume_deficit_pct);

    LiquidityVacuum {
        vacuum_id: Uuid::new_v4(),
        symbol: symbol.to_string(),
        price_range_low: low_bin.price_level,
        price_range_high: high_bin.price_level,
        volume_deficit_pct,
        median_volume,
        actual_volume,
        expected_impact,
        detection_timestamp: Utc::now(),
    }
}

/// Find min and max prices from trade data
fn find_price_range(trades: &[AggTrade]) -> Result<(Decimal, Decimal)> {
    anyhow::ensure!(!trades.is_empty(), "Cannot find price range from empty trades");

    let mut price_min = Decimal::MAX;
    let mut price_max = Decimal::MIN;

    for trade in trades {
        let price = Decimal::from_str(&trade.price)
            .context("Failed to parse trade price")?;
        price_min = std::cmp::min(price_min, price);
        price_max = std::cmp::max(price_max, price);
    }

    Ok((price_min, price_max))
}

/// Identify order walls (large resting orders >10x median volume)
///
/// Detects institutional orders and support/resistance levels by analyzing
/// the current order book depth. Order walls indicate potential price barriers
/// where large liquidity is concentrated.
///
/// # Arguments
/// * `snapshot` - Current order book snapshot
///
/// # Returns
/// Vector of (price, volume, side) tuples for detected walls
/// Side is either "bid" or "ask"
///
/// # Detection Criteria
/// - Volume >10x median for that side of the book
/// - Only considers top 20 levels (most significant)
pub fn identify_order_walls(snapshot: &OrderBookSnapshot) -> Vec<(Decimal, Decimal, &str)> {
    let mut walls = Vec::new();

    // Calculate median bid volume
    let bid_volumes: Vec<Decimal> = snapshot.bids.iter()
        .take(20)
        .filter_map(|(_, qty)| Decimal::from_str(qty).ok())
        .collect();

    if bid_volumes.is_empty() {
        return walls;
    }

    let median_bid_volume = calculate_median_decimal(&bid_volumes);
    let bid_threshold = median_bid_volume * Decimal::from(10);

    // Identify bid walls
    for (price_str, qty_str) in snapshot.bids.iter().take(20) {
        if let (Ok(price), Ok(volume)) = (
            Decimal::from_str(price_str),
            Decimal::from_str(qty_str)
        ) {
            if volume > bid_threshold {
                walls.push((price, volume, "bid"));
            }
        }
    }

    // Calculate median ask volume
    let ask_volumes: Vec<Decimal> = snapshot.asks.iter()
        .take(20)
        .filter_map(|(_, qty)| Decimal::from_str(qty).ok())
        .collect();

    if ask_volumes.is_empty() {
        return walls;
    }

    let median_ask_volume = calculate_median_decimal(&ask_volumes);
    let ask_threshold = median_ask_volume * Decimal::from(10);

    // Identify ask walls
    for (price_str, qty_str) in snapshot.asks.iter().take(20) {
        if let (Ok(price), Ok(volume)) = (
            Decimal::from_str(price_str),
            Decimal::from_str(qty_str)
        ) {
            if volume > ask_threshold {
                walls.push((price, volume, "ask"));
            }
        }
    }

    walls
}

/// Recommend stop-loss placement based on liquidity vacuum analysis
///
/// Suggests optimal stop placement to avoid getting stopped out by wicks
/// into low-liquidity zones. Places stops in high-liquidity areas beyond
/// vacuum zones to increase survival probability.
///
/// # Arguments
/// * `current_price` - Current market price
/// * `vacuums` - Detected liquidity vacuums from volume profile
/// * `direction` - Trade direction ("long" or "short")
///
/// # Returns
/// (recommended_stop_price, explanation_string)
///
/// # Strategy
/// - **Long positions**: Place stops below vacuum zones in solid liquidity
/// - **Short positions**: Place stops above vacuum zones in solid liquidity
/// - Avoids thin liquidity areas that could trigger premature stops
pub fn recommend_stop_placement(
    current_price: Decimal,
    vacuums: &[LiquidityVacuum],
    direction: &str,
) -> (Decimal, String) {
    if vacuums.is_empty() {
        let default_stop_distance = current_price * Decimal::from_str("0.02").unwrap(); // 2% default
        return match direction.to_lowercase().as_str() {
            "long" => (
                current_price - default_stop_distance,
                "No liquidity vacuums detected. Using standard 2% stop distance.".to_string(),
            ),
            "short" => (
                current_price + default_stop_distance,
                "No liquidity vacuums detected. Using standard 2% stop distance.".to_string(),
            ),
            _ => (
                current_price,
                "Invalid direction. Please specify 'long' or 'short'.".to_string(),
            ),
        };
    }

    match direction.to_lowercase().as_str() {
        "long" => {
            // For longs, find vacuums below current price
            let mut relevant_vacuums: Vec<&LiquidityVacuum> = vacuums
                .iter()
                .filter(|v| v.price_range_high < current_price)
                .collect();

            if relevant_vacuums.is_empty() {
                let default_distance = current_price * Decimal::from_str("0.02").unwrap();
                return (
                    current_price - default_distance,
                    "No liquidity vacuums below current price. Using 2% stop.".to_string(),
                );
            }

            // Sort by price descending (closest vacuum first)
            relevant_vacuums.sort_by(|a, b| b.price_range_high.cmp(&a.price_range_high));

            let nearest_vacuum = relevant_vacuums[0];

            // Place stop below the vacuum zone (in solid liquidity)
            let stop_price = nearest_vacuum.price_range_low * Decimal::from_str("0.998").unwrap();
            let explanation = format!(
                "Stop placed at {} (below liquidity vacuum at {}-{}). Vacuum deficit: {:.1}%, impact: {:?}. This avoids thin liquidity that could cause premature stop-outs.",
                stop_price,
                nearest_vacuum.price_range_low,
                nearest_vacuum.price_range_high,
                nearest_vacuum.volume_deficit_pct,
                nearest_vacuum.expected_impact
            );

            (stop_price, explanation)
        }
        "short" => {
            // For shorts, find vacuums above current price
            let mut relevant_vacuums: Vec<&LiquidityVacuum> = vacuums
                .iter()
                .filter(|v| v.price_range_low > current_price)
                .collect();

            if relevant_vacuums.is_empty() {
                let default_distance = current_price * Decimal::from_str("0.02").unwrap();
                return (
                    current_price + default_distance,
                    "No liquidity vacuums above current price. Using 2% stop.".to_string(),
                );
            }

            // Sort by price ascending (closest vacuum first)
            relevant_vacuums.sort_by(|a, b| a.price_range_low.cmp(&b.price_range_low));

            let nearest_vacuum = relevant_vacuums[0];

            // Place stop above the vacuum zone (in solid liquidity)
            let stop_price = nearest_vacuum.price_range_high * Decimal::from_str("1.002").unwrap();
            let explanation = format!(
                "Stop placed at {} (above liquidity vacuum at {}-{}). Vacuum deficit: {:.1}%, impact: {:?}. This avoids thin liquidity that could cause premature stop-outs.",
                stop_price,
                nearest_vacuum.price_range_low,
                nearest_vacuum.price_range_high,
                nearest_vacuum.volume_deficit_pct,
                nearest_vacuum.expected_impact
            );

            (stop_price, explanation)
        }
        _ => (
            current_price,
            "Invalid direction. Please specify 'long' or 'short'.".to_string(),
        ),
    }
}

/// Identify liquidity vacuums from current order book snapshot
///
/// Analyzes the current order book depth to find thin liquidity zones
/// where order volume is <20% of median. Used for stop-loss placement.
///
/// # Arguments
/// * `snapshot` - Current order book snapshot
/// * `symbol` - Trading pair symbol
///
/// # Returns
/// Vector of LiquidityVacuum detected in current book
pub fn identify_liquidity_vacuums(snapshot: &OrderBookSnapshot, symbol: &str) -> Result<Vec<LiquidityVacuum>> {

    // Convert order book to volume bins
    let mut all_levels = Vec::new();

    // Collect all bid levels
    for (price_str, qty_str) in &snapshot.bids {
        if let (Ok(price), Ok(volume)) = (
            Decimal::from_str(price_str),
            Decimal::from_str(qty_str)
        ) {
            all_levels.push(VolumeBin {
                price_level: price,
                volume,
                trade_count: 1, // Not used for orderbook analysis
            });
        }
    }

    // Collect all ask levels
    for (price_str, qty_str) in &snapshot.asks {
        if let (Ok(price), Ok(volume)) = (
            Decimal::from_str(price_str),
            Decimal::from_str(qty_str)
        ) {
            all_levels.push(VolumeBin {
                price_level: price,
                volume,
                trade_count: 1,
            });
        }
    }

    if all_levels.is_empty() {
        return Ok(Vec::new());
    }

    // Sort by price
    all_levels.sort_by(|a, b| a.price_level.cmp(&b.price_level));

    // Reuse the existing identify_liquidity_vacuums logic
    identify_liquidity_vacuums_impl(symbol, &all_levels)
}

/// Internal implementation of liquidity vacuum detection
///
/// Shared logic between trade-based and orderbook-based vacuum detection
fn identify_liquidity_vacuums_impl(symbol: &str, bins: &[VolumeBin]) -> Result<Vec<LiquidityVacuum>> {
    if bins.is_empty() {
        return Ok(Vec::new());
    }

    // Calculate median volume
    let mut volumes: Vec<Decimal> = bins.iter().map(|b| b.volume).collect();
    volumes.sort();
    let median_volume = if volumes.len() % 2 == 0 {
        (volumes[volumes.len() / 2 - 1] + volumes[volumes.len() / 2]) / Decimal::from(2)
    } else {
        volumes[volumes.len() / 2]
    };

    let threshold = median_volume * Decimal::from_str("0.20").unwrap();

    // Sort bins by price for range detection
    let mut sorted_bins = bins.to_vec();
    sorted_bins.sort_by(|a, b| a.price_level.cmp(&b.price_level));

    let mut vacuums = Vec::new();
    let mut vacuum_start: Option<usize> = None;

    for (idx, bin) in sorted_bins.iter().enumerate() {
        if bin.volume < threshold {
            if vacuum_start.is_none() {
                vacuum_start = Some(idx);
            }
        } else if let Some(start_idx) = vacuum_start {
            // End of vacuum zone
            let vacuum = create_vacuum(
                symbol,
                &sorted_bins[start_idx],
                &sorted_bins[idx - 1],
                median_volume,
            );
            vacuums.push(vacuum);
            vacuum_start = None;
        }
    }

    // Handle trailing vacuum
    if let Some(start_idx) = vacuum_start {
        let last_idx = sorted_bins.len() - 1;
        let vacuum = create_vacuum(
            symbol,
            &sorted_bins[start_idx],
            &sorted_bins[last_idx],
            median_volume,
        );
        vacuums.push(vacuum);
    }

    Ok(vacuums)
}

/// Calculate median from a vector of Decimals
fn calculate_median_decimal(values: &[Decimal]) -> Decimal {
    if values.is_empty() {
        return Decimal::ZERO;
    }

    let mut sorted = values.to_vec();
    sorted.sort();

    if sorted.len() % 2 == 0 {
        let mid = sorted.len() / 2;
        (sorted[mid - 1] + sorted[mid]) / Decimal::from(2)
    } else {
        sorted[sorted.len() / 2]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_bin_size() {
        let price_min = Decimal::from(100);
        let price_max = Decimal::from(200);
        let tick_size = Decimal::from_str("0.01").unwrap();

        let bin_size = adaptive_bin_size(price_min, price_max, tick_size);

        // price_range / 100 = 100 / 100 = 1.0
        // tick_size × 10 = 0.01 × 10 = 0.1
        // max(1.0, 0.1) = 1.0
        assert_eq!(bin_size, Decimal::from(1));
    }

    #[test]
    fn test_find_price_range() {
        let trades = vec![
            AggTrade {
                event_type: "aggTrade".to_string(),
                event_time: 0,
                symbol: "BTCUSDT".to_string(),
                agg_trade_id: 1,
                price: "50000.00".to_string(),
                quantity: "1.0".to_string(),
                first_trade_id: 1,
                last_trade_id: 1,
                trade_time: 0,
                is_buyer_maker: false,
                is_best_match: true,
            },
            AggTrade {
                event_type: "aggTrade".to_string(),
                event_time: 0,
                symbol: "BTCUSDT".to_string(),
                agg_trade_id: 2,
                price: "51000.00".to_string(),
                quantity: "1.0".to_string(),
                first_trade_id: 2,
                last_trade_id: 2,
                trade_time: 0,
                is_buyer_maker: false,
                is_best_match: true,
            },
        ];

        let (min, max) = find_price_range(&trades).unwrap();
        assert_eq!(min, Decimal::from(50000));
        assert_eq!(max, Decimal::from(51000));
    }
}
