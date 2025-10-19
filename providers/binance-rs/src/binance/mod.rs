//! Binance API Client
//!
//! This module contains the HTTP client for Binance API integration.

pub mod client;
pub mod types;

#[cfg(feature = "websocket")]
pub mod websocket;

// Re-export commonly used types
pub use client::BinanceClient;
pub use types::ServerTimeResponse;

#[cfg(feature = "websocket")]
pub use websocket::{
    BalanceUpdate, BinanceWebSocketClient, DepthUpdate, ExecutionReport, OutboundAccountPosition,
    TickerUpdate, UserDataEvent,
};
