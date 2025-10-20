//! HTTP request handlers for MCP JSON-RPC endpoints
//!
//! Implements handlers for:
//! - POST /mcp: Main JSON-RPC endpoint
//!   - initialize: Create session
//!   - tools/list: List all available tools
//!   - tools/call: Execute a tool

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use super::error::{HttpTransportError, Result};
use super::jsonrpc::{
    InitializeResult, JsonRpcRequest, JsonRpcResponse, ServerCapabilities, ServerInfo,
    ToolsCapability,
};
use super::session::SessionStore;
use crate::binance::client::BinanceClient;
use crate::grpc::capabilities::CapabilityBuilder;
use crate::pb::{InvokeRequest, Json as PbJson};

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    /// Session store
    pub sessions: SessionStore,

    /// Binance API client
    pub binance_client: BinanceClient,

    /// OrderBook manager (optional)
    #[cfg(feature = "orderbook")]
    pub orderbook_manager: Option<Arc<crate::orderbook::OrderBookManager>>,

    /// Analytics storage (optional)
    #[cfg(feature = "orderbook_analytics")]
    pub analytics_storage: Option<Arc<crate::orderbook::analytics::SnapshotStorage>>,

    /// Trade storage (optional)
    #[cfg(feature = "orderbook_analytics")]
    pub trade_storage: Option<Arc<crate::orderbook::analytics::TradeStorage>>,
}

/// Main JSON-RPC endpoint handler
///
/// POST /mcp
/// Content-Type: application/json
/// Mcp-Session-Id: <uuid> (optional for initialize)
pub async fn handle_jsonrpc(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<JsonRpcRequest>,
) -> Result<Response> {
    tracing::debug!(method = %request.method, "Received JSON-RPC request");

    // Extract session ID from headers (if present)
    let session_id = extract_session_id(&headers)?;

    // Route to appropriate handler based on method
    let response = match request.method.as_str() {
        "initialize" => handle_initialize(state, request).await?,
        "tools/list" => {
            // Validate session for authenticated methods
            if let Some(sid) = session_id {
                state.sessions.validate_session(sid)?;
            } else {
                return Err(HttpTransportError::Session(
                    super::session::SessionError::InvalidSessionId,
                ));
            }
            handle_tools_list(state, request).await?
        }
        "tools/call" => {
            // Validate session
            if let Some(sid) = session_id {
                state.sessions.validate_session(sid)?;
            } else {
                return Err(HttpTransportError::Session(
                    super::session::SessionError::InvalidSessionId,
                ));
            }
            handle_tools_call(state, request).await?
        }
        _ => {
            return Err(HttpTransportError::MethodNotFound(request.method.clone()));
        }
    };

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// Handle initialize method
///
/// Creates a new session and returns session ID in Mcp-Session-Id header
async fn handle_initialize(
    state: AppState,
    request: JsonRpcRequest,
) -> Result<JsonRpcResponse> {
    // Create session
    let client_metadata = HashMap::new(); // Could extract User-Agent, IP, etc.
    let session_id = state.sessions.create_session(client_metadata)?;

    tracing::info!(session_id = %session_id, "Created new HTTP session");

    // Build initialization result
    let result = InitializeResult {
        protocol_version: "2024-11-05".to_string(),
        capabilities: ServerCapabilities {
            tools: Some(ToolsCapability {
                list_changed: Some(false),
            }),
            resources: None,
            prompts: None,
        },
        server_info: ServerInfo {
            name: "binance-provider".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
    };

    let result_json = serde_json::to_value(result)?;

    // Note: Session ID should be returned in response headers by middleware
    // For now, include it in the response body
    let mut result_with_session = serde_json::json!(result_json);
    result_with_session["sessionId"] = serde_json::json!(session_id.to_string());

    Ok(JsonRpcResponse::success(
        result_with_session,
        request.id.unwrap_or(serde_json::json!(null)),
    ))
}

/// Handle tools/list method
///
/// Returns all 21 available tools with their JSON schemas
async fn handle_tools_list(
    _state: AppState,
    request: JsonRpcRequest,
) -> Result<JsonRpcResponse> {
    tracing::debug!("Listing all available tools");

    // Build capabilities to get tool list
    let builder = CapabilityBuilder::new();
    let capabilities = builder
        .build()
        .map_err(|e| HttpTransportError::Internal(e.to_string()))?;

    // Manually build tools JSON array from capabilities
    let tools_array: Vec<serde_json::Value> = capabilities
        .tools
        .iter()
        .map(|tool| {
            // Extract JSON schema from tool.input_schema if present
            let input_schema = tool
                .input_schema
                .as_ref()
                .and_then(|schema| serde_json::from_slice(&schema.value).ok())
                .unwrap_or(serde_json::json!({}));

            serde_json::json!({
                "name": tool.name,
                "description": tool.description,
                "inputSchema": input_schema
            })
        })
        .collect();

    Ok(JsonRpcResponse::success(
        serde_json::json!({ "tools": tools_array }),
        request.id.unwrap_or(serde_json::json!(null)),
    ))
}

/// Handle tools/call method
///
/// Routes tool invocations to the appropriate handler
async fn handle_tools_call(
    state: AppState,
    request: JsonRpcRequest,
) -> Result<JsonRpcResponse> {
    // Extract parameters
    let params = request
        .params
        .ok_or_else(|| HttpTransportError::InvalidParams("Missing params".to_string()))?;

    let tool_name = params["name"]
        .as_str()
        .ok_or_else(|| HttpTransportError::InvalidParams("Missing tool name".to_string()))?;

    let tool_args = params
        .get("arguments")
        .cloned()
        .unwrap_or(serde_json::json!({}));

    tracing::debug!(tool_name = %tool_name, "Calling tool");

    // Convert to gRPC InvokeRequest format
    let invoke_request = InvokeRequest {
        tool_name: tool_name.to_string(),
        payload: Some(PbJson {
            value: serde_json::to_string(&tool_args)?.into_bytes(),
        }),
        correlation_id: request.id.as_ref().and_then(|v| v.as_str()).unwrap_or("").to_string(),
    };

    // Route to tool handler
    #[cfg(all(feature = "orderbook", feature = "orderbook_analytics"))]
    let response = crate::grpc::tools::route_tool(
        &state.binance_client,
        state.orderbook_manager.clone(),
        state.analytics_storage.clone(),
        state.trade_storage.clone(),
        &invoke_request,
    )
    .await?;

    #[cfg(all(feature = "orderbook", not(feature = "orderbook_analytics")))]
    let response = crate::grpc::tools::route_tool(
        &state.binance_client,
        state.orderbook_manager.clone(),
        None,
        None,
        &invoke_request,
    )
    .await?;

    #[cfg(not(feature = "orderbook"))]
    let response = crate::grpc::tools::route_tool(
        &state.binance_client,
        None,
        None,
        None,
        &invoke_request,
    )
    .await?;

    // Convert response to JSON
    let result_json = if let Some(result_pb) = response.result {
        let result_str = String::from_utf8(result_pb.value)
            .map_err(|e| HttpTransportError::Internal(format!("Invalid UTF-8: {}", e)))?;
        serde_json::from_str(&result_str)?
    } else {
        serde_json::json!({ "error": response.error })
    };

    Ok(JsonRpcResponse::success(
        serde_json::json!({
            "content": [{
                "type": "text",
                "text": result_json.to_string()
            }],
            "isError": !response.error.is_empty()
        }),
        request.id.unwrap_or(serde_json::json!(null)),
    ))
}

/// Extract session ID from Mcp-Session-Id header
fn extract_session_id(headers: &HeaderMap) -> Result<Option<Uuid>> {
    if let Some(header_value) = headers.get("mcp-session-id") {
        let session_str = header_value
            .to_str()
            .map_err(|_| HttpTransportError::Session(super::session::SessionError::InvalidSessionId))?;

        let session_id = Uuid::parse_str(session_str)
            .map_err(|_| HttpTransportError::Session(super::session::SessionError::InvalidSessionId))?;

        Ok(Some(session_id))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_session_id() {
        let mut headers = HeaderMap::new();
        let uuid = Uuid::new_v4();

        headers.insert("mcp-session-id", uuid.to_string().parse().unwrap());

        let result = extract_session_id(&headers).unwrap();
        assert_eq!(result, Some(uuid));
    }

    #[test]
    fn test_extract_session_id_missing() {
        let headers = HeaderMap::new();
        let result = extract_session_id(&headers).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_session_id_invalid() {
        let mut headers = HeaderMap::new();
        headers.insert("mcp-session-id", "invalid-uuid".parse().unwrap());

        let result = extract_session_id(&headers);
        assert!(result.is_err());
    }
}
