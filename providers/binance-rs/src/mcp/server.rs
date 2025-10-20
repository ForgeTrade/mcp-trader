//! MCP Server Implementation
//!
//! This module contains the BinanceServer struct which implements the MCP ServerHandler trait.

use crate::binance::BinanceClient;
use rmcp::handler::server::router::tool::ToolRouter;

#[cfg(feature = "orderbook")]
use crate::orderbook::OrderBookManager;
#[cfg(feature = "orderbook")]
use std::sync::Arc;

/// Main Binance MCP Server struct
///
/// This struct holds the server state including Binance API client, tool router,
/// and optional orderbook manager for WebSocket subscriptions.
#[derive(Clone)]
pub struct BinanceServer {
    /// Binance API client for making requests
    pub client: BinanceClient,

    /// Tool router for MCP tool routing
    pub tool_router: ToolRouter<Self>,

    /// Order book manager for WebSocket subscriptions (feature-gated)
    #[cfg(feature = "orderbook")]
    pub orderbook_manager: Arc<OrderBookManager>,
}

impl BinanceServer {
    /// Creates a new Binance server instance
    ///
    /// Initializes the Binance API client, tool router, and optionally creates
    /// an orderbook manager if the orderbook feature is enabled.
    pub fn new() -> Self {
        let client = BinanceClient::new();

        #[cfg(feature = "orderbook")]
        let client_arc = Arc::new(client.clone());
        #[cfg(feature = "orderbook")]
        let orderbook_manager = Arc::new(OrderBookManager::new(client_arc));

        Self {
            client,
            tool_router: Self::tool_router(),
            #[cfg(feature = "orderbook")]
            orderbook_manager,
        }
    }

    /// Creates a new server with API credentials from environment
    ///
    /// Loads API credentials from BINANCE_API_KEY and BINANCE_API_SECRET env vars.
    pub fn with_credentials() -> Self {
        let client = BinanceClient::with_credentials();

        #[cfg(feature = "orderbook")]
        let client_arc = Arc::new(client.clone());
        #[cfg(feature = "orderbook")]
        let orderbook_manager = Arc::new(OrderBookManager::new(client_arc));

        Self {
            client,
            tool_router: Self::tool_router(),
            #[cfg(feature = "orderbook")]
            orderbook_manager,
        }
    }
}

impl Default for BinanceServer {
    fn default() -> Self {
        Self::new()
    }
}
