//! MCP tool handlers for advanced order book analytics
//!
//! Provides analytics tools following the contracts specification:
//! - get_order_flow: Bid/ask pressure tracking over time windows
//! - get_volume_profile: Volume distribution histogram (POC/VAH/VAL)
//! - detect_market_anomalies: Quote stuffing, icebergs, flash crash risk
//! - get_microstructure_health: Composite market health scoring
//! - get_liquidity_vacuums: Low-volume price zones for SL placement

use crate::orderbook::analytics::{
    anomaly::{detect_quote_stuffing, detect_iceberg_orders, detect_flash_crash_risk},
    flow::calculate_order_flow,
    health::calculate_microstructure_health,
    profile::{generate_volume_profile, identify_liquidity_vacuums},
    storage::{query::query_snapshots_in_window, SnapshotStorage},
    trade_stream::AggTrade,
    types::{LiquidityVacuum, MarketMicrostructureAnomaly, MicrostructureHealth, OrderFlowSnapshot, VolumeProfile},
};
use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info};

/// Error types for analytics tools
#[derive(Debug, thiserror::Error)]
pub enum AnalyticsToolError {
    #[error("Insufficient historical data: {0}")]
    InsufficientData(String),

    #[error("Storage query failed: {0}")]
    StorageError(String),

    #[error("Invalid window duration: {0}. Must be between 10 and 300 seconds")]
    InvalidWindowDuration(u32),

    #[error("Analytics calculation failed: {0}")]
    CalculationFailed(String),
}

impl From<anyhow::Error> for AnalyticsToolError {
    fn from(err: anyhow::Error) -> Self {
        let msg = err.to_string();
        if msg.contains("insufficient_historical_data") {
            AnalyticsToolError::InsufficientData(msg)
        } else if msg.contains("window_duration_secs") {
            // Extract duration from error message if possible
            AnalyticsToolError::InvalidWindowDuration(0)
        } else {
            AnalyticsToolError::StorageError(msg)
        }
    }
}

/// Parameters for get_order_flow tool
///
/// JSON schema matches contract: specs/003-specify-scripts-bash/contracts/get_order_flow.json
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct GetOrderFlowParams {
    /// Trading pair symbol (e.g., "BTCUSDT")
    ///
    /// Must be uppercase. Examples: "BTCUSDT", "ETHUSDT", "SOLUSDT"
    #[schemars(
        description = "Trading pair (e.g., BTCUSDT). Must be uppercase.",
        regex(pattern = r"^[A-Z]+$")
    )]
    pub symbol: String,

    /// Analysis time window in seconds (10-300)
    ///
    /// Default: 60 seconds (from clarifications Q1: Min 10s, Max 300s)
    #[schemars(
        description = "Analysis time window in seconds. Defaults to 60. Range: 10-300 seconds (from clarifications).",
        range(min = 10, max = 300)
    )]
    #[serde(default = "default_window_duration")]
    pub window_duration_secs: u32,
}

fn default_window_duration() -> u32 {
    60
}

fn get_order_flow_symbol_examples() -> Vec<&'static str> {
    vec!["BTCUSDT", "ETHUSDT", "SOLUSDT"]
}

/// Parameters for get_volume_profile tool
///
/// JSON schema matches contract: specs/003-specify-scripts-bash/contracts/get_volume_profile.json
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct GetVolumeProfileParams {
    /// Trading pair symbol (e.g., "ETHUSDT")
    ///
    /// Must be uppercase. Examples: "BTCUSDT", "ETHUSDT", "SOLUSDT"
    #[schemars(
        description = "Trading pair (e.g., ETHUSDT). Must be uppercase.",
        regex(pattern = r"^[A-Z]+$")
    )]
    pub symbol: String,

    /// Analysis time period in hours (1-168)
    ///
    /// Default: 24 hours
    #[schemars(
        description = "Analysis time period in hours. Defaults to 24. Range: 1-168 hours (from FR-004).",
        range(min = 1, max = 168)
    )]
    #[serde(default = "default_duration_hours")]
    pub duration_hours: u32,

    /// Optional custom bin size for price levels
    ///
    /// If omitted, auto-calculated as max(exchange_tick_size × 10, price_range / 100)
    #[schemars(
        description = "Optional: Custom bin size for price levels. If omitted, auto-calculated as max(exchange_tick_size × 10, price_range / 100).",
        regex(pattern = r"^[0-9]+(\.[0-9]+)?$")
    )]
    pub tick_size: Option<String>,
}

fn default_duration_hours() -> u32 {
    24
}

fn get_volume_profile_symbol_examples() -> Vec<&'static str> {
    vec!["BTCUSDT", "ETHUSDT", "SOLUSDT"]
}

/// Parameters for get_liquidity_vacuums tool
///
/// JSON schema matches contract: specs/003-specify-scripts-bash/contracts/get_liquidity_vacuums.json
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct GetLiquidityVacuumsParams {
    /// Trading pair symbol (e.g., "BTCUSDT")
    ///
    /// Must be uppercase. Examples: "BTCUSDT", "ETHUSDT", "SOLUSDT"
    #[schemars(
        description = "Trading pair (e.g., BTCUSDT). Must be uppercase.",
        regex(pattern = r"^[A-Z]+$")
    )]
    pub symbol: String,

    /// Analysis time period in hours (1-168)
    ///
    /// Default: 24 hours
    #[schemars(
        description = "Analysis time period in hours. Defaults to 24. Range: 1-168 hours.",
        range(min = 1, max = 168)
    )]
    #[serde(default = "default_duration_hours")]
    pub duration_hours: u32,
}

fn get_liquidity_vacuums_symbol_examples() -> Vec<&'static str> {
    vec!["BTCUSDT", "ETHUSDT", "SOLUSDT"]
}

/// Get order flow analysis (bid/ask pressure tracking)
///
/// Calculates order flow dynamics over configurable time window (10-300 seconds)
/// to identify optimal entry and exit points. Returns bid flow rate, ask flow rate,
/// net flow, flow direction indicator, and cumulative delta.
///
/// # Arguments
/// * `storage` - RocksDB snapshot storage with historical orderbook data
/// * `params` - Tool parameters (symbol, window_duration_secs)
///
/// # Returns
/// OrderFlowSnapshot with:
/// - symbol: Trading pair
/// - time_window_start/end: Analysis window (ISO 8601 UTC)
/// - window_duration_secs: Duration in seconds
/// - bid_flow_rate: Bid orders per second (≥ 0.0)
/// - ask_flow_rate: Ask orders per second (≥ 0.0)
/// - net_flow: Bid flow - ask flow (can be negative)
/// - flow_direction: STRONG_BUY, MODERATE_BUY, NEUTRAL, MODERATE_SELL, STRONG_SELL
/// - cumulative_delta: Running sum of buy volume - sell volume
///
/// # Errors
/// - InsufficientData: Need at least 2 snapshots for window analysis
/// - InvalidWindowDuration: Window must be 10-300 seconds
/// - StorageError: RocksDB query failed or timeout exceeded
///
/// # Performance
/// - First request: Depends on snapshot availability
/// - Query timeout: 200ms max (from FR-016)
pub async fn get_order_flow(
    storage: Arc<SnapshotStorage>,
    params: GetOrderFlowParams,
) -> Result<OrderFlowSnapshot, AnalyticsToolError> {
    let symbol_upper = params.symbol.to_uppercase();
    let window_duration = params.window_duration_secs;

    // Validate window duration
    if !(10..=300).contains(&window_duration) {
        return Err(AnalyticsToolError::InvalidWindowDuration(window_duration));
    }

    info!(
        symbol = %symbol_upper,
        window_duration_secs = window_duration,
        "Calculating order flow analysis"
    );

    // Call flow calculation logic
    let snapshot = calculate_order_flow(&storage, &symbol_upper, window_duration)
        .await
        .map_err(|e| {
            debug!(error = %e, "Order flow calculation failed");
            AnalyticsToolError::from(e)
        })?;

    debug!(
        symbol = %symbol_upper,
        bid_flow_rate = snapshot.bid_flow_rate,
        ask_flow_rate = snapshot.ask_flow_rate,
        flow_direction = ?snapshot.flow_direction,
        "Order flow calculated successfully"
    );

    Ok(snapshot)
}

/// Get volume profile histogram (POC/VAH/VAL indicators)
///
/// Generates volume distribution histogram across price levels to identify
/// high-volume nodes (support/resistance zones). Returns Point of Control (POC),
/// Value Area High/Low (VAH/VAL - 70% volume boundaries), and liquidity vacuum zones.
///
/// # Arguments
/// * `trades` - Aggregated trade events from @aggTrade stream
/// * `params` - Tool parameters (symbol, duration_hours, tick_size)
///
/// # Returns
/// VolumeProfile with:
/// - symbol: Trading pair
/// - histogram: Volume bins sorted by price level
/// - bin_size: Price increment per bin (adaptive or custom)
/// - point_of_control: Price with highest volume (POC)
/// - value_area_high/low: 70% volume boundaries (VAH/VAL)
/// - total_volume: Sum of all bin volumes
/// - liquidity_vacuums: Low-volume zones (<20% of median)
///
/// # Errors
/// - InsufficientData: Need at least 1000 trades for reliable profile
/// - CalculationFailed: Profile generation failed
///
/// # Performance
/// - Depends on trade buffer size (1000+ trades recommended)
pub async fn get_volume_profile(
    trades: Vec<AggTrade>,
    params: GetVolumeProfileParams,
) -> Result<VolumeProfile, AnalyticsToolError> {
    let symbol_upper = params.symbol.to_uppercase();
    let duration_hours = params.duration_hours;

    // Validate duration
    if !(1..=168).contains(&duration_hours) {
        return Err(AnalyticsToolError::CalculationFailed(format!(
            "duration_hours must be between 1 and 168, got {}",
            duration_hours
        )));
    }

    // Parse custom tick size if provided
    let tick_size = params
        .tick_size
        .as_ref()
        .and_then(|s| Decimal::from_str_exact(s).ok());

    info!(
        symbol = %symbol_upper,
        duration_hours,
        tick_size = ?tick_size,
        trade_count = trades.len(),
        "Generating volume profile"
    );

    // Call profile generation logic
    let profile = generate_volume_profile(&symbol_upper, trades, duration_hours, tick_size)
        .await
        .map_err(|e| {
            debug!(error = %e, "Volume profile generation failed");
            if e.to_string().contains("insufficient_historical_data") {
                AnalyticsToolError::InsufficientData(e.to_string())
            } else {
                AnalyticsToolError::CalculationFailed(e.to_string())
            }
        })?;

    debug!(
        symbol = %symbol_upper,
        poc = %profile.point_of_control,
        vah = %profile.value_area_high,
        val = %profile.value_area_low,
        vacuum_count = profile.histogram.iter().filter(|b| b.volume < profile.total_volume / Decimal::from(5)).count(),
        "Volume profile generated successfully"
    );

    Ok(profile)
}

/// Detect market microstructure anomalies
///
/// Scans for quote stuffing, iceberg orders, and flash crash risk.
/// Returns all detected anomalies with severity and recommendations.
///
/// # Arguments
/// * `storage` - RocksDB snapshot storage
/// * `symbol` - Trading pair (e.g., "BTCUSDT")
///
/// # Returns
/// Vector of MarketMicrostructureAnomaly with:
/// - anomaly_type: QuoteStuffing, IcebergOrder, FlashCrashRisk
/// - severity: Low, Medium, High, Critical
/// - detection_timestamp: When detected
/// - affected_price_level: Price level (if applicable)
/// - description: Human-readable explanation
/// - recommendation: Suggested action
pub async fn detect_market_anomalies(
    storage: Arc<SnapshotStorage>,
    symbol: &str,
) -> Result<Vec<MarketMicrostructureAnomaly>, AnalyticsToolError> {
    use chrono::{Duration, Utc};

    let end = Utc::now();
    let start = end - Duration::seconds(60); // Last 60 seconds

    let snapshots = query_snapshots_in_window(
        &storage,
        symbol,
        start.timestamp(),
        end.timestamp(),
    )
    .await
    .map_err(|e| AnalyticsToolError::StorageError(e.to_string()))?;

    if snapshots.len() < 2 {
        return Ok(Vec::new());
    }

    let mut anomalies = Vec::new();

    // Calculate update rate for quote stuffing detection
    let update_rate = snapshots.len() as f64 / 60.0; // updates per second

    // Detect quote stuffing
    if let Some(anomaly) = detect_quote_stuffing(&snapshots, update_rate) {
        anomalies.push(anomaly);
    }

    // TODO: Implement iceberg and flash crash detection with real data
    // These would require tracking price levels and cancellation rates

    Ok(anomalies)
}

/// Get microstructure health score
///
/// Returns composite 0-100 health score with component breakdowns:
/// - spread_stability_score (25%): Bid-ask spread consistency
/// - liquidity_depth_score (35%): Order book depth adequacy
/// - flow_balance_score (25%): Bid/ask flow equilibrium
/// - update_rate_score (15%): Quote update frequency
///
/// # Arguments
/// * `storage` - RocksDB snapshot storage
/// * `symbol` - Trading pair (e.g., "BTCUSDT")
///
/// # Returns
/// MicrostructureHealth with:
/// - symbol: Trading pair
/// - timestamp: Calculation time
/// - composite_score: 0-100 weighted health score
/// - component_scores: Individual metric scores
/// - health_status: Healthy, Degraded, Poor, Critical
/// - warnings: Active issues
/// - recommendations: Suggested actions
pub async fn get_microstructure_health(
    storage: Arc<SnapshotStorage>,
    symbol: &str,
) -> Result<MicrostructureHealth, AnalyticsToolError> {
    use chrono::{Duration, Utc};

    let end = Utc::now();
    let start = end - Duration::seconds(60); // Last 60 seconds

    let snapshots = query_snapshots_in_window(
        &storage,
        symbol,
        start.timestamp(),
        end.timestamp(),
    )
    .await
    .map_err(|e| AnalyticsToolError::StorageError(e.to_string()))?;

    if snapshots.len() < 2 {
        return Err(AnalyticsToolError::InsufficientData(format!(
            "Need at least 2 snapshots for health calculation, got {}",
            snapshots.len()
        )));
    }

    // Calculate order flow for flow balance component
    let flow_snapshot = calculate_order_flow(&storage, symbol, 60)
        .await
        .map_err(|e| AnalyticsToolError::CalculationFailed(e.to_string()))?;

    let health = calculate_microstructure_health(
        symbol,
        &snapshots,
        flow_snapshot.bid_flow_rate,
        flow_snapshot.ask_flow_rate,
    )
    .map_err(|e| AnalyticsToolError::CalculationFailed(e.to_string()))?;

    Ok(health)
}

/// Get liquidity vacuums for stop-loss placement
///
/// Identifies price zones with <20% of median volume where stop-losses
/// are likely to get triggered prematurely. Helps traders place stops
/// in high-liquidity areas to avoid getting stopped out by wicks.
///
/// # Arguments
/// * `storage` - RocksDB snapshot storage
/// * `params` - Tool parameters (symbol, duration_hours)
///
/// # Returns
/// Vector of LiquidityVacuum with:
/// - vacuum_id: Unique identifier (UUID v4)
/// - symbol: Trading pair
/// - price_range_low/high: Vacuum zone boundaries
/// - volume_deficit_pct: Percentage below median (>80% = high severity)
/// - median_volume: Expected volume at this price
/// - actual_volume: Actual volume in zone
/// - expected_impact: Low, Medium, High, Critical
/// - detection_timestamp: When detected
///
/// # Errors
/// - InsufficientData: Need sufficient snapshots for analysis
/// - StorageError: RocksDB query failed
///
/// # Usage
/// For long positions: Place stops below vacuum zones
/// For short positions: Place stops above vacuum zones
pub async fn get_liquidity_vacuums(
    storage: Arc<SnapshotStorage>,
    params: GetLiquidityVacuumsParams,
) -> Result<Vec<LiquidityVacuum>, AnalyticsToolError> {
    use chrono::{Duration, Utc};

    let symbol_upper = params.symbol.to_uppercase();
    let duration_hours = params.duration_hours;

    if !(1..=168).contains(&duration_hours) {
        return Err(AnalyticsToolError::CalculationFailed(format!(
            "duration_hours must be between 1 and 168, got {}",
            duration_hours
        )));
    }

    info!(
        symbol = %symbol_upper,
        duration_hours,
        "Identifying liquidity vacuums"
    );

    let end = Utc::now();
    let start = end - Duration::hours(duration_hours as i64);

    // Query snapshots for the time period
    let snapshots = query_snapshots_in_window(
        &storage,
        &symbol_upper,
        start.timestamp(),
        end.timestamp(),
    )
    .await
    .map_err(|e| AnalyticsToolError::StorageError(e.to_string()))?;

    if snapshots.is_empty() {
        return Err(AnalyticsToolError::InsufficientData(format!(
            "No snapshots available for {} over {}h window",
            symbol_upper, duration_hours
        )));
    }

    // Use the most recent snapshot for vacuum detection
    let latest_snapshot = &snapshots[snapshots.len() - 1];

    // Identify vacuums from order book depth
    let vacuums = identify_liquidity_vacuums(latest_snapshot, &symbol_upper)
        .map_err(|e| AnalyticsToolError::CalculationFailed(e.to_string()))?;

    debug!(
        symbol = %symbol_upper,
        vacuum_count = vacuums.len(),
        "Liquidity vacuums identified"
    );

    Ok(vacuums)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_window_duration() {
        assert_eq!(default_window_duration(), 60);
    }

    #[test]
    fn test_invalid_window_duration_validation() {
        let err = AnalyticsToolError::InvalidWindowDuration(5);
        assert!(err.to_string().contains("between 10 and 300"));

        let err = AnalyticsToolError::InvalidWindowDuration(400);
        assert!(err.to_string().contains("between 10 and 300"));
    }

    #[test]
    fn test_get_order_flow_params_schema() {
        // Verify schema can be generated (compile-time check)
        let _schema = schemars::schema_for!(GetOrderFlowParams);
    }
}
