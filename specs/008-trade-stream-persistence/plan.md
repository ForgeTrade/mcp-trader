# Implementation Plan: Trade Stream Persistence

**Branch**: `008-trade-stream-persistence` | **Date**: 2025-10-19 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/008-trade-stream-persistence/spec.md`

## Summary

Implement background trade stream persistence to collect historical trade execution data for volume profile analytics. Currently, `get_volume_profile` and `get_liquidity_vacuums` tools fail with "need ≥1000 trades for 24h, got 0" because only orderbook snapshots are being collected (Feature 007), but these tools require actual **trade executions** (aggTrade stream). This feature adds a background Tokio task that subscribes to Binance aggTrade WebSocket streams for BTCUSDT and ETHUSDT on service startup, captures every trade, batches them every 1 second, serializes with MessagePack, and stores in RocksDB for analytics queries.

## Technical Context

**Language/Version**: Rust 1.75+
**Primary Dependencies**: tokio (async runtime), tokio-tungstenite (WebSocket with native-tls), rmp-serde (MessagePack), rocksdb (storage), serde (serialization)
**Storage**: RocksDB (key format `trades:{symbol}:{unix_timestamp_ms}`, MessagePack-serialized Vec<AggTrade> for each second batch)
**Testing**: cargo test (unit tests for trade buffer, integration tests for persistence task)
**Target Platform**: Linux server (production: Ubuntu 22.04 at 198.13.46.14)
**Project Type**: Single Rust binary (providers/binance-rs) with analytics feature flag
**Performance Goals**: 60-600 trades/min per symbol collected, <1s query time for 1-hour windows, <3s for 24-hour windows
**Constraints**: Background task must not crash service on errors, batch writes to RocksDB (not per-trade), WebSocket reconnection automatic
**Scale/Scope**: Initial: 2 symbols (BTCUSDT, ETHUSDT), 7-day retention (~100-150MB total), designed for expansion to 20 concurrent symbols

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### ✅ I. Simplicity and Readability
- Background task implementation uses clear async/await patterns
- Trade collection loop is straightforward: connect aggTrade WebSocket → receive trade → batch → store
- Error handling explicit with tracing::error! logs
- No deep nesting or complex control flow

### ✅ II. Library-First Development
- Reusing existing libraries: tokio-tungstenite (WebSocket), rmp-serde (MessagePack), rocksdb (storage)
- Existing WebSocket connection handling patterns from Feature 007
- No custom WebSocket protocol implementation needed

### ✅ III. Justified Abstractions
- No new abstractions introduced - using simple Vec<AggTrade> for batching
- Background task is a simple async function, not a complex abstraction
- Direct use of RocksDB via Arc<DB> without unnecessary wrappers

### ✅ IV. DRY Principle
- Reusing RocksDB storage infrastructure from Feature 007
- WebSocket connection pattern similar to orderbook manager (reusable approach)
- Trade batch serialization follows same MessagePack approach as snapshots

### ✅ V. Service and Repository Patterns
- TradeStorage acts as repository for trade data (new, mirrors SnapshotStorage pattern)
- TradeStreamClient acts as service for WebSocket connections (new, mirrors WebSocket patterns)
- Clear separation: grpc/tools.rs (application) → TradeStorage (repository) → RocksDB (data layer)

### ✅ VI. 12-Factor Methodology
- Config: Symbol list and batch interval can be env vars
- Logs: tracing crate outputs to stdout/stderr per 12-factor
- Processes: Stateless background task, trades batched in-memory only (ephemeral)
- Disposability: Task uses graceful shutdown via tokio::select! with broadcast shutdown signal

### ✅ VII. Minimal Object-Oriented Programming
- Procedural async function for background task
- Using Arc<RocksDB> for shared access, not deep OOP hierarchies
- No inheritance, minimal trait usage

**Result**: ✅ No constitution violations. All principles adhered to.

## Project Structure

### Documentation (this feature)

```
specs/008-trade-stream-persistence/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (trade data schema)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```
providers/binance-rs/
├── src/
│   ├── grpc/
│   │   ├── mod.rs                    # MODIFIED: Spawn trade persistence task, expose trade storage to tools
│   │   └── tools.rs                  # MODIFIED: Connect get_volume_profile/get_liquidity_vacuums to TradeStorage
│   ├── orderbook/
│   │   └── analytics/
│   │       ├── storage/
│   │       │   ├── mod.rs            # EXISTING: RocksDB infrastructure (reused)
│   │       │   └── snapshot.rs       # EXISTING: MessagePack serialization pattern (reference)
│   │       ├── trade_stream.rs       # EXISTING: AggTrade struct definition (reused)
│   │       └── tools.rs              # EXISTING: get_volume_profile/get_liquidity_vacuums tools
│   ├── lib.rs
│   └── main.rs                       # MODIFIED: Spawn trade persistence task on startup
└── Cargo.toml                        # EXISTING: All dependencies already present
```

### New Files Required

```
providers/binance-rs/src/orderbook/analytics/
├── trade_storage.rs      # NEW: TradeStorage repository for trade persistence and queries
└── trade_websocket.rs    # NEW: TradeStreamClient for aggTrade WebSocket connection
```

## Phase 0: Research & Technical Decisions

See [research.md](./research.md) for detailed technology evaluation and decisions.

### Key Research Areas

1. **Binance aggTrade WebSocket Protocol**
   - Endpoint: `wss://stream.binance.com/ws/btcusdt@aggTrade`
   - Message format: JSON with fields: `e`, `E`, `s`, `a`, `p`, `q`, `f`, `l`, `T`, `m`, `M`
   - Reconnection strategy: Automatic reconnect on disconnect with exponential backoff

2. **Trade Batch Size vs Storage Efficiency**
   - Decision: 1-second batches (same as Feature 007 snapshot interval)
   - Rationale: Balances write frequency (~120 writes/min for 2 symbols) vs query efficiency
   - Alternative considered: Per-trade writes (rejected: too many writes), 10-second batches (rejected: coarser granularity)

3. **RocksDB Key Design for Time-Range Queries**
   - Decision: `trades:{symbol}:{unix_timestamp_ms}` (milliseconds for finer granularity than snapshots)
   - Enables prefix scan: `trades:BTCUSDT:` + time range filter
   - Alternative considered: Composite key with start/end timestamps (rejected: more complex, no performance benefit)

4. **Trade Buffer In-Memory Management**
   - Decision: Vec<AggTrade> per symbol, flushed every 1 second
   - Rationale: Simple, minimal memory overhead (~50 trades × 150 bytes = 7.5KB per symbol per second)
   - Alternative considered: Circular buffer (rejected: unnecessary complexity for 1-second windows)

5. **Query Performance Optimization**
   - Decision: Store MessagePack Vec<AggTrade> per second (batch granularity), deserialize on query
   - Rationale: Reduces storage writes, acceptable query performance (<3s for 24h = ~86,400 RocksDB reads)
   - Alternative considered: Store individual trades (rejected: 100x more keys), pre-aggregated VWAP (rejected: analytics tools need raw trades)

## Phase 1: Data Model & API Contracts

See [data-model.md](./data-model.md) for complete entity definitions.

### Core Entities

1. **AggTrade** (Aggregate Trade) - Already exists in trade_stream.rs
   - price: Decimal (execution price)
   - quantity: Decimal (trade size)
   - timestamp: i64 (Unix milliseconds)
   - trade_id: i64 (unique identifier)
   - buyer_is_maker: bool (side determination: true = buy side maker)

2. **TradeBatch** (Storage Unit) - New
   - symbol: String
   - batch_timestamp: i64 (Unix ms, rounded to second)
   - trades: Vec<AggTrade>
   - count: usize (number of trades in batch)

3. **TradeQuery** (Query Parameters) - New
   - symbol: String
   - start_time: i64 (Unix ms)
   - end_time: i64 (Unix ms)

### Storage Layer API

```rust
// Repository pattern for trade persistence
pub struct TradeStorage {
    db: Arc<DB>, // Shared RocksDB instance (same as SnapshotStorage)
}

impl TradeStorage {
    pub fn new(db: Arc<DB>) -> Self;

    // Store a batch of trades (called every 1 second by background task)
    pub async fn store_batch(&self, symbol: &str, batch_timestamp: i64, trades: Vec<AggTrade>) -> Result<()>;

    // Query trades within time window (used by analytics tools)
    pub async fn query_trades(&self, symbol: &str, start_time: i64, end_time: i64) -> Result<Vec<AggTrade>>;

    // Cleanup trades older than retention period (called hourly)
    pub async fn cleanup_old_trades(&self, retention_secs: i64) -> Result<usize>;
}
```

### WebSocket Client API

```rust
// Service pattern for aggTrade WebSocket connection
pub struct TradeStreamClient {
    symbol: String,
    ws_url: String, // wss://stream.binance.com/ws/{symbol}@aggTrade
}

impl TradeStreamClient {
    pub fn new(symbol: String) -> Self;

    // Connect to WebSocket and return stream of trades
    pub async fn connect(&self) -> Result<impl Stream<Item = AggTrade>>;

    // Spawn background task for persistent connection with auto-reconnect
    pub fn spawn_persistent_connection(&self, trade_tx: mpsc::Sender<AggTrade>) -> JoinHandle<()>;
}
```

### Integration with Analytics Tools

Modified tool handlers in `grpc/tools.rs`:

```rust
// Before (Feature 007 - returns empty trades)
async fn handle_get_volume_profile(request: &InvokeRequest) -> Result<Json> {
    let trades = Vec::new(); // TODO: Pull from trade buffer
    let profile = get_volume_profile(trades, params).await?;
    ...
}

// After (Feature 008 - queries TradeStorage)
async fn handle_get_volume_profile(
    request: &InvokeRequest,
    trade_storage: Arc<TradeStorage>,
) -> Result<Json> {
    let end_time = chrono::Utc::now().timestamp_millis();
    let start_time = end_time - (params.duration_hours as i64 * 3600 * 1000);
    let trades = trade_storage.query_trades(&params.symbol, start_time, end_time).await?;
    let profile = get_volume_profile(trades, params).await?;
    ...
}
```

## Phase 2: Task Breakdown

**Prerequisite**: Run `/speckit.tasks` to generate detailed task breakdown with dependencies, acceptance criteria, and test scenarios.

### High-Level Task Groups (Preview)

1. **Setup & Infrastructure** (T001-T005)
   - Verify dependencies (tokio-tungstenite, rmp-serde already present)
   - Create trade_storage.rs and trade_websocket.rs files
   - Verify AggTrade struct compatibility

2. **Storage Layer Implementation** (T006-T015)
   - Implement TradeStorage::new() with shared RocksDB instance
   - Implement TradeStorage::store_batch() with MessagePack serialization
   - Implement TradeStorage::query_trades() with prefix scan + time filter
   - Implement TradeStorage::cleanup_old_trades() with batch delete
   - Unit tests for storage layer

3. **WebSocket Client Implementation** (T016-T025)
   - Implement TradeStreamClient::connect() with aggTrade endpoint
   - Implement reconnection logic with exponential backoff
   - Implement message deserialization (JSON → AggTrade)
   - Error handling for malformed messages
   - Unit tests for WebSocket client

4. **Background Persistence Task** (T026-T035)
   - Implement spawn_trade_persistence_task() function
   - 1-second interval loop with batch flushing
   - Graceful shutdown via broadcast channel
   - Error isolation (continue on individual batch failures)
   - Integration tests for persistence task

5. **Analytics Tool Integration** (T036-T045)
   - Modify handle_get_volume_profile() to query TradeStorage
   - Modify handle_get_liquidity_vacuums() to query TradeStorage
   - Update tool schemas to reflect new data source
   - Integration tests for tool queries
   - Performance testing (query latency)

6. **Service Integration & Deployment** (T046-T055)
   - Spawn trade persistence task in main.rs on startup
   - Pass shutdown signal to persistence task
   - Local testing (70-second run, verify trade collection)
   - Production deployment
   - Post-deployment validation

## Complexity Tracking

### Justified Complexity

1. **WebSocket Reconnection Logic** (Complexity Score: 3/10)
   - **Justification**: Network failures are inevitable, automatic reconnection prevents data gaps
   - **Mitigation**: Exponential backoff prevents reconnect storms, clear error logging
   - **Alternative considered**: Manual restart (rejected: requires operator intervention)

2. **Batch Serialization Strategy** (Complexity Score: 2/10)
   - **Justification**: Reduces write frequency by 100x (batching 100 trades/sec into 1 write/sec)
   - **Mitigation**: MessagePack serialization library handles complexity, Vec<AggTrade> is simple
   - **Alternative considered**: Individual trade writes (rejected: performance impact)

3. **Time-Range Query Implementation** (Complexity Score: 4/10)
   - **Justification**: Analytics tools need arbitrary time windows (1-168 hours)
   - **Mitigation**: RocksDB prefix scan is efficient, query interface simple (start_time, end_time)
   - **Alternative considered**: Fixed-window queries (rejected: limits tool flexibility)

### Avoided Complexity

1. **Trade Deduplication**: Not implemented (assumes Binance provides unique trades)
2. **Trade Aggregation**: Not implemented (analytics tools perform aggregation)
3. **Multi-Exchange Support**: Not implemented (Binance only, out of scope)
4. **Real-Time Streaming**: Not implemented (tools query historical data, no direct WebSocket subscriptions)

## Testing Strategy

### Unit Tests

1. **TradeStorage**
   - store_batch() → verify RocksDB key format and MessagePack serialization
   - query_trades() → verify time-range filtering and deserialization
   - cleanup_old_trades() → verify retention policy enforcement

2. **TradeStreamClient**
   - connect() → mock WebSocket connection and verify AggTrade parsing
   - Error handling → verify malformed message handling

### Integration Tests

1. **Persistence Task**
   - Spawn task → verify trades collected and stored every 1 second
   - Shutdown signal → verify graceful shutdown without data loss
   - Error scenarios → verify task continues after storage failures

2. **Analytics Tool Integration**
   - get_volume_profile() → verify returns data after 10 minutes of collection
   - Query performance → verify <3s query time for 24-hour windows

### Production Validation

1. **Local Testing** (70-second run):
   - Verify ~120 trade batches stored (2 symbols × 60 batches/min)
   - Verify analytics tools return data (no "insufficient trades" errors)

2. **Production Deployment**:
   - Monitor trade collection logs (expected: continuous "Stored N trades" messages)
   - Verify CPU <2%, memory <50MB overhead
   - Confirm analytics tools functional after 10 minutes

## Dependencies & Risks

### Internal Dependencies

- **Feature 007 (Snapshot Persistence)**: Reuses RocksDB storage infrastructure, shutdown signal handling
- **Existing AggTrade struct**: Defined in trade_stream.rs, must be compatible with Binance WebSocket format

### External Dependencies

- **Binance aggTrade WebSocket**: Reliable access to wss://stream.binance.com/ws/aggTrade endpoint
- **RocksDB**: Concurrent writes (trades + orderbook snapshots) must not block

### Risks & Mitigations

1. **Risk**: WebSocket disconnections cause data gaps
   - **Mitigation**: Automatic reconnection with logging, operators alerted via ERROR logs
   - **Impact**: Acceptable for analytics (7 days of data, brief gaps tolerable)

2. **Risk**: High trade velocity during volatility overwhelms batch writes
   - **Mitigation**: Batch writes to RocksDB (1 write/sec regardless of trade count), spawn_blocking prevents async runtime blocking
   - **Impact**: Low (even 1000 trades/sec batches to ~500KB/sec, manageable)

3. **Risk**: RocksDB storage growth exceeds disk capacity
   - **Mitigation**: 7-day retention with hourly cleanup, predictable growth (~15MB/day/symbol)
   - **Impact**: Low (100-150MB total for 2 symbols, server has 50GB+ available)

4. **Risk**: Query performance degrades with large time windows
   - **Mitigation**: RocksDB prefix scan optimized, MessagePack deserialization fast
   - **Impact**: Acceptable (<3s for 24h = ~86,400 keys, within timeout)

## Rollout Plan

### Phase 1: Local Development
1. Implement TradeStorage and TradeStreamClient
2. Write unit tests
3. Test locally with 70-second run

### Phase 2: Production Deployment
1. Commit to 008-trade-stream-persistence branch
2. Deploy via ./infra/deploy-chatgpt.sh to root@198.13.46.14
3. Monitor logs for trade collection activity

### Phase 3: Validation
1. Wait 10 minutes for data accumulation
2. Test analytics tools via ChatGPT at https://mcp-gateway.thevibe.trading/sse/
3. Verify `get_volume_profile` and `get_liquidity_vacuums` return valid results

### Phase 4: Post-Deployment Monitoring
1. Monitor CPU/memory (target: <2% CPU, <50MB RAM)
2. Monitor storage growth (target: ~15MB/day)
3. Monitor query performance (target: <1s for 1h, <3s for 24h)
4. Verify 7-day retention cleanup runs hourly

## Success Criteria

From [spec.md](./spec.md) Success Criteria section:

- ✅ **SC-001**: Analytics tools return valid results after 10 minutes (no "insufficient trades" errors)
- ✅ **SC-002**: Trade collection rate matches Binance velocity (60-600 trades/min)
- ✅ **SC-003**: Storage growth predictable (~10-15 MB/day for 2 symbols)
- ✅ **SC-004**: Background task <2% CPU, <50MB memory
- ✅ **SC-005**: Query time <3s for 24h windows, <1s for 1h windows
- ✅ **SC-006**: 99.9% uptime (tolerates 1.4 min/day downtime for reconnections)

## Next Steps

1. **Research Phase**: Complete [research.md](./research.md) with WebSocket protocol details and batch size benchmarks
2. **Design Phase**: Complete [data-model.md](./data-model.md) and [contracts/](./contracts/) with storage schema
3. **Task Generation**: Run `/speckit.tasks` to generate detailed implementation tasks
4. **Implementation**: Execute tasks in dependency order (storage → WebSocket → persistence → integration)
5. **Testing**: Run unit tests, integration tests, and 70-second local validation
6. **Deployment**: Deploy to production and monitor for 24 hours

---

**Plan Complete** | Ready for `/speckit.tasks` command
