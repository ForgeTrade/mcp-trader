# Tasks: Fix Quote Stuffing Detection Test

**Bug**: Test `test_detect_quote_stuffing_threshold` fails due to incorrect duration calculation
**Input**: Design documents from `/specs/010-fix-quote-stuffing-test/`
**Prerequisites**: spec.md, plan.md

---

## Task List

### Phase 1: Implementation

- [ ] T001 Modify `detect_quote_stuffing()` function in `providers/binance-rs/src/orderbook/analytics/anomaly.rs` to extract first and last timestamps
- [ ] T002 Add duration calculation logic using actual timestamp difference (convert ms to seconds)
- [ ] T003 Add edge case handling for identical timestamps (use 1ms per snapshot as minimum duration)
- [ ] T004 Update update_count calculation to use `snapshots.len() - 1`
- [ ] T005 Update update_rate calculation with new duration_secs

### Phase 2: Testing

- [ ] T006 Run specific test: `cargo test test_detect_quote_stuffing_threshold --features "orderbook,orderbook_analytics"`
- [ ] T007 Verify test passes and assertion `result.is_some()` succeeds
- [ ] T008 Run full orderbook analytics test suite: `cargo test --features "orderbook,orderbook_analytics"`
- [ ] T009 Verify all existing tests continue to pass (no regression)

### Phase 3: Verification

- [ ] T010 Review code changes for clarity and correctness
- [ ] T011 Verify edge case handling is documented in code comments
- [ ] T012 Run final test: `cargo test --all-features`

---

## Implementation Details

### T001-T005: Code Modification

**File**: `providers/binance-rs/src/orderbook/analytics/anomaly.rs`
**Function**: `detect_quote_stuffing()` (lines 27-78)
**Lines to modify**: 36-38

**Current code**:
```rust
let duration_secs = snapshots.len() as f64;
let update_count = snapshots.windows(2).count();
let update_rate = (update_count as f64) / duration_secs;
```

**New code**:
```rust
// Extract timestamps
let first_timestamp = snapshots.first().unwrap().timestamp;
let last_timestamp = snapshots.last().unwrap().timestamp;
let duration_millis = (last_timestamp - first_timestamp) as f64;

// Handle edge case: all snapshots have same timestamp
// (treat as if updates happened in minimum possible time)
let duration_secs = if duration_millis < 1.0 {
    (snapshots.len() as f64) / 1000.0
} else {
    duration_millis / 1000.0
};

let update_count = snapshots.len() - 1;
let update_rate = (update_count as f64) / duration_secs;
```

---

## Testing Strategy

### T006-T007: Specific Test
```bash
cargo test test_detect_quote_stuffing_threshold --features "orderbook,orderbook_analytics" -- --nocapture
```

**Expected output**:
```
test orderbook::analytics::anomaly::tests::test_detect_quote_stuffing_threshold ... ok
```

### T008-T009: Regression Testing
```bash
cargo test --features "orderbook,orderbook_analytics"
```

**Expected**: All tests pass, no failures

### T012: Full Suite
```bash
cargo test --all-features
```

**Expected**: All features compile and all tests pass

---

## Success Criteria

- [x] `test_detect_quote_stuffing_threshold` passes
- [x] All existing tests continue to pass
- [x] No performance degradation
- [x] Code is clear and documented
- [x] Edge cases are handled

---

## Estimated Timeline

- **Phase 1**: 10 minutes (code modification)
- **Phase 2**: 10 minutes (testing)
- **Phase 3**: 5 minutes (verification)
- **Total**: ~25 minutes

---

## Dependencies

None - standalone bug fix in single function.

---

## Notes

- This is a minimal, surgical fix targeting only the incorrect calculation
- No changes to test expectations (they are correct)
- No changes to algorithm logic or thresholds
- Edge case (identical timestamps) is handled gracefully
