---
description: "Task list for fixing inverted spread_bps bug in Binance provider"
---

# Tasks: Fix Inverted Spread BPS Bug

**Input**: Design documents from `/home/limerc/repos/ForgeTrade/mcp-trader/specs/017-specify-scripts-bash/`
**Prerequisites**: plan.md, spec.md, research.md
**Branch**: `017-specify-scripts-bash`

**Tests**: Validation using existing `test_sse_client.py` (no new tests required)

**Organization**: Tasks organized by user story to enable independent bugfix implementation and testing.

## Format: `[ID] [P?] [Story] Description`
- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1)
- Include exact file paths in descriptions

## Path Conventions
- Binance Provider: `/home/limerc/repos/ForgeTrade/mcp-trader/providers/binance-rs/`
- Test script: `/home/limerc/repos/ForgeTrade/mcp-trader/test_sse_client.py`

---

## Phase 1: Setup & Verification

**Purpose**: Verify current buggy behavior and prepare for fix

- [ ] T001 [P] Verify current bug by reading providers/binance-rs/src/orderbook/metrics.rs lines 88-89
- [ ] T002 [P] Confirm test script exists at /home/limerc/repos/ForgeTrade/mcp-trader/test_sse_client.py
- [ ] T003 Document current buggy behavior: best_bid=111,323.58 (higher), best_ask=111,294.22 (lower), spread_bps=-2.6374

**Checkpoint**: Bug location confirmed, test infrastructure ready

---

## Phase 2: User Story 1 - Correct Best Bid/Ask Ordering (Priority: P1) 🎯 MVP

**Goal**: Fix the swapped best_bid and best_ask assignments in OrderBookMetrics struct initialization so that best_bid < best_ask and spread_bps > 0.

**Independent Test**: Call `market.get_orderbook_l1` for BTCUSDT and verify that best_bid < best_ask and spread_bps > 0 in all responses.

### Implementation for User Story 1

- [ ] T004 [US1] Swap best_bid assignment in providers/binance-rs/src/orderbook/metrics.rs line 88 from best_bid.to_string() to best_ask.to_string()
- [ ] T005 [US1] Swap best_ask assignment in providers/binance-rs/src/orderbook/metrics.rs line 89 from best_ask.to_string() to best_bid.to_string()
- [ ] T006 [US1] Add comment explaining the swap in providers/binance-rs/src/orderbook/metrics.rs lines 88-89
- [ ] T007 [US1] Build Binance provider with: cd providers/binance-rs && cargo build --release
- [ ] T008 [US1] Run Binance provider unit tests with: cd providers/binance-rs && cargo test --release
- [ ] T009 [US1] Restart Binance provider process on port 50053 with new binary
- [ ] T010 [US1] Test BTCUSDT orderbook L1 with test_sse_client.py and verify best_bid < best_ask
- [ ] T011 [US1] Test BTCUSDT orderbook L1 and verify spread_bps > 0 (should be +2.6374 instead of -2.6374)
- [ ] T012 [US1] Test BTCUSDT orderbook L1 and verify microprice is between bid and ask: best_bid <= microprice <= best_ask
- [ ] T013 [P] [US1] Test ETHUSDT orderbook L1 with test_sse_client.py for regression testing
- [ ] T014 [P] [US1] Test BNBUSDT orderbook L1 with test_sse_client.py for regression testing
- [ ] T015 [US1] Verify 10 consecutive BTCUSDT orderbook L1 requests all have correct bid/ask ordering

**Checkpoint**: Orderbook L1 tool returns correct bid < ask ordering with positive spread_bps for all trading pairs

---

## Phase 3: Final Validation & Documentation

**Purpose**: Comprehensive testing and documentation of bugfix

- [ ] T016 Verify success criteria SC-001: 100% of orderbook L1 responses have best_bid < best_ask (zero tolerance)
- [ ] T017 Verify success criteria SC-002: 100% of orderbook L1 responses have spread_bps > 0 for non-locked markets
- [ ] T018 Verify success criteria SC-003: Microprice falls between best_bid and best_ask in all responses
- [ ] T019 [P] Document fix in specs/017-specify-scripts-bash/research.md "Verification" section with test results
- [ ] T020 [P] Update specs/017-specify-scripts-bash/checklists/requirements.md with implementation status
- [ ] T021 Check Binance provider logs at /tmp/binance-provider.log for any errors or warnings
- [ ] T022 Verify no negative spread_bps values in provider logs

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **User Story 1 (Phase 2)**: Depends on Setup completion - This is the core bugfix (P1)
- **Final Validation (Phase 3)**: Depends on User Story 1 completion

### User Story Dependencies

- **User Story 1 (P1)**: Independent - single file modification, can start after setup

### Within Each Phase

**Setup Phase**:
- T001, T002 can run in parallel (independent reads)
- T003 depends on T001 completion

**User Story 1 Phase**:
- T004, T005, T006 must run sequentially (same file, same lines)
- T007 depends on T004-T006 (must apply code changes before building)
- T008 depends on T007 (must build before testing)
- T009 depends on T007 (must build before restarting)
- T010, T011, T012 depend on T009 (must restart provider before integration testing)
- T013, T014 can run in parallel with each other after T012 (independent symbols)
- T015 depends on T010-T014 (final verification after all tests pass)

**Final Validation Phase**:
- T016, T017, T018 must run sequentially (build on each other)
- T019, T020 can run in parallel (independent documentation)
- T021, T022 depend on all previous tasks (log analysis)

### Parallel Opportunities

- **Phase 1 Setup**: T001 and T002 can run in parallel
- **Phase 2 User Story 1**: T013 and T014 can run in parallel (different trading pairs)
- **Phase 3 Final Validation**: T019 and T020 can run in parallel (different files)

---

## Parallel Example: Phase 2 (User Story 1) Testing

```bash
# After T012 completes, launch T013 and T014 in parallel:
Task: "Test ETHUSDT orderbook L1 for regression"
Task: "Test BNBUSDT orderbook L1 for regression"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (verify bug location)
2. Complete Phase 2: User Story 1 (apply the 2-line fix)
3. **STOP and VALIDATE**: Test orderbook_l1 with BTCUSDT, ETHUSDT, BNBUSDT
4. Target: 100% correct bid/ask ordering, 100% positive spread_bps

### Single Developer Strategy (Recommended for bugfix)

1. Setup phase (T001-T003) - 2 minutes
2. Apply fix (T004-T006) - 2 minutes
3. Build and test (T007-T009) - 5 minutes
4. Integration testing (T010-T015) - 5 minutes
5. Final validation (T016-T022) - 5 minutes

**Total estimated time**: ~20 minutes

### Incremental Delivery

1. Setup → Bug location verified
2. Apply Fix → Code changed (2 lines swapped)
3. Build → Binary updated
4. Restart → New code running
5. Test → Verify fix works
6. Each step adds value and can be validated independently

---

## Notes

- [P] tasks = different files or independent testing, no dependencies
- [Story] label maps task to specific user story for traceability
- This is a **single user story bugfix** (only US1)
- Use existing test_sse_client.py for validation - no new tests needed
- **Critical**: Lines 88-89 in metrics.rs must be swapped together (single atomic change)
- Success measured by: bid < ask AND spread_bps > 0 for ALL orderbook L1 requests
- All changes isolated to 1 file: `providers/binance-rs/src/orderbook/metrics.rs`
- Fix is **2 lines**: swap two assignments in struct initialization
- Risk: Very low (localized change, easy rollback)
- Impact: Critical (fixes 100% of inverted orderbook data)

---

## File Paths Reference

### Files to Modify
- `providers/binance-rs/src/orderbook/metrics.rs` (lines 88-89) - **PRIMARY FIX**

### Files to Read/Reference
- `providers/binance-rs/src/orderbook/types.rs` (OrderBookMetrics struct definition)
- `test_sse_client.py` (integration testing)
- `specs/017-specify-scripts-bash/research.md` (bug analysis)
- `specs/017-specify-scripts-bash/plan.md` (implementation strategy)

### Build Commands
```bash
cd /home/limerc/repos/ForgeTrade/mcp-trader/providers/binance-rs
cargo build --release
cargo test --release
```

### Test Commands
```bash
cd /home/limerc/repos/ForgeTrade/mcp-trader
uv run --directory mcp-gateway python test_sse_client.py
```

---

## Expected Outcomes

**Before Fix**:
- best_bid: 111,323.58 (HIGHER - wrong!)
- best_ask: 111,294.22 (LOWER - wrong!)
- spread_bps: -2.6374 (NEGATIVE - impossible!)

**After Fix**:
- best_bid: 111,294.22 (LOWER - correct!)
- best_ask: 111,323.58 (HIGHER - correct!)
- spread_bps: +2.6374 (POSITIVE - correct!)
- microprice: between 111,294.22 and 111,323.58 (correct!)
