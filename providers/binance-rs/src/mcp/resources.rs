//! MCP Resources Implementation
//!
//! This module provides MCP resources for exposing Binance data through URIs.
//! Resources can be read and subscribed to for real-time updates.
//!
//! ## Resource URIs
//!
//! - `binance://account` - Account information
//! - `binance://balances` - All account balances
//! - `binance://balances/{asset}` - Specific asset balance
//! - `binance://trades` - Recent trades (last 100)
//! - `binance://trades/{symbol}` - Trades for specific symbol
//! - `binance://orders` - All open orders
//! - `binance://orders/{symbol}` - Open orders for specific symbol

use crate::binance::BinanceClient;
use rmcp::model::ResourceContents;
use serde_json::json;

/// Binance resource URI scheme
pub const BINANCE_SCHEME: &str = "binance";

/// Resource paths
pub mod paths {
    pub const ACCOUNT: &str = "account";
    pub const BALANCES: &str = "balances";
    pub const TRADES: &str = "trades";
    pub const ORDERS: &str = "orders";
}

/// Lists all available resources
pub fn list_resources() -> Vec<rmcp::model::Annotated<rmcp::model::RawResource>> {
    use rmcp::model::{Annotated, RawResource};

    vec![
        Annotated::new(
            RawResource {
                uri: format!("{}://{}", BINANCE_SCHEME, paths::ACCOUNT),
                name: "Account Information".to_string(),
                description: Some("Binance account information including permissions and status".to_string()),
                mime_type: Some("application/json".to_string()),
                title: None,
                size: None,
                icons: None,
            },
            None,
        ),
        Annotated::new(
            RawResource {
                uri: format!("{}://{}", BINANCE_SCHEME, paths::BALANCES),
                name: "Account Balances".to_string(),
                description: Some("All account balances across all assets".to_string()),
                mime_type: Some("application/json".to_string()),
                title: None,
                size: None,
                icons: None,
            },
            None,
        ),
        Annotated::new(
            RawResource {
                uri: format!("{}://{}", BINANCE_SCHEME, paths::TRADES),
                name: "Recent Trades".to_string(),
                description: Some("Recent account trades (last 100)".to_string()),
                mime_type: Some("application/json".to_string()),
                title: None,
                size: None,
                icons: None,
            },
            None,
        ),
        Annotated::new(
            RawResource {
                uri: format!("{}://{}", BINANCE_SCHEME, paths::ORDERS),
                name: "Open Orders".to_string(),
                description: Some("All currently open orders".to_string()),
                mime_type: Some("application/json".to_string()),
                title: None,
                size: None,
                icons: None,
            },
            None,
        ),
    ]
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
pub async fn read_resource(
    client: &BinanceClient,
    uri: &str,
) -> Result<ResourceContents, String> {
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
        paths::ACCOUNT => read_account_resource(client).await,
        paths::BALANCES => {
            if path_parts.len() > 1 {
                // Specific asset: binance://balances/BTC
                read_balance_resource(client, Some(path_parts[1])).await
            } else {
                // All balances: binance://balances
                read_balance_resource(client, None).await
            }
        }
        paths::TRADES => {
            if path_parts.len() > 1 {
                // Symbol trades: binance://trades/BTCUSDT
                read_trades_resource(client, Some(path_parts[1])).await
            } else {
                // All trades: binance://trades
                read_trades_resource(client, None).await
            }
        }
        paths::ORDERS => {
            if path_parts.len() > 1 {
                // Symbol orders: binance://orders/BTCUSDT
                read_orders_resource(client, Some(path_parts[1])).await
            } else {
                // All orders: binance://orders
                read_orders_resource(client, None).await
            }
        }
        _ => Err(format!("Unknown resource type: {}", resource_type)),
    }
}

/// Reads account information resource
async fn read_account_resource(client: &BinanceClient) -> Result<ResourceContents, String> {
    let account = client
        .get_account()
        .await
        .map_err(|e| format!("Failed to get account: {}", e))?;

    let json_data = serde_json::to_value(account)
        .map_err(|e| format!("Failed to serialize account: {}", e))?;

    Ok(ResourceContents::TextResourceContents {
        uri: format!("{}://{}", BINANCE_SCHEME, paths::ACCOUNT),
        mime_type: Some("application/json".to_string()),
        text: serde_json::to_string_pretty(&json_data).unwrap(),
        meta: None,
    })
}

/// Reads balance resource
///
/// # Arguments
///
/// * `client` - Binance API client
/// * `asset` - Optional asset filter (e.g., "BTC")
async fn read_balance_resource(
    client: &BinanceClient,
    asset: Option<&str>,
) -> Result<ResourceContents, String> {
    let account = client
        .get_account()
        .await
        .map_err(|e| format!("Failed to get account: {}", e))?;

    let balances = if let Some(asset_name) = asset {
        // Filter for specific asset
        account
            .balances
            .iter()
            .filter(|b| b.asset.eq_ignore_ascii_case(asset_name))
            .collect::<Vec<_>>()
    } else {
        // Return all balances with non-zero amounts
        account
            .balances
            .iter()
            .filter(|b| b.free.parse::<f64>().unwrap_or(0.0) > 0.0
                     || b.locked.parse::<f64>().unwrap_or(0.0) > 0.0)
            .collect::<Vec<_>>()
    };

    let json_data = json!({
        "balances": balances,
        "timestamp": account.update_time,
    });

    let uri = if let Some(asset_name) = asset {
        format!("{}://{}/{}", BINANCE_SCHEME, paths::BALANCES, asset_name)
    } else {
        format!("{}://{}", BINANCE_SCHEME, paths::BALANCES)
    };

    Ok(ResourceContents::TextResourceContents {
        uri,
        mime_type: Some("application/json".to_string()),
        text: serde_json::to_string_pretty(&json_data).unwrap(),
        meta: None,
    })
}

/// Reads trades resource
///
/// # Arguments
///
/// * `client` - Binance API client
/// * `symbol` - Optional symbol filter (e.g., "BTCUSDT")
async fn read_trades_resource(
    client: &BinanceClient,
    symbol: Option<&str>,
) -> Result<ResourceContents, String> {
    let trades = if let Some(sym) = symbol {
        // Get trades for specific symbol (last 100)
        client
            .get_my_trades(sym, None)
            .await
            .map_err(|e| format!("Failed to get trades for {}: {}", sym, e))?
    } else {
        // For all trades, we'd need to query each symbol
        // For now, return empty array with a message
        vec![]
    };

    let json_data = json!({
        "trades": trades,
        "count": trades.len(),
        "symbol": symbol,
    });

    let uri = if let Some(sym) = symbol {
        format!("{}://{}/{}", BINANCE_SCHEME, paths::TRADES, sym)
    } else {
        format!("{}://{}", BINANCE_SCHEME, paths::TRADES)
    };

    Ok(ResourceContents::TextResourceContents {
        uri,
        mime_type: Some("application/json".to_string()),
        text: serde_json::to_string_pretty(&json_data).unwrap(),
        meta: None,
    })
}

/// Reads orders resource
///
/// # Arguments
///
/// * `client` - Binance API client
/// * `symbol` - Optional symbol filter (e.g., "BTCUSDT")
async fn read_orders_resource(
    client: &BinanceClient,
    symbol: Option<&str>,
) -> Result<ResourceContents, String> {
    let orders = if let Some(sym) = symbol {
        // Get orders for specific symbol
        client
            .get_open_orders(Some(sym))
            .await
            .map_err(|e| format!("Failed to get orders for {}: {}", sym, e))?
    } else {
        // Get all open orders
        client
            .get_open_orders(None)
            .await
            .map_err(|e| format!("Failed to get orders: {}", e))?
    };

    let json_data = json!({
        "orders": orders,
        "count": orders.len(),
        "symbol": symbol,
    });

    let uri = if let Some(sym) = symbol {
        format!("{}://{}/{}", BINANCE_SCHEME, paths::ORDERS, sym)
    } else {
        format!("{}://{}", BINANCE_SCHEME, paths::ORDERS)
    };

    Ok(ResourceContents::TextResourceContents {
        uri,
        mime_type: Some("application/json".to_string()),
        text: serde_json::to_string_pretty(&json_data).unwrap(),
        meta: None,
    })
}
