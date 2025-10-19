//! Advanced orderbook analytics module
//!
//! Provides order flow analysis, volume profile generation, anomaly detection,
//! microstructure health scoring, and liquidity mapping capabilities.
//!
//! This module is feature-gated behind `orderbook_analytics` and extends the
//! base `orderbook` feature with statistical analysis and time-series storage.

#[cfg(feature = "orderbook_analytics")]
pub mod types;

#[cfg(feature = "orderbook_analytics")]
pub mod storage;

#[cfg(feature = "orderbook_analytics")]
pub mod flow;

// Re-export key types for convenience
#[cfg(feature = "orderbook_analytics")]
pub use types::{
    FlowDirection, Severity, ImpactLevel, Direction,
    OrderFlowSnapshot,
};

#[cfg(feature = "orderbook_analytics")]
pub use storage::Storage;
