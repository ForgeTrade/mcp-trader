//! MCP Tool Router and ServerHandler Implementation
//!
//! This module implements the MCP protocol ServerHandler trait and provides
//! tool routing for Binance API operations using rmcp SDK macros.

use crate::mcp::server::BinanceServer;
use crate::mcp::types::{OrderbookParam, SymbolParam};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::handler::server::ServerHandler;
use rmcp::model::{
    CallToolResult, Content, ErrorData, Implementation,
    InitializeResult, ListResourcesResult, PaginatedRequestParam,
    ProtocolVersion, ReadResourceRequestParam, ReadResourceResult,
    ResourcesCapability, ServerCapabilities, ToolsCapability,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::{tool, tool_handler, tool_router};
use serde_json::json;

/// MCP Tool Router for Binance operations
///
/// Uses the #[tool_router] macro to automatically generate routing logic
/// and JSON Schema for all tools.
#[tool_router(vis = "pub")]
impl BinanceServer {
    /// Get current Binance server time
    ///
    /// Returns the current server time in milliseconds since Unix epoch.
    /// Useful for time synchronization and validating server connectivity.
    #[tool(
        description = "Returns current Binance server time in milliseconds since Unix epoch"
    )]
    pub async fn get_server_time(&self) -> Result<CallToolResult, ErrorData> {
        let server_time = self
            .client
            .get_server_time()
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        let response_json = json!({
            "serverTime": server_time
        });

        Ok(CallToolResult::success(vec![Content::text(
            response_json.to_string(),
        )]))
    }

    /// Get 24-hour ticker price change statistics
    ///
    /// Returns detailed 24-hour price statistics for a trading pair including
    /// price change, volume, high/low prices, and more.
    #[tool(description = "Get 24-hour ticker price change statistics for a trading pair")]
    pub async fn get_ticker(
        &self,
        params: Parameters<SymbolParam>,
    ) -> Result<CallToolResult, ErrorData> {
        let symbol = params.0.symbol.to_uppercase();

        let ticker = self
            .client
            .get_24hr_ticker(&symbol)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        let response_json = json!(ticker);

        Ok(CallToolResult::success(vec![Content::text(
            response_json.to_string(),
        )]))
    }

    /// Get order book depth
    ///
    /// Returns current order book bids and asks for a trading pair.
    /// Depth limit can be 5, 10, 20, 50, 100, 500, 1000, or 5000.
    #[tool(description = "Get current order book depth (bids and asks) for a trading pair")]
    pub async fn get_orderbook(
        &self,
        params: Parameters<OrderbookParam>,
    ) -> Result<CallToolResult, ErrorData> {
        let symbol = params.0.symbol.to_uppercase();
        let limit = params.0.limit;

        let orderbook = self
            .client
            .get_order_book(&symbol, limit)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        let response_json = json!(orderbook);

        Ok(CallToolResult::success(vec![Content::text(
            response_json.to_string(),
        )]))
    }
}

/// ServerHandler trait implementation
///
/// Uses the #[tool_handler] macro to automatically wire the tool router
/// to the ServerHandler trait.
#[tool_handler(router = self.tool_router)]
impl ServerHandler for BinanceServer {
    /// Returns server information and capabilities
    ///
    /// This is called during MCP initialization to communicate server metadata
    /// and supported features to the client.
    fn get_info(&self) -> InitializeResult {
        InitializeResult {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(false),
                }),
                resources: Some(ResourcesCapability {
                    subscribe: Some(false),
                    list_changed: Some(false),
                }),
                ..Default::default()
            },
            server_info: Implementation {
                name: "binance-provider".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                title: Some("Binance MCP Provider".to_string()),
                website_url: Some("https://github.com/tradeforge/mcp-trader".to_string()),
                icons: None,
            },
            instructions: Some(
                "Binance MCP Provider for trading and market data. \
                Provides tools for market data operations and resources for account data."
                    .to_string(),
            ),
        }
    }

    /// Lists all available resources
    ///
    /// Returns a list of resources that can be accessed via read_resource.
    async fn list_resources(
        &self,
        _params: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        Ok(ListResourcesResult {
            resources: crate::mcp::resources::list_resources(),
            next_cursor: None,
        })
    }

    /// Reads a specific resource by URI
    ///
    /// # Arguments
    ///
    /// * `params` - Request parameters containing the URI
    /// * `_context` - Request context
    ///
    /// # Returns
    ///
    /// Resource contents as JSON
    async fn read_resource(
        &self,
        params: ReadResourceRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, ErrorData> {
        let contents = crate::mcp::resources::read_resource(&self.client, &params.uri)
            .await
            .map_err(|e| ErrorData::internal_error(e, None))?;

        Ok(ReadResourceResult { contents: vec![contents] })
    }
}
