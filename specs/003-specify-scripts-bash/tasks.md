# Implementation Tasks: Advanced Order Book Analytics & Streamable HTTP Transport

**Feature**: 003-specify-scripts-bash
**Branch**: `003-specify-scripts-bash`
**Generated**: 2025-10-19
**Total Tasks**: 88

---

## Task Organization

Tasks are organized by **user story** to enable independent implementation and testing. Each user story phase can be completed independently and delivers a testable increment.

### User Stories (from spec.md):
- **US1** (P1): Order Flow Analysis for Trade Timing
- **US2** (P2): Volume Profile for Support/Resistance Discovery
- **US3** (P3): Market Microstructure Anomaly Detection
- **US4** (P4): Liquidity Mapping and Smart Order Placement
- **US5** (P5): ChatGPT MCP Integration via Streamable HTTP

---

## Phase 1: Setup (8 tasks)

**Goal**: Initialize dependencies and project structure for analytics and HTTP transport.

### Tasks

- [x] T001 Add `rocksdb = "0.23.0"` dependency to providers/binance-rs/Cargo.toml [dependencies] section
- [x] T002 Add `statrs = "0.18.0"` dependency to providers/binance-rs/Cargo.toml [dependencies] section
- [x] T003 Add `rmp-serde = "1.3.0"` dependency to providers/binance-rs/Cargo.toml [dependencies] section (MessagePack serialization)
- [x] T004 Add `uuid = { version = "1.11", features = ["v4", "serde"] }` dependency to providers/binance-rs/Cargo.toml [dependencies] section
- [x] T005 Add `axum = "0.8"` and `tower = "0.5"` dependencies to providers/binance-rs/Cargo.toml [dependencies] section
- [x] T006 Create `orderbook_analytics` feature in providers/binance-rs/Cargo.toml [features] section extending `orderbook` feature
- [x] T007 Create `http_transport` feature in providers/binance-rs/Cargo.toml [features] section
- [x] T008 Create directory structure: providers/binance-rs/src/orderbook/analytics/ and providers/binance-rs/src/orderbook/analytics/storage/

---

## Phase 2: Foundational Infrastructure (8 tasks)

**Goal**: Build shared storage and type foundations required by all user stories.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

### Tasks

- [x] T009 [P] Create providers/binance-rs/src/orderbook/analytics/mod.rs with feature gate `#[cfg(feature = "orderbook_analytics")]` and public API exports
- [x] T010 [P] Create providers/binance-rs/src/orderbook/analytics/types.rs with FlowDirection enum (STRONG_BUY, MODERATE_BUY, NEUTRAL, MODERATE_SELL, STRONG_SELL)
- [x] T011 [P] Add Severity enum to providers/binance-rs/src/orderbook/analytics/types.rs (Low, Medium, High, Critical)
- [x] T012 [P] Add ImpactLevel enum to providers/binance-rs/src/orderbook/analytics/types.rs (FastMovement, ModerateMovement, Negligible)
- [x] T013 [P] Add Direction enum to providers/binance-rs/src/orderbook/analytics/types.rs (Accumulation, Distribution)
- [x] T014 Create providers/binance-rs/src/orderbook/analytics/storage/mod.rs with RocksDB initialization and key format `{symbol}:{unix_timestamp_sec}`
- [x] T015 Implement snapshot capture logic in providers/binance-rs/src/orderbook/analytics/storage/snapshot.rs (1-second interval, MessagePack serialization)
- [x] T016 Implement historical query with prefix scan in providers/binance-rs/src/orderbook/analytics/storage/query.rs (<200ms target, async with timeout)

---

## Phase 3: User Story 1 - Order Flow Analysis (P1) (12 tasks)

**Story Goal**: Algorithmic traders can see order flow dynamics (bid/ask pressure) to identify optimal entry/exit points.

**Independent Test**: Request order flow for BTCUSDT over 60 seconds via Claude Code, verify bid_flow_rate, ask_flow_rate, net_flow, and flow_direction are calculated correctly and interpreted as "buying pressure increasing" or "selling pressure dominant".

### Tasks

- [x] T017 [P] [US1] Add OrderFlowSnapshot struct to providers/binance-rs/src/orderbook/analytics/types.rs with fields: symbol, time_window_start/end, window_duration_secs, bid_flow_rate, ask_flow_rate, net_flow, flow_direction, cumulative_delta
- [x] T018 [P] [US1] Create providers/binance-rs/src/orderbook/analytics/flow.rs with `calculate_order_flow()` async function signature
- [x] T019 [US1] Implement query_snapshots_in_window() in providers/binance-rs/src/orderbook/analytics/flow.rs using RocksDB prefix scan for time range
- [x] T020 [US1] Implement aggregate_bid_ask_counts() in providers/binance-rs/src/orderbook/analytics/flow.rs counting order additions per side from snapshots
- [x] T021 [US1] Implement calculate_flow_rates() in providers/binance-rs/src/orderbook/analytics/flow.rs dividing counts by duration (orders/sec)
- [x] T022 [US1] Implement determine_flow_direction() in providers/binance-rs/src/orderbook/analytics/flow.rs using ratios (>2x = STRONG, 1.2-2x = MODERATE, 0.8-1.2 = NEUTRAL)
- [x] T023 [US1] Implement calculate_cumulative_delta() in providers/binance-rs/src/orderbook/analytics/flow.rs (running sum of buy_volume - sell_volume over window)
- [x] T024 [US1] Create `get_order_flow` tool in providers/binance-rs/src/orderbook/analytics/tools.rs with JSON schema for symbol and window_duration_secs parameters
- [x] T025 [US1] Update providers/binance-rs/src/grpc/tools.rs to add routing for binance.get_order_flow tool calling analytics::tools::get_order_flow
- [x] T026 [US1] Update providers/binance-rs/src/orderbook/mod.rs to export analytics submodule with feature gate
- [x] T027 [US1] Verify get_order_flow returns FlowDirection enum with correct interpretation for high bid/ask ratios
- [x] T028 [US1] Test liquidity withdrawal detection (>50 order cancellations/min on bid side) via order flow analysis

---

## Phase 4: User Story 2 - Volume Profile (P2) (14 tasks)

**Story Goal**: Technical analysts can see volume distribution across price levels to identify support/resistance zones.

**Independent Test**: Request volume profile for ETHUSDT over 24 hours via Claude Code, verify POC (Point of Control), VAH (Value Area High), VAL (Value Area Low) are identified correctly, and liquidity vacuum zones are highlighted.

### Tasks

- [x] T029 [P] [US2] Add VolumeProfile struct to providers/binance-rs/src/orderbook/analytics/types.rs with histogram, bin_size, bin_count, POC, VAH, VAL fields
- [x] T030 [P] [US2] Add VolumeBin struct to providers/binance-rs/src/orderbook/analytics/types.rs (price_level, volume, trade_count)
- [x] T031 [P] [US2] Add LiquidityVacuum struct to providers/binance-rs/src/orderbook/analytics/types.rs (vacuum_id, price_range_low/high, volume_deficit_pct, expected_impact)
- [x] T032 [P] [US2] Create providers/binance-rs/src/orderbook/analytics/trade_stream.rs with @aggTrade WebSocket connection logic (wss://stream.binance.com:9443/ws/<symbol>@aggTrade)
- [x] T033 [US2] Implement exponential backoff reconnection (1s, 2s, 4s, 8s, max 60s) in providers/binance-rs/src/orderbook/analytics/trade_stream.rs
- [x] T034 [P] [US2] Create providers/binance-rs/src/orderbook/analytics/profile.rs with `generate_volume_profile()` async function
- [x] T035 [US2] Implement adaptive_bin_size() in providers/binance-rs/src/orderbook/analytics/profile.rs using formula `max(tick_size × 10, price_range / 100)`
- [x] T036 [US2] Implement bin_trades_by_price() in providers/binance-rs/src/orderbook/analytics/profile.rs grouping @aggTrade data into bins
- [x] T037 [US2] Implement find_poc_vah_val() in providers/binance-rs/src/orderbook/analytics/profile.rs (POC = max volume bin, VAH/VAL = 70% volume boundaries)
- [x] T038 [US2] Implement identify_liquidity_vacuums() in providers/binance-rs/src/orderbook/analytics/profile.rs (volume <20% of median with severity classification)
- [x] T039 [US2] Create `get_volume_profile` tool in providers/binance-rs/src/orderbook/analytics/tools.rs with JSON schema for symbol, duration_hours, tick_size parameters
- [x] T040 [US2] Update providers/binance-rs/src/grpc/tools.rs to add routing for binance.get_volume_profile tool
- [x] T041 [US2] Verify volume profile POC at $3,500 with 45% volume triggers "approaching high-volume support zone" interpretation
- [x] T042 [US2] Test liquidity vacuum detection between $3,550-$3,580 with stop loss placement recommendations

---

## Phase 5: User Story 3 - Anomaly Detection (P3) (12 tasks)

**Story Goal**: Risk managers can detect market microstructure anomalies (quote stuffing, icebergs, flash crashes) to avoid manipulated conditions.

**Independent Test**: Request "Check for market anomalies in BTCUSDT" via Claude Code, simulate quote stuffing (>500 updates/sec, <10% fills) and verify system detects with High severity and actionable recommendations.

### Tasks

- [x] T043 [P] [US3] Add MarketMicrostructureAnomaly struct to providers/binance-rs/src/orderbook/analytics/types.rs (anomaly_id, anomaly_type enum, severity, confidence, description, recommended_action)
- [x] T044 [P] [US3] Add AnomalyType enum to providers/binance-rs/src/orderbook/analytics/types.rs (QuoteStuffing, IcebergOrder, FlashCrashRisk)
- [x] T045 [P] [US3] Add MicrostructureHealthScore struct to providers/binance-rs/src/orderbook/analytics/types.rs (score 0-100, components, interpretation enum)
- [x] T046 [P] [US3] Create providers/binance-rs/src/orderbook/analytics/anomaly.rs with detect_quote_stuffing() function (>500 updates/sec threshold, <10% fill rate)
- [x] T047 [US3] Implement detect_iceberg_orders() in providers/binance-rs/src/orderbook/analytics/anomaly.rs (refill rate >5x median, 95% confidence using statrs)
- [x] T048 [US3] Implement detect_flash_crash_risk() in providers/binance-rs/src/orderbook/analytics/anomaly.rs (liquidity drain >80%, spread widening >10x, cancellation rate >90%)
- [x] T049 [US3] Add severity calculation logic in providers/binance-rs/src/orderbook/analytics/anomaly.rs (Medium if 500-750 updates/sec, High if 750-1000, Critical if >1000)
- [x] T050 [P] [US3] Create providers/binance-rs/src/orderbook/analytics/health.rs with calculate_microstructure_health() function
- [x] T051 [US3] Implement component scoring in providers/binance-rs/src/orderbook/analytics/health.rs (spread_stability 25%, liquidity_depth 35%, flow_balance 25%, update_rate 15%)
- [x] T052 [US3] Create `detect_market_anomalies` tool in providers/binance-rs/src/orderbook/analytics/tools.rs returning Vec<MarketMicrostructureAnomaly>
- [x] T053 [US3] Create `get_microstructure_health` tool in providers/binance-rs/src/orderbook/analytics/tools.rs returning MicrostructureHealthScore
- [x] T054 [US3] Update providers/binance-rs/src/grpc/tools.rs to add routing for binance.detect_market_anomalies and binance.get_microstructure_health tools

---

## Phase 6: User Story 4 - Liquidity Mapping (P4) (10 tasks)

**Story Goal**: Advanced traders can identify liquidity vacuums and absorption events to optimize stop loss placement.

**Independent Test**: Request "Map liquidity for SOLUSDT" via Claude Code, verify vacuum zones with severity (Critical/High/Medium), price ranges, volume deficit percentages, and stop placement recommendations are returned.

### Tasks

- [x] T055 [P] [US4] Add AbsorptionEvent struct to providers/binance-rs/src/orderbook/analytics/types.rs (event_id, price_level, absorbed_volume, refill_count, suspected_entity, direction)
- [x] T056 [P] [US4] Implement detect_absorption_events() in providers/binance-rs/src/orderbook/analytics/flow.rs (large orders >5x median absorbing pressure without price movement)
- [x] T057 [US4] Implement identify_order_walls() in providers/binance-rs/src/orderbook/analytics/profile.rs (detect large bid/ask walls in orderbook depth)
- [x] T058 [US4] Implement recommend_stop_placement() in providers/binance-rs/src/orderbook/analytics/profile.rs (avoid vacuum zones, suggest levels outside liquidity gaps)
- [x] T059 [US4] Create `get_liquidity_vacuums` tool in providers/binance-rs/src/orderbook/analytics/tools.rs with JSON schema for symbol and lookback_minutes parameters
- [x] T060 [US4] Update providers/binance-rs/src/grpc/tools.rs to add routing for binance.get_liquidity_vacuums tool
- [x] T061 [US4] Verify absorption event detection at $144.00 absorbing 250 SOL triggers "whale accumulation" interpretation
- [x] T062 [US4] Test vacuum zone identification at $145.50-$148.20 with 85% below median volume
- [x] T063 [US4] Verify stop placement recommendations avoid vacuum zones and suggest $143.80 or $148.50 levels
- [x] T064 [US4] Test that get_order_flow returns absorption events with absorbed_volume and refill_count

---

## Phase 7: User Story 5 - Streamable HTTP Transport (P5) (18 tasks)

**Story Goal**: ChatGPT users can connect via Streamable HTTP MCP protocol through single `/mcp` endpoint.

**Independent Test**: Configure ChatGPT connector with `/mcp` endpoint URL, send initialize request, verify `Mcp-Session-Id` header returned, then execute tools/list and tools/call for binance.detect_market_anomalies with proper JSON-RPC 2.0 responses.

### Tasks

- [x] T065 [P] [US5] Create providers/binance-rs/src/transport/ directory
- [x] T066 [P] [US5] Create providers/binance-rs/src/transport/http/ directory
- [x] T067 [P] [US5] Add StreamableHttpSession struct to providers/binance-rs/src/transport/http/session.rs (session_id UUID, client_metadata, created_at, last_activity, expires_at 30min)
- [x] T068 [P] [US5] Add McpJsonRpcMessage struct to providers/binance-rs/src/transport/http/jsonrpc.rs (jsonrpc "2.0", method, params, id, result/error)
- [x] T069 [P] [US5] Create providers/binance-rs/src/transport/http/mod.rs with Axum router setup (POST /mcp route)
- [x] T070 [US5] Implement create_session() in providers/binance-rs/src/transport/http/session.rs (generate UUID, store in HashMap with 50 concurrent limit)
- [x] T071 [US5] Implement validate_session() in providers/binance-rs/src/transport/http/session.rs (check Mcp-Session-Id header, verify timeout)
- [x] T072 [US5] Implement cleanup_expired_sessions() in providers/binance-rs/src/transport/http/session.rs (remove sessions >30min idle)
- [x] T073 [US5] Create providers/binance-rs/src/transport/http/handler.rs with POST `/mcp` endpoint handler
- [x] T074 [US5] Implement handle_initialize() in providers/binance-rs/src/transport/http/handler.rs (create session, return Mcp-Session-Id header)
- [x] T075 [US5] Implement handle_tools_list() in providers/binance-rs/src/transport/http/handler.rs (return all 21 tools with JSON schemas)
- [x] T076 [US5] Implement handle_tools_call() in providers/binance-rs/src/transport/http/handler.rs (route to tool implementations, return MCP content array format)
- [x] T077 [US5] Create providers/binance-rs/src/transport/http/error.rs with JSON-RPC error responses (code -32002 missing session, -32001 invalid session, -32000 limit exceeded)
- [x] T078 [US5] Update providers/binance-rs/src/main.rs to add --mode flag (grpc|http) with clap argument parsing
- [ ] T079 [US5] Implement HTTP server startup in providers/binance-rs/src/main.rs for --mode http (bind to HOST:PORT from env)
- [ ] T080 [US5] Implement graceful shutdown for HTTP server in providers/binance-rs/src/main.rs (SIGTERM, SIGINT)
- [ ] T081 [US5] Update providers/binance-rs/src/lib.rs to export transport module with feature gate `#[cfg(feature = "http_transport")]`
- [ ] T082 [US5] Test HTTP initialize returns Mcp-Session-Id header and subsequent tools/list works with valid session

---

## Phase 8: Integration & Polish (6 tasks)

**Goal**: Verify all 21 tools work via both transports, update documentation, ensure feature flags compile correctly.

### Tasks

- [ ] T083 Verify build succeeds with `cargo build --release` (all features enabled by default)
- [ ] T084 Verify build succeeds with `cargo build --release --no-default-features --features websocket` (no analytics, no HTTP)
- [ ] T085 Test gRPC mode: start with `--mode grpc --port 50053` and verify all 21 tools accessible via Python MCP gateway
- [ ] T086 Test HTTP mode: start with `--mode http --port 8080` and verify all 21 tools accessible via curl POST /mcp
- [ ] T087 Update providers/binance-rs/README.md with analytics tools usage examples and HTTP transport configuration
- [ ] T088 Create providers/binance-rs/.env.example with HOST, PORT, BINANCE_API_KEY, BINANCE_SECRET_KEY placeholders

---

## Dependencies & Implementation Strategy

### User Story Dependencies

```
Phase 1 (Setup) → Phase 2 (Foundational)
                      ↓
      ┌──────────────┬───────────┬───────────┬──────────┐
      ↓              ↓           ↓           ↓          ↓
    US1 (P1)      US2 (P2)    US3 (P3)    US4 (P4)   US5 (P5)
  Order Flow   Volume Profile Anomalies  Liquidity   HTTP
  (Independent) (Independent) (Independent) (Depends  (Independent)
                                             on US1)
      ↓              ↓           ↓           ↓          ↓
                Phase 8 (Integration & Polish)
```

**US4 depends on US1**: Absorption event detection uses order flow calculations (T056 depends on T023).

**All other stories are independent**: Can be implemented in any order after Phase 2.

### Parallel Execution Opportunities

**Phase 1** (Setup): All tasks can run sequentially (dependency edits to same Cargo.toml).

**Phase 2** (Foundational):
- T009-T013 (type definitions) can run in parallel [P]
- T014-T016 (storage) must run sequentially (depends on T009 types)

**Phase 3** (US1):
- T017-T018 can run in parallel [P] (different files)
- T019-T023 must run sequentially (flow.rs implementation)
- T024-T028 integration tasks run sequentially

**Phase 4** (US2):
- T029-T032 can run in parallel [P] (different files: types.rs, trade_stream.rs, profile.rs)
- T033-T040 must run sequentially (profile.rs implementation + integration)

**Phase 5** (US3):
- T043-T046, T050 can run in parallel [P] (different files: types.rs, anomaly.rs, health.rs)
- T047-T049 must run sequentially (anomaly.rs implementation)
- T051-T054 integration tasks run sequentially

**Phase 6** (US4):
- T055-T056 can run in parallel [P] (types.rs, flow.rs in different sections)
- T057-T064 must run sequentially (profile.rs additions + testing)

**Phase 7** (US5):
- T065-T069 can run in parallel [P] (directory creation, type definitions in different files)
- T070-T082 must run sequentially (session management + HTTP handler + main.rs integration)

**Phase 8** (Integration): All tasks run sequentially (build verification, testing, documentation).

### MVP Recommendation

**Minimum Viable Product** = Phase 1 + Phase 2 + **Phase 3 (US1)** only.

This delivers:
- ✅ Order flow analysis (bid/ask pressure tracking)
- ✅ Real-time flow direction indicators (STRONG_BUY/SELL, etc.)
- ✅ Cumulative delta calculations
- ✅ 1 new MCP tool: `binance.get_order_flow`
- ✅ RocksDB time-series storage foundation
- ✅ Feature flag architecture (`orderbook_analytics`)

**Estimated effort**: 28 tasks (Phase 1: 8 + Phase 2: 8 + Phase 3: 12)

**Independent value**: Traders get real-time order flow insights without needing volume profile, anomaly detection, or HTTP transport.

---

**Total Tasks**: 88
**Parallelizable Tasks**: 24 (27% can run concurrently)
**User Stories**: 5 (US1-US5)
**Feature Flags**: 2 (`orderbook_analytics`, `http_transport`)
**New Tools**: 5 analytics + 1 transport = 6 new capabilities
