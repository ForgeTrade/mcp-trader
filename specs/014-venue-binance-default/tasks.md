---
description: "Task breakdown for Feature 014 - Standardize Venue Parameter with Binance Default"
---

# Tasks: Feature 014 - Standardize Venue Parameter with Binance Default

**Input**: Design documents from `/specs/014-venue-binance-default/`
**Prerequisites**: IMPLEMENTATION.md (required), spec.md (required for user stories)

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`
- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2)
- Include exact file paths in descriptions

## Path Conventions
- Python project: `mcp-gateway/mcp_gateway/` for source code
- Feature 013 infrastructure already exists and is working

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Create centralized venue mapping configuration

- [ ] T001 Create venue name mapping constants (VENUE_MAPPING, PUBLIC_VENUES) in mcp-gateway/mcp_gateway/config.py or appropriate config module
- [ ] T002 [P] Identify current provider registry location in mcp-gateway/mcp_gateway/main.py

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core venue mapping and validation infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [ ] T003 Update UnifiedToolRouter.__init__ to accept and store venue mapping in mcp-gateway/mcp_gateway/adapters/unified_router.py
- [ ] T004 Update provider registry initialization to use venue mapping in mcp-gateway/mcp_gateway/main.py
- [ ] T005 Add venue default handling in UnifiedToolRouter.route_tool_call() - set venue = arguments.get("venue", "binance") in mcp-gateway/mcp_gateway/adapters/unified_router.py
- [ ] T006 Add venue name normalization logic (public name → internal provider ID mapping) in UnifiedToolRouter.route_tool_call() in mcp-gateway/mcp_gateway/adapters/unified_router.py
- [ ] T007 Update venue validation to check against PUBLIC_VENUES list in UnifiedToolRouter.route_tool_call() in mcp-gateway/mcp_gateway/adapters/unified_router.py
- [ ] T008 Update all error messages to use public venue names (not internal IDs) in mcp-gateway/mcp_gateway/adapters/unified_router.py

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - API User Calls Unified Tools Without Specifying Venue (Priority: P1) 🎯 MVP

**Goal**: Enable API users to omit venue parameter and automatically use Binance as default

**Independent Test**: Call `market.get_ticker(instrument="BTCUSDT")` without venue parameter → succeeds with Binance data

### Implementation for User Story 1

- [ ] T009 [P] [US1] Update market.get_ticker tool definition - make venue optional with default "binance" in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T010 [P] [US1] Update market.get_orderbook_l1 tool definition - make venue optional with default "binance" in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T011 [P] [US1] Update market.get_orderbook_l2 tool definition - make venue optional with default "binance" in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T012 [P] [US1] Update market.get_klines tool definition - make venue optional with default "binance" in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T013 [P] [US1] Update market.get_recent_trades tool definition - make venue optional with default "binance" in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T014 [P] [US1] Update market.get_exchange_info tool definition - make venue optional with default "binance" in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T015 [P] [US1] Update market.get_avg_price tool definition - make venue optional with default "binance" in mcp-gateway/mcp_gateway/sse_server.py

- [ ] T016 [P] [US1] Update trade.place_order tool definition - make venue optional with default "binance" in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T017 [P] [US1] Update trade.cancel_order tool definition - make venue optional with default "binance" in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T018 [P] [US1] Update trade.get_order tool definition - make venue optional with default "binance" in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T019 [P] [US1] Update trade.get_open_orders tool definition - make venue optional with default "binance" in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T020 [P] [US1] Update trade.get_all_orders tool definition - make venue optional with default "binance" in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T021 [P] [US1] Update trade.get_account tool definition - make venue optional with default "binance" in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T022 [P] [US1] Update trade.get_my_trades tool definition - make venue optional with default "binance" in mcp-gateway/mcp_gateway/sse_server.py

- [ ] T023 [P] [US1] Update analytics.get_orderbook_health tool definition - make venue optional with default "binance" in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T024 [P] [US1] Update analytics.get_order_flow tool definition - make venue optional with default "binance" in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T025 [P] [US1] Update analytics.get_volume_profile tool definition - make venue optional with default "binance" in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T026 [P] [US1] Update analytics.detect_market_anomalies tool definition - make venue optional with default "binance" in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T027 [P] [US1] Update analytics.get_microstructure_health tool definition - make venue optional with default "binance" in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T028 [P] [US1] Update analytics.detect_liquidity_vacuums tool definition - make venue optional with default "binance" in mcp-gateway/mcp_gateway/sse_server.py

**Requirements Covered**: FR-001, FR-002, FR-003, FR-011

**Checkpoint**: At this point, User Story 1 should be fully functional - all 20 tools accept omitted venue parameter and default to Binance

---

## Phase 4: User Story 2 - API User Sees Clean Provider List (Priority: P2)

**Goal**: Show only "binance" in venue enums, hide test providers and internal naming

**Independent Test**: Inspect tool schema for market.get_ticker → venue enum shows only ["binance"], no "binance-rs", "hello-go", "hello-rs"

### Implementation for User Story 2

- [ ] T029 [P] [US2] Update market.get_ticker venue enum to ["binance"] only in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T030 [P] [US2] Update market.get_orderbook_l1 venue enum to ["binance"] only in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T031 [P] [US2] Update market.get_orderbook_l2 venue enum to ["binance"] only in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T032 [P] [US2] Update market.get_klines venue enum to ["binance"] only in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T033 [P] [US2] Update market.get_recent_trades venue enum to ["binance"] only in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T034 [P] [US2] Update market.get_exchange_info venue enum to ["binance"] only in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T035 [P] [US2] Update market.get_avg_price venue enum to ["binance"] only in mcp-gateway/mcp_gateway/sse_server.py

- [ ] T036 [P] [US2] Update trade.place_order venue enum to ["binance"] only in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T037 [P] [US2] Update trade.cancel_order venue enum to ["binance"] only in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T038 [P] [US2] Update trade.get_order venue enum to ["binance"] only in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T039 [P] [US2] Update trade.get_open_orders venue enum to ["binance"] only in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T040 [P] [US2] Update trade.get_all_orders venue enum to ["binance"] only in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T041 [P] [US2] Update trade.get_account venue enum to ["binance"] only in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T042 [P] [US2] Update trade.get_my_trades venue enum to ["binance"] only in mcp-gateway/mcp_gateway/sse_server.py

- [ ] T043 [P] [US2] Update analytics.get_orderbook_health venue enum to ["binance"] only in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T044 [P] [US2] Update analytics.get_order_flow venue enum to ["binance"] only in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T045 [P] [US2] Update analytics.get_volume_profile venue enum to ["binance"] only in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T046 [P] [US2] Update analytics.detect_market_anomalies venue enum to ["binance"] only in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T047 [P] [US2] Update analytics.get_microstructure_health venue enum to ["binance"] only in mcp-gateway/mcp_gateway/sse_server.py
- [ ] T048 [P] [US2] Update analytics.detect_liquidity_vacuums venue enum to ["binance"] only in mcp-gateway/mcp_gateway/sse_server.py

**Requirements Covered**: FR-004, FR-005, FR-006, FR-010, FR-012

**Checkpoint**: At this point, User Stories 1 AND 2 should both work - all tools have optional venue with clean "binance" enum

---

## Phase 5: Polish & Cross-Cutting Concerns

**Purpose**: Testing, deployment, and verification across all user stories

- [ ] T049 Test default venue behavior - call tools without venue parameter
- [ ] T050 [P] Test explicit venue="binance" - verify backward compatibility
- [ ] T051 [P] Test invalid venues (binance-rs, hello-go, hello-rs) - verify error messages
- [ ] T052 [P] Verify tool schemas show venue as optional with ["binance"] enum
- [ ] T053 Deploy to staging and verify all 20 tools work with optional venue
- [ ] T054 Deploy to production (198.13.46.14:3001)
- [ ] T055 Verify production deployment - test omitted venue and explicit venue="binance"

**Requirements Covered**: SC-001, SC-002, SC-003, SC-004, SC-005, SC-006, SC-007

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-4)**: Both depend on Foundational phase completion
  - User stories can then proceed in parallel (if staffed)
  - Or sequentially in priority order (P1 → P2)
- **Polish (Phase 5)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P2)**: Can start after Foundational (Phase 2) - Independent of US1 (but typically follows US1 in same files)

### Within Each User Story

- All Tool definition updates within a story marked [P] can run in parallel
- All tool updates affect the same file (sse_server.py) but different Tool() definitions

### Parallel Opportunities

- All Setup tasks (T001-T002) can run in parallel
- All Foundational tasks (T003-T008) must run sequentially (same file, dependencies)
- All US1 tool updates (T009-T028) can run in parallel (different Tool definitions)
- All US2 enum updates (T029-T048) can run in parallel (different Tool definitions)
- US1 and US2 can be combined in practice (update both venue optionality AND enum in same edit)
- All Polish tasks (T050-T052) can run in parallel

---

## Parallel Example: User Story 1 (Make Venue Optional)

```bash
# Launch all tool definition updates for User Story 1 together:
Task: "Update market.get_ticker tool definition - make venue optional with default 'binance'"
Task: "Update market.get_orderbook_l1 tool definition - make venue optional with default 'binance'"
Task: "Update market.get_orderbook_l2 tool definition - make venue optional with default 'binance'"
# ... (all 20 tools)

# Each task modifies a different Tool() definition in sse_server.py
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (venue mapping config)
2. Complete Phase 2: Foundational (router default + validation) - CRITICAL
3. Complete Phase 3: User Story 1 (20 tools accept optional venue)
4. **STOP and VALIDATE**: Test tools without venue parameter → should use Binance
5. Deploy/demo if ready

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 → Test independently → Deploy/Demo (MVP: Optional venue working!)
3. Add User Story 2 → Test independently → Deploy/Demo (Clean enums, complete!)
4. Each story adds value without breaking previous stories

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1 (make venue optional for all 20 tools)
   - Developer B: User Story 2 (clean up venue enums for all 20 tools)
3. Both developers edit the same file (sse_server.py) but different aspects of Tool definitions

**Note**: In practice, US1 and US2 can be done together in a single pass through the file (update both optionality AND enum for each Tool)

---

## Requirements Coverage

| Requirement | Task(s) | Status |
|-------------|---------|--------|
| FR-001 (Optional venue parameter) | T005, T009-T028 | Covered |
| FR-002 (Default to binance) | T005 | Covered |
| FR-003 (Backward compatibility) | T005, T050 | Covered |
| FR-004 (Public name "binance") | T001, T006, T029-T048 | Covered |
| FR-005 (Hide test providers) | T001, T004 | Covered |
| FR-006 (Internal ID mapping) | T001, T003, T006 | Covered |
| FR-007 (Invalid venue error) | T007, T051 | Covered |
| FR-008 (Validate against public venues) | T007 | Covered |
| FR-009 (Public names in errors) | T008 | Covered |
| FR-010 (Enum with "binance" only) | T029-T048 | Covered |
| FR-011 (Venue marked optional) | T009-T028 | Covered |
| FR-012 (Tool descriptions indicate default) | T009-T028 | Covered |
| SC-001 (All tools work without venue) | T049, T055 | Covered |
| SC-002 (Clean schemas) | T052, T029-T048 | Covered |
| SC-003 (Backward compatible) | T050 | Covered |
| SC-004 (Error messages for invalid venues) | T051 | Covered |
| SC-005 (Only "binance" in messages) | T008, T055 | Covered |
| SC-006 (Documentation clarity) | T052 | Covered |
| SC-007 (Improved ergonomics) | T049, T055 | Covered |

---

## Notes

- [P] tasks = different Tool() definitions or different files, can run in parallel
- [Story] label maps task to specific user story for traceability (US1, US2)
- Each user story should be independently completable and testable
- US1 (make venue optional) can be tested independently before US2 (clean enums)
- US2 (clean enums) builds on US1 but is also independently valuable
- Both stories edit sse_server.py, so in practice can be combined for efficiency
- Feature 013 (20 unified tools) provides the foundation - this feature refines the API surface
