//! Microstructure health scoring
//!
//! Composite 0-100 health score combining:
//! - Spread stability (25% weight)
//! - Liquidity depth (35% weight)
//! - Flow balance (25% weight)
//! - Update rate (15% weight)

use anyhow::Result;
use chrono::Utc;

use crate::orderbook::analytics::{
    storage::snapshot::OrderBookSnapshot, types::MicrostructureHealth,
};

/// Calculate market microstructure health score
///
/// # Arguments
/// * `symbol` - Trading pair
/// * `snapshots` - Recent orderbook snapshots (recommended: last 60 seconds)
/// * `bid_flow_rate` - Bid orders per second
/// * `ask_flow_rate` - Ask orders per second
///
/// # Returns
/// MicrostructureHealth with composite score and component breakdowns
pub fn calculate_microstructure_health(
    symbol: &str,
    snapshots: &[OrderBookSnapshot],
    bid_flow_rate: f64,
    ask_flow_rate: f64,
) -> Result<MicrostructureHealth> {
    anyhow::ensure!(
        !snapshots.is_empty(),
        "Need at least one snapshot for health calculation"
    );

    // Component scoring (weights from FR-012, T051)
    let spread_stability_score = calculate_spread_stability_score(snapshots);
    let liquidity_depth_score = calculate_liquidity_depth_score(snapshots);
    let flow_balance_score = calculate_flow_balance_score(bid_flow_rate, ask_flow_rate);
    let update_rate_score = calculate_update_rate_score(snapshots);

    // Weighted average (from T051):
    // - spread_stability: 25%
    // - liquidity_depth: 35%
    // - flow_balance: 25%
    // - update_rate: 15%
    let overall_score = (spread_stability_score * 0.25)
        + (liquidity_depth_score * 0.35)
        + (flow_balance_score * 0.25)
        + (update_rate_score * 0.15);

    // Classify health level
    let health_level = classify_health_level(overall_score);

    // Generate recommendation
    let recommended_action = generate_recommendation(overall_score, &health_level);

    Ok(MicrostructureHealth {
        symbol: symbol.to_string(),
        timestamp: Utc::now(),
        overall_score,
        spread_stability_score,
        liquidity_depth_score,
        flow_balance_score,
        update_rate_score,
        health_level,
        recommended_action,
    })
}

/// Calculate spread stability score (0-100)
///
/// Measures coefficient of variation of spread over time window.
/// Lower CV = higher stability = higher score
fn calculate_spread_stability_score(snapshots: &[OrderBookSnapshot]) -> f64 {
    if snapshots.len() < 2 {
        return 50.0; // Neutral score
    }

    let spreads: Vec<f64> = snapshots
        .iter()
        .filter_map(|s| {
            if s.bids.is_empty() || s.asks.is_empty() {
                return None;
            }
            let best_bid: f64 = s.bids[0].0.parse().ok()?;
            let best_ask: f64 = s.asks[0].0.parse().ok()?;
            Some(best_ask - best_bid)
        })
        .collect();

    if spreads.is_empty() {
        return 0.0;
    }

    let mean: f64 = spreads.iter().sum::<f64>() / spreads.len() as f64;
    let variance: f64 =
        spreads.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / spreads.len() as f64;
    let std_dev = variance.sqrt();

    let cv = if mean > 0.0 {
        (std_dev / mean) * 100.0
    } else {
        100.0
    };

    // Convert CV to score: lower CV = higher score
    // CV < 5% = 100, CV > 50% = 0
    (100.0 - cv.min(50.0) * 2.0).max(0.0)
}

/// Calculate liquidity depth score (0-100)
///
/// Measures total depth (bid + ask volume) relative to historical average.
/// Higher depth = higher score
fn calculate_liquidity_depth_score(snapshots: &[OrderBookSnapshot]) -> f64 {
    if snapshots.is_empty() {
        return 0.0;
    }

    let latest = &snapshots[snapshots.len() - 1];

    let current_depth: f64 = latest
        .bids
        .iter()
        .chain(latest.asks.iter())
        .filter_map(|(_, qty)| qty.parse::<f64>().ok())
        .sum();

    // Calculate historical average depth
    let avg_depth: f64 = snapshots
        .iter()
        .map(|s| {
            s.bids
                .iter()
                .chain(s.asks.iter())
                .filter_map(|(_, qty)| qty.parse::<f64>().ok())
                .sum::<f64>()
        })
        .sum::<f64>()
        / snapshots.len() as f64;

    if avg_depth == 0.0 {
        return 0.0;
    }

    // Score based on ratio to average
    // depth = avg → 50, depth = 2x avg → 100, depth = 0.5x avg → 0
    let ratio = current_depth / avg_depth;
    (ratio * 50.0).min(100.0).max(0.0)
}

/// Calculate flow balance score (0-100)
///
/// Measures bid/ask flow rate balance. Balanced flow = high score.
/// Imbalance indicates one-sided pressure.
fn calculate_flow_balance_score(bid_flow_rate: f64, ask_flow_rate: f64) -> f64 {
    if bid_flow_rate == 0.0 && ask_flow_rate == 0.0 {
        return 50.0; // Neutral if no flow
    }

    let total_flow = bid_flow_rate + ask_flow_rate;
    if total_flow == 0.0 {
        return 50.0;
    }

    let bid_ratio = bid_flow_rate / total_flow;
    let ask_ratio = ask_flow_rate / total_flow;

    // Perfect balance (50/50) = 100 score
    // Complete imbalance (100/0 or 0/100) = 0 score
    let imbalance = (bid_ratio - 0.5).abs() * 2.0; // 0 = balanced, 1 = completely imbalanced
    ((1.0 - imbalance) * 100.0).max(0.0)
}

/// Calculate update rate score (0-100)
///
/// Measures orderbook update frequency. Moderate updates = healthy.
/// Too few updates = stale, too many = potential manipulation.
fn calculate_update_rate_score(snapshots: &[OrderBookSnapshot]) -> f64 {
    if snapshots.len() < 2 {
        return 50.0;
    }

    // Assuming 1-second intervals
    let update_rate = snapshots.len() as f64;

    // Optimal range: 10-100 updates/sec
    // Below 10 = stale (linearly decrease to 0 at 0 updates)
    // 10-100 = healthy (score 100)
    // Above 100 = potential manipulation (linearly decrease to 0 at 500)
    if update_rate < 10.0 {
        (update_rate / 10.0) * 100.0
    } else if update_rate <= 100.0 {
        100.0
    } else if update_rate <= 500.0 {
        100.0 - ((update_rate - 100.0) / 400.0) * 100.0
    } else {
        0.0
    }
}

/// Classify health level based on overall score
fn classify_health_level(score: f64) -> String {
    match score {
        s if s >= 80.0 => "Excellent".to_string(),
        s if s >= 60.0 => "Good".to_string(),
        s if s >= 40.0 => "Fair".to_string(),
        s if s >= 20.0 => "Poor".to_string(),
        _ => "Critical".to_string(),
    }
}

/// Generate trading recommendation based on health score
fn generate_recommendation(score: f64, level: &str) -> String {
    match level {
        "Excellent" => "Market conditions optimal - safe to execute large orders",
        "Good" => "Market conditions healthy - normal trading recommended",
        "Fair" => "Market conditions acceptable - use limit orders and monitor closely",
        "Poor" => "Market conditions degraded - reduce position sizes and avoid market orders",
        "Critical" => "Market conditions unhealthy - avoid trading until conditions improve",
        _ => "Unknown health level",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_health_level() {
        assert_eq!(classify_health_level(95.0), "Excellent");
        assert_eq!(classify_health_level(70.0), "Good");
        assert_eq!(classify_health_level(50.0), "Fair");
        assert_eq!(classify_health_level(30.0), "Poor");
        assert_eq!(classify_health_level(10.0), "Critical");
    }

    #[test]
    fn test_calculate_flow_balance_score() {
        assert_eq!(calculate_flow_balance_score(50.0, 50.0), 100.0); // Perfect balance
        assert!((calculate_flow_balance_score(70.0, 30.0) - 60.0).abs() < 1.0); // 70/30 imbalance
        assert_eq!(calculate_flow_balance_score(100.0, 0.0), 0.0); // Complete imbalance
    }

    #[test]
    fn test_calculate_update_rate_score() {
        let snapshots_low = vec![
            OrderBookSnapshot {
                bids: vec![],
                asks: vec![],
                update_id: 1,
                timestamp: 0,
            };
            5
        ]; // 5 updates

        let snapshots_optimal = vec![
            OrderBookSnapshot {
                bids: vec![],
                asks: vec![],
                update_id: 1,
                timestamp: 0,
            };
            50
        ]; // 50 updates

        assert!(calculate_update_rate_score(&snapshots_low) < 100.0);
        assert_eq!(calculate_update_rate_score(&snapshots_optimal), 100.0);
    }
}
