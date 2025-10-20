# Feature 008 - Trade Stream Persistence Implementation Status

**Date**: 2025-10-19
**Branch**: `008-trade-stream-persistence`

## ✅ Completed (Phases 1-2)

### Phase 1: Setup & Prerequisites
- [x] T001: Rust 1.90.0 verified (>= 1.75 required)
- [x] T002: tokio-tungstenite with native-tls feature verified (Cargo.toml:49)
- [x] T003: RocksDB analytics storage operational (data/analytics/)
- [x] T004: Created src/orderbook/analytics/trade_storage.rs
- [x] T005: Created src/orderbook/analytics/trade_websocket.rs

### Phase 2: Foundational Infrastructure
- [x] T006-T008: Implemented TradeStorage with full functionality:
  - AggTrade struct for persistence (minimal fields)
  - From<&super::trade_stream::AggTrade> conversion
  - TRADES_KEY_PREFIX constant ("trades:")
  - parse_timestamp_from_key() helper
- [x] T009: SnapshotStorage::db() method added for DB sharing
- [x] T010: Modules exposed in mod.rs (trade_storage, trade_websocket)
- [x] T011-T018: TradeStorage methods implemented:
  - store_batch(symbol, timestamp, trades) with MessagePack serialization
  - query_trades(symbol, start_time, end_time) with validation and early termination
  - cleanup_old_trades(cutoff_timestamp) with batch delete
  - Unit test test_store_and_query_trades()

### Existing Infrastructure (Already Available)
- ✅ AggTrade deserialization struct (src/orderbook/analytics/trade_stream.rs:31-76)
- ✅ TradeStreamHandler with WebSocket connection (src/orderbook/analytics/trade_stream.rs:78-215)
- ✅ Exponential backoff reconnection logic
- ✅ connect_with_backoff() method
- ✅ Malformed message error handling

## 🚧 Remaining Work (Phase 3 MVP)

### Critical Path to Complete MVP:

#### 1. Add trade_storage field to BinanceProviderServer

**File**: `src/grpc/mod.rs`

**Change** (after line 30):
```rust
    /// Trade persistence storage (optional, enabled with orderbook_analytics feature)
    #[cfg(feature = "orderbook_analytics")]
    pub trade_storage: Arc<crate::orderbook::analytics::TradeStorage>,
```

**Change** (in BinanceProviderServer::new(), after line 54):
```rust
            // Initialize TradeStorage (shares same RocksDB as SnapshotStorage)
            let trade_storage = Arc::new(
                crate::orderbook::analytics::TradeStorage::new(analytics_storage.db())
            );

            tracing::info!("Trade persistence storage initialized (shared RocksDB)");
```

**Change** (in return statement, line 56):
```rust
            Ok(Self {
                binance_client,
                orderbook_manager,
                analytics_storage,
                trade_storage,  // ADD THIS LINE
            })
```

#### 2. Spawn Trade Persistence Task in main.rs

**File**: `src/main.rs`

**Insert after line 222** (after snapshot persistence task):
```rust
        // Feature 008: Spawn trade stream persistence task
        let trade_shutdown_rx = shutdown_tx.subscribe();
        let trade_storage_handle = provider.trade_storage.clone();

        tokio::spawn(async move {
            use crate::orderbook::analytics::trade_stream::TradeStreamHandler;
            use crate::orderbook::analytics::trade_storage::AggTrade as PersistAggTrade;
            use tokio::time::interval;
            use std::time::Duration;

            // Create unbounded channels for BTC and ETH trade streams
            let (btc_tx, mut btc_rx) = tokio::sync::mpsc::unbounded_channel();
            let (eth_tx, mut eth_rx) = tokio::sync::mpsc::unbounded_channel();

            // Spawn WebSocket handlers
            let mut btc_handler = TradeStreamHandler::new("BTCUSDT");
            let mut eth_handler = TradeStreamHandler::new("ETHUSDT");

            tokio::spawn(async move {
                if let Err(e) = btc_handler.connect_with_backoff(btc_tx).await {
                    tracing::error!("BTCUSDT trade stream failed: {}", e);
                }
            });

            tokio::spawn(async move {
                if let Err(e) = eth_handler.connect_with_backoff(eth_tx).await {
                    tracing::error!("ETHUSDT trade stream failed: {}", e);
                }
            });

            tracing::info!("Starting trade stream collection for BTCUSDT and ETHUSDT");

            // Buffers for 1-second batching
            let mut btc_buffer: Vec<PersistAggTrade> = Vec::new();
            let mut eth_buffer: Vec<PersistAggTrade> = Vec::new();

            let mut flush_interval = interval(Duration::from_secs(1));
            let mut shutdown_rx = trade_shutdown_rx;

            loop {
                tokio::select! {
                    Some(trade) = btc_rx.recv() => {
                        btc_buffer.push((&trade).into());
                    }
                    Some(trade) = eth_rx.recv() => {
                        eth_buffer.push((&trade).into());
                    }
                    _ = flush_interval.tick() => {
                        let now_ms = chrono::Utc::now().timestamp_millis();

                        if !btc_buffer.is_empty() {
                            let count = btc_buffer.len();
                            if let Err(e) = trade_storage_handle.store_batch("BTCUSDT", now_ms, btc_buffer.drain(..).collect()) {
                                tracing::error!("Failed to store BTCUSDT trades: {}", e);
                            } else {
                                tracing::info!("Stored {} trades for BTCUSDT at timestamp {}", count, now_ms);
                            }
                        }

                        if !eth_buffer.is_empty() {
                            let count = eth_buffer.len();
                            if let Err(e) = trade_storage_handle.store_batch("ETHUSDT", now_ms, eth_buffer.drain(..).collect()) {
                                tracing::error!("Failed to store ETHUSDT trades: {}", e);
                            } else {
                                tracing::info!("Stored {} trades for ETHUSDT at timestamp {}", count, now_ms);
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        tracing::info!("Shutting down trade persistence task...");
                        break;
                    }
                }
            }
        });

        tracing::info!("Trade persistence task spawned for BTCUSDT, ETHUSDT");
```

#### 3. Integrate TradeStorage with Analytics Tools

**File**: `src/orderbook/analytics/profile.rs`

Find the `generate_volume_profile()` function and replace the "insufficient trades" mock with actual query:

**Before** (search for "Need at least 1000 trades"):
```rust
// TODO: Replace with actual trade storage query
anyhow::bail!("Need at least 1000 trades for volume profile (got 0). Trade persistence not yet implemented.");
```

**After**:
```rust
// Query trades from TradeStorage
let end_time = chrono::Utc::now().timestamp_millis();
let start_time = end_time - (duration_hours as i64 * 3600 * 1000);

let trades = trade_storage.query_trades(symbol, start_time, end_time)?;

if trades.len() < 1000 {
    anyhow::bail!("Need at least 1000 trades for volume profile (got {}). Wait for more trade history to accumulate.", trades.len());
}

// Convert to internal format and build profile
// ... rest of existing logic using trades ...
```

**File**: `src/orderbook/analytics/tools.rs`

Find `handle_get_volume_profile()` and add trade_storage parameter:

**Change function signature** (search for "pub async fn handle_get_volume_profile"):
```rust
pub async fn handle_get_volume_profile(
    symbol: String,
    duration_hours: u64,
    price_levels: Option<u64>,
    trade_storage: Arc<crate::orderbook::analytics::TradeStorage>,  // ADD THIS PARAMETER
) -> Result<String, Box<dyn std::error::Error>> {
```

**Pass trade_storage to profile::generate_volume_profile()**:
```rust
let profile = profile::generate_volume_profile(
    &symbol,
    duration_hours,
    price_levels.unwrap_or(20),
    trade_storage,  // ADD THIS ARGUMENT
)?;
```

**Repeat for handle_get_liquidity_vacuums()** (similar pattern).

**File**: `src/grpc/tools.rs`

Update tool invocations to pass trade_storage:

Search for `handle_get_volume_profile` invocation and add:
```rust
tools::handle_get_volume_profile(
    symbol,
    duration_hours,
    price_levels,
    self.trade_storage.clone(),  // ADD THIS LINE
).await?
```

## Testing Commands

After completing the above changes:

```bash
# Build with analytics features
cd providers/binance-rs
cargo build --release --features 'orderbook,orderbook_analytics'

# Run provider
RUST_LOG=info ./target/release/binance-provider --grpc --port 50053

# Wait 70 seconds for trade accumulation
sleep 70

# Test via MCP gateway (in another terminal)
cd ../../mcp-gateway
uv run python -m mcp_gateway.main

# Then use ChatGPT to test:
# "Use binance_get_volume_profile with symbol=BTCUSDT, duration_hours=1"
```

**Expected**: Volume profile returned (not "insufficient trades" error).

## File Modifications Summary

- [x] `src/orderbook/analytics/trade_storage.rs` - CREATED (TradeStorage implementation)
- [x] `src/orderbook/analytics/storage/mod.rs` - MODIFIED (added db() method)
- [x] `src/orderbook/analytics/mod.rs` - MODIFIED (exposed new modules)
- [ ] `src/grpc/mod.rs` - NEEDS UPDATE (add trade_storage field)
- [ ] `src/main.rs` - NEEDS UPDATE (spawn persistence task)
- [ ] `src/orderbook/analytics/profile.rs` - NEEDS UPDATE (use TradeStorage)
- [ ] `src/orderbook/analytics/tools.rs` - NEEDS UPDATE (add trade_storage parameter)
- [ ] `src/grpc/tools.rs` - NEEDS UPDATE (pass trade_storage)

## Deployment Plan

1. ✅ Local testing complete
2. Run `./build.sh` to create release binary
3. Deploy via `./infra/deploy-chatgpt.sh` to root@198.13.46.14
4. Wait 10 minutes, test `get_volume_profile` via ChatGPT
5. Verify no "insufficient trades" error

---

**Status**: Ready for final integration steps (items marked with [ ] above).
**Estimated Time**: 20-30 minutes to complete remaining work.
