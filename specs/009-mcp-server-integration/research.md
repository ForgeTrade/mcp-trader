# MCP Implementation Patterns Research

**Source**: Analysis of mcp-binance-rs project (rmcp SDK 0.8.1)
**Date**: 2025-10-20

## Summary

The mcp-binance-rs project provides comprehensive MCP server implementation using rmcp SDK 0.8.1 with macro-driven routing, dual transport support (stdio/SSE), and production-ready session management.

## Key Findings

### 1. rmcp SDK 0.8.1 Architecture

**Decision**: Use proc macros for automatic tool/prompt routing
**Rationale**: Eliminates boilerplate, ensures type safety, generates JSON schemas automatically
**Alternatives considered**: Manual routing rejected - error-prone and maintenance burden

**Pattern**:
```rust
#[tool_router(vis = "pub")]
impl BinanceServer {
    #[tool(description = "Get 24hr ticker")]
    pub async fn get_ticker(&self, params: Parameters<SymbolParam>) -> Result<Call Tool Result, ErrorData>
}
```

**Dependencies**:
- rmcp = { version = "0.8.1", features = ["server", "macros", "transport-io", "transport-sse-server"] }
- schemars = "1.0.4" (JSON schema generation)

### 2. Stdio Transport

**Decision**: Use rmcp built-in stdio transport
**Rationale**: Single-line initialization, automatic stdin/stdout handling, proven in production
**Alternatives considered**: Custom transport rejected - unnecessary complexity

**Pattern**:
```rust
let service = BinanceServer::new().serve(stdio()).await?;
service.waiting().await?;
```

**Key requirement**: Logs must go to stderr (stdout reserved for MCP protocol)

### 3. SSE Transport (Streamable HTTP)

**Decision**: Axum HTTP server with POST-only endpoints and session headers
**Rationale**: rmcp SDK provides built-in SSE support, matches March 2025 spec
**Alternatives considered**: Custom WebSocket transport rejected - SSE sufficient for requirements

**Architecture**:
- POST /mcp - Main endpoint for all MCP operations
- POST /mcp/message - Backward compatibility
- Session management: Arc<RwLock<HashMap<ConnectionId, SessionMetadata>>>
- Timeout: 30 seconds idle, 50 max concurrent connections
- Heartbeat task: Cleanup stale sessions every 30s

**Pattern**:
```rust
pub struct SseConfig {
    pub bind: SocketAddr,
    pub keep_alive: Option<Duration>,  // 30s default
    pub cancellation_token: CancellationToken,
}
```

### 4. MCP Resources

**Decision**: Custom URI scheme (binance://category/identifier)
**Rationale**: Clean, hierarchical, easy to parse and validate
**Alternatives considered**: Flat URIs rejected - less intuitive for users

**URI Design**:
```
binance://market/btcusdt        - Market data
binance://account/balances      - Account balances
binance://orders/open           - Open orders
```

**Pattern**:
```rust
pub enum ResourceCategory { Market, Account, Orders }

impl ResourceUri {
    pub fn parse(uri: &str) -> Result<Self, String>
}
```

**Content Format**: Markdown (text/markdown MIME type) for all resources

### 5. MCP Prompts

**Decision**: Macro-driven prompts with JsonSchema parameter types
**Rationale**: Automatic parameter validation, schema generation, type safety
**Alternatives considered**: String-based prompts rejected - no type safety

**Pattern**:
```rust
#[prompt(name = "trading_analysis", description = "Analyze market conditions")]
pub async fn trading_analysis(
    &self,
    Parameters(args): Parameters<TradingAnalysisArgs>,
) -> Result<GetPromptResult, ErrorData> {
    // Fetch data, format markdown, return prompt
}
```

**Parameter Types**:
```rust
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct TradingAnalysisArgs {
    pub symbol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<TradingStrategy>,
}
```

## Best Practices Identified

1. **Error Handling**: Convert all errors to `ErrorData::internal_error()` with recovery suggestions
2. **Logging**: Use tracing with stderr output (stdout reserved for stdio transport)
3. **Feature Flags**: Conditional compilation for optional features (orderbook_analytics, SSE)
4. **Session Isolation**: Per-session credential storage, automatic cleanup on timeout
5. **Type Safety**: Heavy use of JsonSchema for parameter validation
6. **Reusability**: Wrap existing tool implementations with MCP protocol layer

## Implementation Plan

See `plan.md` for detailed implementation steps based on these patterns.
