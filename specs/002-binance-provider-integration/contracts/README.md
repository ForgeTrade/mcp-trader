# Provider Contract Reference

**Feature**: 002-binance-provider-integration
**Date**: 2025-10-18
**Purpose**: gRPC contract documentation for Binance provider

## Contract Location

The Binance provider implements the shared `provider.proto` contract located at:

```
/home/limerc/repos/ForgeTrade/mcp-trader/pkg/proto/provider.proto
```

This contract is shared across all providers in the MCP Gateway system (hello-go, binance-rs, future providers).

## Contract Overview

The provider.proto defines the `Provider` gRPC service with 5 RPCs:

```protobuf
service Provider {
  rpc ListCapabilities(google.protobuf.Empty) returns (Capabilities);
  rpc Invoke(InvokeRequest) returns (InvokeResponse);
  rpc ReadResource(ResourceRequest) returns (ResourceResponse);
  rpc GetPrompt(PromptRequest) returns (PromptResponse);
  rpc Stream(StreamRequest) returns (stream CloudEvent);
}
```

## Binance Provider Implementation

### 1. ListCapabilities RPC

**Purpose**: Discover all tools, resources, and prompts exposed by the provider

**Request**: `google.protobuf.Empty` (no parameters)

**Response**: `Capabilities`
```protobuf
message Capabilities {
  repeated Tool tools = 1;              // 16 tools
  repeated Resource resources = 2;       // 4 resources
  repeated Prompt prompts = 3;           // 2 prompts
  string provider_version = 4;           // "0.1.0"
}
```

**Binance Implementation**:
- Returns 16 tools (market data, account, trading, orderbook)
- Returns 4 resources (market data, account balances, orders)
- Returns 2 prompts (trading_analysis, portfolio_risk)
- Schemas embedded as JSON bytes in `Tool.input_schema`

**Example Response** (abbreviated):
```json
{
  "tools": [
    {
      "name": "get_ticker",
      "description": "Get 24-hour ticker statistics for a symbol",
      "input_schema": {
        "value": "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"type\":\"object\",\"properties\":{\"symbol\":{\"type\":\"string\"}},\"required\":[\"symbol\"]}"
      }
    },
    // ... 15 more tools
  ],
  "resources": [
    {
      "uri_scheme": "binance",
      "description": "Market data for trading symbols",
      "mime_type": "text/markdown"
    },
    // ... 3 more resources
  ],
  "prompts": [
    {
      "name": "trading_analysis",
      "description": "Market analysis with trading recommendations",
      "args_schema": {
        "value": "{\"type\":\"object\",\"properties\":{\"symbol\":{\"type\":\"string\"},\"strategy\":{\"type\":\"string\"},\"risk_tolerance\":{\"type\":\"string\"}},\"required\":[\"symbol\"]}"
      }
    },
    // ... 1 more prompt
  ],
  "provider_version": "0.1.0"
}
```

---

### 2. Invoke RPC

**Purpose**: Execute a tool with given arguments

**Request**: `InvokeRequest`
```protobuf
message InvokeRequest {
  string tool_name = 1;         // e.g., "get_ticker"
  Json payload = 2;             // Tool arguments as JSON bytes
  string correlation_id = 3;    // For distributed tracing
}
```

**Response**: `InvokeResponse`
```protobuf
message InvokeResponse {
  Json result = 1;              // Tool result as JSON bytes (if successful)
  string error = 2;             // Error message (if failed, result is null)
}
```

**Binance Implementation**:
- Routes `tool_name` to appropriate tool handler
- Validates `payload` against tool's JSON schema
- Calls Binance API (REST or WebSocket)
- Returns result as JSON bytes or error message
- Never exposes API secrets in error messages

**Example Request** (get_ticker):
```json
{
  "tool_name": "get_ticker",
  "payload": {
    "value": "{\"symbol\":\"BTCUSDT\"}"
  },
  "correlation_id": "abc-123-xyz"
}
```

**Example Response** (success):
```json
{
  "result": {
    "value": "{\"symbol\":\"BTCUSDT\",\"priceChange\":\"500.00\",\"priceChangePercent\":\"1.0\",\"lastPrice\":\"50500.00\",\"volume\":\"12345.67\"}"
  },
  "error": ""
}
```

**Example Response** (error):
```json
{
  "result": null,
  "error": "Invalid symbol: Symbol INVALID not found on Binance"
}
```

---

### 3. ReadResource RPC

**Purpose**: Read a resource by URI

**Request**: `ResourceRequest`
```protobuf
message ResourceRequest {
  string uri = 1;               // e.g., "binance://market/btcusdt"
  string correlation_id = 2;
}
```

**Response**: `ResourceResponse`
```protobuf
message ResourceResponse {
  bytes content = 1;            // Resource content (markdown)
  string mime_type = 2;         // "text/markdown"
  string error = 3;             // Error message (if read failed)
}
```

**Binance Implementation**:
- Parses URI format: `binance://<category>/<identifier>`
- Supported categories: `market`, `account`, `orders`
- Fetches data from Binance API (multiple endpoints if needed)
- Formats response as markdown table
- Requires authentication for `account` and `orders` categories

**Example Request**:
```json
{
  "uri": "binance://market/btcusdt",
  "correlation_id": "abc-123-xyz"
}
```

**Example Response**:
```json
{
  "content": "# BTCUSDT Market Data\n\n| Metric | Value |\n|--------|-------|\n| Last Price | $50,500.00 |\n| 24h Change | +1.0% |\n| 24h Volume | 12,345.67 BTC |\n| Best Bid | $50,499.00 |\n| Best Ask | $50,501.00 |\n",
  "mime_type": "text/markdown",
  "error": ""
}
```

**Supported URIs**:
1. `binance://market/{symbol}` - Market data for symbol (public)
2. `binance://account/balances` - Account balances (requires auth)
3. `binance://orders/open` - Open orders (requires auth)
4. `binance://account/trades` - Trade history (requires auth)

---

### 4. GetPrompt RPC

**Purpose**: Get a prompt template with arguments substituted

**Request**: `PromptRequest`
```protobuf
message PromptRequest {
  string prompt_name = 1;       // e.g., "trading_analysis"
  Json arguments = 2;           // Prompt arguments as JSON
  string correlation_id = 3;
}
```

**Response**: `PromptResponse`
```protobuf
message PromptResponse {
  repeated PromptMessage messages = 1;  // Role + content messages
  string error = 2;
}

message PromptMessage {
  string role = 1;              // "user", "assistant", "system"
  string content = 2;           // Message content
}
```

**Binance Implementation**:
- Supports 2 prompts: `trading_analysis`, `portfolio_risk`
- Validates arguments against prompt schema
- Fetches required data from Binance API (account info, market data)
- Generates structured messages with parameters substituted
- Returns multi-turn conversation format

**Example Request** (trading_analysis):
```json
{
  "prompt_name": "trading_analysis",
  "arguments": {
    "value": "{\"symbol\":\"BTCUSDT\",\"strategy\":\"swing\",\"risk_tolerance\":\"moderate\"}"
  },
  "correlation_id": "abc-123-xyz"
}
```

**Example Response**:
```json
{
  "messages": [
    {
      "role": "user",
      "content": "Analyze the BTCUSDT market for swing trading with moderate risk tolerance. Provide entry/exit recommendations based on current market conditions."
    },
    {
      "role": "assistant",
      "content": "Based on current BTCUSDT analysis:\n\nMarket Overview:\n- Current Price: $50,500\n- 24h Change: +1.0%\n- Volume: 12,345 BTC\n- RSI: 58 (neutral)\n\nSwing Trade Recommendation (Moderate Risk):\n- Entry: $49,800 - $50,200\n- Stop Loss: $48,500 (-3%)\n- Take Profit 1: $52,000 (+3%)\n- Take Profit 2: $54,000 (+7%)\n\nRationale: Market showing bullish momentum with healthy volume..."
    }
  ],
  "error": ""
}
```

**Prompt Schemas**:
1. **trading_analysis**:
   - Required: `symbol` (string)
   - Optional: `strategy` (string), `risk_tolerance` (string)

2. **portfolio_risk**:
   - No arguments (uses account data)
   - Requires authentication

---

### 5. Stream RPC (NOT IMPLEMENTED)

**Purpose**: Stream events from provider to gateway

**Status**: Not implemented by Binance provider

**Rationale**: OrderBook updates use internal WebSocket subscriptions with local cache, not streamed to gateway. This keeps the provider stateless from the gateway's perspective.

**Request**: `StreamRequest`
```protobuf
message StreamRequest {
  string topic = 1;             // e.g., "binance.orderbook.btcusdt"
}
```

**Response**: `stream CloudEvent` (server streaming)

**Binance Implementation**: Returns `UNIMPLEMENTED` status code

---

## Contract Compliance

The Binance provider fully implements 4 of 5 RPCs from provider.proto:

| RPC | Status | Notes |
|-----|--------|-------|
| ListCapabilities | ✅ Implemented | Returns 16 tools, 4 resources, 2 prompts |
| Invoke | ✅ Implemented | Routes to 16 tool handlers |
| ReadResource | ✅ Implemented | Supports 4 resource URIs |
| GetPrompt | ✅ Implemented | Supports 2 prompt templates |
| Stream | ❌ Not Implemented | OrderBook uses internal WebSocket, not streamed |

---

## Type Definitions

### Json Message

**Purpose**: Flexible JSON payload wrapper

```protobuf
message Json {
  bytes value = 1;  // Raw JSON bytes
}
```

**Usage**:
- Tool input parameters (InvokeRequest.payload)
- Tool output results (InvokeResponse.result)
- Resource arguments (if any)
- Prompt arguments (PromptRequest.arguments)
- JSON Schema definitions (Tool.input_schema, Tool.output_schema)

**Serialization**:
- Rust: `serde_json::to_vec()` / `serde_json::from_slice()`
- Python: `json.dumps().encode('utf-8')` / `json.loads(bytes.decode('utf-8'))`
- Go: `json.Marshal()` / `json.Unmarshal()`

---

## Error Handling

**Contract Pattern**: Errors returned in response message field, not gRPC status codes

**Rationale**:
- Preserves detailed error context
- Allows user-friendly messages
- Simplifies gateway error handling

**Error Response Examples**:

**Validation Error**:
```json
{
  "result": null,
  "error": "Input validation failed: missing required field 'symbol'"
}
```

**API Error**:
```json
{
  "result": null,
  "error": "Binance API error: -1121 Invalid symbol"
}
```

**Authentication Error**:
```json
{
  "result": null,
  "error": "Authentication required: Please set BINANCE_API_KEY and BINANCE_API_SECRET"
}
```

**Rate Limit Error**:
```json
{
  "result": null,
  "error": "Rate limit exceeded: 1200 weight/minute limit reached. Retry after 60 seconds"
}
```

---

## Gateway Integration

The Python gateway (`mcp_gateway/adapters/grpc_client.py`) uses the provider contract to:

1. **Discover capabilities** on startup via ListCapabilities
2. **Route tool calls** to appropriate provider via Invoke
3. **Validate inputs** against JSON schemas before invocation
4. **Cache capabilities** for fast tool lookup
5. **Handle errors** gracefully with user-friendly messages

**Gateway → Provider Flow**:
```
1. Gateway starts
2. Gateway calls ListCapabilities on each provider
3. Gateway caches tool/resource/prompt metadata
4. User calls tool via MCP
5. Gateway looks up tool → provider mapping
6. Gateway validates input against cached schema
7. Gateway calls Invoke on provider with serialized JSON
8. Provider executes and returns result/error
9. Gateway returns to user
```

---

## Testing the Contract

### With grpcurl

```bash
# List all services
grpcurl -plaintext localhost:50052 list

# Output: provider.v1.Provider

# Describe Provider service
grpcurl -plaintext localhost:50052 describe provider.v1.Provider

# Call ListCapabilities
grpcurl -plaintext -d '{}' localhost:50052 provider.v1.Provider/ListCapabilities

# Call Invoke (get_ticker)
grpcurl -plaintext -d '{
  "tool_name": "get_ticker",
  "payload": {"value": "{\"symbol\":\"BTCUSDT\"}"},
  "correlation_id": "test-001"
}' localhost:50052 provider.v1.Provider/Invoke

# Call ReadResource
grpcurl -plaintext -d '{
  "uri": "binance://market/btcusdt",
  "correlation_id": "test-002"
}' localhost:50052 provider.v1.Provider/ReadResource
```

### With Python Client

```python
from mcp_gateway.adapters.grpc_client import ProviderGRPCClient

async def test_binance_provider():
    client = ProviderGRPCClient("binance-rs", "localhost:50052")

    # List capabilities
    caps = await client.list_capabilities()
    print(f"Tools: {len(caps['tools'])}")  # Should be 16

    # Invoke tool
    result = await client.invoke("get_ticker", {"symbol": "BTCUSDT"}, "test-001")
    print(result)

    # Read resource
    content = await client.read_resource("binance://market/btcusdt", "test-002")
    print(content.decode('utf-8'))

    await client.close()
```

---

**Contract Version**: v1 (provider.v1)
**Last Updated**: 2025-10-18
**Maintained By**: MCP Gateway Team
