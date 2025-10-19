//! Core data types for advanced order book analytics
//!
//! This module defines all entities and enums used across the analytics system.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// Supporting Enums
// ============================================================================

/// Order flow direction indicator based on bid/ask flow ratio
///
/// Thresholds:
/// - STRONG_BUY: bid_flow > 2.0 × ask_flow
/// - MODERATE_BUY: bid_flow 1.2-2.0 × ask_flow
/// - NEUTRAL: bid_flow ≈ ask_flow (0.8-1.2 ratio)
/// - MODERATE_SELL: ask_flow 1.2-2.0 × bid_flow
/// - STRONG_SELL: ask_flow > 2.0 × bid_flow
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FlowDirection {
    #[serde(rename = "STRONG_BUY")]
    StrongBuy,
    #[serde(rename = "MODERATE_BUY")]
    ModerateBuy,
    #[serde(rename = "NEUTRAL")]
    Neutral,
    #[serde(rename = "MODERATE_SELL")]
    ModerateSell,
    #[serde(rename = "STRONG_SELL")]
    StrongSell,
}

impl FlowDirection {
    /// Determine flow direction from bid/ask flow rates
    pub fn from_flow_rates(bid_flow_rate: f64, ask_flow_rate: f64) -> Self {
        if ask_flow_rate == 0.0 {
            return Self::StrongBuy;
        }
        if bid_flow_rate == 0.0 {
            return Self::StrongSell;
        }

        let ratio = bid_flow_rate / ask_flow_rate;

        if ratio > 2.0 {
            Self::StrongBuy
        } else if ratio >= 1.2 {
            Self::ModerateBuy
        } else if ratio >= 0.8 {
            Self::Neutral
        } else if ratio >= 0.5 {
            Self::ModerateSell
        } else {
            Self::StrongSell
        }
    }
}

/// Anomaly detection severity based on confidence score
///
/// Mapping:
/// - Low: Confidence 0.5-0.7
/// - Medium: Confidence 0.7-0.85
/// - High: Confidence 0.85-0.95
/// - Critical: Confidence >0.95 (triggers alert)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    /// Determine severity from confidence score
    pub fn from_confidence(confidence: f64) -> Self {
        if confidence > 0.95 {
            Self::Critical
        } else if confidence > 0.85 {
            Self::High
        } else if confidence > 0.7 {
            Self::Medium
        } else {
            Self::Low
        }
    }
}

/// Expected price movement impact through liquidity vacuum
///
/// Thresholds:
/// - FastMovement: Deficit >80%, expect >2% rapid move
/// - ModerateMovement: Deficit 50-80%, expect 1-2% move
/// - Negligible: Deficit <50%, minimal impact
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum ImpactLevel {
    FastMovement,
    ModerateMovement,
    Negligible,
}

impl ImpactLevel {
    /// Determine impact from volume deficit percentage
    pub fn from_deficit_pct(deficit_pct: f64) -> Self {
        if deficit_pct > 80.0 {
            Self::FastMovement
        } else if deficit_pct > 50.0 {
            Self::ModerateMovement
        } else {
            Self::Negligible
        }
    }
}

/// Absorption event direction (bid-side vs ask-side)
///
/// - Accumulation: Bid-side absorption (buying pressure absorbed)
/// - Distribution: Ask-side absorption (selling pressure absorbed)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Direction {
    Accumulation,
    Distribution,
}

// ============================================================================
// Core Entities
// ============================================================================

/// Order flow snapshot over configurable time window
///
/// Tracks buying/selling pressure dynamics via bid/ask order addition/cancellation rates.
/// No individual order ID tracking - aggregated counts only.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OrderFlowSnapshot {
    /// Trading pair symbol (e.g., "BTCUSDT")
    #[schemars(regex(pattern = r"^[A-Z]{4,12}$"))]
    pub symbol: String,

    /// Window start timestamp
    pub time_window_start: DateTime<Utc>,

    /// Window end timestamp (must be ≤ now)
    pub time_window_end: DateTime<Utc>,

    /// Duration in seconds (10, 30, 60, 300)
    #[schemars(range(min = 10, max = 300))]
    pub window_duration_secs: u32,

    /// Bid orders per second (≥ 0.0)
    #[schemars(range(min = 0.0))]
    pub bid_flow_rate: f64,

    /// Ask orders per second (≥ 0.0)
    #[schemars(range(min = 0.0))]
    pub ask_flow_rate: f64,

    /// Bid flow - ask flow (can be negative)
    pub net_flow: f64,

    /// Categorical pressure indicator
    pub flow_direction: FlowDirection,

    /// Running sum of (buy volume - sell volume)
    pub cumulative_delta: f64,
}

/// Volume profile histogram with POC/VAH/VAL
///
/// Shows volume distribution across price levels using adaptive tick-based binning.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VolumeProfile {
    /// Trading pair symbol
    #[schemars(regex(pattern = r"^[A-Z]{4,12}$"))]
    pub symbol: String,

    /// Analysis period start
    pub time_period_start: DateTime<Utc>,

    /// Analysis period end (must be ≤ now)
    pub time_period_end: DateTime<Utc>,

    /// Lowest price in histogram
    #[schemars(with = "String")]
    pub price_range_low: Decimal,

    /// Highest price in histogram
    #[schemars(with = "String")]
    pub price_range_high: Decimal,

    /// Price bin width (adaptive tick-based)
    #[schemars(with = "String")]
    pub bin_size: Decimal,

    /// Number of bins in histogram (1-200)
    #[schemars(range(min = 1, max = 200))]
    pub bin_count: usize,

    /// Volume distribution histogram
    pub histogram: Vec<VolumeBin>,

    /// Sum of all bin volumes
    #[schemars(with = "String")]
    pub total_volume: Decimal,

    /// Price level with highest volume (POC)
    #[schemars(with = "String")]
    pub point_of_control: Decimal,

    /// Upper boundary of value area (70% volume)
    #[schemars(with = "String")]
    pub value_area_high: Decimal,

    /// Lower boundary of value area (70% volume)
    #[schemars(with = "String")]
    pub value_area_low: Decimal,
}

/// Single bin in volume profile histogram
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VolumeBin {
    /// Center price of bin
    #[schemars(with = "String")]
    pub price_level: Decimal,

    /// Total volume traded at this level
    #[schemars(with = "String")]
    pub volume: Decimal,

    /// Number of trades in bin
    pub trade_count: u64,
}

/// Market microstructure anomaly
///
/// Detected abnormal market behavior including:
/// - Quote stuffing (>500 updates/sec, <10% fills)
/// - Iceberg orders (>5x median refill rate)
/// - Flash crash risk (>80% depth loss, >10x spread, >90% cancellation rate)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MarketMicrostructureAnomaly {
    /// Unique identifier (UUID v4)
    #[schemars(with = "String")]
    pub anomaly_id: Uuid,

    /// Trading pair symbol
    #[schemars(regex(pattern = r"^[A-Z]{4,12}$"))]
    pub symbol: String,

    /// Type of anomaly detected
    pub anomaly_type: AnomalyType,

    /// When anomaly was detected
    pub detection_timestamp: DateTime<Utc>,

    /// Detection confidence (0.0-1.0)
    #[schemars(range(min = 0.0, max = 1.0))]
    pub confidence_score: f64,

    /// Price levels involved (for iceberg/flash crash)
    #[schemars(with = "Vec<String>")]
    pub affected_price_levels: Vec<Decimal>,

    /// Severity based on confidence + type
    pub severity: Severity,

    /// Human-readable advice (1-100 chars)
    #[schemars(length(min = 1, max = 100))]
    pub recommended_action: String,

    /// Type-specific extra context
    pub metadata: serde_json::Value,
}

/// Anomaly type variants
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
pub enum AnomalyType {
    QuoteStuffing {
        /// Updates/sec (>500 triggers)
        update_rate: f64,
        /// Percentage (<10% triggers)
        fill_rate: f64,
    },
    IcebergOrder {
        #[schemars(with = "String")]
        price_level: Decimal,
        /// >5x median triggers
        refill_rate_multiplier: f64,
        median_refill_rate: f64,
    },
    FlashCrashRisk {
        /// >80% triggers
        depth_loss_pct: f64,
        /// >10x average triggers
        spread_multiplier: f64,
        /// >90% triggers
        cancellation_rate: f64,
    },
}

/// Liquidity vacuum
///
/// Price range with abnormally low volume (<20% of median), indicating potential
/// for rapid price movement through the vacuum.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LiquidityVacuum {
    /// Unique identifier (UUID v4)
    #[schemars(with = "String")]
    pub vacuum_id: Uuid,

    /// Trading pair symbol
    #[schemars(regex(pattern = r"^[A-Z]{4,12}$"))]
    pub symbol: String,

    /// Lower boundary
    #[schemars(with = "String")]
    pub price_range_low: Decimal,

    /// Upper boundary (> price_range_low)
    #[schemars(with = "String")]
    pub price_range_high: Decimal,

    /// Volume <20% of median (0.0-100.0)
    #[schemars(range(min = 0.0, max = 100.0))]
    pub volume_deficit_pct: f64,

    /// Median volume for comparison
    #[schemars(with = "String")]
    pub median_volume: Decimal,

    /// Volume in vacuum range
    #[schemars(with = "String")]
    pub actual_volume: Decimal,

    /// Predicted price movement speed
    pub expected_impact: ImpactLevel,

    /// When detected
    pub detection_timestamp: DateTime<Utc>,
}

/// Absorption event
///
/// Large order absorbing market pressure without price movement (whale accumulation/distribution).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AbsorptionEvent {
    /// Unique identifier (UUID v4)
    #[schemars(with = "String")]
    pub event_id: Uuid,

    /// Trading pair symbol
    #[schemars(regex(pattern = r"^[A-Z]{4,12}$"))]
    pub symbol: String,

    /// Exact price of absorption
    #[schemars(with = "String")]
    pub price_level: Decimal,

    /// Cumulative volume absorbed
    #[schemars(with = "String")]
    pub absorbed_volume: Decimal,

    /// Number of refills observed (≥ 1)
    #[schemars(range(min = 1))]
    pub refill_count: u32,

    /// First refill timestamp
    pub first_detected: DateTime<Utc>,

    /// Most recent refill (≤ now)
    pub last_updated: DateTime<Utc>,

    /// Heuristic entity classification
    pub suspected_entity_type: EntityType,

    /// Accumulation (bid) or Distribution (ask)
    pub direction: Direction,
}

/// Entity type classification for absorption events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum EntityType {
    /// Refill rate 1-3x median
    MarketMaker,
    /// Refill rate >5x median (iceberg-like)
    Whale,
    /// Insufficient data
    Unknown,
}

/// Market microstructure health score
///
/// Composite 0-100 health score combining spread stability, liquidity depth,
/// flow balance, and update rate metrics.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MicrostructureHealth {
    /// Trading pair symbol
    #[schemars(regex(pattern = r"^[A-Z]{4,12}$"))]
    pub symbol: String,

    /// When health was calculated
    pub timestamp: DateTime<Utc>,

    /// Composite health score (0-100)
    #[schemars(range(min = 0.0, max = 100.0))]
    pub overall_score: f64,

    /// Spread stability component score (0-100)
    #[schemars(range(min = 0.0, max = 100.0))]
    pub spread_stability_score: f64,

    /// Liquidity depth component score (0-100)
    #[schemars(range(min = 0.0, max = 100.0))]
    pub liquidity_depth_score: f64,

    /// Flow balance component score (0-100)
    #[schemars(range(min = 0.0, max = 100.0))]
    pub flow_balance_score: f64,

    /// Update rate component score (0-100)
    #[schemars(range(min = 0.0, max = 100.0))]
    pub update_rate_score: f64,

    /// Health level classification
    pub health_level: String,

    /// Trading guidance based on health
    pub recommended_action: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flow_direction_from_rates() {
        assert_eq!(
            FlowDirection::from_flow_rates(100.0, 40.0),
            FlowDirection::StrongBuy
        );
        assert_eq!(
            FlowDirection::from_flow_rates(60.0, 50.0),
            FlowDirection::ModerateBuy
        );
        assert_eq!(
            FlowDirection::from_flow_rates(50.0, 50.0),
            FlowDirection::Neutral
        );
        assert_eq!(
            FlowDirection::from_flow_rates(40.0, 60.0),
            FlowDirection::ModerateSell
        );
        assert_eq!(
            FlowDirection::from_flow_rates(20.0, 100.0),
            FlowDirection::StrongSell
        );
    }

    #[test]
    fn test_severity_from_confidence() {
        assert_eq!(Severity::from_confidence(0.96), Severity::Critical);
        assert_eq!(Severity::from_confidence(0.90), Severity::High);
        assert_eq!(Severity::from_confidence(0.75), Severity::Medium);
        assert_eq!(Severity::from_confidence(0.60), Severity::Low);
    }

    #[test]
    fn test_impact_level_from_deficit() {
        assert_eq!(
            ImpactLevel::from_deficit_pct(85.0),
            ImpactLevel::FastMovement
        );
        assert_eq!(
            ImpactLevel::from_deficit_pct(65.0),
            ImpactLevel::ModerateMovement
        );
        assert_eq!(ImpactLevel::from_deficit_pct(40.0), ImpactLevel::Negligible);
    }
}
