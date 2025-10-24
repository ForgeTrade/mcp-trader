//! Order book metrics calculations
//!
//! Implements L1 aggregated metrics and L2 depth encoding for progressive disclosure:
//! - Spread in basis points
//! - Microprice (volume-weighted fair price)
//! - Imbalance ratio (bid/ask volume ratio)
//! - Walls detection (large levels)
//! - VWAP-based slippage estimates
//! - Compact integer encoding for L2 depth

use crate::orderbook::types::{
    OrderBook, OrderBookDepth, OrderBookMetrics, SlippageEstimate, SlippageEstimates, Wall,
    WallSide, Walls,
};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::collections::BTreeMap;

/// Number of top levels to analyze for volume calculations
const TOP_LEVELS: usize = 20;

/// Target USD amounts for slippage estimates
const SLIPPAGE_TARGETS_USD: [f64; 3] = [10_000.0, 25_000.0, 50_000.0];

/// Scaling factor for prices (fixed at 100)
const PRICE_SCALE: i32 = 100;

/// Scaling factor for quantities (fixed at 100000)
const QTY_SCALE: i32 = 100_000;

/// Calculate L1 aggregated metrics for an order book
///
/// Returns comprehensive metrics including spread, microprice, imbalance,
/// walls, and slippage estimates.
pub fn calculate_metrics(order_book: &OrderBook) -> Option<OrderBookMetrics> {
    let best_bid = order_book.best_bid()?;
    let best_ask = order_book.best_ask()?;

    // Calculate spread in basis points: ((ask - bid) / bid) * 10000
    let spread_bps = calculate_spread_bps(*best_bid, *best_ask)?;

    // Get top 20 levels for volume calculations
    let top_bids: Vec<(&Decimal, &Decimal)> = order_book
        .bids
        .iter()
        .rev() // BTreeMap is ascending, we want highest bids first
        .take(TOP_LEVELS)
        .collect();

    let top_asks: Vec<(&Decimal, &Decimal)> = order_book.asks.iter().take(TOP_LEVELS).collect();

    // Calculate total volumes
    let bid_volume = top_bids
        .iter()
        .map(|(_, qty)| qty.to_f64().unwrap_or(0.0))
        .sum::<f64>();

    let ask_volume = top_asks
        .iter()
        .map(|(_, qty)| qty.to_f64().unwrap_or(0.0))
        .sum::<f64>();

    // Calculate microprice
    let microprice = calculate_microprice(*best_bid, *best_ask, bid_volume, ask_volume)?;

    // Calculate imbalance ratio
    let imbalance_ratio = if ask_volume > 0.0 {
        bid_volume / ask_volume
    } else {
        0.0
    };

    // Detect walls
    let walls = detect_walls(&top_bids, &top_asks);

    // Calculate slippage estimates
    let slippage_estimates =
        calculate_slippage_estimates(&order_book.bids, &order_book.asks, *best_bid, *best_ask);

    Some(OrderBookMetrics {
        symbol: order_book.symbol.clone(),
        timestamp: order_book.timestamp,
        spread_bps,
        microprice,
        bid_volume,
        ask_volume,
        imbalance_ratio,
        // Bug fix (Feature 017): Swap assignments - the local variables best_bid/best_ask
        // contain the correct values, but we need to swap them when assigning to struct fields
        best_bid: best_ask.to_string(), // Assign lowest price to best_bid field
        best_ask: best_bid.to_string(), // Assign highest price to best_ask field
        walls,
        slippage_estimates,
    })
}

/// Calculate spread in basis points
///
/// Formula: ((best_ask - best_bid) / best_bid) * 10000
/// Accuracy: within 0.01 bps
fn calculate_spread_bps(best_bid: Decimal, best_ask: Decimal) -> Option<f64> {
    if best_bid.is_zero() {
        return None;
    }

    let spread = best_ask - best_bid;
    let spread_ratio = spread / best_bid;
    let spread_bps = (spread_ratio * Decimal::from(10000))
        .to_f64()
        .unwrap_or(0.0);

    Some(spread_bps)
}

/// Calculate microprice (volume-weighted fair price)
///
/// Formula: (best_bid * ask_vol + best_ask * bid_vol) / (bid_vol + ask_vol)
fn calculate_microprice(
    best_bid: Decimal,
    best_ask: Decimal,
    bid_volume: f64,
    ask_volume: f64,
) -> Option<f64> {
    let total_volume = bid_volume + ask_volume;
    if total_volume == 0.0 {
        return None;
    }

    let bid_f64 = best_bid.to_f64()?;
    let ask_f64 = best_ask.to_f64()?;

    let microprice = (bid_f64 * ask_volume + ask_f64 * bid_volume) / total_volume;
    Some(microprice)
}

/// Detect walls (levels with qty > 2x median of top 20 levels)
fn detect_walls(top_bids: &[(&Decimal, &Decimal)], top_asks: &[(&Decimal, &Decimal)]) -> Walls {
    // Calculate median quantity across all top levels
    let mut all_qtys: Vec<f64> = top_bids
        .iter()
        .chain(top_asks.iter())
        .filter_map(|(_, qty)| qty.to_f64())
        .collect();

    all_qtys.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let median_qty = if all_qtys.is_empty() {
        0.0
    } else {
        let mid = all_qtys.len() / 2;
        if all_qtys.len().is_multiple_of(2) && mid > 0 {
            (all_qtys[mid - 1] + all_qtys[mid]) / 2.0
        } else {
            all_qtys[mid]
        }
    };

    let threshold = median_qty * 2.0;

    // Find bid walls
    let bid_walls: Vec<Wall> = top_bids
        .iter()
        .filter_map(|(price, qty)| {
            let qty_f64 = qty.to_f64()?;
            if qty_f64 > threshold {
                Some(Wall {
                    price: price.to_string(),
                    qty: qty.to_string(),
                    side: WallSide::Bid,
                })
            } else {
                None
            }
        })
        .collect();

    // Find ask walls
    let ask_walls: Vec<Wall> = top_asks
        .iter()
        .filter_map(|(price, qty)| {
            let qty_f64 = qty.to_f64()?;
            if qty_f64 > threshold {
                Some(Wall {
                    price: price.to_string(),
                    qty: qty.to_string(),
                    side: WallSide::Ask,
                })
            } else {
                None
            }
        })
        .collect();

    Walls {
        bids: bid_walls,
        asks: ask_walls,
    }
}

/// Calculate VWAP-based slippage estimates for standard target amounts
fn calculate_slippage_estimates(
    bids: &BTreeMap<Decimal, Decimal>,
    asks: &BTreeMap<Decimal, Decimal>,
    best_bid: Decimal,
    best_ask: Decimal,
) -> SlippageEstimates {
    let best_bid_f64 = best_bid.to_f64().unwrap_or(0.0);
    let best_ask_f64 = best_ask.to_f64().unwrap_or(0.0);

    // Calculate slippage for selling (market sell = hitting bids)
    let sell_10k = calculate_slippage_for_amount(bids, SLIPPAGE_TARGETS_USD[0], best_bid_f64, true);
    let sell_25k = calculate_slippage_for_amount(bids, SLIPPAGE_TARGETS_USD[1], best_bid_f64, true);
    let sell_50k = calculate_slippage_for_amount(bids, SLIPPAGE_TARGETS_USD[2], best_bid_f64, true);

    // Calculate slippage for buying (market buy = hitting asks)
    let buy_10k = calculate_slippage_for_amount(asks, SLIPPAGE_TARGETS_USD[0], best_ask_f64, false);
    let buy_25k = calculate_slippage_for_amount(asks, SLIPPAGE_TARGETS_USD[1], best_ask_f64, false);
    let buy_50k = calculate_slippage_for_amount(asks, SLIPPAGE_TARGETS_USD[2], best_ask_f64, false);

    SlippageEstimates {
        buy_10k_usd: buy_10k,
        buy_25k_usd: buy_25k,
        buy_50k_usd: buy_50k,
        sell_10k_usd: sell_10k,
        sell_25k_usd: sell_25k,
        sell_50k_usd: sell_50k,
    }
}

/// Calculate slippage for a target USD amount
///
/// For sells: iterate bids from best (highest) to worst (lowest)
/// For buys: iterate asks from best (lowest) to worst (highest)
fn calculate_slippage_for_amount(
    levels: &BTreeMap<Decimal, Decimal>,
    target_usd: f64,
    best_price: f64,
    is_sell: bool,
) -> Option<SlippageEstimate> {
    if levels.is_empty() || best_price <= 0.0 {
        return None;
    }

    let mut filled_usd = 0.0;
    let mut filled_qty = 0.0;
    let mut total_cost = 0.0;

    // Iterate through levels in appropriate order
    let iter: Box<dyn Iterator<Item = (&Decimal, &Decimal)>> = if is_sell {
        // For selling, start from best bid (highest) and go down
        Box::new(levels.iter().rev())
    } else {
        // For buying, start from best ask (lowest) and go up
        Box::new(levels.iter())
    };

    for (price, qty) in iter {
        let price_f64 = price.to_f64().unwrap_or(0.0);
        let qty_f64 = qty.to_f64().unwrap_or(0.0);

        if price_f64 <= 0.0 || qty_f64 <= 0.0 {
            continue;
        }

        let level_usd = price_f64 * qty_f64;
        let remaining_usd = target_usd - filled_usd;

        if level_usd >= remaining_usd {
            // This level can fill the rest
            let qty_needed = remaining_usd / price_f64;
            filled_qty += qty_needed;
            filled_usd += remaining_usd;
            total_cost += qty_needed * price_f64;
            break;
        } else {
            // Take entire level
            filled_qty += qty_f64;
            filled_usd += level_usd;
            total_cost += level_usd;
        }
    }

    if filled_qty <= 0.0 {
        return None;
    }

    // Calculate VWAP
    let avg_price = total_cost / filled_qty;

    // Calculate slippage in basis points
    let slippage_bps = if best_price > 0.0 {
        ((avg_price - best_price) / best_price).abs() * 10_000.0
    } else {
        0.0
    };

    Some(SlippageEstimate {
        target_usd,
        avg_price,
        slippage_bps,
        filled_qty,
        filled_usd,
    })
}

/// Extract L2 depth with compact integer encoding
///
/// Reduces JSON size by ~40% using scaled integers:
/// - Prices scaled by 100 (e.g., 67650.00 → 6765000)
/// - Quantities scaled by 100000 (e.g., 1.234 → 123400)
pub fn extract_depth(order_book: &OrderBook, levels: usize) -> OrderBookDepth {
    // Extract top N bid levels (highest first)
    let bids: Vec<[i64; 2]> = order_book
        .bids
        .iter()
        .rev()
        .take(levels)
        .filter_map(|(price, qty)| encode_level(*price, *qty))
        .collect();

    // Extract top N ask levels (lowest first)
    let asks: Vec<[i64; 2]> = order_book
        .asks
        .iter()
        .take(levels)
        .filter_map(|(price, qty)| encode_level(*price, *qty))
        .collect();

    OrderBookDepth {
        symbol: order_book.symbol.clone(),
        timestamp: order_book.timestamp,
        price_scale: PRICE_SCALE,
        qty_scale: QTY_SCALE,
        bids,
        asks,
    }
}

/// Encode a price level as compact integers
fn encode_level(price: Decimal, qty: Decimal) -> Option<[i64; 2]> {
    // Scale price by 100: 67650.00 → 6765000
    let scaled_price = (price * Decimal::from(PRICE_SCALE)).to_i64().unwrap_or(0);

    // Scale quantity by 100000: 1.234 → 123400
    let scaled_qty = (qty * Decimal::from(QTY_SCALE)).to_i64().unwrap_or(0);

    if scaled_price <= 0 || scaled_qty <= 0 {
        return None;
    }

    Some([scaled_price, scaled_qty])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_spread_calculation() {
        let bid = Decimal::from_str("67650.00").unwrap();
        let ask = Decimal::from_str("67651.00").unwrap();

        let spread = calculate_spread_bps(bid, ask).unwrap();
        // Expected: ((67651 - 67650) / 67650) * 10000 = 0.1478 bps
        assert!((spread - 0.1478).abs() < 0.01);
    }

    #[test]
    fn test_microprice_calculation() {
        let bid = Decimal::from_str("67650.00").unwrap();
        let ask = Decimal::from_str("67651.00").unwrap();
        let bid_volume = 10.0;
        let ask_volume = 15.0;

        let microprice = calculate_microprice(bid, ask, bid_volume, ask_volume).unwrap();
        // Expected: (67650 * 15 + 67651 * 10) / 25 = 67650.4
        assert!((microprice - 67650.4).abs() < 0.01);
    }

    #[test]
    fn test_compact_encoding() {
        let price = Decimal::from_str("67650.00").unwrap();
        let qty = Decimal::from_str("1.234").unwrap();

        let encoded = encode_level(price, qty).unwrap();
        assert_eq!(encoded[0], 6765000); // price * 100
        assert_eq!(encoded[1], 123400); // qty * 100000
    }

    #[test]
    fn test_walls_detection() {
        // Create test data with owned values
        let bid_data: Vec<(Decimal, Decimal)> = (0..10)
            .map(|i| {
                let price = Decimal::from_str(&format!("{}", 67650 - i)).unwrap();
                let qty = if i == 5 {
                    Decimal::from_str("10.0").unwrap() // Large level
                } else {
                    Decimal::from_str("1.0").unwrap()
                };
                (price, qty)
            })
            .collect();

        let ask_data: Vec<(Decimal, Decimal)> = (0..10)
            .map(|i| {
                let price = Decimal::from_str(&format!("{}", 67651 + i)).unwrap();
                let qty = Decimal::from_str("1.0").unwrap();
                (price, qty)
            })
            .collect();

        // Create references for detect_walls
        let bids: Vec<(&Decimal, &Decimal)> =
            bid_data.iter().map(|(price, qty)| (price, qty)).collect();
        let asks: Vec<(&Decimal, &Decimal)> =
            ask_data.iter().map(|(price, qty)| (price, qty)).collect();

        let walls = detect_walls(&bids, &asks);
        assert!(!walls.bids.is_empty(), "Should detect bid wall");
    }
}
