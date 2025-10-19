# Data Model

**Feature**: 003-specify-scripts-bash (Advanced Order Book Analytics & Streamable HTTP Transport)
**Date**: 2025-10-19
**Status**: Complete

## Overview

This document specifies all data entities, their fields, validation rules, relationships, and lifecycle states for the Advanced Order Book Analytics and Streamable HTTP Transport feature.

---

## Analytics Domain Models

### 1. OrderFlowSnapshot

**Purpose**: Captures bid/ask pressure dynamics over a time window for trade timing analysis

**Rust Definition**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderFlowSnapshot {
    pub symbol: String,
    pub time_window_start: DateTime<Utc>,
    pub time_window_end: DateTime<Utc>,
    pub window_duration_secs: u32,
    pub bid_flow_rate: f64,
    pub ask_flow_rate: f64,
    pub net_flow: f64,
    pub flow_direction: FlowDirection,
    pub cumulative_delta: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FlowDirection {
    StrongBuy,
    ModerateBuy,
    Neutral,
    ModerateSell,
    StrongSell,
}
```

**Field Specifications**:

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| symbol | String | Non-empty, uppercase (e.g., "BTCUSDT") | Trading pair symbol |
| time_window_start | DateTime<Utc> | Must be <= time_window_end | Start of analysis window (UTC) |
| time_window_end | DateTime<Utc> | Must be >= time_window_start | End of analysis window (UTC) |
| window_duration_secs | u32 | 10 ≤ value ≤ 300 (from clarifications) | Duration in seconds |
| bid_flow_rate | f64 | >= 0.0 | Bid orders per second |
| ask_flow_rate | f64 | >= 0.0 | Ask orders per second |
| net_flow | f64 | = bid_flow_rate - ask_flow_rate | Net pressure |
| flow_direction | FlowDirection | Must match threshold rules | Enum variant |
| cumulative_delta | f64 | Running sum | Buy volume - sell volume over window |

**Validation Rules**:
- `time_window_end - time_window_start == window_duration_secs` (within 1s tolerance)
- `flow_direction` must match thresholds from FR-003:
  - `StrongBuy`: bid_flow_rate / ask_flow_rate > 2.0
  - `ModerateBuy`: 1.2 < ratio <= 2.0
  - `Neutral`: 0.8 < ratio <= 1.2
  - `ModerateSell`: 0.5 < ratio <= 0.8
  - `StrongSell`: ratio <= 0.5

**Serialization**: JSON for MCP tool responses

**Example**:
```json
{
  "symbol": "BTCUSDT",
  "time_window_start": "2025-10-19T14:30:00Z",
  "time_window_end": "2025-10-19T14:31:00Z",
  "window_duration_secs": 60,
  "bid_flow_rate": 45.2,
  "ask_flow_rate": 18.7,
  "net_flow": 26.5,
  "flow_direction": "StrongBuy",
  "cumulative_delta": 1250.5
}
```

---

### 2. VolumeProfile

**Purpose**: Histogram of traded volume across price levels for support/resistance identification

**Rust Definition**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeProfile {
    pub symbol: String,
    pub histogram: Vec<VolumeBin>,
    pub bin_size: Decimal,
    pub point_of_control: Decimal,
    pub value_area_high: Decimal,
    pub value_area_low: Decimal,
    pub total_volume: Decimal,
    pub time_period_hours: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeBin {
    pub price_level: Decimal,
    pub volume: Decimal,
    pub trade_count: u64,
    pub percentage_of_total: f64,
}
```

**Field Specifications**:

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| symbol | String | Non-empty, uppercase | Trading pair |
| histogram | Vec<VolumeBin> | Sorted by price_level (ascending), non-empty | Price-volume bins |
| bin_size | Decimal | > 0, adaptive (tick × 10 or price_range / 100) | Price increment per bin |
| point_of_control | Decimal | Must be in histogram range | Price with highest volume |
| value_area_high | Decimal | >= value_area_low | Upper 70% volume boundary |
| value_area_low | Decimal | <= value_area_high | Lower 70% volume boundary |
| total_volume | Decimal | Sum of all bin volumes | Total traded volume |
| time_period_hours | u32 | 1 ≤ value ≤ 168 (from FR-004) | Analysis duration |

**VolumeBin Constraints**:
- `price_level` must be unique within histogram
- `volume` >= 0
- `trade_count` >= 0
- `percentage_of_total` = (bin.volume / total_volume) × 100
- Sum of all `percentage_of_total` ≈ 100.0 (within 0.01% tolerance)

**Validation Rules**:
- `point_of_control` must equal the price_level of the bin with maximum volume
- `value_area_high` and `value_area_low` must bound exactly 70% of total_volume
- `histogram` must be sorted ascending by price_level

**Calculation**:
- Bin size = max(exchange_tick_size × 10, price_range / 100)

**Example**:
```json
{
  "symbol": "ETHUSDT",
  "histogram": [
    {"price_level": "3490.00", "volume": "125.5", "trade_count": 342, "percentage_of_total": 8.2},
    {"price_level": "3500.00", "volume": "680.3", "trade_count": 1823, "percentage_of_total": 44.5},
    {"price_level": "3510.00", "volume": "220.1", "trade_count": 567, "percentage_of_total": 14.4}
  ],
  "bin_size": "10.00",
  "point_of_control": "3500.00",
  "value_area_high": "3510.00",
  "value_area_low": "3490.00",
  "total_volume": "1528.9",
  "time_period_hours": 24
}
```

---

### 3. MarketMicrostructureAnomaly

**Purpose**: Detected abnormal market behavior (quote stuffing, icebergs, flash crash risks)

**Rust Definition**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketMicrostructureAnomaly {
    pub anomaly_id: Uuid,
    pub anomaly_type: AnomalyType,
    pub severity: Severity,
    pub confidence: f64,
    pub timestamp: DateTime<Utc>,
    pub affected_price_levels: Vec<Decimal>,
    pub description: String,
    pub recommended_action: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AnomalyType {
    QuoteStuffing,
    IcebergOrder,
    FlashCrashRisk,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}
```

**Field Specifications**:

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| anomaly_id | Uuid | v4 UUID | Unique anomaly identifier |
| anomaly_type | AnomalyType | One of 3 variants | Classification |
| severity | Severity | Low/Medium/High/Critical | Risk level |
| confidence | f64 | 0.0 ≤ value ≤ 1.0 | Detection confidence |
| timestamp | DateTime<Utc> | When detected | Detection time |
| affected_price_levels | Vec<Decimal> | May be empty for flash crash | Impacted prices |
| description | String | Non-empty, < 500 chars | Human-readable explanation |
| recommended_action | String | Non-empty, < 500 chars | Actionable guidance |

**Validation Rules**:
- Only report anomalies with `confidence >= 0.95` (from FR-009, clarifications)
- **QuoteStuffing severity thresholds** (from FR-008):
  - Medium: 500-750 updates/sec
  - High: 750-1000 updates/sec
  - Critical: >1000 updates/sec
- **IcebergOrder detection**: z-score > 1.96 (from clarifications), refill rate >5× median
- **FlashCrashRisk triggers**: Requires ALL of:
  - Liquidity drain: >80% depth loss in <1s
  - Spread widening: >10× rolling 24h average (from clarifications)
  - Cancellation rate: >90% of updates

**Example**:
```json
{
  "anomaly_id": "550e8400-e29b-41d4-a716-446655440000",
  "anomaly_type": "QuoteStuffing",
  "severity": "High",
  "confidence": 0.98,
  "timestamp": "2025-10-19T14:35:22Z",
  "affected_price_levels": [],
  "description": "Quote stuffing detected: 850 orderbook updates/sec with 8% fill rate - potential HFT manipulation",
  "recommended_action": "Delay order execution, widen limit order spreads"
}
```

---

### 4. MicrostructureHealthScore

**Purpose**: Composite 0-100 market health assessment

**Rust Definition**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicrostructureHealthScore {
    pub score: u8,
    pub components: HealthComponents,
    pub interpretation: HealthInterpretation,
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthComponents {
    pub spread_stability: u8,
    pub liquidity_depth: u8,
    pub flow_balance: u8,
    pub update_rate: u8,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum HealthInterpretation {
    Excellent,  // 80-100
    Good,       // 60-79
    Fair,       // 40-59
    Poor,       // 20-39
    Critical,   // 0-19
}
```

**Field Specifications**:

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| score | u8 | 0 ≤ value ≤ 100 | Composite health score |
| components | HealthComponents | Each component 0-100 | Sub-scores |
| interpretation | HealthInterpretation | Maps from score | Health category |
| timestamp | DateTime<Utc> | Calculation time | When measured |
| symbol | String | Non-empty | Trading pair |

**Component Weights** (from FR-011):
- spread_stability: 25%
- liquidity_depth: 35%
- flow_balance: 25%
- update_rate: 15%

**Calculation**:
```
score = (spread_stability × 0.25) + (liquidity_depth × 0.35) + (flow_balance × 0.25) + (update_rate × 0.15)
```

**Interpretation Mapping**:
| Score Range | Interpretation |
|-------------|----------------|
| 80-100 | Excellent |
| 60-79 | Good |
| 40-59 | Fair |
| 20-39 | Poor |
| 0-19 | Critical |

**Example**:
```json
{
  "score": 72,
  "components": {
    "spread_stability": 85,
    "liquidity_depth": 68,
    "flow_balance": 70,
    "update_rate": 75
  },
  "interpretation": "Good",
  "timestamp": "2025-10-19T14:40:00Z",
  "symbol": "BTCUSDT"
}
```

---

### 5. LiquidityVacuum

**Purpose**: Low-volume price zones prone to rapid price movement

**Rust Definition**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityVacuum {
    pub vacuum_id: Uuid,
    pub symbol: String,
    pub price_range_low: Decimal,
    pub price_range_high: Decimal,
    pub volume_deficit_pct: f64,
    pub severity: Severity,
    pub expected_impact: ImpactLevel,
    pub detection_timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ImpactLevel {
    FastMovement,      // Expect rapid price discovery
    ModerateMovement,  // Moderate speed
}
```

**Field Specifications**:

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| vacuum_id | Uuid | v4 UUID | Unique vacuum identifier |
| symbol | String | Non-empty | Trading pair |
| price_range_low | Decimal | < price_range_high | Lower boundary |
| price_range_high | Decimal | > price_range_low | Upper boundary |
| volume_deficit_pct | f64 | > 20.0 (detection threshold) | % below median |
| severity | Severity | Medium/High/Critical | Risk level |
| expected_impact | ImpactLevel | Based on deficit | Movement speed |
| detection_timestamp | DateTime<Utc> | When found | Detection time |

**Validation Rules**:
- `price_range_low < price_range_high` (strictly less than)
- `volume_deficit_pct > 20.0` (minimum threshold for detection from FR-012)

**Severity Mapping**:
| Volume Deficit | Severity |
|----------------|----------|
| >80% | Critical |
| 50-80% | High |
| 20-50% | Medium |

**Example**:
```json
{
  "vacuum_id": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
  "symbol": "SOLUSDT",
  "price_range_low": "145.50",
  "price_range_high": "148.20",
  "volume_deficit_pct": 85.0,
  "severity": "Critical",
  "expected_impact": "FastMovement",
  "detection_timestamp": "2025-10-19T14:45:00Z"
}
```

---

### 6. AbsorptionEvent

**Purpose**: Large order repeatedly absorbing market pressure without price movement

**Rust Definition**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbsorptionEvent {
    pub event_id: Uuid,
    pub symbol: String,
    pub price_level: Decimal,
    pub absorbed_volume: Decimal,
    pub refill_count: u32,
    pub suspected_entity: EntityType,
    pub direction: Direction,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum EntityType {
    MarketMaker,
    Whale,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Direction {
    Accumulation,  // Buying
    Distribution,  // Selling
}
```

**Field Specifications**:

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| event_id | Uuid | v4 UUID | Unique event identifier |
| symbol | String | Non-empty | Trading pair |
| price_level | Decimal | > 0 | Price where absorption occurs |
| absorbed_volume | Decimal | > 0, >5× median size (from FR-014) | Total absorbed |
| refill_count | u32 | > 0 | Times order refilled |
| suspected_entity | EntityType | Classification heuristic | Actor type |
| direction | Direction | Accumulation or Distribution | Buy vs sell |
| timestamp | DateTime<Utc> | First absorption detected | Event start |

**Detection Criteria** (from FR-014):
- Order size >5× median size for symbol
- Repeatedly absorbs market orders without price moving through level
- Refills after partial fills (indicates large hidden order)

**Example**:
```json
{
  "event_id": "3f333df6-90a4-4fda-8dd3-9485d27cee36",
  "symbol": "SOLUSDT",
  "price_level": "144.00",
  "absorbed_volume": "250.0",
  "refill_count": 8,
  "suspected_entity": "Whale",
  "direction": "Accumulation",
  "timestamp": "2025-10-19T14:50:00Z"
}
```

---

## Transport Domain Models

### 7. StreamableHttpSession

**Purpose**: Tracks active MCP HTTP client connection state

**Rust Definition**:
```rust
#[derive(Debug, Clone)]
pub struct StreamableHttpSession {
    pub session_id: Uuid,
    pub client_metadata: ClientMetadata,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ClientMetadata {
    pub ip_address: IpAddr,
    pub user_agent: Option<String>,
}
```

**Field Specifications**:

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| session_id | Uuid | v4 UUID from `Mcp-Session-Id` header | Unique session ID |
| client_metadata | ClientMetadata | IP required, user-agent optional | Client info |
| created_at | DateTime<Utc> | Immutable after creation | Session start |
| last_activity | DateTime<Utc> | Updated on each request | Last request time |
| expires_at | DateTime<Utc> | created_at + 30 minutes (from FR-020) | Expiration time |

**Lifecycle**:
1. **Create**: On first `initialize` request → generate session_id, set created_at, expires_at
2. **Update**: On each subsequent request → update last_activity
3. **Expire**: When `Utc::now() > expires_at` → remove from session store
4. **Cleanup**: Background task runs every 5 minutes, removes expired sessions

**Storage**: In-memory `Arc<DashMap<Uuid, StreamableHttpSession>>` (from research.md)

**Limits** (from FR-020):
- Maximum 50 concurrent sessions
- 30-minute timeout from last activity

**Example** (internal representation, not serialized to client):
```rust
StreamableHttpSession {
    session_id: Uuid::parse_str("a1b2c3d4-e5f6-4a5b-8c7d-9e8f7a6b5c4d").unwrap(),
    client_metadata: ClientMetadata {
        ip_address: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
        user_agent: Some("ChatGPT-MCP/1.0".to_string()),
    },
    created_at: Utc.with_ymd_and_hms(2025, 10, 19, 14, 30, 0).unwrap(),
    last_activity: Utc.with_ymd_and_hms(2025, 10, 19, 14, 45, 0).unwrap(),
    expires_at: Utc.with_ymd_and_hms(2025, 10, 19, 15, 0, 0).unwrap(),
}
```

---

### 8. McpJsonRpcMessage

**Purpose**: JSON-RPC 2.0 formatted MCP message container

**Rust Definition**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpJsonRpcRequest {
    pub jsonrpc: String,  // Always "2.0"
    pub method: String,
    pub params: Option<serde_json::Value>,
    pub id: RequestId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpJsonRpcResponse {
    pub jsonrpc: String,  // Always "2.0"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: RequestId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    String(String),
    Number(i64),
    Null,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}
```

**Field Specifications**:

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| jsonrpc | String | Must equal "2.0" | JSON-RPC version |
| method | String | "initialize" / "tools/list" / "tools/call" | MCP method |
| params | Option<Value> | Required for tools/call | Method parameters |
| id | RequestId | String, number, or null | Request identifier |
| result | Option<Value> | Mutually exclusive with error | Success response |
| error | Option<JsonRpcError> | Mutually exclusive with result | Error response |

**Error Codes** (from FR-021):
| Code | HTTP Status | Meaning |
|------|-------------|---------|
| -32002 | 400 | Missing `Mcp-Session-Id` header |
| -32001 | 404 | Invalid/expired session ID |
| -32000 | 503 | Session limit exceeded (50 max) |
| -32700 | 400 | Parse error (invalid JSON) |
| -32600 | 400 | Invalid request (missing fields) |
| -32601 | 404 | Method not found |
| -32602 | 400 | Invalid params |

**Validation Rules**:
- `jsonrpc` must exactly match "2.0"
- Response must have exactly one of `result` or `error`, not both
- `id` must match the request `id`

**Example Request**:
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
  "id": "req-001"
}
```

**Example Success Response**:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\"symbol\":\"BTCUSDT\",\"bid_flow_rate\":45.2,...}"
      }
    ],
    "isError": false
  },
  "id": "req-001"
}
```

**Example Error Response**:
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32002,
    "message": "Missing Mcp-Session-Id header"
  },
  "id": "req-001"
}
```

---

## Storage Schema

### RocksDB Key-Value Store (for analytics snapshots)

**Key Format** (from research.md):
- Binary encoding: `{symbol_bytes[6]}:{unix_timestamp_u64_be[8]}`
- Total: 14 bytes per key
- Example: `"BTCUSDT"` (padded to 6) + timestamp 1729350000 → `[66,84,67,85,83,68,0,0,0,1,115,30,155,144]`

**Value Format**:
- MessagePack-serialized `OrderbookSnapshot` (not detailed here, exists in base orderbook feature)
- Zstd compressed
- Average size: ~3KB per snapshot (from research.md)

**Index**: Prefix-based scan using symbol (first 6 bytes) for efficient time-range queries

**Retention**: 7-day rolling window, automatic cleanup via background thread

**Size Limit**: 1GB hard limit (from clarifications Q5), fail new writes with `storage_limit_exceeded` error

---

## Entity Relationships

```
OrderFlowSnapshot
    └─ calculated_from → RocksDB snapshots (time-series)
    └─ may_contain → AbsorptionEvent (embedded in response)

VolumeProfile
    └─ calculated_from → Binance @aggTrade stream
    └─ identifies → LiquidityVacuum (embedded in response)

MarketMicrostructureAnomaly
    └─ detected_from → OrderbookSnapshot + trade data
    └─ references → affected_price_levels (prices)

MicrostructureHealthScore
    └─ computed_from → OrderFlowSnapshot + spread + depth metrics

StreamableHttpSession
    └─ validates → McpJsonRpcRequest (session required for non-initialize)
```

**No foreign keys** (no relational database) - relationships are logical/computational only.

---

## Data Model Summary

| Entity | Storage | Size | Lifecycle |
|--------|---------|------|-----------|
| OrderFlowSnapshot | Computed on-demand | ~500 bytes JSON | Ephemeral (not persisted) |
| VolumeProfile | Computed on-demand | ~5-50 KB JSON (100-1000 bins) | Ephemeral |
| MarketMicrostructureAnomaly | Computed on-demand | ~300 bytes JSON | Ephemeral |
| MicrostructureHealthScore | Computed on-demand | ~200 bytes JSON | Ephemeral |
| LiquidityVacuum | Computed on-demand | ~250 bytes JSON | Ephemeral |
| AbsorptionEvent | Computed on-demand | ~200 bytes JSON | Ephemeral |
| StreamableHttpSession | In-memory (DashMap) | ~200 bytes | 30-minute TTL |
| McpJsonRpcMessage | Transient (HTTP request/response) | Variable | Request lifetime |
| RocksDB snapshots | Disk (compressed) | ~1KB per snapshot | 7-day retention |

**Total storage footprint**: ~1GB for 20 symbols × 7 days of snapshots + ~10KB for 50 HTTP sessions = **~1.01GB maximum**
