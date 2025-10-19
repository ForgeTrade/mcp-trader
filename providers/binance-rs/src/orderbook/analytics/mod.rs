//! Advanced order book analytics module
//!
//! This module provides advanced analytics capabilities for orderbook data:
//! - Order flow analysis (bid/ask pressure tracking)
//! - Volume profile generation (POC/VAH/VAL indicators)
//! - Anomaly detection (quote stuffing, icebergs, flash crashes)
//! - Liquidity vacuum mapping
//! - Microstructure health scoring

#[cfg(feature = "orderbook_analytics")]
pub mod storage;

#[cfg(feature = "orderbook_analytics")]
pub mod types;

#[cfg(feature = "orderbook_analytics")]
pub mod flow;

#[cfg(feature = "orderbook_analytics")]
pub mod trade_stream;

#[cfg(feature = "orderbook_analytics")]
pub mod profile;

#[cfg(feature = "orderbook_analytics")]
pub mod anomaly;

#[cfg(feature = "orderbook_analytics")]
pub mod health;

#[cfg(feature = "orderbook_analytics")]
pub mod tools;

#[cfg(feature = "orderbook_analytics")]
pub use storage::SnapshotStorage;

#[cfg(feature = "orderbook_analytics")]
pub use types::*;
