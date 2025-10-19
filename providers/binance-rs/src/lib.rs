// Library exports for binance-provider

pub mod error;
pub mod grpc;
pub mod pb;

// Binance API integration modules
pub mod binance; // Binance API client
pub mod config; // Configuration management

#[cfg(feature = "orderbook")]
pub mod orderbook; // WebSocket orderbook manager
