//! MCP tool handlers for order book depth analysis
//!
//! Provides three tools following progressive disclosure strategy:
//! - get_orderbook_metrics: L1 aggregated metrics (15% token cost)
//! - get_orderbook_depth: L2 depth with compact encoding (50-100% token cost)
//! - get_orderbook_health: Service health monitoring

use crate::orderbook::manager::{ManagerError, OrderBookManager};
use crate::orderbook::metrics;
use crate::orderbook::types::{OrderBookDepth, OrderBookHealth, OrderBookMetrics};
use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::Arc;
use tracing::{debug, info};

/// Error types for order book tools
#[derive(Debug, thiserror::Error)]
pub enum OrderBookToolError {
    #[error("Symbol not found: {0}")]
    SymbolNotFound(String),

    #[error("Symbol limit reached: cannot track more than 20 symbols")]
    SymbolLimitReached,

    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    #[error("Initialization failed for {symbol}: {message}")]
    InitializationFailed { symbol: String, message: String },

    #[error("Invalid levels parameter: {0}. Must be between 1 and 100")]
    InvalidLevels(usize),

    #[error("Failed to calculate metrics: {0}")]
    MetricsCalculationFailed(String),
}

impl From<ManagerError> for OrderBookToolError {
    fn from(err: ManagerError) -> Self {
        match err {
            ManagerError::SymbolNotFound(s) => OrderBookToolError::SymbolNotFound(s),
            ManagerError::SymbolLimitReached => OrderBookToolError::SymbolLimitReached,
            ManagerError::RateLimitExceeded(e) => {
                OrderBookToolError::RateLimitExceeded(e.to_string())
            }
            ManagerError::InitializationFailed { symbol, source } => {
                OrderBookToolError::InitializationFailed {
                    symbol,
                    message: source.to_string(),
                }
            }
            ManagerError::RestApiError(e) | ManagerError::WebSocketError(e) => {
                OrderBookToolError::InitializationFailed {
                    symbol: "unknown".to_string(),
                    message: e,
                }
            }
        }
    }
}

/// Parameters for get_orderbook_metrics tool
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct GetOrderBookMetricsParams {
    /// Trading pair symbol (e.g., "BTCUSDT")
    #[schemars(description = "Trading pair symbol (e.g., 'BTCUSDT', 'ETHUSDT')")]
    pub symbol: String,
}

/// Parameters for get_orderbook_depth tool
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct GetOrderBookDepthParams {
    /// Trading pair symbol (e.g., "BTCUSDT")
    #[schemars(description = "Trading pair symbol (e.g., 'BTCUSDT', 'ETHUSDT')")]
    pub symbol: String,

    /// Number of levels to return (1-100, default: 20)
    #[schemars(
        description = "Number of price levels to return (1-100). Default: 20 for L2-lite, use 100 for L2-full"
    )]
    #[serde(default = "default_levels")]
    pub levels: usize,
}

fn default_levels() -> usize {
    20
}

/// Get L1 aggregated metrics for quick spread assessment
///
/// Provides lightweight analysis (15% token cost vs L2-full):
/// - Spread in basis points
/// - Microprice (volume-weighted fair price)
/// - Bid/ask volume imbalance
/// - Wall detection (large levels)
/// - VWAP-based slippage estimates
///
/// First request: 2-3s (lazy initialization)
/// Subsequent requests: <200ms (cached data)
pub async fn get_orderbook_metrics(
    manager: Arc<OrderBookManager>,
    params: GetOrderBookMetricsParams,
) -> Result<OrderBookMetrics, OrderBookToolError> {
    let symbol_upper = params.symbol.to_uppercase();
    info!(symbol = %symbol_upper, "Fetching order book metrics");

    // Get order book (lazy initialization on first request)
    let order_book = manager.get_order_book(&symbol_upper).await?;

    // Calculate metrics
    let metrics = metrics::calculate_metrics(&order_book).ok_or_else(|| {
        OrderBookToolError::MetricsCalculationFailed(format!(
            "Failed to calculate metrics for {}. Order book may be empty or invalid.",
            symbol_upper
        ))
    })?;

    debug!(
        symbol = %symbol_upper,
        spread_bps = metrics.spread_bps,
        imbalance_ratio = metrics.imbalance_ratio,
        "Calculated order book metrics"
    );

    Ok(metrics)
}

/// Get L2 depth with compact integer encoding
///
/// Token cost: 50% (L2-lite with 20 levels) or 100% (L2-full with 100 levels)
///
/// Compact encoding:
/// - price_scale = 100 (e.g., 67650.00 → 6765000)
/// - qty_scale = 100000 (e.g., 1.234 → 123400)
///
/// First request: 2-3s (lazy initialization)
/// Subsequent requests: <300ms (cached data)
pub async fn get_orderbook_depth(
    manager: Arc<OrderBookManager>,
    params: GetOrderBookDepthParams,
) -> Result<OrderBookDepth, OrderBookToolError> {
    let symbol_upper = params.symbol.to_uppercase();
    let levels = params.levels;

    // Validate levels parameter
    if !(1..=100).contains(&levels) {
        return Err(OrderBookToolError::InvalidLevels(levels));
    }

    info!(
        symbol = %symbol_upper,
        levels,
        "Fetching order book depth"
    );

    // Get order book (lazy initialization on first request)
    let order_book = manager.get_order_book(&symbol_upper).await?;

    // Extract depth with compact encoding
    let depth = metrics::extract_depth(&order_book, levels);

    debug!(
        symbol = %symbol_upper,
        bid_levels = depth.bids.len(),
        ask_levels = depth.asks.len(),
        "Extracted order book depth"
    );

    Ok(depth)
}

/// Get service health status
///
/// Returns operational visibility:
/// - Overall status (ok/degraded/error)
/// - Number of active symbol subscriptions (0-20)
/// - Data freshness (last update age in ms)
/// - WebSocket connection status
///
/// Latency: <50ms (no external API calls)
pub async fn get_orderbook_health(
    manager: Arc<OrderBookManager>,
) -> Result<OrderBookHealth, OrderBookToolError> {
    debug!("Fetching order book health status");

    let health = manager.get_health().await;

    info!(
        status = ?health.status,
        active_symbols = health.orderbook_symbols_active,
        last_update_age_ms = health.last_update_age_ms,
        "Retrieved order book health status"
    );

    Ok(health)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_levels() {
        assert_eq!(default_levels(), 20);
    }

    #[test]
    fn test_invalid_levels_validation() {
        let err = OrderBookToolError::InvalidLevels(0);
        assert!(err.to_string().contains("between 1 and 100"));

        let err = OrderBookToolError::InvalidLevels(101);
        assert!(err.to_string().contains("between 1 and 100"));
    }
}
