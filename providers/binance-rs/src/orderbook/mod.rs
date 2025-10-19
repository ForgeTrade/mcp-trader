//! Order book depth analysis tools for Binance MCP server
//!
//! This module provides progressive disclosure order book analysis:
//! - L1 aggregated metrics (spread, microprice, imbalance, walls, slippage)
//! - L2 depth data with compact integer encoding (token-efficient)
//! - Service health monitoring for data freshness validation
//!
//! Architecture: WebSocket + Local L2 Cache with REST API fallback
//! - Sub-100ms latency for warm requests via local cache
//! - Lazy initialization: subscribe on first request per symbol
//! - Up to 20 concurrent symbols with client-side rate limiting

#[cfg(feature = "orderbook")]
pub mod types;

#[cfg(feature = "orderbook")]
pub mod manager;

#[cfg(feature = "orderbook")]
pub mod metrics;

#[cfg(feature = "orderbook")]
pub mod websocket;

#[cfg(feature = "orderbook")]
pub mod rate_limiter;

#[cfg(feature = "orderbook")]
pub mod tools;

#[cfg(feature = "orderbook_analytics")]
pub mod analytics;

#[cfg(feature = "orderbook")]
pub use types::{
    OrderBook, OrderBookDepth, OrderBookHealth, OrderBookMetrics, SlippageEstimate,
    SlippageEstimates, Wall,
};

#[cfg(feature = "orderbook")]
pub use manager::OrderBookManager;

#[cfg(feature = "orderbook")]
pub use tools::{get_orderbook_depth, get_orderbook_health, get_orderbook_metrics};
