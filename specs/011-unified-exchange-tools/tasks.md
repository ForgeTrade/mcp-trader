# Tasks: Unified Multi-Exchange Gateway

**Input**: Design documents from `/specs/011-unified-exchange-tools/`
**Prerequisites**: plan.md (✓), spec.md (✓)

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`
- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions
- Gateway: `mcp-gateway/mcp_gateway/`
- Binance Provider: `providers/binance-rs/src/`
- Proto: `pkg/proto/`
- Tests: `mcp-gateway/tests/`

---

## Phase 0: Quick Fixes (CRITICAL - Unblocks multi-provider)

**Purpose**: Enable basic multi-provider support by fixing immediate blockers (FR-046 to FR-049)

- [X] T001 [P] Fix tool name mapping in mcp-gateway/mcp_gateway/document_registry.py: change binance_get_* → binance.get_* to match capability tool names (FR-046)
- [X] T002 Remove hardcoded Binance search in mcp-gateway/mcp_gateway/sse_server.py: load all providers dynamically (FR-047)
- [X] T003 [P] Relax symbol regex in providers/binance-rs/src/grpc/capabilities.rs: support BTC-USDT format (FR-048)
- [X] T004 [P] Implement per-tool TTL in mcp-gateway/mcp_gateway/cache.py: replace global 5s with configurable map (FR-049)

**Checkpoint**: Gateway can load multiple providers, SSE exposes all provider tools, symbol validation accepts common formats

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project structure and dependencies for unified tools layer

- [X] T005 Add pydantic dependency to mcp-gateway/pyproject.toml for request/response validation
- [X] T006 [P] Create directory structure: mcp-gateway/mcp_gateway/schemas/unified/ for unified tool schemas
- [X] T007 [P] Create directory structure: mcp-gateway/mcp_gateway/schemas/providers/ for provider schemas
- [X] T008 [P] Create directory structure: mcp-gateway/mcp_gateway/adapters/ for routing and normalization
- [X] T009 [P] Create directory structure: mcp-gateway/mcp_gateway/services/ for business logic

**Checkpoint**: Project structure ready for unified tools implementation

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T010 Add capability metadata fields to pkg/proto/provider.proto: tags, auth_required, stability, rate_limit_group (FR-011, D-002)
- [X] T011 Create base ProviderClient class in mcp-gateway/mcp_gateway/adapters/grpc_client.py with connection pooling and health checks
- [X] T012 [P] Create unified schema base structures in mcp-gateway/mcp_gateway/schemas/unified/base.py: common fields like timestamp, venue, latency_ms
- [X] T013 [P] Create provider schema registry structure in mcp-gateway/mcp_gateway/schemas/providers/binance.json for Binance response schemas
- [X] T014 Implement configuration loading in mcp-gateway/mcp_gateway/config.py: expose_unified_only, expose_provider_tools, per-provider rate limits (FR-026, FR-027, FR-029)

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - AI Client Queries Market Data Across Exchanges (Priority: P1) 🎯 MVP

**Goal**: Enable AI clients to query ticker data without knowing which exchange to use, with automatic routing and normalized responses

**Independent Test**: Query `market.get_ticker` with `{venue: "binance", instrument: "BTCUSDT"}` and verify normalized response contains mid, spread_bps, bid, ask, volume, timestamp

### Implementation for User Story 1

- [X] T015 [P] [US1] Define unified ticker schema in mcp-gateway/mcp_gateway/schemas/unified/market_ticker.json with mandatory fields: mid, spread_bps, bid, ask, volume, timestamp (FR-008)
- [X] T016 [P] [US1] Define unified orderbook L1 schema in mcp-gateway/mcp_gateway/schemas/unified/market_orderbook_l1.json
- [X] T017 [US1] Create UnifiedToolRouter class in mcp-gateway/mcp_gateway/adapters/unified_router.py with venue-based routing logic (FR-003, FR-018)
- [X] T018 [US1] Implement ticker normalization for Binance in mcp-gateway/mcp_gateway/adapters/schema_adapter.py: transform bidPrice/askPrice → bid/ask, calculate mid and spread_bps (FR-007, FR-008)
- [X] T019 [US1] Implement orderbook L1 normalization for Binance in mcp-gateway/mcp_gateway/adapters/schema_adapter.py (FR-009)
- [X] T020 [US1] Register unified tools in mcp-gateway/mcp_gateway/sse_server.py: market.get_ticker and market.get_orderbook_l1 with venue parameter (FR-001, FR-002)
- [X] T021 [US1] Wire unified tool invocations through UnifiedToolRouter in mcp-gateway/mcp_gateway/sse_server.py (FR-028)
- [X] T022 [US1] Add error handling for non-existent instruments in UnifiedToolRouter: return structured error with alternatives (US1 Scenario 4)
- [X] T023 [P] [US1] Create integration test in mcp-gateway/tests/integration/test_unified_ticker.py: verify Binance ticker normalization
- [X] T024 [P] [US1] Create integration test in mcp-gateway/tests/integration/test_unified_routing.py: verify venue-based routing

**Checkpoint**: User Story 1 fully functional - AI clients can query market.get_ticker with normalized responses

---

## Phase 4: User Story 6 - SSE Gateway Exposes Only Unified Tools (Priority: P1)

**Goal**: Reduce tool count from 100+ to 10-20 unified tools for ChatGPT, preventing choice overload

**Independent Test**: Connect ChatGPT via SSE with expose_unified_only: true and verify only unified tools are listed (no binance.* tools by default)

### Implementation for User Story 6

- [ ] T025 [US6] Implement tool filtering logic in mcp-gateway/mcp_gateway/sse_server.py based on expose_unified_only flag (FR-026)
- [ ] T026 [US6] Implement provider tool whitelist in mcp-gateway/mcp_gateway/sse_server.py: expose only tools matching expose_provider_tools patterns (FR-027)
- [ ] T027 [US6] Update capability response construction in mcp-gateway/mcp_gateway/sse_server.py: return only unified + whitelisted tools (FR-005, FR-024, FR-025)
- [ ] T028 [US6] Add venue parameter enum generation in mcp-gateway/mcp_gateway/sse_server.py: populate from registered providers (US6 Scenario 4)
- [ ] T029 [P] [US6] Create integration test in mcp-gateway/tests/integration/test_sse_tool_filtering.py: verify tool count < 20 with 5 providers

**Checkpoint**: User Story 6 complete - ChatGPT sees curated unified tool list, not provider explosion

---

## Phase 5: User Story 2 - Add New Exchange Provider Without Breaking Clients (Priority: P1)

**Goal**: Add OKX provider and verify existing AI clients automatically gain access without client-side changes

**Independent Test**: Deploy OKX provider, verify gateway exposes market.get_ticker with venue: "okx", verify existing SSE clients see new venue

### Implementation for User Story 2

- [ ] T030 [P] [US2] Define OKX provider schema in mcp-gateway/mcp_gateway/schemas/providers/okx.json for ticker and orderbook responses
- [ ] T031 [US2] Implement ticker normalization for OKX in mcp-gateway/mcp_gateway/adapters/schema_adapter.py: handle BTC-USDT format with hyphen (US2 Scenario 3)
- [ ] T032 [US2] Implement orderbook normalization for OKX in mcp-gateway/mcp_gateway/adapters/schema_adapter.py: transform {price, size} → {price, quantity, cumulative}
- [ ] T033 [US2] Update UnifiedToolRouter in mcp-gateway/mcp_gateway/adapters/unified_router.py: support dynamic provider registration (FR-024, US2 Scenario 2)
- [ ] T034 [US2] Implement capability refresh mechanism in mcp-gateway/mcp_gateway/main.py: periodic reload (default: 60s) (FR-039, FR-040)
- [ ] T035 [P] [US2] Create integration test in mcp-gateway/tests/integration/test_multi_provider_routing.py: verify OKX + Binance routing

**Checkpoint**: User Story 2 complete - New providers can be added without breaking existing clients

---

## Phase 6: User Story 3 - Query Orderbook Depth Across Multiple Exchanges (Priority: P2)

**Goal**: Enable cross-exchange liquidity comparison with parallel multi-venue queries and normalized orderbook responses

**Independent Test**: Invoke market.get_orderbook_l2 with {instrument: "BTC-USDT", venues: ["binance", "okx"], depth: 10} and verify normalized bid/ask levels from both venues

### Implementation for User Story 3

- [ ] T036 [P] [US3] Define unified orderbook L2 schema in mcp-gateway/mcp_gateway/schemas/unified/market_orderbook_l2.json with depth parameter and level schema (FR-009)
- [ ] T037 [US3] Implement multi-venue fan-out in mcp-gateway/mcp_gateway/adapters/unified_router.py: parallel requests when venues is array (FR-020)
- [ ] T038 [US3] Implement response aggregation in mcp-gateway/mcp_gateway/adapters/unified_router.py: combine multi-venue results with per-venue status (FR-021)
- [ ] T039 [US3] Implement partial failure handling in mcp-gateway/mcp_gateway/adapters/unified_router.py: return successful data + structured errors for failed venues (US3 Scenario 3)
- [ ] T040 [US3] Add orderbook metadata to responses in mcp-gateway/mcp_gateway/adapters/schema_adapter.py: timestamp, latency_ms, exchange name (US3 Scenario 4)
- [ ] T041 [US3] Register market.get_orderbook_l2 tool in mcp-gateway/mcp_gateway/sse_server.py with venues array parameter (FR-001, FR-002)
- [ ] T042 [P] [US3] Create integration test in mcp-gateway/tests/integration/test_multi_venue_orderbook.py: verify parallel fan-out and aggregation

**Checkpoint**: User Story 3 complete - Multi-venue orderbook queries work with normalized depth metrics

---

## Phase 7: User Story 4 - Resolve Instrument Symbols Across Exchange Formats (Priority: P2)

**Goal**: Enable canonical instrument IDs (btc:perp:usdt) to be automatically mapped to exchange-specific symbols (Binance: BTCUSDT, OKX: BTC-USDT-SWAP)

**Independent Test**: Query market.get_ticker with canonical instrument_id: "btc:perp:usdt" and venue: "binance", verify gateway translates to "BTCUSDT"

### Implementation for User Story 4

- [ ] T043 [US4] Create InstrumentRegistry service in mcp-gateway/mcp_gateway/services/instrument_registry.py with in-memory cache (FR-012, FR-016)
- [ ] T044 [US4] Implement canonical instrument_id format validation in mcp-gateway/mcp_gateway/services/instrument_registry.py: {venue}:{market_type}:{base}-{quote} (FR-013)
- [ ] T045 [US4] Implement instrument metadata storage in mcp-gateway/mcp_gateway/services/instrument_registry.py: tick_size, lot_size, min_notional, contract_size, etc. (FR-014)
- [ ] T046 [US4] Implement forward lookup in mcp-gateway/mcp_gateway/services/instrument_registry.py: canonical ID → native symbol (US4 Scenario 1, 2)
- [ ] T047 [US4] Implement reverse lookup in mcp-gateway/mcp_gateway/services/instrument_registry.py: native symbol → canonical ID(s) (FR-015, US4 Scenario 3)
- [ ] T048 [US4] Populate registry from provider capabilities in mcp-gateway/mcp_gateway/services/instrument_registry.py (FR-016)
- [ ] T049 [US4] Create registry.list_instruments tool in mcp-gateway/mcp_gateway/sse_server.py with filters: venue, market_type, base_currency, quote_currency (FR-017)
- [ ] T050 [US4] Integrate InstrumentRegistry into UnifiedToolRouter in mcp-gateway/mcp_gateway/adapters/unified_router.py: translate canonical IDs before provider calls
- [ ] T051 [US4] Add venue-specific constraints to instrument metadata responses: min_qty, tick_size, etc. (US4 Scenario 4)
- [ ] T052 [P] [US4] Create unit test in mcp-gateway/tests/unit/test_instrument_registry.py: verify canonical ID translation
- [ ] T053 [P] [US4] Create integration test in mcp-gateway/tests/integration/test_canonical_instruments.py: verify cross-exchange symbol resolution

**Checkpoint**: User Story 4 complete - Canonical instrument IDs work across all exchanges

---

## Phase 8: Rate Limiting & Observability (Production Readiness)

**Goal**: Implement production-ready reliability with rate limiting, circuit breakers, and observability (FR-029 to FR-040)

**Independent Test**: Load test with 100 concurrent requests across 5 providers, verify rate limits enforced, metrics emitted, circuit breaker triggers on failures

### Implementation

- [ ] T054 [P] Create RateLimitService in mcp-gateway/mcp_gateway/services/rate_limiter.py with per-provider budgets (FR-029)
- [ ] T055 [P] Implement per-category rate limits in mcp-gateway/mcp_gateway/services/rate_limiter.py: market_data, account_data, orders (FR-030)
- [ ] T056 [P] Implement request queuing in mcp-gateway/mcp_gateway/services/rate_limiter.py with configurable depth (default: 100) (FR-032)
- [ ] T057 Integrate RateLimitService into UnifiedToolRouter in mcp-gateway/mcp_gateway/adapters/unified_router.py: check budgets before provider calls
- [ ] T058 Add RATE_LIMIT_EXCEEDED error handling in mcp-gateway/mcp_gateway/adapters/unified_router.py with retry-after timestamp (FR-031)
- [ ] T059 [P] Implement circuit breaker logic in mcp-gateway/mcp_gateway/adapters/grpc_client.py: mark provider unhealthy after N consecutive failures (FR-036)
- [ ] T060 [P] Add structured logging in mcp-gateway/mcp_gateway/adapters/unified_router.py: {provider, tool, request_id, error_code, error_message, timestamp} (FR-037)
- [ ] T061 [P] Implement metrics emission in mcp-gateway/mcp_gateway/adapters/grpc_client.py: request_count, error_count, latency_p50_ms, latency_p99_ms, rate_limit_hits (FR-035)
- [ ] T062 Create health check endpoint in mcp-gateway/mcp_gateway/main.py: expose per-provider status at /health (FR-038)
- [ ] T063 [P] Create load test in mcp-gateway/tests/integration/test_load_performance.py: 100 concurrent requests, verify p95 latency < 2s

**Checkpoint**: Production-ready reliability - rate limiting, circuit breakers, and observability complete

---

## Phase 9: User Story 5 - Place Orders Through Unified Interface (Priority: P3)

**Goal**: Enable order placement through normalized unified interface with venue-specific routing

**Independent Test**: Invoke order.place with {venue: "binance", instrument: "BTCUSDT", side: "buy", quantity: 0.01, price: 50000, order_type: "limit"} and verify normalized order confirmation

**NOTE**: This user story is P3 and can be deferred if focusing on market data first

### Implementation for User Story 5

- [ ] T064 [P] [US5] Define unified order placement schema in mcp-gateway/mcp_gateway/schemas/unified/order_place.json: normalized parameters (side, quantity, price, order_type)
- [ ] T065 [P] [US5] Define unified order confirmation schema in mcp-gateway/mcp_gateway/schemas/unified/order_confirmation.json: order_id, status, filled_quantity, average_price (US5 Scenario 2)
- [ ] T066 [US5] Implement order placement normalization for Binance in mcp-gateway/mcp_gateway/adapters/schema_adapter.py: translate to native REST API format (US5 Scenario 1)
- [ ] T067 [US5] Implement error normalization in mcp-gateway/mcp_gateway/adapters/schema_adapter.py: map exchange errors to standard codes (INSUFFICIENT_FUNDS, etc.) (US5 Scenario 3)
- [ ] T068 [US5] Add authentication validation in mcp-gateway/mcp_gateway/adapters/unified_router.py: check credentials before order placement (US5 Scenario 4, FR-043, FR-044)
- [ ] T069 [US5] Implement private tool classification in mcp-gateway/mcp_gateway/sse_server.py: mark order.place as private, require expose_private_tools flag (FR-041, FR-042)
- [ ] T070 [US5] Register order.place tool in mcp-gateway/mcp_gateway/sse_server.py with authentication requirements (FR-001)
- [ ] T071 [P] [US5] Create integration test in mcp-gateway/tests/integration/test_order_placement.py: verify order normalization and error handling

**Checkpoint**: User Story 5 complete - Order placement works through unified interface (optional for MVP)

---

## Phase 10: Additional Unified Tools (Expand Coverage)

**Goal**: Implement remaining unified tools for comprehensive market data coverage (FR-001)

### Implementation

- [ ] T072 [P] Define unified klines schema in mcp-gateway/mcp_gateway/schemas/unified/market_klines.json
- [ ] T073 [P] Define unified trades schema in mcp-gateway/mcp_gateway/schemas/unified/market_trades.json
- [ ] T074 [P] Define unified volume profile schema in mcp-gateway/mcp_gateway/schemas/unified/analytics_volume_profile.json
- [ ] T075 [P] Define unified market anomalies schema in mcp-gateway/mcp_gateway/schemas/unified/analytics_market_anomalies.json
- [ ] T076 [P] Define unified liquidity vacuums schema in mcp-gateway/mcp_gateway/schemas/unified/analytics_liquidity_vacuums.json
- [ ] T077 [P] Implement klines normalization in mcp-gateway/mcp_gateway/adapters/schema_adapter.py
- [ ] T078 [P] Implement trades normalization in mcp-gateway/mcp_gateway/adapters/schema_adapter.py
- [ ] T079 [P] Implement volume profile normalization in mcp-gateway/mcp_gateway/adapters/schema_adapter.py
- [ ] T080 [P] Implement market anomalies normalization in mcp-gateway/mcp_gateway/adapters/schema_adapter.py
- [ ] T081 [P] Implement liquidity vacuums normalization in mcp-gateway/mcp_gateway/adapters/schema_adapter.py
- [ ] T082 Register market.get_klines tool in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T083 Register market.get_trades tool in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T084 Register analytics.get_volume_profile tool in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T085 Register analytics.get_market_anomalies tool in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T086 Register analytics.get_liquidity_vacuums tool in mcp-gateway/mcp_gateway/sse_server.py

**Checkpoint**: Complete unified tools suite (10-15 tools) available

---

## Phase 11: Advanced Features (Batch Operations & Schema Versioning)

**Goal**: Implement batch RPC and schema versioning for advanced use cases (FR-033, FR-010)

### Implementation

- [ ] T087 [P] Add InvokeBatch message to pkg/proto/provider.proto accepting array of tool invocations (FR-033)
- [ ] T088 [P] Implement batch request handling in mcp-gateway/mcp_gateway/adapters/grpc_client.py: execute array of invocations and return array of results
- [ ] T089 [P] Implement schema versioning in mcp-gateway/mcp_gateway/schemas/unified/: support market.get_ticker.v1 format (FR-010)
- [ ] T090 [P] Add schema version tracking in mcp-gateway/mcp_gateway/schemas/ with migration notes and deprecation flags (FR-010)

**Checkpoint**: Advanced features complete - batch operations and versioned schemas available

---

## Phase 12: Polish & Cross-Cutting Concerns

**Purpose**: Final improvements affecting multiple user stories

- [ ] T091 [P] Add error message sanitization in mcp-gateway/mcp_gateway/adapters/unified_router.py: remove API keys, IPs, stack traces (FR-045)
- [ ] T092 [P] Implement fallback routing in mcp-gateway/mcp_gateway/adapters/unified_router.py: retry with secondary venue on failure (FR-022, FR-023)
- [ ] T093 [P] Add schema validation for provider responses in mcp-gateway/mcp_gateway/adapters/schema_adapter.py: detect breaking changes (Edge Case: schema drift)
- [ ] T094 [P] Implement default venue selection in mcp-gateway/mcp_gateway/adapters/unified_router.py: use config when venue parameter omitted (Edge Case: omitted venue)
- [ ] T095 [P] Add depth limit validation in mcp-gateway/mcp_gateway/adapters/unified_router.py: max 100 levels for orderbook (Edge Case: large orderbook)
- [ ] T096 [P] Create contract tests in mcp-gateway/tests/contract/test_provider_schemas.py: verify provider response schemas
- [ ] T097 [P] Add unit tests for UnifiedToolRouter in mcp-gateway/tests/unit/test_unified_router.py
- [ ] T098 [P] Add unit tests for schema_adapter in mcp-gateway/tests/unit/test_schema_adapter.py
- [ ] T099 [P] Add unit tests for RateLimitService in mcp-gateway/tests/unit/test_rate_limiter.py
- [ ] T100 [P] Documentation: Update README with unified tools usage examples
- [ ] T101 Code cleanup and refactoring: remove commented code, improve naming
- [ ] T102 Performance optimization: review and optimize hot paths
- [ ] T103 Security review: verify authentication handling, error sanitization

---

## Dependencies & Execution Order

### Phase Dependencies

- **Quick Fixes (Phase 0)**: No dependencies - CRITICAL, start immediately
- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Quick Fixes + Setup completion - BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Foundational completion - Core MVP
- **User Story 6 (Phase 4)**: Depends on User Story 1 - Requires unified tools to filter
- **User Story 2 (Phase 5)**: Depends on User Story 1 - Builds on routing infrastructure
- **User Story 3 (Phase 6)**: Depends on User Story 1, 2 - Requires multi-provider support
- **User Story 4 (Phase 7)**: Depends on User Story 1, 2 - Requires provider abstraction
- **Rate Limiting (Phase 8)**: Depends on User Story 1, 2, 3 - Production hardening
- **User Story 5 (Phase 9)**: Depends on Phase 8 - P3, can be deferred
- **Additional Tools (Phase 10)**: Depends on User Story 1 - Extends coverage
- **Advanced Features (Phase 11)**: Depends on Phase 10 - Optional enhancements
- **Polish (Phase 12)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 6 (P1)**: Can start after US1 - Requires unified tools to exist
- **User Story 2 (P1)**: Can start after US1 - Extends to multiple providers
- **User Story 3 (P2)**: Can start after US1, US2 - Requires multi-provider routing
- **User Story 4 (P2)**: Can start after US1, US2 - Requires provider abstraction
- **User Story 5 (P3)**: Can start after Phase 8 - Optional, low priority

### Within Each User Story

- Schema definitions before normalization implementation
- Router implementation before tool registration
- Core functionality before error handling
- Implementation before tests (tests verify implementation)

### Parallel Opportunities

- All Quick Fixes tasks marked [P] can run in parallel
- All Setup tasks marked [P] can run in parallel
- Schema definitions within a phase can run in parallel
- Tests for a user story can run in parallel
- Once US1 complete, US6 can start while US2 is being implemented
- US3 and US4 can be implemented in parallel after US1+US2 complete

---

## Parallel Example: User Story 1

```bash
# Launch schema definitions in parallel:
Task: "Define unified ticker schema in mcp-gateway/mcp_gateway/schemas/unified/market_ticker.json"
Task: "Define unified orderbook L1 schema in mcp-gateway/mcp_gateway/schemas/unified/market_orderbook_l1.json"

# Then implement routing and normalization sequentially (depends on schemas):
Task: "Create UnifiedToolRouter class"
Task: "Implement ticker normalization for Binance"

# Then launch tests in parallel:
Task: "Create integration test for unified ticker"
Task: "Create integration test for unified routing"
```

---

## Implementation Strategy

### MVP First (Quick Fixes + User Story 1 + User Story 6)

1. Complete Phase 0: Quick Fixes (2-3 days)
2. Complete Phase 1: Setup (1 day)
3. Complete Phase 2: Foundational (2 days)
4. Complete Phase 3: User Story 1 (3-4 days)
5. Complete Phase 4: User Story 6 (1-2 days)
6. **STOP and VALIDATE**: Test US1 + US6 independently
7. Deploy/demo if ready

**MVP Deliverable**: AI clients can query market data through unified tools with tool filtering

### Incremental Delivery

1. MVP (Quick Fixes + US1 + US6) → Test independently → Deploy/Demo
2. Add User Story 2 (multi-provider) → Test independently → Deploy/Demo
3. Add User Story 3 (multi-venue) + US4 (canonical IDs) → Test independently → Deploy/Demo
4. Add Phase 8 (rate limiting + observability) → Production ready → Deploy
5. Add User Story 5 (orders) → Optional enhancement → Deploy
6. Add Phase 10 (additional tools) → Expand coverage → Deploy

### Parallel Team Strategy

With multiple developers:

1. Team completes Quick Fixes + Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1
   - Developer B: User Story 6 (after US1 schemas ready)
   - Developer C: Start User Story 2 setup (schemas, provider configs)
3. After US1+US6 complete:
   - Developer A: User Story 2 (multi-provider)
   - Developer B: User Story 3 (multi-venue)
   - Developer C: User Story 4 (instrument registry)
4. After US2+US3+US4 complete:
   - Team: Phase 8 (rate limiting + observability) together
5. Stories complete and integrate independently

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Quick Fixes (Phase 0) are CRITICAL - unblock multi-provider support
- Stop at any checkpoint to validate story independently
- Commit after each task or logical group
- MVP = Quick Fixes + US1 + US6 (enables unified tools with filtering)
- US5 (order placement) is P3 and can be deferred for market data focus
- Additional tools (Phase 10) can be implemented incrementally based on demand
