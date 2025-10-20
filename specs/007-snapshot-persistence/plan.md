# Implementation Plan: OrderBook Snapshot Persistence

**Branch**: `007-snapshot-persistence` | **Date**: 2025-10-19 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/007-snapshot-persistence/spec.md`

## Summary

Implement background snapshot persistence for orderbook data to enable historical analytics. Currently, 5 analytics tools (get_order_flow, get_volume_profile, detect_market_anomalies, get_microstructure_health, get_liquidity_vacuums) fail with "Insufficient historical data" errors because WebSocket connections are lazy and snapshots aren't being persisted to RocksDB. This feature adds a background Tokio task that subscribes to BTCUSDT and ETHUSDT WebSocket streams on service startup, captures orderbook snapshots every 1 second, serializes them with MessagePack, and stores them in RocksDB for analytics queries.

## Technical Context

**Language/Version**: Rust 1.75+
**Primary Dependencies**: tokio (async runtime), tokio-tungstenite (WebSocket), rmp-serde (MessagePack), rocksdb (storage), chrono (timestamps)
**Storage**: RocksDB (key-value store with key format `{symbol}:{unix_timestamp_sec}`, MessagePack-serialized OrderBookSnapshot values)
**Testing**: cargo test (unit tests for storage, integration tests for persistence task)
**Target Platform**: Linux server (production: Ubuntu 22.04 at 198.13.46.14)
**Project Type**: Single Rust binary (providers/binance-rs) with analytics feature flag
**Performance Goals**: 60+ snapshots per symbol per minute (1-second intervals), sub-200ms orderbook query latency maintained
**Constraints**: Background task must not crash service on errors, persistence failures logged but non-blocking, RocksDB writes via spawn_blocking to avoid blocking async runtime
**Scale/Scope**: Initial: 2 symbols (BTCUSDT, ETHUSDT), 7-day retention (~600MB/week), designed for expansion to 20 concurrent symbols

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### ✅ I. Simplicity and Readability
- Background task implementation uses clear async/await patterns
- Snapshot persistence loop is straightforward: connect WebSocket → capture snapshot → serialize → store
- Error handling explicit with tracing::error! logs
- No deep nesting or complex control flow

### ✅ II. Library-First Development
- Reusing existing libraries: tokio-tungstenite (WebSocket), rmp-serde (MessagePack), rocksdb (storage)
- OrderBookManager WebSocket logic already exists, extending not reimplementing
- SnapshotStorage API already implemented in previous work

### ✅ III. Justified Abstractions
- No new abstractions introduced - using existing OrderBookSnapshot struct
- Background task is a simple async function, not a complex abstraction
- Direct use of SnapshotStorage.put() without unnecessary wrappers

### ✅ IV. DRY Principle
- Reusing OrderBookSnapshot serialization logic from analytics module
- WebSocket subscription logic centralized in OrderBookManager
- No duplication of persistence code across symbols (loop handles both BTCUSDT and ETHUSDT)

### ✅ V. Service and Repository Patterns
- SnapshotStorage acts as repository for orderbook data (already implemented)
- OrderBookManager acts as service for WebSocket connections (already implemented)
- Clear separation: grpc/mod.rs (application layer) → OrderBookManager (service) → SnapshotStorage (repository)

### ✅ VI. 12-Factor Methodology
- Config: Symbol list and persistence interval can be env vars (FR-001 specifies BTCUSDT/ETHUSDT but extensible)
- Logs: tracing crate outputs to stdout/stderr per 12-factor
- Processes: Stateless background task, no shared mutable state
- Disposability: Task uses graceful shutdown via tokio::select! with shutdown signal

### ✅ VII. Minimal Object-Oriented Programming
- Procedural async function for background task
- Using Arc<OrderBookManager> and Arc<SnapshotStorage> for shared access, not deep OOP hierarchies
- No inheritance, minimal trait usage

**Result**: ✅ No constitution violations. All principles adhered to.

## Project Structure

### Documentation (this feature)

```
specs/[###-feature]/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```
providers/binance-rs/
├── src/
│   ├── grpc/
│   │   ├── mod.rs                    # MODIFIED: Spawn background persistence task on startup
│   │   ├── capabilities.rs
│   │   └── tools.rs
│   ├── orderbook/
│   │   ├── mod.rs                    # MODIFIED: Add eager WebSocket subscription for persistence
│   │   ├── manager.rs                # POTENTIALLY MODIFIED: Expose subscription API if needed
│   │   └── analytics/
│   │       ├── storage/
│   │       │   ├── mod.rs            # EXISTING: SnapshotStorage.put() used for persistence
│   │       │   ├── snapshot.rs       # EXISTING: OrderBookSnapshot serialization
│   │       │   └── query.rs          # EXISTING: Query functions for analytics tools
│   │       └── tools.rs              # EXISTING: Analytics tools that consume snapshots
│   ├── lib.rs
│   └── main.rs
└── Cargo.toml                        # EXISTING: All dependencies already present
```

**Structure Decision**: Single Rust binary project (Option 1). All changes are within `providers/binance-rs/src/`. The feature modifies existing modules (grpc/mod.rs, orderbook/mod.rs) and reuses existing analytics storage infrastructure. No new files required - implementation adds a background task function and modifies initialization logic.

## Complexity Tracking

*Fill ONLY if Constitution Check has violations that must be justified*

**No violations**: All constitution principles are adhered to. No complexity tracking required.

