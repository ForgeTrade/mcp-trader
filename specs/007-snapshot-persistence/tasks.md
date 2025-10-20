# Tasks: OrderBook Snapshot Persistence

**Input**: Design documents from `/specs/007-snapshot-persistence/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/persistence-api.md

**Tests**: No test tasks generated - spec.md does not explicitly request TDD approach

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`
- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions
- **Single Rust project**: `providers/binance-rs/src/` (per plan.md)
- All tasks modify existing binance-rs provider code

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and verification

- [ ] T001 Verify all dependencies present in providers/binance-rs/Cargo.toml (tokio, tokio-tungstenite with native-tls, rmp-serde, rocksdb, chrono)
- [ ] T002 Verify RocksDB analytics storage initialized in providers/binance-rs/src/grpc/mod.rs (lines 43-54)
- [ ] T003 [P] Verify OrderBookSnapshot serialization exists in providers/binance-rs/src/orderbook/analytics/storage/snapshot.rs

**Checkpoint**: All infrastructure already exists - no new dependencies or files needed

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before user story implementation

**⚠️ CRITICAL**: WebSocket TLS support and eager subscription must work before snapshot persistence can begin

- [ ] T004 Verify tokio-tungstenite has native-tls feature in providers/binance-rs/Cargo.toml line 49 (should already be fixed per commit 9306311)
- [ ] T005 Verify OrderBookManager supports public subscribe() method in providers/binance-rs/src/orderbook/mod.rs
- [ ] T006 Create shutdown signal infrastructure using tokio::sync::broadcast channel in providers/binance-rs/src/grpc/mod.rs

**Checkpoint**: Foundation ready - background persistence task can now be implemented

---

## Phase 3: User Story 1 - Analytics Tools Get Historical Data (Priority: P1) 🎯 MVP

**Goal**: Enable analytics tools (get_order_flow, get_volume_profile, get_microstructure_health, get_liquidity_vacuums, detect_market_anomalies) to retrieve historical orderbook data instead of returning "Insufficient historical data" errors

**Independent Test**: After service runs for 65 seconds, call `binance_get_order_flow` with `symbol="BTCUSDT", window_duration_secs=60` and verify it returns valid OrderFlow data (not an error)

### Background Persistence Task Implementation

- [ ] T007 [US1] Create spawn_snapshot_persistence_task() function in providers/binance-rs/src/orderbook/analytics/storage/mod.rs
- [ ] T008 [US1] Implement 1-second interval loop using tokio::time::interval in spawn_snapshot_persistence_task()
- [ ] T009 [US1] Add graceful shutdown handling via tokio::select! with shutdown_rx in snapshot persistence task
- [ ] T010 [US1] Implement snapshot capture logic: get OrderBook from OrderBookManager for each symbol
- [ ] T011 [US1] Implement snapshot serialization: OrderBookSnapshot::from_orderbook() then to_bytes()
- [ ] T012 [US1] Implement snapshot storage: SnapshotStorage.put(symbol, timestamp, bytes) with error handling
- [ ] T013 [US1] Add INFO-level logging for successful snapshot persistence ("Stored snapshot for {symbol} at {timestamp}")
- [ ] T014 [US1] Add ERROR-level logging for failed snapshot persistence with error details

### WebSocket Eager Subscription

- [ ] T015 [US1] Modify providers/binance-rs/src/grpc/mod.rs to pre-subscribe to BTCUSDT WebSocket before spawning persistence task
- [ ] T016 [US1] Modify providers/binance-rs/src/grpc/mod.rs to pre-subscribe to ETHUSDT WebSocket before spawning persistence task
- [ ] T017 [US1] Add INFO logging for each pre-subscription ("Pre-subscribed to {symbol} for snapshot persistence")

### Task Integration

- [ ] T018 [US1] Spawn snapshot persistence task in providers/binance-rs/src/grpc/mod.rs after OrderBookManager initialization
- [ ] T019 [US1] Pass shutdown_rx to persistence task and ensure it's included in server shutdown sequence
- [ ] T020 [US1] Verify persistence task spawns with correct parameters (storage, manager, ["BTCUSDT", "ETHUSDT"], shutdown_rx)

**Checkpoint**: At this point, analytics tools should receive historical data after 60+ seconds of service uptime. User Story 1 acceptance criteria met.

---

## Phase 4: User Story 2 - Operators Monitor Data Collection (Priority: P2)

**Goal**: System operators can observe snapshot persistence activity through logs to verify data collection is working and troubleshoot issues

**Independent Test**: Start service, check logs for "Stored snapshot for BTCUSDT at [timestamp]" messages appearing every 1 second

### Logging Enhancements

- [ ] T021 [P] [US2] Add DEBUG-level logging for snapshot capture details in providers/binance-rs/src/orderbook/analytics/storage/mod.rs
- [ ] T022 [P] [US2] Add WARN-level logging for empty orderbook skips ("Skipping snapshot for {symbol}: empty orderbook")
- [ ] T023 [US2] Ensure structured logging uses tracing macros with symbol and timestamp fields

### Operational Visibility

- [ ] T024 [US2] Add startup log message indicating background persistence task started successfully
- [ ] T025 [US2] Add shutdown log message when persistence task receives shutdown signal

**Checkpoint**: Operators can monitor persistence activity through clear, structured logs. User Story 2 acceptance criteria met.

---

## Phase 5: User Story 3 - Service Stability Under Errors (Priority: P3)

**Goal**: Service continues operating normally even when snapshot persistence encounters errors, ensuring live orderbook functionality remains available

**Independent Test**: Simulate RocksDB write failures (disk full scenario) and verify service doesn't crash and continues serving live orderbook data

### Error Resilience

- [ ] T026 [US3] Wrap SnapshotStorage.put() calls in if let Err(e) pattern to prevent task panic in providers/binance-rs/src/orderbook/analytics/storage/mod.rs
- [ ] T027 [US3] Verify WebSocket connection failures don't crash persistence task (OrderBookManager handles reconnection)
- [ ] T028 [US3] Add error logging for serialization failures without crashing task

### Stability Validation

- [ ] T029 [US3] Verify live orderbook tools (orderbook_l1, orderbook_l2) continue working during persistence errors
- [ ] T030 [US3] Ensure persistence task loop continues after individual snapshot failures

**Checkpoint**: Service maintains stability and live functionality even when persistence fails. User Story 3 acceptance criteria met.

---

## Phase 6: Integration & Deployment

**Purpose**: Final integration, testing, and production deployment

### Local Testing

- [ ] T031 Build project with analytics features: `cargo build --release --features 'orderbook,orderbook_analytics'` in providers/binance-rs/
- [ ] T032 Run provider locally with ANALYTICS_DATA_PATH=./data/test-persistence for 70 seconds
- [ ] T033 Verify 110+ snapshot persistence log entries appear in 70 seconds (expected ~120)
- [ ] T034 Test analytics tool get_order_flow with 60-second window returns data (not "Insufficient data" error)
- [ ] T035 Verify RocksDB directory created and contains SST files (indicates snapshots stored)

### Production Deployment

- [ ] T036 Commit changes to 007-snapshot-persistence branch with descriptive message
- [ ] T037 Deploy to production server root@198.13.46.14 using ./infra/deploy-chatgpt.sh
- [ ] T038 Monitor production logs for "Pre-subscribed to BTCUSDT/ETHUSDT for snapshot persistence" messages
- [ ] T039 Wait 65 seconds then test analytics tools via ChatGPT MCP integration
- [ ] T040 Verify production analytics tools return data (no "Insufficient historical data" errors)

### Post-Deployment Validation

- [ ] T041 Monitor CPU usage - verify background task uses <1% CPU
- [ ] T042 Monitor memory usage - verify persistence task uses <10MB RSS
- [ ] T043 Verify live orderbook latency remains <200ms p99 (persistence doesn't block)
- [ ] T044 Check RocksDB storage growth (~3.6MB/hour expected for 2 symbols)
- [ ] T045 Verify service stability over 24-hour period (no crashes, continuous snapshot logging)

**Checkpoint**: Feature deployed to production and validated. All user stories tested end-to-end.

---

## Implementation Strategy

### MVP Scope (Minimum Viable Product)

**MVP = User Story 1 Only** (Tasks T007-T020)

This delivers the core value proposition:
- Analytics tools receive historical data within 60 seconds
- Background persistence runs automatically on startup
- WebSocket connections are eager (not lazy)

**Why this is sufficient**:
- Solves the primary problem: "Insufficient historical data" errors
- Demonstrates the feature works end-to-end
- Can be tested independently via quickstart.md guide

### Incremental Delivery Order

1. **Week 1**: MVP (User Story 1) - Tasks T001-T020
   - Deliverable: Analytics tools work with historical data
   - Test: Run locally for 65 seconds, verify get_order_flow returns data

2. **Week 2**: Operational Visibility (User Story 2) - Tasks T021-T025
   - Deliverable: Enhanced logging for operators
   - Test: Monitor logs, verify structured logging appears

3. **Week 3**: Error Resilience (User Story 3) - Tasks T026-T030
   - Deliverable: Service stability under failures
   - Test: Simulate RocksDB failures, verify service continues

4. **Week 4**: Production Deployment - Tasks T031-T045
   - Deliverable: Feature live in production
   - Test: Analytics tools work in ChatGPT via https://mcp-gateway.thevibe.trading/sse/

### Parallel Execution Opportunities

**User Story 1** (can split among 2 developers):
- Dev A: Tasks T007-T014 (background persistence task implementation)
- Dev B: Tasks T015-T020 (WebSocket eager subscription + integration)
- Both work in parallel, integrate at T018

**User Story 2** (independent from US1 - can start immediately after foundation):
- All logging tasks (T021-T025) are parallelizable [P]
- Can be implemented while US1 is in progress

**User Story 3** (requires US1 complete):
- Tasks T026-T028 can be done in parallel [P] (different error scenarios)
- T029-T030 are validation tasks (sequential)

### Dependencies Visualization

```
Foundation (T001-T006)
    ↓
├─→ User Story 1 (T007-T020) ← MVP Blocking
│   ├─→ Background Task (T007-T014)
│   └─→ WebSocket Eager Sub (T015-T020)
│
├─→ User Story 2 (T021-T025) ← Independent (can run parallel to US1)
│
└─→ User Story 3 (T026-T030) ← Depends on US1
    ↓
    Integration & Deployment (T031-T045)
```

### Testing Approach (Per User Story)

**User Story 1** - Independent Test:
```bash
# After implementing T007-T020:
cargo build --release --features 'orderbook,orderbook_analytics'
./target/release/binance-provider --grpc --port 50053 &
sleep 65
# Test via ChatGPT: Use binance_get_order_flow tool
# Expected: Returns OrderFlow data (not error)
```

**User Story 2** - Independent Test:
```bash
# After implementing T021-T025:
./target/release/binance-provider --grpc --port 50053 2>&1 | tee logs.txt
# Check logs.txt for structured persistence messages
grep "Stored snapshot" logs.txt | wc -l
# Expected: ~120 entries per minute
```

**User Story 3** - Independent Test:
```bash
# After implementing T026-T030:
# Simulate disk full: chmod 444 data/analytics-test/
./target/release/binance-provider --grpc --port 50053
# Verify service continues running (doesn't crash)
# Verify orderbook_l1 tool still works
```

---

## Task Summary

**Total Tasks**: 45
- **Phase 1 (Setup)**: 3 tasks (T001-T003)
- **Phase 2 (Foundational)**: 3 tasks (T004-T006)
- **Phase 3 (User Story 1 - MVP)**: 14 tasks (T007-T020)
- **Phase 4 (User Story 2)**: 5 tasks (T021-T025)
- **Phase 5 (User Story 3)**: 5 tasks (T026-T030)
- **Phase 6 (Integration & Deployment)**: 15 tasks (T031-T045)

**Parallelization**:
- 8 tasks marked [P] can run in parallel
- User Story 2 can run parallel to User Story 1
- Estimated 40% time savings with parallel execution

**MVP Delivery**: Tasks T001-T020 (23 tasks)
- Estimated effort: 1-2 days for single developer
- Delivers core value: Analytics tools receive historical data

**Full Feature Delivery**: All 45 tasks
- Estimated effort: 5-7 days with parallel execution
- Estimated effort: 10-14 days sequential execution
