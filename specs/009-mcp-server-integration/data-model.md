# Data Model: MCP Server Integration

**Feature**: 009-mcp-server-integration
**Date**: 2025-10-20

## Overview

This document defines the data entities for the MCP server integration. All entities are ephemeral (in-memory only) with no persistent storage.

## Core Entities

### 1. MCP Session

Represents an active connection between an MCP client and the Binance provider via SSE transport.

**Attributes**:
- `connection_id`: UUID (v4) - Unique session identifier
- `created_at`: DateTime<Utc> - Session creation timestamp
- `last_activity`: DateTime<Utc> - Last request timestamp (for timeout tracking)
- `remote_addr`: String - Client IP address
- `user_agent`: Option<String> - Client user agent string
- `credentials`: Option<BinanceCredentials> - Per-session API credentials

**Lifecycle**:
- Created: On SSE handshake (POST /mcp/sse)
- Updated: On each MCP request (update last_activity)
- Destroyed: After 30 seconds idle or explicit disconnect

**Storage**: In-memory `Arc<RwLock<HashMap<ConnectionId, SessionMetadata>>>`

**Relationships**:
- One session can have many WebSocket subscriptions (via orderbook manager)
- Sessions are independent (no cross-session data sharing)

---

### 2. MCP Resource

Represents a data endpoint accessible via URI scheme (binance://category/identifier).

**Attributes**:
- `uri`: String - Full resource URI (e.g., "binance://market/btcusdt")
- `category`: ResourceCategory enum - Resource type (Market, Account, Orders)
- `identifier`: Option<String> - Resource-specific ID (e.g., symbol, account)
- `mime_type`: String - Content MIME type (always "text/markdown")
- `name`: String - Human-readable name
- `description`: Option<String> - Resource description

**Categories**:
```rust
pub enum ResourceCategory {
    Market,   // Market data resources (ticker, orderbook)
    Account,  // Account information (balances, positions)
    Orders,   // Order management (open orders, history)
}
```

**URI Format**:
```
binance://{category}/{identifier}

Examples:
- binance://market/btcusdt
- binance://account/balances
- binance://orders/open
```

**Content Format**: Markdown text with formatted data (tables, lists, headers)

**Lifecycle**: Stateless - no persistent state, generated on each read

---

### 3. MCP Prompt

Represents a guided analysis workflow template with parameters.

**Attributes**:
- `name`: String - Prompt identifier (e.g., "trading_analysis")
- `description`: String - Human-readable description
- `parameters`: Vec<PromptParameter> - Required/optional parameters
- `message_template`: String - Markdown template for prompt content

**Parameter Types**:
```rust
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct SymbolParam {
    pub symbol: String,  // Trading pair (e.g., "BTCUSDT")
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct TradingAnalysisArgs {
    pub symbol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<TradingStrategy>,
}

pub enum TradingStrategy {
    Aggressive,
    Balanced,
    Conservative,
}
```

**Prompt List**:
1. `trading_analysis` - Market analysis and trading recommendations
2. `portfolio_risk` - Portfolio risk assessment
3. `market_microstructure_analysis` - Advanced microstructure analysis (requires orderbook_analytics feature)
4. `order_flow_trading` - Order flow-based signals
5. `liquidity_mapping` - Liquidity vacuum and wall identification

**Lifecycle**: Stateless - generated on each request based on current market data

---

### 4. Transport Mode

Represents the communication protocol used for MCP messages.

**Variants**:
```rust
pub enum TransportMode {
    Stdio,  // Standard I/O (local)
    Sse,    // Server-Sent Events over HTTPS (remote)
    Grpc,   // gRPC (existing, not MCP)
}
```

**Selection**: Via command-line flags at startup
```bash
binance-provider --stdio       # MCP via stdio
binance-provider --sse --port 3000  # MCP via SSE
binance-provider --grpc --port 50053  # gRPC (existing)
```

**Mutually Exclusive**: Only one transport mode active at a time

---

## Supporting Types

### Connection ID
```rust
pub struct ConnectionId(pub Uuid);
```
- UUID v4 for uniqueness
- Used as session key in SessionManager HashMap

### Session Metadata
```rust
pub struct SessionMetadata {
    pub connection_id: ConnectionId,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub remote_addr: String,
    pub user_agent: Option<String>,
    pub credentials: Option<BinanceCredentials>,
}
```

### Resource URI
```rust
pub struct ResourceUri {
    pub scheme: String,        // Always "binance"
    pub category: ResourceCategory,
    pub identifier: Option<String>,
}
```

---

## Data Flow

### Stdio Transport
```
Client → stdin → BinanceServer → Tool/Resource/Prompt Handler → stdout → Client
```

### SSE Transport
```
Client → POST /mcp/sse → SessionManager (create session) → return Connection-ID
Client → POST /mcp/message (with Connection-ID) → SessionManager (lookup) → BinanceServer → Tool/Resource/Prompt Handler → JSON response
```

---

## Validation Rules

1. **Session Timeout**: Sessions idle >30s are automatically cleaned up
2. **Max Sessions**: Maximum 50 concurrent SSE sessions
3. **URI Format**: Must match pattern `binance://{category}/{identifier}`
4. **Category Values**: Must be one of: market, account, orders
5. **Credentials**: Optional but required for authenticated operations

---

**Notes**:
- All timestamps use UTC timezone
- No persistent storage - all data in memory
- Session cleanup runs every 30 seconds via heartbeat task
- Credentials are per-session and never logged or persisted to disk
