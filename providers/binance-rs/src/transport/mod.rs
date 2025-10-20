//! MCP transport layer
//!
//! Provides multiple transport options for MCP protocol:
//! - Stdio: Standard I/O for local connections (MCP)
//! - SSE: Server-Sent Events for remote HTTPS connections (MCP)
//! - gRPC: High-performance binary protocol (existing)
//! - HTTP: JSON-RPC 2.0 over HTTP with session management (existing)

pub mod http;

#[cfg(feature = "mcp_server")]
pub mod stdio;

#[cfg(feature = "mcp_server")]
pub mod sse;

/// Transport mode selection for MCP server
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportMode {
    /// Standard I/O transport (local MCP connections)
    #[cfg(feature = "mcp_server")]
    Stdio,

    /// Server-Sent Events transport (remote HTTPS MCP connections)
    #[cfg(feature = "mcp_server")]
    Sse,

    /// gRPC transport (existing provider mode)
    Grpc,

    /// HTTP transport (existing provider mode)
    Http,
}

impl TransportMode {
    /// Returns the default transport mode
    pub fn default() -> Self {
        Self::Grpc
    }
}
