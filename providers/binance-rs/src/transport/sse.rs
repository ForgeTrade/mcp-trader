//! SSE (Server-Sent Events) transport for MCP
//!
//! Uses rmcp's built-in SSE server implementation for remote HTTPS connections.
//! This transport enables web-based access to the Binance MCP server.

pub use rmcp::transport::sse_server::{SseServer, SseServerConfig};

// Re-export CancellationToken for convenience (required by SseServerConfig)
pub use tokio_util::sync::CancellationToken;
