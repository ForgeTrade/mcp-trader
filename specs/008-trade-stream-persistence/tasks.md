# Tasks: Trade Stream Persistence

**Feature**: 008-trade-stream-persistence
**Date**: 2025-10-19
**Branch**: `008-trade-stream-persistence`

## Task Summary

**Total Tasks**: 50
**MVP Tasks** (User Story 1 - P1): 27 tasks
**Post-MVP Tasks** (User Stories 2-3): 8 tasks
**Support Tasks** (Setup, Integration, Deployment): 15 tasks

**Estimated Effort**:
- MVP (User Story 1): ~8-12 hours
- Post-MVP (User Stories 2-3): ~2-3 hours
- Full Feature: ~12-16 hours

---

## Phase 1: Setup & Prerequisites (5 tasks)

**Purpose**: Verify environment, dependencies, and project structure before implementation.

- [ ] [T001] [--] [--] Verify Rust 1.75+ toolchain installed: `rustc --version` ≥ 1.75 (providers/binance-rs/)
- [ ] [T002] [--] [--] Verify Cargo.toml has tokio-tungstenite with native-tls feature (providers/binance-rs/Cargo.toml:49)
- [ ] [T003] [--] [--] Verify Feature 007 RocksDB analytics storage is operational: `ls data/analytics/` shows SST files (providers/binance-rs/)
- [ ] [T004] [--] [--] Create new module file `src/orderbook/analytics/trade_storage.rs` (providers/binance-rs/)
- [ ] [T005] [--] [--] Create new module file `src/orderbook/analytics/trade_websocket.rs` (providers/binance-rs/)

**Dependencies**: None (prerequisite checks)

**Independent Test**: After T005, `cargo check --features 'orderbook,orderbook_analytics'` compiles with no errors for new empty modules.

---

## Phase 2: Foundational Infrastructure (5 tasks)

**Purpose**: Implement core building blocks required by all user stories (storage, deserialization, shared infrastructure).

- [ ] [T006] [--] [--] Add WebSocket message deserialization struct `AggTradeMessage` with serde derives (src/orderbook/analytics/trade_websocket.rs)
- [ ] [T007] [--] [--] Implement `parse_timestamp_from_key()` helper function for extracting timestamp from RocksDB keys (src/orderbook/analytics/trade_storage.rs)
- [ ] ] [T008] [--] [--] Define RocksDB key format constant `TRADES_KEY_PREFIX = "trades:"` (src/orderbook/analytics/trade_storage.rs)
- [ ] [T009] [--] [--] Add shutdown signal receiver to `main.rs` for graceful termination: `tokio::sync::watch::Receiver<bool>` (src/main.rs)
- [ ] [T010] [--] [--] Update `mod.rs` to expose new modules: `pub mod trade_storage;` and `pub mod trade_websocket;` (src/orderbook/analytics/mod.rs)

**Dependencies**: Phase 1 complete

**Independent Test**: After T010, `cargo build --features 'orderbook,orderbook_analytics'` succeeds with no warnings for unused items.

---

## Phase 3: User Story 1 - Analytics Tools Access Historical Trades (P1 MVP) (27 tasks)

**User Story**: When ChatGPT users invoke `get_volume_profile` or `get_liquidity_vacuums`, the tools successfully retrieve and analyze historical trade data instead of returning "insufficient trades" errors.

**Independent Test**: After Phase 3 complete, service runs for 10 minutes, then calling `get_volume_profile` with `symbol=BTCUSDT, duration_hours=1` returns valid volume profile with POC/VAH/VAL metrics (not error).

### Subphase 3A: TradeStorage Repository (8 tasks)

- [ ] [T011] [P1] [US1] Implement `TradeStorage` struct with RocksDB Arc reference (src/orderbook/analytics/trade_storage.rs)
- [ ] [T012] [P1] [US1] Implement `TradeStorage::new(db: Arc<DB>)` constructor (src/orderbook/analytics/trade_storage.rs)
- [ ] [T013] [P1] [US1] Implement `store_batch()` method: serialize Vec<AggTrade> with MessagePack, write to RocksDB (src/orderbook/analytics/trade_storage.rs)
- [ ] [T014] [P1] [US1] Implement `query_trades()` method: prefix scan with time-range filter, deserialize batches (src/orderbook/analytics/trade_storage.rs)
- [ ] [T015] [P1] [US1] Add timestamp validation in `query_trades()`: reject if start_time > end_time or window > 7 days (src/orderbook/analytics/trade_storage.rs)
- [ ] [T016] [P1] [US1] Add early termination optimization in `query_trades()`: stop scan if timestamp > end_time (src/orderbook/analytics/trade_storage.rs)
- [ ] [T017] [P1] [US1] Implement `cleanup_old_trades()` method: delete batches older than 7 days via WriteBatch (src/orderbook/analytics/trade_storage.rs)
- [ ] [T018] [P1] [US1] Add unit test `test_store_and_query_trades()`: store 100 trades, query 1-hour window, verify count (src/orderbook/analytics/trade_storage.rs)

**Dependencies**: Phase 2 (T008 key format constant)

### Subphase 3B: TradeStreamClient WebSocket (9 tasks)

- [ ] [T019] [P1] [US1] Implement `TradeStreamClient` struct with symbol, websocket_url, reconnect_delay fields (src/orderbook/analytics/trade_websocket.rs)
- [ ] [T020] [P1] [US1] Implement `TradeStreamClient::new(symbol: &str)` constructor with endpoint `wss://stream.binance.com/ws/{symbol_lower}@aggTrade` (src/orderbook/analytics/trade_websocket.rs)
- [ ] [T021] [P1] [US1] Implement `connect_websocket()` async method: establish tokio-tungstenite connection with native-tls (src/orderbook/analytics/trade_websocket.rs)
- [ ] [T022] [P1] [US1] Add WebSocket message parsing: deserialize JSON to `AggTradeMessage`, convert to `AggTrade` struct (src/orderbook/analytics/trade_websocket.rs)
- [ ] [T023] [P1] [US1] Add malformed message error handling: log error, skip message, continue processing (src/orderbook/analytics/trade_websocket.rs)
- [ ] [T024] [P1] [US1] Implement exponential backoff reconnection: start at 1s, double on failure, max 60s (src/orderbook/analytics/trade_websocket.rs)
- [ ] [T025] [P1] [US1] Add reconnection logging: ERROR level on failure, INFO level on reconnect success (src/orderbook/analytics/trade_websocket.rs)
- [ ] [T026] [P1] [US1] Implement `spawn_persistent_connection()` method: loop with reconnection, send trades via mpsc channel (src/orderbook/analytics/trade_websocket.rs)
- [ ] [T027] [P1] [US1] Add unit test `test_parse_aggtrade_message()`: parse example JSON, verify fields match contract (src/orderbook/analytics/trade_websocket.rs)

**Dependencies**: Phase 2 (T006 AggTradeMessage struct)

### Subphase 3C: Persistence Task (5 tasks)

- [ ] [T028] [P1] [US1] Create `spawn_trade_persistence_task()` function in main.rs: initialize Vec<AggTrade> buffers for BTCUSDT and ETHUSDT (src/main.rs)
- [ ] [T029] [P1] [US1] Spawn 2 TradeStreamClient tasks for BTCUSDT and ETHUSDT, receive trades via mpsc channels (src/main.rs)
- [ ] [T030] [P1] [US1] Implement 1-second interval flush with tokio::time::interval: batch trades, call trade_storage.store_batch() (src/main.rs)
- [ ] [T031] [P1] [US1] Add INFO-level logging on batch write: "Stored N trades for SYMBOL at timestamp X" (src/main.rs)
- [ ] [T032] [P1] [US1] Add shutdown signal handling: gracefully close WebSocket connections and flush remaining trades (src/main.rs)

**Dependencies**: Subphase 3A (T013 store_batch), Subphase 3B (T026 spawn_persistent_connection)

### Subphase 3D: Analytics Tool Integration (5 tasks)

- [ ] [T033] [P1] [US1] Pass `trade_storage: Arc<TradeStorage>` parameter to `handle_get_volume_profile()` (src/grpc/tools.rs)
- [ ] [T034] [P1] [US1] Replace mock "insufficient trades" error in `handle_get_volume_profile()` with `trade_storage.query_trades()` call (src/grpc/tools.rs)
- [ ] [T035] [P1] [US1] Pass `trade_storage: Arc<TradeStorage>` parameter to `handle_get_liquidity_vacuums()` (src/grpc/tools.rs)
- [ ] [T036] [P1] [US1] Replace mock "insufficient trades" error in `handle_get_liquidity_vacuums()` with `trade_storage.query_trades()` call (src/grpc/tools.rs)
- [ ] [T037] [P1] [US1] Update tool handler initialization in `start_grpc_server()` to pass shared TradeStorage instance (src/main.rs)

**Dependencies**: Subphase 3A (T014 query_trades)

---

## Phase 4: User Story 2 - Operators Monitor Trade Collection (P2) (4 tasks)

**User Story**: When operators check service logs, they can verify trade stream collection status (connection state, trade rates, storage growth) without needing external monitoring tools.

**Independent Test**: After service runs for 5 minutes, logs show "Stored N trades" entries every 1-2 seconds, and total trades collected matches expected rate (60-600 trades/min per symbol).

- [ ] [T038] [P2] [US2] Add WARN-level logging for empty batches: "No trades received in last second for SYMBOL" (src/main.rs)
- [ ] [T039] [P2] [US2] Add DEBUG-level logging for WebSocket connection state changes: "Trade WebSocket connected successfully" (src/orderbook/analytics/trade_websocket.rs)
- [ ] [T040] [P2] [US2] Add metrics collection for trade batch count: increment counter on each store_batch() call (src/orderbook/analytics/trade_storage.rs)
- [ ] [T041] [P2] [US2] Add INFO-level logging on reconnection with backoff delay: "Retrying WebSocket connection in Xs" (src/orderbook/analytics/trade_websocket.rs)

**Dependencies**: Phase 3 complete

---

## Phase 5: User Story 3 - Service Stability During Failures (P3) (4 tasks)

**User Story**: When network failures, Binance API outages, or WebSocket disconnections occur, the service continues collecting orderbook snapshots (Feature 007) and automatically reconnects trade streams when connectivity resumes.

**Independent Test**: Simulate network failure by blocking wss://stream.binance.com for 2 minutes, then unblock. Service logs show reconnection attempts, orderbook snapshot collection continues uninterrupted, and trade collection resumes within 60 seconds of unblock.

- [ ] [T042] [P3] [US3] Add integration test `test_websocket_reconnect_on_failure()`: simulate disconnect, verify reconnection within 60s (tests/integration_websocket.rs)
- [ ] [T043] [P3] [US3] Add error handling for RocksDB write failures: log ERROR, continue collection (buffer trades in-memory until next flush) (src/orderbook/analytics/trade_storage.rs)
- [ ] [T044] [P3] [US3] Add timeout handling for WebSocket message reception: reconnect if no message received in 5 minutes (src/orderbook/analytics/trade_websocket.rs)
- [ ] [T045] [P3] [US3] Add panic recovery for persistence task: catch panics, log error, restart task (src/main.rs)

**Dependencies**: Phase 3 complete

---

## Phase 6: Integration, Testing & Deployment (5 tasks)

**Purpose**: End-to-end validation, local testing, and production rollout.

- [ ] [T046] [--] [--] Run cargo test with analytics features: `cargo test --features 'orderbook,orderbook_analytics'` all tests pass (providers/binance-rs/)
- [ ] [T047] [--] [--] Run local integration test following quickstart.md: service runs 10 minutes, analytics tools return data (providers/binance-rs/)
- [ ] [T048] [--] [--] Verify storage growth rate: after 1 hour, `du -sh data/analytics/` shows ~15 MB growth (providers/binance-rs/)
- [ ] [T049] [--] [--] Deploy to production server via `./infra/deploy-chatgpt.sh` to root@198.13.46.14 (infra/)
- [ ] [T050] [--] [--] Post-deployment validation: wait 10 minutes, test `get_volume_profile` via ChatGPT, verify no "insufficient trades" error (remote)

**Dependencies**: Phase 3 complete (MVP), Phases 4-5 recommended but not blocking

---

## Dependency Graph

```
Phase 1 (Setup)
    ↓
Phase 2 (Foundational)
    ↓
    ├─→ Subphase 3A (TradeStorage)
    │       ↓
    ├─→ Subphase 3B (TradeStreamClient)
    │       ↓
    └─→ Subphase 3C (Persistence Task) ← depends on 3A + 3B
            ↓
        Subphase 3D (Tool Integration) ← depends on 3A
            ↓
        Phase 4 (US2 - Monitoring) ← optional, parallel with Phase 5
            ↓
        Phase 5 (US3 - Resilience) ← optional, parallel with Phase 4
            ↓
        Phase 6 (Integration & Deployment)
```

**Parallel Execution Opportunities**:
1. **Subphase 3A and 3B can run in parallel** (TradeStorage and TradeStreamClient are independent)
2. **Phase 4 and Phase 5 can run in parallel** (monitoring and resilience are independent enhancements)

---

## MVP Scope (User Story 1 Only)

**Minimum Viable Product**: Phase 1 + Phase 2 + Phase 3 (27 MVP tasks)

**What's Included**:
- Trade stream WebSocket connection for BTCUSDT and ETHUSDT
- 1-second batch persistence to RocksDB
- Analytics tools (`get_volume_profile`, `get_liquidity_vacuums`) query historical trades
- Basic reconnection on disconnect (exponential backoff)

**What's Excluded** (Post-MVP):
- Enhanced operational logging (Phase 4)
- Advanced resilience testing (Phase 5)

**MVP Validation**:
After completing Phase 3, run local test:
```bash
cd providers/binance-rs
cargo build --release --features 'orderbook,orderbook_analytics'
./target/release/binance-provider --grpc --port 50053

# Wait 10 minutes

# In another terminal, test via mcp-gateway
cd ../../mcp-gateway
uv run python -m mcp_gateway.main
# ChatGPT: "Use binance_get_volume_profile with symbol=BTCUSDT, duration_hours=1"

# Expected: Volume profile returned (not "insufficient trades" error)
```

---

## Task Checklist Format

Each task follows the format:
```
- [ ] [TaskID] [Priority] [Story] Description with file path
```

**Legend**:
- **TaskID**: T001-T050 (sequential)
- **Priority**: P1 (MVP), P2, P3, or -- (infrastructure)
- **Story**: US1, US2, US3, or -- (not story-specific)
- **Description**: Actionable task with specific implementation detail
- **File path**: Primary file modified (in parentheses)

---

## Validation Summary

**User Story Coverage**:
- ✅ User Story 1 (P1): 27 tasks (T011-T037) → Analytics tools access historical trades
- ✅ User Story 2 (P2): 4 tasks (T038-T041) → Operators monitor collection
- ✅ User Story 3 (P3): 4 tasks (T042-T045) → Service stability during failures

**Independent Test Criteria**:
- ✅ User Story 1: Service runs 10 min → `get_volume_profile` returns data (not error)
- ✅ User Story 2: After 5 min, logs show trade batch writes every 1-2 seconds
- ✅ User Story 3: Network failure simulation → service reconnects within 60s, orderbook unaffected

**Dependency Validation**:
- ✅ All tasks have clear prerequisites (Phases 1-2 are foundational)
- ✅ Parallel execution opportunities identified (3A||3B, 4||5)
- ✅ MVP scope clearly defined (Phases 1-3 only)

**File Coverage**:
- ✅ `src/orderbook/analytics/trade_storage.rs` (new, 8 tasks)
- ✅ `src/orderbook/analytics/trade_websocket.rs` (new, 9 tasks)
- ✅ `src/main.rs` (modified, 8 tasks)
- ✅ `src/grpc/tools.rs` (modified, 5 tasks)
- ✅ `src/orderbook/analytics/mod.rs` (modified, 1 task)
- ✅ `tests/integration_websocket.rs` (new, 1 task)

---

**Tasks Generation Complete** | 50 tasks created | MVP: 27 tasks | Ready for `/speckit.implement`
