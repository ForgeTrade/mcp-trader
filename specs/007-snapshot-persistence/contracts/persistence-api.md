# Snapshot Persistence API Contract

**Feature**: 007-snapshot-persistence
**Date**: 2025-10-19
**Phase**: 1 (Design)

## Overview

This document defines the internal Rust API contract for the snapshot persistence feature. Since this is a background task feature (not a user-facing API), the "contract" describes the function signatures and behavior guarantees for the persistence task and its integration points.

## Background Task API

### spawn_snapshot_persistence_task

**Purpose**: Initialize and spawn the background snapshot persistence task

**Signature**:
```rust
pub async fn spawn_snapshot_persistence_task(
    storage: Arc<SnapshotStorage>,
    manager: Arc<OrderBookManager>,
    symbols: &[&str],
    shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) -> tokio::task::JoinHandle<()>
```

**Parameters**:
- `storage`: Thread-safe handle to RocksDB snapshot storage
- `manager`: Thread-safe handle to WebSocket orderbook manager
- `symbols`: List of symbols to persist (e.g., `&["BTCUSDT", "ETHUSDT"]`)
- `shutdown_rx`: Broadcast receiver for graceful shutdown signal

**Returns**: `JoinHandle` for the spawned task (allows awaiting task completion on shutdown)

**Behavior**:
1. Spawns a new Tokio task that runs independently
2. For each symbol, subscribes to orderbook updates via OrderBookManager
3. Every 1 second, captures current orderbook snapshot for all symbols
4. Serializes snapshots to MessagePack and stores in RocksDB
5. Logs persistence success/failure at appropriate levels (INFO/ERROR)
6. Gracefully shuts down when `shutdown_rx` receives signal

**Error Handling**:
- WebSocket connection failures: Logged at WARN, task continues (OrderBookManager handles reconnection)
- RocksDB write failures: Logged at ERROR, task continues (service doesn't crash)
- Serialization failures: Logged at ERROR, task continues with next symbol

**Pre-conditions**:
- `SnapshotStorage` must be initialized with valid RocksDB path
- `OrderBookManager` must be created (but symbols don't need pre-subscription)
- Shutdown channel must be created before calling this function

**Post-conditions**:
- Background task is running independently
- Snapshots begin appearing in RocksDB within 1-2 seconds
- Analytics tools can query historical data after accumulation period (60+ seconds)

---

## Storage API Contract (Existing)

### SnapshotStorage::put

**Signature**:
```rust
pub async fn put(&self, symbol: &str, timestamp_sec: i64, value: &[u8]) -> Result<()>
```

**Behavior**:
- Stores MessagePack-serialized snapshot in RocksDB
- Key format: `{symbol}:{timestamp_sec}` (e.g., "BTCUSDT:1737158400")
- Uses `spawn_blocking` to avoid blocking Tokio runtime
- Atomic write (either succeeds or returns error)

**Error Conditions**:
- RocksDB disk full → `Err(anyhow!("Failed to write snapshot to RocksDB"))`
- Invalid UTF-8 in key → Not possible with current implementation
- RocksDB corruption → Propagates RocksDB error

**Performance Guarantee**: <10ms p99 latency for single snapshot write (500 bytes)

---

### SnapshotStorage::get

**Signature**:
```rust
pub async fn get(&self, symbol: &str, timestamp_sec: i64) -> Result<Option<Vec<u8>>>
```

**Behavior**:
- Retrieves snapshot by exact key
- Returns `Ok(None)` if key doesn't exist
- Uses `spawn_blocking` for async safety

**Error Conditions**:
- RocksDB read error → `Err(anyhow!("Failed to read snapshot from RocksDB"))`

**Performance Guarantee**: <5ms p99 latency for single key lookup

---

## OrderBookManager API Contract (Assumed - Existing Service)

### OrderBookManager::subscribe

**Signature**:
```rust
pub async fn subscribe(&self, symbol: &str) -> Result<()>
```

**Behavior**:
- Establishes WebSocket connection to Binance depth stream for symbol
- Returns immediately after connection initiated (doesn't wait for first update)
- Handles reconnection automatically with exponential backoff

**Error Conditions**:
- Invalid symbol format → `Err("Invalid symbol")`
- WebSocket connection failure → Logs error, retries automatically
- Too many concurrent subscriptions (>20) → `Err("Max subscriptions exceeded")`

**Idempotency**: Calling `subscribe()` for already-subscribed symbol is a no-op

---

### OrderBookManager::get_orderbook

**Signature**:
```rust
pub async fn get_orderbook(&self, symbol: &str) -> Result<OrderBook>
```

**Behavior**:
- Returns current orderbook state for symbol
- If symbol not subscribed, initiates lazy subscription (current behavior)
- Blocks until first orderbook update received (up to 5 seconds timeout)

**Error Conditions**:
- Symbol not subscribed and subscription fails → `Err("Failed to subscribe")`
- Timeout waiting for first update → `Err("Timeout waiting for orderbook")`

**Performance Guarantee**: <200ms for subscribed symbols, <5s for unsubscribed symbols

---

## Serialization API Contract (Existing)

### OrderBookSnapshot::from_orderbook

**Signature**:
```rust
pub fn from_orderbook(orderbook: &OrderBook) -> Self
```

**Behavior**:
- Extracts top 20 bid/ask levels from full orderbook
- Generates Unix timestamp via `chrono::Utc::now().timestamp()`
- Converts Decimal prices/quantities to strings for MessagePack

**Guarantees**:
- Always succeeds (no Result type - infallible)
- Snapshot timestamp matches capture time within ±1 second

---

### OrderBookSnapshot::to_bytes

**Signature**:
```rust
pub fn to_bytes(&self) -> Result<Vec<u8>>
```

**Behavior**:
- Serializes snapshot to MessagePack binary format via `rmp-serde`

**Error Conditions**:
- Serialization failure (extremely rare) → `Err("Failed to serialize snapshot to MessagePack")`

**Performance Guarantee**: <100μs for typical snapshot (20 levels)

---

### OrderBookSnapshot::from_bytes

**Signature**:
```rust
pub fn from_bytes(data: &[u8]) -> Result<Self>
```

**Behavior**:
- Deserializes MessagePack bytes to OrderBookSnapshot

**Error Conditions**:
- Invalid MessagePack format → `Err("Failed to deserialize snapshot from MessagePack")`
- Schema mismatch → `Err("Deserialization error: missing field")`

**Performance Guarantee**: <50μs for typical snapshot

---

## Integration Points

### 1. grpc/mod.rs (Server Initialization)

**Integration Location**: Server initialization function (after creating OrderBookManager and SnapshotStorage)

**Required Changes**:
```rust
// In grpc/mod.rs initialization function

// 1. Create shutdown channel
let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);

// 2. Pre-subscribe to persistence symbols
for symbol in ["BTCUSDT", "ETHUSDT"] {
    orderbook_manager.subscribe(symbol).await?;
    tracing::info!("Pre-subscribed to {} for snapshot persistence", symbol);
}

// 3. Spawn persistence task
let persistence_handle = spawn_snapshot_persistence_task(
    analytics_storage.clone(),
    orderbook_manager.clone(),
    &["BTCUSDT", "ETHUSDT"],
    shutdown_rx,
);

// 4. On shutdown, signal and await task completion
tokio::select! {
    _ = tokio::signal::ctrl_c() => {
        tracing::info!("Shutdown signal received");
        shutdown_tx.send(()).ok();
        persistence_handle.await.ok();
    }
}
```

**Backward Compatibility**: No breaking changes - existing orderbook functionality unaffected

---

### 2. orderbook/mod.rs (Eager Subscription)

**Current Behavior**: WebSocket subscriptions are lazy (start on first orderbook_l1/l2 request)

**Required Change**: Expose `subscribe()` method publicly for pre-subscription

**Validation**:
```rust
// Ensure OrderBookManager::subscribe is pub (not pub(crate))
impl OrderBookManager {
    pub async fn subscribe(&self, symbol: &str) -> Result<()> {
        // existing implementation
    }
}
```

---

## Logging Contract

### Log Levels

**INFO**: Successful operations and milestones
- "Pre-subscribed to BTCUSDT for snapshot persistence"
- "Stored snapshot for BTCUSDT at timestamp 1737158400"
- "Snapshot persistence task shutting down"

**WARN**: Recoverable errors that don't require immediate action
- "Skipping snapshot for BTCUSDT: empty orderbook"
- "WebSocket connection temporarily disrupted for ETHUSDT"

**ERROR**: Failures that indicate problems but don't crash service
- "Failed to persist BTCUSDT snapshot: RocksDB write error"
- "Failed to serialize snapshot for ETHUSDT: [details]"

**DEBUG**: Detailed trace information (disabled in production)
- "Captured snapshot for BTCUSDT at timestamp 1737158400"

### Log Format

All logs MUST use structured logging via `tracing` crate:

```rust
tracing::info!(symbol = %symbol, timestamp = %timestamp, "Stored snapshot");
tracing::error!(symbol = %symbol, error = %e, "Failed to persist snapshot");
```

---

## Testing Contract

### Unit Tests (Existing)

- `OrderBookSnapshot::to_bytes() / from_bytes()` roundtrip
- `SnapshotStorage::put() / get()` operations

### Integration Tests (New)

**Test 1**: Snapshot accumulation over time
```rust
#[tokio::test]
async fn test_background_persistence_accumulates_snapshots() {
    // Setup: Initialize storage + manager + task
    // Wait: 65 seconds
    // Assert: >=60 snapshots for BTCUSDT in RocksDB
}
```

**Test 2**: Service continues after RocksDB failure
```rust
#[tokio::test]
async fn test_persistence_errors_dont_crash_service() {
    // Setup: Mock RocksDB to fail writes
    // Run: Persistence task for 10 seconds
    // Assert: Task still running, errors logged
}
```

**Test 3**: Analytics tools receive historical data
```rust
#[tokio::test]
async fn test_analytics_tools_query_persisted_snapshots() {
    // Setup: Run persistence task for 120 seconds
    // Execute: get_order_flow(symbol="BTCUSDT", window=60)
    // Assert: Returns valid OrderFlow data (no "Insufficient data" error)
}
```

---

## Performance Contract

### Guarantees

1. **Snapshot Persistence Rate**: ≥58 snapshots/min/symbol (allowing 2 missed snapshots for errors)
2. **Background Task Overhead**: <1% CPU usage on production server
3. **Memory Footprint**: <10MB for persistence task (excluding RocksDB cache)
4. **Orderbook Latency**: Sub-200ms p99 for live orderbook queries (persistence doesn't block)

### SLOs (Service Level Objectives)

- **Availability**: Persistence task runs 99.9% of uptime (brief interruptions during restarts acceptable)
- **Data Completeness**: ≥95% of expected snapshots persisted (allowing 5% loss for network/disk errors)
- **Query Latency**: Analytics tools return results in <500ms for 1-hour time windows

---

**Contract Version**: 1.0.0
**Last Updated**: 2025-10-19
**Breaking Changes**: None (all APIs are internal and backward-compatible)
