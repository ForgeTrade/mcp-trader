//! MCP Resources Implementation
//!
//! This module provides MCP resources for exposing Binance data through URIs.
//! Resources can be read and subscribed to for real-time updates.
//!

use crate::binance::BinanceClient;
use rmcp::model::ResourceContents;
use serde_json::json;

/// Binance resource URI scheme
pub const BINANCE_SCHEME: &str = "binance";

/// Resource paths

/// Lists all available resources
pub fn list_resources() -> Vec<rmcp::model::Annotated<rmcp::model::RawResource>> {
    // No resources exposed via MCP after removing account/order management
    vec![]
}

/// Reads a resource by URI
///
/// # Arguments
///
/// * `client` - Binance API client
/// * `uri` - Resource URI (e.g., "binance://account")
///
/// # Returns
///
/// Resource contents as JSON
pub async fn read_resource(client: &BinanceClient, uri: &str) -> Result<ResourceContents, String> {
    // Parse URI
    let uri_parts: Vec<&str> = uri.split("://").collect();
    if uri_parts.len() != 2 {
        return Err(format!("Invalid URI format: {}", uri));
    }

    let scheme = uri_parts[0];
    if scheme != BINANCE_SCHEME {
        return Err(format!("Unsupported URI scheme: {}", scheme));
    }

    let path_parts: Vec<&str> = uri_parts[1].split('/').collect();
    let resource_type = path_parts[0];

    match resource_type {
        _ => Err(format!("Unknown resource type: {}", resource_type)),
    }
}
