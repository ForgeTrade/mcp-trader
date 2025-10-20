# Bug Specification: Fix Quote Stuffing Detection Test

**Type**: Bug Fix
**Priority**: P3 (Medium)
**Component**: Orderbook Analytics - Anomaly Detection
**Affected File**: `providers/binance-rs/src/orderbook/analytics/anomaly.rs`

## Problem Description

The `test_detect_quote_stuffing_threshold` test is failing due to incorrect duration calculation in the `detect_quote_stuffing()` function.

### Failing Test
```
test orderbook::analytics::anomaly::tests::test_detect_quote_stuffing_threshold ... FAILED
assertion failed: result.is_some()
at providers/binance-rs/src/orderbook/analytics/anomaly.rs:253
```

### Root Cause

The `detect_quote_stuffing()` function incorrectly calculates the update rate by assuming each snapshot represents a fixed 1-second interval:

```rust
// Current implementation (lines 36-38)
let duration_secs = snapshots.len() as f64;  // Assumes 600 snapshots = 600 seconds
let update_count = snapshots.windows(2).count();  // = 599 pairs
let update_rate = (update_count as f64) / duration_secs;  // = 599/600 = 0.998 updates/sec
```

This produces an incorrect result:
- **Calculated**: 0.998 updates/sec
- **Actual**: Should be 600 updates/sec (600 updates in 1 second based on timestamps)
- **Threshold**: 500 updates/sec required for detection
- **Result**: 0.998 < 500, so `None` is returned instead of detecting the anomaly

### Expected Behavior

The function should use actual timestamps from the `OrderBookSnapshot.timestamp` field to calculate the real duration and update rate.

## Success Criteria

1. The `test_detect_quote_stuffing_threshold` test passes
2. The `detect_quote_stuffing()` function correctly calculates update rate based on actual timestamps
3. All other existing tests continue to pass
4. No regression in anomaly detection functionality

## Affected Code

**File**: `providers/binance-rs/src/orderbook/analytics/anomaly.rs`
**Function**: `detect_quote_stuffing()` (lines 27-78)
**Test**: `test_detect_quote_stuffing_threshold()` (lines 244-258)

## Non-Goals

- This fix does NOT add new features
- This fix does NOT change the quote stuffing detection algorithm or thresholds
- This fix does NOT modify the test expectations (they are correct as-is)
