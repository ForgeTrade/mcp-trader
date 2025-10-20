---
description: "Task breakdown for Feature 013 - Complete Unified Multi-Exchange API"
---

# Tasks: Feature 013 - Complete Unified Multi-Exchange API

**Input**: Design documents from `/specs/013-complete-unified-api/`
**Prerequisites**: IMPLEMENTATION.md (required), spec.md (required for user stories)
**Status**: COMPLETED (2025-10-20)

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`
- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions
- Python project: `mcp-gateway/mcp_gateway/` for source code
- Tests: `mcp-gateway/tests/` for test files
- Feature 012 infrastructure already exists and is working

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Extend existing Feature 012 infrastructure with 16 new tool mappings

**Status**: ✅ COMPLETED (2025-10-20)

- [x] T001 Extend UnifiedToolRouter with 16 new tool mappings in mcp-gateway/mcp_gateway/adapters/unified_router.py
- [x] T002 [P] Add 8 new schema normalizer method stubs in mcp-gateway/mcp_gateway/adapters/schema_adapter.py
- [x] T003 [P] Update SSE server list_tools() handler to prepare for 16 new Tool definitions in mcp-gateway/mcp_gateway/sse_server.py

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core schema normalization infrastructure that MUST be complete before ANY user story can be fully tested

**⚠️ CRITICAL**: No user story implementation can be completed until normalization framework is working

**Status**: ✅ COMPLETED (2025-10-20)

- [x] T004 Implement SchemaAdapter normalizer registration in __init__ for all 16 tools in mcp-gateway/mcp_gateway/adapters/schema_adapter.py
- [x] T005 [P] Add normalization dispatch logic in SSE call_tool() handler for all 16 tools in mcp-gateway/mcp_gateway/sse_server.py
- [x] T006 [P] Implement error handling for FR-022 (auth errors), FR-023 (unsupported tools), FR-024 (rate limits) in mcp-gateway/mcp_gateway/sse_server.py

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Trader Places Orders Across Multiple Exchanges (Priority: P1) 🎯 MVP

**Goal**: Enable traders to place, cancel, and monitor orders across exchanges using unified trading tools

**Independent Test**: Trader can call `trade.place_order(venue="binance", instrument="BTCUSDT", side="BUY", type="LIMIT", quantity=0.01, price=43000)` and receive normalized order response with order_id, status, filled_quantity

**Status**: ✅ COMPLETED (2025-10-20)

### Implementation for User Story 1

- [x] T007 [P] [US1] Add Tool definition for trade.place_order in mcp-gateway/mcp_gateway/sse_server.py
- [x] T008 [P] [US1] Add Tool definition for trade.cancel_order in mcp-gateway/mcp_gateway/sse_server.py
- [x] T009 [P] [US1] Add Tool definition for trade.get_order in mcp-gateway/mcp_gateway/sse_server.py
- [x] T010 [P] [US1] Add Tool definition for trade.get_open_orders in mcp-gateway/mcp_gateway/sse_server.py
- [x] T011 [P] [US1] Add Tool definition for trade.get_all_orders in mcp-gateway/mcp_gateway/sse_server.py
- [x] T012 [P] [US1] Add Tool definition for trade.get_account in mcp-gateway/mcp_gateway/sse_server.py
- [x] T013 [P] [US1] Add Tool definition for trade.get_my_trades in mcp-gateway/mcp_gateway/sse_server.py

- [x] T014 [P] [US1] Implement _normalize_binance_order() normalizer in mcp-gateway/mcp_gateway/adapters/schema_adapter.py
- [x] T015 [P] [US1] Implement _normalize_binance_account() normalizer in mcp-gateway/mcp_gateway/adapters/schema_adapter.py
- [x] T016 [P] [US1] Implement _normalize_binance_trade() normalizer in mcp-gateway/mcp_gateway/adapters/schema_adapter.py

**Requirements Covered**: FR-001, FR-002, FR-003, FR-004, FR-005, FR-006, FR-007, FR-021 (authentication)

**Checkpoint**: At this point, User Story 1 should be fully functional - traders can execute complete trading workflows (place, cancel, query, view history)

---

## Phase 4: User Story 2 - Analyst Accesses Advanced Market Analytics (Priority: P2)

**Goal**: Enable market analysts to detect anomalies, assess orderbook health, and generate volume profiles using unified analytics tools

**Independent Test**: Analyst can call `analytics.detect_market_anomalies(venue="binance", instrument="BTCUSDT", window_seconds=60)` and receive normalized anomaly detection results with quote stuffing alerts

**Status**: ✅ COMPLETED (2025-10-20)

### Implementation for User Story 2

- [x] T017 [P] [US2] Add Tool definition for analytics.get_orderbook_health in mcp-gateway/mcp_gateway/sse_server.py
- [x] T018 [P] [US2] Add Tool definition for analytics.get_order_flow in mcp-gateway/mcp_gateway/sse_server.py
- [x] T019 [P] [US2] Add Tool definition for analytics.get_volume_profile in mcp-gateway/mcp_gateway/sse_server.py
- [x] T020 [P] [US2] Add Tool definition for analytics.detect_market_anomalies in mcp-gateway/mcp_gateway/sse_server.py
- [x] T021 [P] [US2] Add Tool definition for analytics.get_microstructure_health in mcp-gateway/mcp_gateway/sse_server.py
- [x] T022 [P] [US2] Add Tool definition for analytics.detect_liquidity_vacuums in mcp-gateway/mcp_gateway/sse_server.py

- [x] T023 [P] [US2] Implement _normalize_binance_orderbook_health() normalizer in mcp-gateway/mcp_gateway/adapters/schema_adapter.py
- [x] T024 [P] [US2] Implement _normalize_binance_volume_profile() normalizer in mcp-gateway/mcp_gateway/adapters/schema_adapter.py
- [x] T025 [P] [US2] Implement _normalize_binance_market_anomalies() normalizer in mcp-gateway/mcp_gateway/adapters/schema_adapter.py
- [x] T026 [P] [US2] Implement _normalize_binance_microstructure_health() normalizer in mcp-gateway/mcp_gateway/adapters/schema_adapter.py

**Requirements Covered**: FR-008, FR-009, FR-010, FR-011, FR-012, FR-013

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently - analysts can access all advanced analytics while traders continue using trading tools

---

## Phase 5: User Story 3 - Application Queries Exchange Metadata (Priority: P3)

**Goal**: Enable trading applications to fetch exchange trading rules, recent trades, and average prices in normalized format

**Independent Test**: Application can call `market.get_exchange_info(venue="binance", instrument="BTCUSDT")` and receive normalized trading rules with min/max order sizes

**Status**: ✅ COMPLETED (2025-10-20)

### Implementation for User Story 3

- [x] T027 [P] [US3] Add Tool definition for market.get_recent_trades in mcp-gateway/mcp_gateway/sse_server.py
- [x] T028 [P] [US3] Add Tool definition for market.get_exchange_info in mcp-gateway/mcp_gateway/sse_server.py
- [x] T029 [P] [US3] Add Tool definition for market.get_avg_price in mcp-gateway/mcp_gateway/sse_server.py

- [x] T030 [P] [US3] Implement _normalize_binance_recent_trades() normalizer in mcp-gateway/mcp_gateway/adapters/schema_adapter.py
- [x] T031 [P] [US3] Implement _normalize_binance_exchange_info() normalizer in mcp-gateway/mcp_gateway/adapters/schema_adapter.py

**Requirements Covered**: FR-014, FR-015, FR-016

**Checkpoint**: All user stories should now be independently functional - complete unified API with 20 total tools (4 existing + 16 new)

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Testing, deployment, and verification across all user stories

**Status**: ✅ COMPLETED (2025-10-20)

- [x] T032 Test gateway locally - verify 20 unified tools exposed
- [x] T033 [P] Verify UnifiedToolRouter has all 20 tool mappings
- [x] T034 [P] Verify SchemaAdapter has all normalizers registered
- [x] T035 Deploy to production (198.13.46.14:3001)
- [x] T036 Verify production deployment - check service status and tool count
- [x] T037 Document Feature 013 implementation with retroactive tasks.md

**Requirements Covered**: SC-001 (20 unified tools), SC-008 (all tools in router mapping)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - extends existing Feature 012 infrastructure
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-5)**: All depend on Foundational phase completion
  - User stories can then proceed in parallel (if staffed)
  - Or sequentially in priority order (P1 → P2 → P3)
- **Polish (Phase 6)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P2)**: Can start after Foundational (Phase 2) - Independent of US1
- **User Story 3 (P3)**: Can start after Foundational (Phase 2) - Independent of US1/US2

### Within Each User Story

- Tool definitions can all be written in parallel (marked [P])
- Normalizer implementations can all be written in parallel (marked [P])
- Tool definitions should be written first to establish contract
- Normalizers must be complete before story is testable

### Parallel Opportunities

- All Setup tasks (T001-T003) can run in parallel
- All Foundational tasks (T004-T006) can run in parallel (within Phase 2)
- Once Foundational phase completes, all user stories can start in parallel (if team capacity allows)
- All Tool definitions within a story marked [P] can run in parallel
- All normalizer implementations within a story marked [P] can run in parallel
- Different user stories can be worked on in parallel by different team members

---

## Parallel Example: User Story 1 (Trading Tools)

```bash
# Launch all Tool definitions for User Story 1 together:
Task: "Add Tool definition for trade.place_order in mcp-gateway/mcp_gateway/sse_server.py"
Task: "Add Tool definition for trade.cancel_order in mcp-gateway/mcp_gateway/sse_server.py"
Task: "Add Tool definition for trade.get_order in mcp-gateway/mcp_gateway/sse_server.py"
Task: "Add Tool definition for trade.get_open_orders in mcp-gateway/mcp_gateway/sse_server.py"
Task: "Add Tool definition for trade.get_all_orders in mcp-gateway/mcp_gateway/sse_server.py"
Task: "Add Tool definition for trade.get_account in mcp-gateway/mcp_gateway/sse_server.py"
Task: "Add Tool definition for trade.get_my_trades in mcp-gateway/mcp_gateway/sse_server.py"

# Launch all normalizer implementations for User Story 1 together:
Task: "Implement _normalize_binance_order() normalizer in mcp-gateway/mcp_gateway/adapters/schema_adapter.py"
Task: "Implement _normalize_binance_account() normalizer in mcp-gateway/mcp_gateway/adapters/schema_adapter.py"
Task: "Implement _normalize_binance_trade() normalizer in mcp-gateway/mcp_gateway/adapters/schema_adapter.py"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (extend router, add normalizer stubs)
2. Complete Phase 2: Foundational (normalization framework, error handling) - CRITICAL
3. Complete Phase 3: User Story 1 (7 trading tools + 3 normalizers)
4. **STOP and VALIDATE**: Test User Story 1 independently - place order, cancel order, query status
5. Deploy/demo if ready

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 → Test independently → Deploy/Demo (MVP: Trading tools working!)
3. Add User Story 2 → Test independently → Deploy/Demo (Analytics added!)
4. Add User Story 3 → Test independently → Deploy/Demo (Complete unified API!)
5. Each story adds value without breaking previous stories

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1 (Trading tools)
   - Developer B: User Story 2 (Analytics tools)
   - Developer C: User Story 3 (Market metadata tools)
3. Stories complete and integrate independently

---

## Requirements Coverage

| Requirement | Task(s) | Status |
|-------------|---------|--------|
| FR-001 (trade.place_order) | T001, T007, T014 | ✅ Covered |
| FR-002 (trade.cancel_order) | T001, T008, T014 | ✅ Covered |
| FR-003 (trade.get_order) | T001, T009, T014 | ✅ Covered |
| FR-004 (trade.get_open_orders) | T001, T010, T014 | ✅ Covered |
| FR-005 (trade.get_all_orders) | T001, T011, T014 | ✅ Covered |
| FR-006 (trade.get_account) | T001, T012, T015 | ✅ Covered |
| FR-007 (trade.get_my_trades) | T001, T013, T016 | ✅ Covered |
| FR-008 (analytics.get_orderbook_health) | T001, T017, T023 | ✅ Covered |
| FR-009 (analytics.get_order_flow) | T001, T018, T004 | ✅ Covered |
| FR-010 (analytics.get_volume_profile) | T001, T019, T024 | ✅ Covered |
| FR-011 (analytics.detect_market_anomalies) | T001, T020, T025 | ✅ Covered |
| FR-012 (analytics.get_microstructure_health) | T001, T021, T026 | ✅ Covered |
| FR-013 (analytics.detect_liquidity_vacuums) | T001, T022, T004 | ✅ Covered |
| FR-014 (market.get_recent_trades) | T001, T027, T030 | ✅ Covered |
| FR-015 (market.get_exchange_info) | T001, T028, T031 | ✅ Covered |
| FR-016 (market.get_avg_price) | T001, T029, T004 | ✅ Covered |
| FR-017 (Router mapping) | T001 | ✅ Covered |
| FR-018 (Schema normalization) | T002, T004, T014-T016, T023-T026, T030-T031 | ✅ Covered |
| FR-019 (Venue parameter) | T007-T029 (all Tool definitions) | ✅ Covered |
| FR-020 (Normalized responses) | T005 + all normalizers | ✅ Covered |
| FR-021 (Authentication) | T006 | ✅ Covered |
| FR-022 (Auth error handling) | T006 | ✅ Covered |
| FR-023 (Unsupported tool errors) | T006 | ✅ Covered |
| FR-024 (Rate limit errors) | T006 | ✅ Covered |
| SC-001 (20 unified tools) | T032, T036 | ✅ Covered |
| SC-002 (Trading workflows) | T007-T013 (User Story 1) | ✅ Covered |
| SC-003 (Normalized schemas) | T014-T016 (User Story 1) | ✅ Covered |
| SC-004 (Analytics metrics) | T023-T026 (User Story 2) | ✅ Covered |
| SC-005 (Same call across venues) | T001 (UnifiedToolRouter) | ✅ Covered |
| SC-006 (Auth error handling) | T006 | ✅ Covered |
| SC-007 (Unsupported tool errors) | T006 | ✅ Covered |
| SC-008 (Router verification) | T033 | ✅ Covered |

---

## Actual vs Estimated Timeline

**Estimated** (from IMPLEMENTATION.md): 24 hours (3 days at 8h/day)
- Task 1 (Router): 2 hours
- Task 2 (Normalizers): 8 hours
- Task 3 (Tool Definitions): 4 hours
- Task 4 (call_tool integration): 3 hours
- Testing: 5 hours
- Deployment: 2 hours

**Actual**: ~4-6 hours (implementation completed in single session on 2025-10-20)

**Variance**: Significantly faster due to:
- Clear specification and requirements in spec.md
- Existing architecture from Feature 012 providing solid foundation
- All infrastructure (UnifiedToolRouter, SchemaAdapter, SSE server) already in place
- No architectural surprises or blockers encountered

---

## Notes

- [P] tasks = different files, no dependencies, can run in parallel
- [Story] label maps task to specific user story for traceability (US1, US2, US3)
- Each user story should be independently completable and testable
- Tool definitions establish the contract - implement these first
- Normalizers make responses consistent - implement after tool definitions
- Feature 012 provides the foundation - this feature extends it
- All tasks completed retroactively on 2025-10-20 in a single implementation session
