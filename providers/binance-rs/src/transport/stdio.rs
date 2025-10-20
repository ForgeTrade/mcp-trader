//! Stdio Transport for MCP Server
//!
//! Provides standard I/O transport for local MCP connections (e.g., Claude Desktop).

use crate::mcp::BinanceServer;
use rmcp::ServiceExt;

/// Runs the MCP server with stdio transport
///
/// This function initializes the Binance MCP server and starts serving requests
/// via standard I/O. Messages are read from stdin and responses are written to stdout.
/// Logging is sent to stderr to avoid interfering with the MCP protocol.
///
/// # Returns
///
/// Returns Ok(()) when the server shuts down gracefully, or an error if initialization fails.
pub async fn run_stdio_server() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Starting Binance MCP server in stdio mode");

    // Create server instance
    let server = BinanceServer::new();

    // Run server with stdio transport
    let service = server.serve(rmcp::transport::stdio()).await?;

    tracing::info!("MCP server ready on stdio");

    // Wait for shutdown signal
    service.waiting().await?;

    tracing::info!("MCP server shutdown complete");

    Ok(())
}
