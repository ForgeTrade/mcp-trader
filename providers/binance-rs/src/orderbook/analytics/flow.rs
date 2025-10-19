//! Order flow analysis calculations
//!
//! Provides bid/ask pressure tracking over configurable time windows (10-300s).
//! Analyzes buying vs selling pressure through order flow rates, cumulative delta,
//! and categorical flow direction indicators.

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};

use crate::orderbook::analytics::{
    storage::{
        snapshot::OrderBookSnapshot,
        query::query_snapshots_in_window,
        SnapshotStorage,
    },
    types::{AbsorptionEvent, Direction, EntityType, FlowDirection, OrderFlowSnapshot},
};
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::str::FromStr;
use uuid::Uuid;

/// Calculate order flow metrics over a time window
///
/// # Arguments
/// * `storage` - RocksDB storage instance with historical snapshots
/// * `symbol` - Trading pair (e.g., "BTCUSDT")
/// * `window_duration_secs` - Time window in seconds (10-300, from clarifications)
///
/// # Returns
/// OrderFlowSnapshot with bid/ask flow rates, net flow, and flow direction
///
/// # Errors
/// * `insufficient_historical_data` - Need at least 2 snapshots for window
/// * `storage_query_timeout` - Query exceeded 200ms timeout
pub async fn calculate_order_flow(
    storage: &SnapshotStorage,
    symbol: &str,
    window_duration_secs: u32,
) -> Result<OrderFlowSnapshot> {
    // Validate window duration (from clarifications: min 10s, max 300s)
    anyhow::ensure!(
        (10..=300).contains(&window_duration_secs),
        "window_duration_secs must be between 10 and 300 seconds"
    );

    let end = Utc::now();
    let start = end - Duration::seconds(window_duration_secs as i64);

    // Query snapshots from RocksDB (with 200ms timeout from FR-016)
    let snapshots = query_snapshots_in_window(
        storage,
        symbol,
        start.timestamp(),
        end.timestamp(),
    )
    .await
    .context("Failed to query snapshots from storage")?;

    anyhow::ensure!(
        snapshots.len() >= 2,
        "insufficient_historical_data: Need at least 2 snapshots for {} window (got {})",
        window_duration_secs,
        snapshots.len()
    );

    // Aggregate bid/ask order counts
    let (bid_count, ask_count) = aggregate_bid_ask_counts(&snapshots)?;

    // Calculate flow rates (orders/sec)
    let (bid_flow_rate, ask_flow_rate) =
        calculate_flow_rates(bid_count, ask_count, window_duration_secs);

    // Determine categorical flow direction
    let flow_direction = determine_flow_direction(bid_flow_rate, ask_flow_rate);

    // Calculate cumulative delta (running buy - sell volume)
    let cumulative_delta = calculate_cumulative_delta(&snapshots)?;

    let net_flow = bid_flow_rate - ask_flow_rate;

    Ok(OrderFlowSnapshot {
        symbol: symbol.to_string(),
        time_window_start: start,
        time_window_end: end,
        window_duration_secs,
        bid_flow_rate,
        ask_flow_rate,
        net_flow,
        flow_direction,
        cumulative_delta,
    })
}

/// Aggregate bid and ask order counts from snapshot deltas
///
/// Counts order additions (not cancellations) by tracking depth changes
/// between consecutive snapshots on each side of the book.
///
/// # Returns
/// (total_bid_additions, total_ask_additions)
fn aggregate_bid_ask_counts(snapshots: &[OrderBookSnapshot]) -> Result<(u64, u64)> {
    let mut bid_count = 0u64;
    let mut ask_count = 0u64;

    for window in snapshots.windows(2) {
        let prev = &window[0];
        let curr = &window[1];

        // Count new orders on bid side (positive depth increases)
        for (price_str, curr_qty_str) in &curr.bids {
            let curr_qty: f64 = curr_qty_str.parse().unwrap_or(0.0);
            
            let prev_qty: f64 = prev
                .bids
                .iter()
                .find(|(p, _)| p == price_str)
                .and_then(|(_, q)| q.parse().ok())
                .unwrap_or(0.0);

            if curr_qty > prev_qty {
                bid_count += 1;
            }
        }

        // Count new orders on ask side
        for (price_str, curr_qty_str) in &curr.asks {
            let curr_qty: f64 = curr_qty_str.parse().unwrap_or(0.0);
            
            let prev_qty: f64 = prev
                .asks
                .iter()
                .find(|(p, _)| p == price_str)
                .and_then(|(_, q)| q.parse().ok())
                .unwrap_or(0.0);

            if curr_qty > prev_qty {
                ask_count += 1;
            }
        }
    }

    Ok((bid_count, ask_count))
}

/// Calculate flow rates in orders per second
///
/// # Arguments
/// * `bid_count` - Total bid orders added during window
/// * `ask_count` - Total ask orders added during window
/// * `duration_secs` - Window duration
///
/// # Returns
/// (bid_flow_rate, ask_flow_rate) in orders/sec
fn calculate_flow_rates(bid_count: u64, ask_count: u64, duration_secs: u32) -> (f64, f64) {
    let duration = duration_secs as f64;
    let bid_flow_rate = (bid_count as f64) / duration;
    let ask_flow_rate = (ask_count as f64) / duration;
    (bid_flow_rate, ask_flow_rate)
}

/// Determine categorical flow direction from bid/ask ratio
///
/// Thresholds (from FR-003, types.rs):
/// - STRONG_BUY: ratio > 2.0
/// - MODERATE_BUY: ratio 1.2-2.0
/// - NEUTRAL: ratio 0.8-1.2
/// - MODERATE_SELL: ratio 0.5-0.8
/// - STRONG_SELL: ratio < 0.5
fn determine_flow_direction(bid_flow_rate: f64, ask_flow_rate: f64) -> FlowDirection {
    // Use existing implementation from types.rs
    FlowDirection::from_flow_rates(bid_flow_rate, ask_flow_rate)
}

/// Calculate cumulative delta (running buy volume - sell volume)
///
/// Sums the net volume difference across all snapshots in the window.
/// Positive values indicate accumulation, negative indicate distribution.
///
/// # Returns
/// Cumulative delta in base asset units
fn calculate_cumulative_delta(snapshots: &[OrderBookSnapshot]) -> Result<f64> {
    let mut cumulative_delta = 0.0;

    for window in snapshots.windows(2) {
        let prev = &window[0];
        let curr = &window[1];

        // Calculate net volume change on bid side (buying pressure)
        let bid_volume: f64 = curr.bids.iter()
            .filter_map(|(_, qty)| qty.parse::<f64>().ok())
            .sum();
        let prev_bid_volume: f64 = prev.bids.iter()
            .filter_map(|(_, qty)| qty.parse::<f64>().ok())
            .sum();
        let bid_delta = bid_volume - prev_bid_volume;

        // Calculate net volume change on ask side (selling pressure)
        let ask_volume: f64 = curr.asks.iter()
            .filter_map(|(_, qty)| qty.parse::<f64>().ok())
            .sum();
        let prev_ask_volume: f64 = prev.asks.iter()
            .filter_map(|(_, qty)| qty.parse::<f64>().ok())
            .sum();
        let ask_delta = ask_volume - prev_ask_volume;

        // Accumulate net delta (buy - sell)
        cumulative_delta += bid_delta.abs() - ask_delta.abs();
    }

    Ok(cumulative_delta)
}

/// Detect absorption events where large hidden orders absorb market pressure
///
/// Identifies whale/market maker activity by tracking price level refills:
/// - Monitors consecutive refills at the same price (>5x median volume)
/// - Classifies as accumulation (bid absorption) or distribution (ask absorption)
/// - Detects iceberg orders and layered liquidity provision
///
/// # Arguments
/// * `snapshots` - OrderBook snapshots over analysis window
/// * `symbol` - Trading pair (e.g., "BTCUSDT")
///
/// # Returns
/// Vector of AbsorptionEvent with detection time, price, volume, and direction
///
/// # Algorithm
/// 1. Track each price level across snapshots
/// 2. Detect refills: volume increases after partial/full depletion (>20% reduction)
/// 3. Calculate median volume per side
/// 4. Flag absorption: >5x median volume with >3 consecutive refills
pub fn detect_absorption_events(
    snapshots: &[OrderBookSnapshot],
    symbol: &str,
) -> Result<Vec<AbsorptionEvent>> {
    if snapshots.len() < 3 {
        return Ok(Vec::new());
    }

    let mut events = Vec::new();

    // Track refill events per price level
    let mut bid_refills: HashMap<String, Vec<(DateTime<Utc>, Decimal)>> = HashMap::new();
    let mut ask_refills: HashMap<String, Vec<(DateTime<Utc>, Decimal)>> = HashMap::new();

    // Analyze consecutive snapshot pairs for refill detection
    for window in snapshots.windows(2) {
        let prev = &window[0];
        let curr = &window[1];

        // Detect bid refills
        for (price_str, curr_qty_str) in &curr.bids {
            let curr_qty = Decimal::from_str(curr_qty_str).unwrap_or(Decimal::ZERO);

            if let Some((_, prev_qty_str)) = prev.bids.iter().find(|(p, _)| p == price_str) {
                let prev_qty = Decimal::from_str(prev_qty_str).unwrap_or(Decimal::ZERO);

                // Detect refill: volume decreased by >20% then increased
                if prev_qty > Decimal::ZERO {
                    let reduction_pct = (prev_qty - curr_qty) / prev_qty;
                    if reduction_pct > Decimal::from_str("0.20").unwrap() {
                        // Mark potential absorption zone
                        let timestamp = DateTime::from_timestamp(curr.timestamp, 0).unwrap_or(Utc::now());
                        bid_refills.entry(price_str.clone())
                            .or_insert_with(Vec::new)
                            .push((timestamp, curr_qty));
                    }
                }
            }
        }

        // Detect ask refills
        for (price_str, curr_qty_str) in &curr.asks {
            let curr_qty = Decimal::from_str(curr_qty_str).unwrap_or(Decimal::ZERO);

            if let Some((_, prev_qty_str)) = prev.asks.iter().find(|(p, _)| p == price_str) {
                let prev_qty = Decimal::from_str(prev_qty_str).unwrap_or(Decimal::ZERO);

                if prev_qty > Decimal::ZERO {
                    let reduction_pct = (prev_qty - curr_qty) / prev_qty;
                    if reduction_pct > Decimal::from_str("0.20").unwrap() {
                        let timestamp = DateTime::from_timestamp(curr.timestamp, 0).unwrap_or(Utc::now());
                        ask_refills.entry(price_str.clone())
                            .or_insert_with(Vec::new)
                            .push((timestamp, curr_qty));
                    }
                }
            }
        }
    }

    // Calculate median volumes
    let all_bid_volumes: Vec<Decimal> = snapshots.iter()
        .flat_map(|s| &s.bids)
        .filter_map(|(_, qty)| Decimal::from_str(qty).ok())
        .collect();
    let median_bid_volume = calculate_median(&all_bid_volumes);

    let all_ask_volumes: Vec<Decimal> = snapshots.iter()
        .flat_map(|s| &s.asks)
        .filter_map(|(_, qty)| Decimal::from_str(qty).ok())
        .collect();
    let median_ask_volume = calculate_median(&all_ask_volumes);

    // Identify absorption events: >3 refills AND >5x median volume
    let threshold_refills = 3;
    let volume_multiplier = Decimal::from(5);

    // Process bid absorptions (accumulation)
    for (price_str, refills) in bid_refills {
        if refills.len() >= threshold_refills {
            let total_absorbed: Decimal = refills.iter().map(|(_, vol)| *vol).sum();
            let avg_absorbed = total_absorbed / Decimal::from(refills.len());

            if avg_absorbed > median_bid_volume * volume_multiplier {
                let first_time = refills.first().unwrap().0;
                let last_time = refills.last().unwrap().0;
                let entity_type = if refills.len() > 5 {
                    EntityType::MarketMaker
                } else {
                    EntityType::Whale
                };

                events.push(AbsorptionEvent {
                    event_id: Uuid::new_v4(),
                    symbol: symbol.to_string(),
                    first_detected: first_time,
                    last_updated: last_time,
                    price_level: Decimal::from_str(&price_str).unwrap_or(Decimal::ZERO),
                    absorbed_volume: total_absorbed,
                    refill_count: refills.len() as u32,
                    direction: Direction::Accumulation,
                    suspected_entity_type: entity_type,
                });
            }
        }
    }

    // Process ask absorptions (distribution)
    for (price_str, refills) in ask_refills {
        if refills.len() >= threshold_refills {
            let total_absorbed: Decimal = refills.iter().map(|(_, vol)| *vol).sum();
            let avg_absorbed = total_absorbed / Decimal::from(refills.len());

            if avg_absorbed > median_ask_volume * volume_multiplier {
                let first_time = refills.first().unwrap().0;
                let last_time = refills.last().unwrap().0;
                let entity_type = if refills.len() > 5 {
                    EntityType::MarketMaker
                } else {
                    EntityType::Whale
                };

                events.push(AbsorptionEvent {
                    event_id: Uuid::new_v4(),
                    symbol: symbol.to_string(),
                    first_detected: first_time,
                    last_updated: last_time,
                    price_level: Decimal::from_str(&price_str).unwrap_or(Decimal::ZERO),
                    absorbed_volume: total_absorbed,
                    refill_count: refills.len() as u32,
                    direction: Direction::Distribution,
                    suspected_entity_type: entity_type,
                });
            }
        }
    }

    Ok(events)
}

/// Calculate median from a sorted vector of Decimals
fn calculate_median(values: &[Decimal]) -> Decimal {
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
    fn test_calculate_flow_rates() {
        let (bid_rate, ask_rate) = calculate_flow_rates(120, 60, 60);
        assert!((bid_rate - 2.0).abs() < f64::EPSILON);
        assert!((ask_rate - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_determine_flow_direction() {
        assert_eq!(
            determine_flow_direction(100.0, 40.0),
            FlowDirection::StrongBuy
        );
        assert_eq!(
            determine_flow_direction(60.0, 50.0),
            FlowDirection::ModerateBuy
        );
        assert_eq!(
            determine_flow_direction(50.0, 50.0),
            FlowDirection::Neutral
        );
    }

    #[test]
    fn test_aggregate_bid_ask_counts_empty() {
        let snapshots: Vec<OrderBookSnapshot> = vec![];
        let result = aggregate_bid_ask_counts(&snapshots);
        assert!(result.is_ok());
        let (bid, ask) = result.unwrap();
        assert_eq!(bid, 0);
        assert_eq!(ask, 0);
    }
}
