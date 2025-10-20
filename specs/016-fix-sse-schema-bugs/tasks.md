---
description: "Task list for fixing SSE schema normalization bugs"
---

# Tasks: Fix SSE Schema Normalization Bugs

**Input**: Design documents from `/specs/016-fix-sse-schema-bugs/`
**Prerequisites**: plan.md, spec.md
**Branch**: `016-fix-sse-schema-bugs`

**Tests**: No new tests required - validation using existing `test_sse_client.py`

**Organization**: Tasks organized by user story to enable independent bugfix implementation and testing.

## Format: `[ID] [P?] [Story] Description`
- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions
- Single Python project: `mcp-gateway/mcp_gateway/`
- Test script: `test_sse_client.py` (repository root)

---

## Phase 1: Research & Root Cause Analysis

**Purpose**: Identify exact root causes of schema normalization failures before implementing fixes

- [X] T001 [P] Analyze orderbook_l1 normalization logic in mcp-gateway/mcp_gateway/adapters/schema_adapter.py
- [X] T002 [P] Analyze klines normalization logic in mcp-gateway/mcp_gateway/adapters/schema_adapter.py
- [X] T003 [P] Trace venue parameter flow through mcp-gateway/mcp_gateway/adapters/unified_router.py
- [X] T004 Compare working tool normalizers (ticker, volume_profile) vs broken ones in mcp-gateway/mcp_gateway/adapters/schema_adapter.py
- [X] T005 Document root causes and fix approaches in specs/016-fix-sse-schema-bugs/research.md

**Checkpoint**: Root causes identified and documented - ready for implementation

---

## Phase 2: User Story 1 - Orderbook Data Access (Priority: P1) 🎯 MVP

**Goal**: Fix market.get_orderbook_l1 to correctly parse bids/asks arrays from Binance provider response

**Independent Test**: Call `market.get_orderbook_l1` via SSE with ETHUSDT and verify it returns valid bid/ask/spread data instead of "missing bids or asks" error

### Implementation for User Story 1

- [X] T006 [US1] Fix orderbook_l1 normalization to correctly extract best_bid from provider response in mcp-gateway/mcp_gateway/adapters/schema_adapter.py
- [X] T007 [US1] Fix orderbook_l1 normalization to correctly extract best_ask from provider response in mcp-gateway/mcp_gateway/adapters/schema_adapter.py
- [X] T008 [US1] Add validation to check for best_bid/best_ask presence before normalization in mcp-gateway/mcp_gateway/adapters/schema_adapter.py
- [X] T009 [US1] Run test_sse_client.py to verify market.get_orderbook_l1 returns valid orderbook data for ETHUSDT
- [X] T010 [US1] Verify no regression in other tools (ticker, volume_profile, orderbook_health) by running test_sse_client.py

**Checkpoint**: market.get_orderbook_l1 tool working correctly with valid bid/ask/spread data

---

## Phase 3: User Story 2 - Historical Candlestick Data Access (Priority: P1)

**Goal**: Fix market.get_klines to correctly parse array responses using integer indices instead of string keys

**Independent Test**: Call `market.get_klines` via SSE with BTCUSDT, interval="1h", limit=5 and verify it returns 5 valid candlesticks without "list indices must be integers" error

### Implementation for User Story 2

- [X] T011 [US2] Fix klines normalization to use integer array indices instead of string-keyed access in mcp-gateway/mcp_gateway/adapters/schema_adapter.py
- [X] T012 [US2] Handle array of arrays vs array of objects structure in klines response in mcp-gateway/mcp_gateway/adapters/schema_adapter.py
- [X] T013 [US2] Add error handling for unexpected klines response formats in mcp-gateway/mcp_gateway/adapters/schema_adapter.py
- [X] T014 [US2] Run test_sse_client.py to verify market.get_klines returns 5 valid candlesticks for BTCUSDT
- [X] T015 [US2] Verify no regression in previously fixed orderbook_l1 tool by running test_sse_client.py

**Checkpoint**: market.get_klines tool working correctly with valid candlestick data

---

## Phase 4: User Story 3 - Venue Parameter Defaulting (Priority: P2)

**Goal**: Automatically default venue parameter to "binance" when not explicitly specified by user

**Independent Test**: Call any market.* tool without venue parameter and verify it defaults to "binance" and executes successfully

### Implementation for User Story 3

- [X] T016 [US3] Add venue="binance" default when venue parameter is None in mcp-gateway/mcp_gateway/adapters/unified_router.py (Already implemented in Feature 014)
- [X] T017 [US3] Ensure normalized responses include venue="binance" field instead of venue=None in mcp-gateway/mcp_gateway/adapters/schema_adapter.py (Already working)
- [X] T018 [US3] Update tool definitions in mcp-gateway/mcp_gateway/sse_server.py to reflect venue default behavior (documentation only) (Not needed - already documented)
- [X] T019 [US3] Run test_sse_client.py without explicit venue parameter to verify defaulting works
- [X] T020 [US3] Verify all tool responses show venue="binance" not venue="N/A" or venue=None

**Checkpoint**: Venue parameter defaults correctly to "binance" across all tools

---

## Phase 5: Final Validation & Documentation

**Purpose**: Comprehensive testing and documentation of fixes

- [X] T021 Run complete test_sse_client.py suite and verify 100% pass rate (5/5 tools working)
- [X] T022 [P] Update specs/016-fix-sse-schema-bugs/quickstart.md with before/after test results (Not applicable - no quickstart.md exists)
- [X] T023 [P] Document changes made to schema_adapter.py and unified_router.py in commit message (Documented in research.md)
- [X] T024 Verify no venue=None or venue="N/A" in any tool responses
- [X] T025 Check SSE server logs for any residual normalization errors

---

## Dependencies & Execution Order

### Phase Dependencies

- **Research (Phase 1)**: No dependencies - can start immediately
- **User Story 1 (Phase 2)**: Depends on Research completion - First bugfix (P1)
- **User Story 2 (Phase 3)**: Depends on Research completion - Can run in parallel with US1 (different bug in same file)
- **User Story 3 (Phase 4)**: Depends on US1 and US2 completion - Lower priority (P2)
- **Final Validation (Phase 5)**: Depends on all user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Research - Independent orderbook bug
- **User Story 2 (P1)**: Can start after Research - Independent klines bug (same file as US1 but different function)
- **User Story 3 (P2)**: Should wait for US1 and US2 - Enhances UX across all fixed tools

### Within Each User Story

- Root cause analysis before implementation
- Fix core normalization logic before validation
- Fix error handling after core logic
- Test individual story before moving to next
- Check for regressions after each story

### Parallel Opportunities

- **Phase 1 Research**: T001, T002, T003 can run in parallel (independent analysis)
- **US1 vs US2**: Can implement in parallel after research (different bugs, same file but different functions)
- **Final Validation**: T022 and T023 can run in parallel (documentation tasks)

**Note**: While US1 and US2 modify the same file (schema_adapter.py), they target different normalization functions (orderbook_l1 vs klines), allowing parallel work if careful with merge conflicts.

---

## Parallel Example: Research Phase

```bash
# Launch all research tasks together:
Task: "Analyze orderbook_l1 normalization logic in schema_adapter.py"
Task: "Analyze klines normalization logic in schema_adapter.py"
Task: "Trace venue parameter flow through unified_router.py"
```

## Parallel Example: US1 + US2 (if team capacity allows)

```bash
# Developer A works on User Story 1:
Task: "Fix orderbook_l1 bids array extraction in schema_adapter.py"
Task: "Fix orderbook_l1 asks array extraction in schema_adapter.py"

# Developer B works on User Story 2 simultaneously:
Task: "Fix klines array access in schema_adapter.py"
Task: "Handle array structure variations in schema_adapter.py"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Research (identify root causes)
2. Complete Phase 2: User Story 1 (fix orderbook_l1)
3. **STOP and VALIDATE**: Test orderbook_l1 independently with test_sse_client.py
4. Target: 80% pass rate (4/5 tools working)

### Incremental Delivery

1. Complete Research → Root causes documented
2. Add User Story 1 → Test independently → 80% pass rate (orderbook_l1 fixed)
3. Add User Story 2 → Test independently → 100% pass rate (klines fixed)
4. Add User Story 3 → Test independently → 100% pass rate + better UX (venue defaulting)
5. Each story adds value without breaking previous fixes

### Single Developer Strategy (Recommended for bugfix)

1. Complete Research phase (all tasks T001-T005)
2. Fix User Story 1 (T006-T010) → Validate
3. Fix User Story 2 (T011-T015) → Validate
4. Fix User Story 3 (T016-T020) → Validate
5. Final validation (T021-T025)

---

## Notes

- [P] tasks = different files or independent analysis, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story fixes an independent bug and should be testable separately
- Use existing test_sse_client.py for validation - no new tests needed
- Commit after each user story phase to enable easy rollback if needed
- Stop at any checkpoint to validate story independently
- Target: 60% → 80% → 100% pass rate progression
- All changes isolated to 2 files: schema_adapter.py (primary) and unified_router.py (venue defaulting)
