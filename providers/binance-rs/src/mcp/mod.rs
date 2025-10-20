//! Model Context Protocol (MCP) server implementation for Binance provider
//!
//! This module provides MCP server capabilities including:
//! - Tool invocation (market data, analytics, trading)
//! - Resource access (market data via URIs)
//! - Guided prompts (analysis workflows)
//!
//! The implementation uses rmcp SDK 0.8.1 with procedural macros for routing.

pub mod handler;
pub mod resources;
pub mod server;
pub mod types;
// Additional modules will be added as implementation progresses
// pub mod prompts;

// Re-exports
pub use server::BinanceServer;
