# Data Model: OrderBook Snapshot Persistence

**Feature**: 007-snapshot-persistence
**Date**: 2025-10-19
**Phase**: 1 (Design)

## Overview

This document defines the data structures and their relationships for the orderbook snapshot persistence feature. All entities are already implemented in the codebase - this feature primarily involves wiring them together into a background persistence workflow.

## Core Entities

### 1. OrderBookSnapshot

**Purpose**: Represents a point-in-time capture of orderbook state for time-series storage

**Location**: `providers/binance-rs/src/orderbook/analytics/storage/snapshot.rs:14-62`

**Structure**:
```rust
pub struct OrderBookSnapshot {
    /// Top 20 bid levels (price, quantity)
    pub bids: Vec<(String, String)>,  // Decimal as strings for MessagePack

    /// Top 20 ask levels (price, quantity)
    pub asks: Vec<(String, String)>,

    /// Binance update ID for ordering
    pub update_id: u64,

    /// Capture timestamp (Unix seconds)
    pub timestamp: i64,
}
```

**Validation Rules**:
- `bids` and `asks`: 0-20 levels each (truncated from full orderbook)
- `update_id`: Binance-provided sequence number (monotonically increasing per symbol)
- `timestamp`: Unix epoch seconds (generated via `chrono::Utc::now().timestamp()`)
- Empty orderbook snapshots (both bids/asks empty) are skipped, not stored

**Serialization**:
- Format: MessagePack binary (via `rmp-serde` crate)
- Size: ~500 bytes per snapshot (20 levels × 2 sides × ~12 bytes/level)
- Methods: `to_bytes() -> Result<Vec<u8>>` and `from_bytes(&[u8]) -> Result<Self>`

**Relationships**:
- **Derived from**: `OrderBook` (via `OrderBookSnapshot::from_orderbook(&OrderBook)`)
- **Stored in**: RocksDB (via `SnapshotStorage.put(symbol, timestamp, bytes)`)
- **Queried by**: Analytics tools (via `query_snapshots_in_window()`)

---

### 2. SnapshotStorage

**Purpose**: Repository for persisting and retrieving orderbook snapshots from RocksDB

**Location**: `providers/binance-rs/src/orderbook/analytics/storage/mod.rs:18-121`

**Structure**:
```rust
pub struct SnapshotStorage {
    db: Arc<DB>,  // RocksDB handle
}
```

**Key API Methods**:
- `new<P: AsRef<Path>>(path: P) -> Result<Self>` - Initialize RocksDB at given path
- `put(&self, symbol: &str, timestamp_sec: i64, value: &[u8]) -> Result<()>` - Store snapshot
- `get(&self, symbol: &str, timestamp_sec: i64) -> Result<Option<Vec<u8>>>` - Retrieve snapshot
- `cleanup_old_snapshots(&self, retention_secs: i64) -> Result<usize>` - Delete expired data

**Storage Schema**:
- **Key Format**: `{symbol}:{unix_timestamp_sec}` (e.g., "BTCUSDT:1737158400")
- **Value Format**: MessagePack-serialized `OrderBookSnapshot`
- **Indexing**: Prefix bloom filter on symbol (first 10 characters)
- **Compression**: Zstd (configured in `new()`)

**Concurrency**:
- Thread-safe via `Arc<DB>` (RocksDB handles internal locking)
- Async-safe via `tokio::task::spawn_blocking` (prevents blocking Tokio runtime)

**Relationships**:
- **Stores**: `OrderBookSnapshot` instances
- **Queried by**: `query_snapshots_in_window()` for analytics
- **Cleaned by**: `cleanup_old_snapshots()` background task (7-day retention)

---

### 3. OrderBookManager

**Purpose**: Service for managing WebSocket connections and orderbook state

**Location**: `providers/binance-rs/src/orderbook/mod.rs` (exact structure TBD - existing service)

**Responsibilities**:
- Subscribe to Binance WebSocket depth streams
- Maintain live orderbook state per symbol
- Handle connection failures with exponential backoff reconnection
- Broadcast orderbook updates via `tokio::sync::watch` channels

**API (assumed based on existing usage)**:
```rust
impl OrderBookManager {
    /// Subscribe to orderbook stream for symbol (lazy initialization)
    pub async fn subscribe(&self, symbol: &str) -> Result<()>;

    /// Get current orderbook snapshot
    pub async fn get_orderbook(&self, symbol: &str) -> Result<OrderBook>;

    /// Get watch receiver for orderbook updates (for persistence task)
    pub fn watch_orderbook(&self, symbol: &str) -> Result<tokio::sync::watch::Receiver<OrderBook>>;
}
```

**State Transitions**:
```
Unsubscribed → Connecting → Connected → [Disconnected] → Reconnecting → Connected
                   ↓                           ↑
                 Error ------------------------|
                            (exponential backoff)
```

**Relationships**:
- **Publishes**: `OrderBook` updates via watch channels
- **Consumed by**: Background persistence task (subscribes to updates)
- **Consumed by**: grpc tools (orderbook_l1, orderbook_l2)

---

### 4. SymbolSubscription (Conceptual)

**Purpose**: Configuration entity defining which symbols to persist

**Location**: Not a Rust struct - configuration data (hardcoded initially)

**Initial Configuration**:
```rust
const PERSISTENT_SYMBOLS: &[&str] = &["BTCUSDT", "ETHUSDT"];
```

**Future Extension** (out of scope for this feature):
- Environment variable: `PERSISTENT_SYMBOLS=BTCUSDT,ETHUSDT,BNBUSDT`
- Runtime configuration endpoint

---

## Data Flow

### Snapshot Persistence Flow

```
1. Service Startup (grpc/mod.rs)
   ├─→ Initialize SnapshotStorage (RocksDB)
   ├─→ Initialize OrderBookManager
   ├─→ Pre-subscribe to BTCUSDT and ETHUSDT WebSocket streams
   └─→ Spawn background persistence task

2. Background Task (every 1 second)
   For each symbol in [BTCUSDT, ETHUSDT]:
     ├─→ Get latest OrderBook from OrderBookManager
     ├─→ Create OrderBookSnapshot from OrderBook
     ├─→ Serialize to MessagePack bytes
     ├─→ Store in RocksDB (SnapshotStorage.put)
     └─→ Log success/failure

3. Analytics Query (e.g., get_order_flow tool)
   ├─→ Call query_snapshots_in_window(storage, symbol, start, end)
   ├─→ RocksDB prefix scan for {symbol}:{start} to {symbol}:{end}
   ├─→ Deserialize MessagePack snapshots
   └─→ Return Vec<OrderBookSnapshot> for analysis
```

### Error Handling Flow

```
1. WebSocket Connection Failure
   ├─→ OrderBookManager logs error
   ├─→ OrderBookManager triggers reconnection with backoff
   └─→ Persistence task continues (skips empty snapshots)

2. RocksDB Write Failure
   ├─→ SnapshotStorage.put() returns Err
   ├─→ Persistence task logs error at ERROR level
   └─→ Persistence task continues (service doesn't crash)

3. Snapshot Serialization Failure
   ├─→ OrderBookSnapshot.to_bytes() returns Err
   ├─→ Persistence task logs error
   └─→ Persistence task continues with next symbol
```

## State Invariants

### Consistency Guarantees

1. **Snapshot Completeness**: Each stored snapshot contains full bid/ask levels (up to 20) or is skipped entirely (no partial snapshots)

2. **Timestamp Ordering**: Within a symbol's snapshots, timestamps are monotonically increasing (guaranteed by 1-second interval + Unix time)

3. **Symbol Isolation**: Snapshots for different symbols are independent - failure to persist BTCUSDT doesn't affect ETHUSDT persistence

4. **Persistence Independence**: Background persistence failures do NOT affect live orderbook functionality (orderbook_l1/l2 tools continue working)

### Validation Rules

1. **Empty Orderbook Check**: Skip persistence if `orderbook.bids.is_empty() && orderbook.asks.is_empty()`

2. **Timestamp Range**: Snapshot timestamps MUST be within [now - 5 seconds, now + 1 second] to detect clock skew

3. **Update ID Sequence**: Update IDs SHOULD increase (but non-monotonic updates are logged as warnings, not errors)

## Performance Characteristics

### Storage Growth

- **Rate**: 2 snapshots/sec (1/sec × 2 symbols)
- **Size per snapshot**: ~500 bytes
- **Hourly growth**: ~3.6 MB (500 bytes × 7200 snapshots/hour)
- **Weekly growth**: ~600 MB (before cleanup)
- **7-day retention**: Automatic cleanup keeps storage bounded

### Query Performance

- **Range Query**: <200ms for 1-hour window (3600 snapshots)
- **Index Lookup**: O(log n) via RocksDB prefix scan
- **Deserialization**: <10μs per snapshot (MessagePack is fast)

### Concurrency

- **Background task**: 1 dedicated Tokio task
- **RocksDB writes**: Offloaded to blocking thread pool (doesn't block async runtime)
- **WebSocket updates**: Non-blocking (OrderBookManager handles concurrency)

## Testing Strategy

### Unit Tests
- `OrderBookSnapshot::to_bytes() / from_bytes()` roundtrip (existing)
- `SnapshotStorage::put() / get()` operations (existing)

### Integration Tests
- Background task stores 60 snapshots over 60 seconds
- Analytics tools can query historical data after 60 seconds
- Service continues running after RocksDB write failure
- WebSocket reconnection doesn't lose snapshots

### System Tests
- Deploy to testnet, run for 24 hours, verify 7-day retention cleanup
- Load test: 20 concurrent symbols, verify <200ms orderbook latency maintained

## Future Enhancements (Out of Scope)

1. **Dynamic Symbol Management**: Add/remove symbols at runtime via API
2. **Variable Persistence Intervals**: Different frequencies for different symbols
3. **Snapshot Compression**: Delta encoding for consecutive snapshots
4. **Distributed Storage**: Replicate snapshots to S3 for disaster recovery
5. **Real-time Streaming**: Push snapshots to external consumers (Kafka, Redis Streams)

---

**Phase 1 (Data Model) Complete**: All entities documented. No schema changes required (all structures already exist).
