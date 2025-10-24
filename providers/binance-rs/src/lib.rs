// Library exports for binance-provider

pub mod error;
pub mod grpc;
pub mod pb;

#[cfg(feature = "http_transport")]
pub mod transport; // MCP transport layer (HTTP)

// Binance API integration modules
pub mod binance; // Binance API client
pub mod config; // Configuration management

#[cfg(feature = "orderbook")]
pub mod orderbook; // WebSocket orderbook manager

#[cfg(feature = "mcp_server")]
pub mod mcp; // MCP server implementation

// Market data report generation (requires orderbook feature)
#[cfg(feature = "orderbook")]
pub mod report; // Unified market intelligence report generator
