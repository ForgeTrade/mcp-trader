# Streamable HTTP MCP Transport Contract

**Version**: 1.0
**MCP Specification**: March 2025
**Protocol**: JSON-RPC 2.0 over HTTP

---

## Overview

This contract specifies the Streamable HTTP transport implementation for MCP (Model Context Protocol), enabling direct AI client access (ChatGPT, Claude, etc.) to the binance-rs provider without requiring the Python MCP gateway intermediary.

**Key Features**:
- Single POST `/mcp` endpoint for all MCP operations
- Session-based authentication via `Mcp-Session-Id` header
- JSON-RPC 2.0 message format
- Support for all 21 tools (16 base + 5 analytics)
- 50 concurrent session limit with 30-minute timeout

---

## Endpoint

### POST /mcp

**Base URL**: Configured via environment variables `HOST` and `PORT` (default: `http://0.0.0.0:8080`)

**Content-Type**: `application/json`

**Methods Supported**:
- `initialize` - Create session and negotiate capabilities
- `tools/list` - List all available MCP tools
- `tools/call` - Execute a specific tool

---

## Authentication

### Mcp-Session-Id Header

**Format**: UUID v4 (e.g., `a1b2c3d4-e5f6-4a5b-8c7d-9e8f7a6b5c4d`)

**Rules**:
- **NOT required** for `initialize` method (first request creates session)
- **Required** for all other methods (`tools/list`, `tools/call`)
- **Case-insensitive** header name (HTTP spec)
- **Returned** in response headers on successful `initialize`

**Session Lifecycle**:
1. **Creation**: `initialize` request → server generates UUID → returns in `Mcp-Session-Id` response header
2. **Usage**: Client includes header in all subsequent requests
3. **Expiration**: 30 minutes of inactivity (from FR-020, clarifications)
4. **Renewal**: Each valid request resets the 30-minute timer
5. **Limit**: Maximum 50 concurrent sessions (from FR-020)

**Example**:
```
POST /mcp
Mcp-Session-Id: 550e8400-e29b-41d4-a716-446655440000
Content-Type: application/json
```

---

## Request Format (JSON-RPC 2.0)

All requests must conform to JSON-RPC 2.0 specification:

```json
{
  "jsonrpc": "2.0",
  "method": "<method_name>",
  "params": { /* method-specific parameters */ },
  "id": "<request_identifier>"
}
```

### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| jsonrpc | string | Yes | Must be exactly `"2.0"` |
| method | string | Yes | One of: `initialize`, `tools/list`, `tools/call` |
| params | object | No | Method parameters (required for `tools/call`, optional for `initialize`) |
| id | string\|number\|null | Yes | Request identifier (echoed in response) |

---

## Response Format

### Success Response

```json
{
  "jsonrpc": "2.0",
  "result": { /* method-specific result */ },
  "id": "<request_identifier>"
}
```

**Headers** (on `initialize` only):
```
Mcp-Session-Id: <uuid>
Content-Type: application/json
```

### Error Response

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": <error_code>,
    "message": "<error_message>",
    "data": { /* optional additional context */ }
  },
  "id": "<request_identifier>"
}
```

**HTTP Status Codes** (from FR-021):

| HTTP Status | JSON-RPC Code | Scenario |
|-------------|---------------|----------|
| 200 OK | (none) | Success |
| 400 Bad Request | -32002 | Missing `Mcp-Session-Id` header |
| 400 Bad Request | -32700 | Parse error (invalid JSON) |
| 400 Bad Request | -32600 | Invalid request (missing required fields) |
| 400 Bad Request | -32602 | Invalid params |
| 404 Not Found | -32001 | Invalid or expired session ID |
| 404 Not Found | -32601 | Method not found |
| 503 Service Unavailable | -32000 | Session limit exceeded (50 max) |

---

## Methods

### 1. initialize

Creates a new session and negotiates protocol capabilities.

**Request**:
```json
{
  "jsonrpc": "2.0",
  "method": "initialize",
  "params": {
    "protocolVersion": "2024-11-05",
    "capabilities": {
      "tools": {}
    },
    "clientInfo": {
      "name": "ChatGPT",
      "version": "1.0"
    }
  },
  "id": "init-001"
}
```

**Response** (200 OK):
```json
{
  "jsonrpc": "2.0",
  "result": {
    "protocolVersion": "2024-11-05",
    "capabilities": {
      "tools": {}
    },
    "serverInfo": {
      "name": "binance-rs-provider",
      "version": "0.1.0"
    }
  },
  "id": "init-001"
}
```

**Response Headers**:
```
Mcp-Session-Id: 550e8400-e29b-41d4-a716-446655440000
Content-Type: application/json
```

**Error Cases**:
- 503 Service Unavailable (code -32000): Session limit exceeded (50 concurrent sessions)

---

### 2. tools/list

Returns all available MCP tools with their schemas.

**Request**:
```json
{
  "jsonrpc": "2.0",
  "method": "tools/list",
  "params": {},
  "id": "list-001"
}
```
*Requires `Mcp-Session-Id` header*

**Response** (200 OK):
```json
{
  "jsonrpc": "2.0",
  "result": {
    "tools": [
      {
        "name": "binance.get_order_flow",
        "description": "Calculate order flow dynamics...",
        "inputSchema": {
          "type": "object",
          "properties": {
            "symbol": {"type": "string", "pattern": "^[A-Z]+$"},
            "window_duration_secs": {"type": "integer", "minimum": 10, "maximum": 300, "default": 60}
          },
          "required": ["symbol"]
        }
      },
      {
        "name": "binance.get_volume_profile",
        "description": "Generate volume distribution histogram...",
        "inputSchema": { /* ... */ }
      }
      /* ... 19 more tools (total 21: 16 base + 5 analytics) ... */
    ]
  },
  "id": "list-001"
}
```

**Error Cases**:
- 400 Bad Request (code -32002): Missing `Mcp-Session-Id` header
- 404 Not Found (code -32001): Invalid or expired session ID

---

### 3. tools/call

Executes a specific tool with provided arguments.

**Request**:
```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "binance.get_order_flow",
    "arguments": {
      "symbol": "BTCUSDT",
      "window_duration_secs": 60
    }
  },
  "id": "call-001"
}
```
*Requires `Mcp-Session-Id` header*

**Response** (200 OK):
```json
{
  "jsonrpc": "2.0",
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\"symbol\":\"BTCUSDT\",\"time_window_start\":\"2025-10-19T14:30:00Z\",\"time_window_end\":\"2025-10-19T14:31:00Z\",\"window_duration_secs\":60,\"bid_flow_rate\":45.2,\"ask_flow_rate\":18.7,\"net_flow\":26.5,\"flow_direction\":\"StrongBuy\",\"cumulative_delta\":1250.5}"
      }
    ],
    "isError": false
  },
  "id": "call-001"
}
```

**Error Response** (tool execution failure):
```json
{
  "jsonrpc": "2.0",
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Error: insufficient_historical_data - Need 45 more snapshots for 60s window analysis"
      }
    ],
    "isError": true
  },
  "id": "call-001"
}
```
*Note: Tool execution errors return 200 OK with `isError: true` in result, not JSON-RPC error*

**Error Cases** (protocol/transport errors):
- 400 Bad Request (code -32002): Missing `Mcp-Session-Id` header
- 400 Bad Request (code -32602): Invalid arguments (schema validation failure)
- 404 Not Found (code -32001): Invalid or expired session ID
- 404 Not Found (code -32601): Tool name not found

---

## Session Management

### Session Storage

**Implementation**: In-memory `Arc<DashMap<Uuid, StreamableHttpSession>>` (from research.md)

**Capacity**: 50 concurrent sessions (from FR-020)

**Timeout**: 30 minutes from last activity (from FR-020, clarifications)

**Cleanup**: Background task runs every 5 minutes to remove expired sessions

### Session Data Structure

```rust
pub struct StreamableHttpSession {
    pub session_id: Uuid,
    pub client_metadata: ClientMetadata,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

pub struct ClientMetadata {
    pub ip_address: IpAddr,
    pub user_agent: Option<String>,
}
```

### Session Validation Flow

```
Client Request → Extract Mcp-Session-Id header
                 ↓
              Session exists in DashMap?
              ↓                    ↓
            Yes                   No
              ↓                    ↓
      Expired (now > expires_at)? 404 (code -32002)
      ↓                    ↓
    Yes                  No
      ↓                    ↓
404 (-32001)      Update last_activity
                  Reset expires_at
                  Process request
```

---

## CORS Configuration

**Policy**: Permissive (from research.md)

**Headers**:
```
Access-Control-Allow-Origin: *
Access-Control-Allow-Methods: POST, OPTIONS
Access-Control-Allow-Headers: Content-Type, Mcp-Session-Id
Access-Control-Max-Age: 86400
```

**Rationale**: Enables browser-based MCP clients (ChatGPT web interface, custom UIs)

---

## Health Check Endpoint

### GET /health

**Purpose**: Kubernetes/Docker health monitoring

**Response** (200 OK):
```json
{
  "status": "healthy",
  "active_sessions": 12,
  "max_sessions": 50,
  "uptime_seconds": 86400
}
```

**No authentication required** (public endpoint)

---

## Environment Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| HOST | `0.0.0.0` | Bind address |
| PORT | `8080` | HTTP port |
| BINANCE_API_KEY | (required) | Binance API credentials |
| BINANCE_SECRET_KEY | (required) | Binance API credentials |
| LOG_LEVEL | `info` | Logging level (trace, debug, info, warn, error) |

**Example**:
```bash
HOST=127.0.0.1 PORT=3000 ./binance-provider --mode http
```

---

## Usage Examples

### ChatGPT Connector Configuration

```json
{
  "schema_version": "v1",
  "name_for_human": "Binance Analytics",
  "name_for_model": "binance_analytics",
  "description_for_human": "Real-time cryptocurrency market analytics",
  "description_for_model": "Provides order flow, volume profile, anomaly detection, and microstructure health analysis for Binance cryptocurrency markets",
  "api": {
    "type": "mcp_streamable_http",
    "url": "https://your-server.com/mcp"
  }
}
```

### cURL Testing

**1. Initialize Session**:
```bash
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "initialize",
    "params": {},
    "id": "1"
  }' \
  -i  # -i to see headers
```

**Response**:
```
HTTP/1.1 200 OK
Mcp-Session-Id: 550e8400-e29b-41d4-a716-446655440000
Content-Type: application/json

{"jsonrpc":"2.0","result":{...},"id":"1"}
```

**2. List Tools**:
```bash
SESSION_ID="550e8400-e29b-41d4-a716-446655440000"
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/list",
    "params": {},
    "id": "2"
  }'
```

**3. Call Order Flow Tool**:
```bash
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "binance.get_order_flow",
      "arguments": {
        "symbol": "BTCUSDT",
        "window_duration_secs": 120
      }
    },
    "id": "3"
  }'
```

---

## Security Considerations

### No Additional Authentication

- HTTP transport relies on existing Binance API credentials (same as gRPC mode)
- No OAuth, API keys, or custom auth mechanisms for MCP protocol
- Session ID is **not** a security token (only prevents accidental session mixing)

### HTTPS Deployment

- **HTTP mode runs on plain HTTP** (no TLS/SSL in binary)
- **Production deployments MUST use reverse proxy** (nginx, Traefik, Kubernetes Ingress) for HTTPS termination
- **Session IDs transmitted over plain HTTP are visible** to network observers

### Rate Limiting

- No built-in rate limiting per session (relies on Binance API rate limits)
- Deployment platform should implement external rate limiting if needed

---

## Limitations & Non-Goals

### What This Transport Does NOT Support

- ❌ **WebSocket transport** (only HTTP request-response)
- ❌ **Streaming SSE responses** (long-running operations return when complete)
- ❌ **Persistent sessions across provider restarts** (sessions are in-memory only)
- ❌ **DELETE /mcp endpoint for explicit session termination** (sessions expire via timeout)
- ❌ **Load balancing across multiple provider instances** (single-instance deployment)
- ❌ **Custom authentication beyond Binance API credentials**

### Scope Boundaries (from spec.md)

- **In scope**: Direct AI client access (ChatGPT, Claude) without Python gateway
- **Out of scope**: Migration of existing Python MCP gateway to HTTP (gateway remains gRPC-only)

---

## Compatibility

### Tool Set Compatibility

**100% compatibility** between gRPC and HTTP transports (from FR-025, SC-014):
- Same 21 tools available in both modes
- Same JSON schemas and validation
- Same error messages
- Tool implementations are shared (transport layer is abstraction only)

### Feature Flag Independence

**http_transport is independent of orderbook_analytics** (from clarifications Q3, FR-022):
- `http_transport` alone: Exposes 16 base tools via HTTP (no analytics)
- `http_transport + orderbook_analytics`: Exposes all 21 tools via HTTP
- `orderbook_analytics` alone: Analytics available only via gRPC

---

## Reference Implementation

See `providers/binance-rs/src/transport/http/` for:
- `mod.rs`: Axum router setup
- `handler.rs`: POST /mcp endpoint implementation
- `session.rs`: Session management with DashMap
- `jsonrpc.rs`: JSON-RPC 2.0 message routing
- `error.rs`: Error response formatting

**Main binary**: `providers/binance-rs/src/main.rs` with `--mode http` flag

---

**Contract Version**: 1.0
**Last Updated**: 2025-10-19
**Specification Compliance**: MCP March 2025, JSON-RPC 2.0
