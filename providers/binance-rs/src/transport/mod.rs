//! MCP transport layer
//!
//! Provides multiple transport options for MCP protocol:
//! - gRPC: High-performance binary protocol
//! - HTTP: JSON-RPC 2.0 over HTTP with session management

pub mod http;
