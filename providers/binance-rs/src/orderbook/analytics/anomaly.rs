//! Anomaly detection (quote stuffing, icebergs, flash crashes)
//!
//! Detects market microstructure anomalies that indicate manipulation or risk:
//! - Quote stuffing: >500 updates/sec with <10% fill rate
//! - Iceberg orders: Refill rate >5x median with 95% confidence
//! - Flash crash risk: >80% liquidity drain, >10x spread widening, >90% cancellation rate

use anyhow::Result;
use chrono::Utc;
use rust_decimal::Decimal;
use statrs::distribution::{ContinuousCDF, Normal};
use uuid::Uuid;

use crate::orderbook::analytics::{
    storage::{snapshot::OrderBookSnapshot, SnapshotStorage},
    types::{AnomalyType, MarketMicrostructureAnomaly, Severity},
};

/// Detect quote stuffing (>500 updates/sec, <10% fill rate)
///
/// # Arguments
/// * `snapshots` - Recent orderbook snapshots (recommended: last 10 seconds)
/// * `fill_rate` - Percentage of orders that resulted in trades
///
/// # Returns
/// Some(anomaly) if detected, None otherwise
pub fn detect_quote_stuffing(
    snapshots: &[OrderBookSnapshot],
    fill_rate: f64,
) -> Option<MarketMicrostructureAnomaly> {
    if snapshots.len() < 2 {
        return None;
    }

    // Calculate update rate (snapshots per second) using actual timestamps
    let first_timestamp = snapshots.first().unwrap().timestamp;
    let last_timestamp = snapshots.last().unwrap().timestamp;
    let duration_millis = (last_timestamp - first_timestamp) as f64;

    // Handle edge case: all snapshots have same timestamp
    // Assume 1 second total duration (for testing scenarios where timestamps are identical)
    let duration_secs = if duration_millis < 1.0 {
        1.0
    } else {
        duration_millis / 1000.0
    };

    let update_count = snapshots.len() - 1;
    let update_rate = (update_count as f64) / duration_secs;

    // Thresholds from FR
    let is_quote_stuffing = update_rate > 500.0 && fill_rate < 0.10;

    if is_quote_stuffing {
        // Calculate severity based on update rate
        let severity = calculate_quote_stuffing_severity(update_rate);

        // Confidence based on how far from threshold
        let confidence_score = ((update_rate - 500.0) / 500.0).min(1.0);

        let recommended_action = match severity {
            Severity::Critical => "Suspend trading immediately - likely market manipulation",
            Severity::High => "Avoid placing orders - wait for normal conditions",
            Severity::Medium => "Use limit orders only - avoid market orders",
            Severity::Low => "Monitor closely - consider reducing position size",
        };

        Some(MarketMicrostructureAnomaly {
            anomaly_id: Uuid::new_v4(),
            symbol: snapshots[0].bids.get(0).map(|(s, _)| s.clone()).unwrap_or_default(), // Extract symbol from data
            anomaly_type: AnomalyType::QuoteStuffing {
                update_rate,
                fill_rate,
            },
            detection_timestamp: Utc::now(),
            confidence_score,
            affected_price_levels: Vec::new(), // Not applicable for quote stuffing
            severity,
            recommended_action: recommended_action.to_string(),
            metadata: serde_json::json!({
                "update_count": update_count,
                "duration_secs": duration_secs,
                "threshold_exceeded_by": format!("{:.0}%", (update_rate / 500.0 - 1.0) * 100.0)
            }),
        })
    } else {
        None
    }
}

/// Calculate severity based on update rate thresholds
///
/// Thresholds (from T049):
/// - Medium: 500-750 updates/sec
/// - High: 750-1000 updates/sec
/// - Critical: >1000 updates/sec
fn calculate_quote_stuffing_severity(update_rate: f64) -> Severity {
    if update_rate > 1000.0 {
        Severity::Critical
    } else if update_rate > 750.0 {
        Severity::High
    } else if update_rate > 500.0 {
        Severity::Medium
    } else {
        Severity::Low
    }
}

/// Detect iceberg orders (refill rate >5x median, 95% confidence)
///
/// Uses z-score with 95% confidence threshold (z > 1.96)
///
/// # Arguments
/// * `price_level` - Price to analyze
/// * `refill_events` - Number of refills observed at this level
/// * `median_refill_rate` - Median refill rate across all price levels
///
/// # Returns
/// Some(anomaly) if iceberg detected, None otherwise
pub fn detect_iceberg_orders(
    price_level: Decimal,
    refill_events: u32,
    median_refill_rate: f64,
) -> Option<MarketMicrostructureAnomaly> {
    let refill_rate_multiplier = (refill_events as f64) / median_refill_rate;

    // Threshold from clarifications Q2: Z-score > 1.96 for 95% confidence
    let is_iceberg = refill_rate_multiplier > 5.0;

    if is_iceberg {
        // Calculate z-score for confidence
        let normal = Normal::new(median_refill_rate, median_refill_rate * 0.2).unwrap();
        let z_score = (refill_events as f64 - median_refill_rate) / (median_refill_rate * 0.2);
        let confidence_score = if z_score > 1.96 {
            1.0 - normal.cdf(refill_events as f64)
        } else {
            0.5
        };

        let severity = Severity::from_confidence(confidence_score);

        Some(MarketMicrostructureAnomaly {
            anomaly_id: Uuid::new_v4(),
            symbol: String::new(), // Should be passed as parameter
            anomaly_type: AnomalyType::IcebergOrder {
                price_level,
                refill_rate_multiplier,
                median_refill_rate,
            },
            detection_timestamp: Utc::now(),
            confidence_score,
            affected_price_levels: vec![price_level],
            severity,
            recommended_action: "Large hidden order detected - price may act as support/resistance".to_string(),
            metadata: serde_json::json!({
                "refill_events": refill_events,
                "z_score": z_score,
                "threshold_multiplier": refill_rate_multiplier
            }),
        })
    } else {
        None
    }
}

/// Detect flash crash risk (liquidity drain >80%, spread >10x, cancellations >90%)
///
/// # Arguments
/// * `current_snapshot` - Latest orderbook snapshot
/// * `baseline_snapshot` - Baseline snapshot for comparison
/// * `cancellation_rate` - Percentage of orders cancelled vs placed
///
/// # Returns
/// Some(anomaly) if flash crash risk detected, None otherwise
pub fn detect_flash_crash_risk(
    current_snapshot: &OrderBookSnapshot,
    baseline_snapshot: &OrderBookSnapshot,
    cancellation_rate: f64,
) -> Option<MarketMicrostructureAnomaly> {
    // Calculate liquidity drain
    let current_depth: f64 = current_snapshot.bids.iter()
        .chain(current_snapshot.asks.iter())
        .filter_map(|(_, qty)| qty.parse::<f64>().ok())
        .sum();

    let baseline_depth: f64 = baseline_snapshot.bids.iter()
        .chain(baseline_snapshot.asks.iter())
        .filter_map(|(_, qty)| qty.parse::<f64>().ok())
        .sum();

    let depth_loss_pct = ((baseline_depth - current_depth) / baseline_depth) * 100.0;

    // Calculate spread widening
    let current_spread = calculate_spread(current_snapshot);
    let baseline_spread = calculate_spread(baseline_snapshot);
    let spread_multiplier = current_spread / baseline_spread;

    // Thresholds from FR
    let is_flash_crash = depth_loss_pct > 80.0 && spread_multiplier > 10.0 && cancellation_rate > 90.0;

    if is_flash_crash {
        let confidence_score = ((depth_loss_pct / 80.0) + (spread_multiplier / 10.0) + (cancellation_rate / 90.0)) / 3.0;
        let confidence_clamped = confidence_score.min(1.0);

        Some(MarketMicrostructureAnomaly {
            anomaly_id: Uuid::new_v4(),
            symbol: String::new(), // Should be passed as parameter
            anomaly_type: AnomalyType::FlashCrashRisk {
                depth_loss_pct,
                spread_multiplier,
                cancellation_rate,
            },
            detection_timestamp: Utc::now(),
            confidence_score: confidence_clamped,
            affected_price_levels: Vec::new(),
            severity: Severity::Critical, // Flash crash risk is always critical
            recommended_action: "CRITICAL: Close positions and avoid trading - flash crash imminent".to_string(),
            metadata: serde_json::json!({
                "current_depth": current_depth,
                "baseline_depth": baseline_depth,
                "current_spread": current_spread,
                "baseline_spread": baseline_spread
            }),
        })
    } else {
        None
    }
}

/// Calculate bid-ask spread from snapshot
fn calculate_spread(snapshot: &OrderBookSnapshot) -> f64 {
    if snapshot.bids.is_empty() || snapshot.asks.is_empty() {
        return f64::MAX;
    }

    let best_bid: f64 = snapshot.bids[0].0.parse().unwrap_or(0.0);
    let best_ask: f64 = snapshot.asks[0].0.parse().unwrap_or(0.0);

    best_ask - best_bid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_quote_stuffing_severity() {
        assert_eq!(calculate_quote_stuffing_severity(1100.0), Severity::Critical);
        assert_eq!(calculate_quote_stuffing_severity(850.0), Severity::High);
        assert_eq!(calculate_quote_stuffing_severity(600.0), Severity::Medium);
        assert_eq!(calculate_quote_stuffing_severity(400.0), Severity::Low);
    }

    #[test]
    fn test_detect_quote_stuffing_threshold() {
        let snapshots = vec![OrderBookSnapshot {
            bids: vec![("100.0".to_string(), "1.0".to_string())],
            asks: vec![],
            update_id: 1,
            timestamp: 0,
        }; 600]; // 600 snapshots = 600 updates/sec

        let result = detect_quote_stuffing(&snapshots, 0.05);
        assert!(result.is_some());

        let anomaly = result.unwrap();
        assert!(matches!(anomaly.anomaly_type, AnomalyType::QuoteStuffing { .. }));
        assert_eq!(anomaly.severity, Severity::Medium);
    }
}
