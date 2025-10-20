# Bug Analysis: SSE Schema Normalization

**Date**: 2025-10-20
**Branch**: `016-fix-sse-schema-bugs`

## Summary of Investigation

Based on code analysis and test results, I've identified the root causes of the two failing tools and one UX issue.

---

## Issue 1: Orderbook L1 - Missing Bids/Asks

### Error Message
```
Normalization failed for binance.orderbook_l1: Invalid orderbook: missing bids or asks
```

### Root Cause Analysis

**Location**: `mcp-gateway/mcp_gateway/adapters/schema_adapter.py:168-236`

**Current Implementation**:
```python
def _normalize_binance_orderbook_l1(self, raw: Dict[str, Any]) -> Dict[str, Any]:
    # Extract top of book (FR-009)
    if not raw.get("bids") or not raw.get("asks"):
        raise ValueError("Invalid orderbook: missing bids or asks")

    best_bid = raw["bids"][0]
    best_ask = raw["asks"][0]
```

**Problem**: The normalizer expects `raw["bids"]` and `raw["asks"]` to be present at the top level of the response dictionary. However, the actual provider response structure may differ.

**Possible Provider Response Formats**:
1. **Nested in "orderbook" key**:
   ```json
   {
     "orderbook": {
       "bids": [[...]],
       "asks": [[...]]
     }
   }
   ```

2. **Different field names** (e.g., "bid_levels", "ask_levels")

3. **Empty arrays being treated as falsy** by `raw.get("bids")`

**Investigation Needed**:
- Inspect actual provider response from `binance.orderbook_l1` tool
- Compare with working tools (ticker, volume_profile) to see response format
- Check if gRPC client wraps response in additional structure

**Fix Approach**:
1. Log raw provider response to understand actual structure
2. Update normalizer to handle correct response format
3. Add defensive checks for empty vs missing arrays
4. Consider adding response unwrapping if needed

---

## Issue 2: Klines - Invalid Array Access

### Error Message
```
Provider binance failed to execute binance.get_klines: list indices must be integers or slices, not str
```

### Root Cause Analysis

**Location**: `mcp-gateway/mcp_gateway/sse_server.py:895-897`

**Current Implementation**:
```python
elif name == "market.get_klines":
    # Klines don't need normalization yet - just add venue and latency
    pass
```

**Problem**: Klines are NOT being normalized at all! The comment says "just add venue and latency" but then does nothing. The error "list indices must be integers" suggests that:

1. **Provider returns array of klines** (correct format)
2. **Code somewhere tries to access it with string keys** like `response["key"]` instead of `response[0]`
3. **This likely happens in the provider itself** or in the router when trying to add metadata

**Investigation Needed**:
- Check if provider response is wrapped differently than other tools
- See if there's generic response handling code that assumes dict structure
- Check if router or sse_server code tries to inject fields assuming dict

**Missing Normalizer**:
There is NO `_normalize_binance_klines` function in schema_adapter.py! This should:
- Parse array of klines
- Convert string prices to floats
- Normalize field names (open/high/low/close/volume)
- Handle Binance-specific array structure `[timestamp, open, high, low, close, volume, ...]`

**Fix Approach**:
1. Create `_normalize_binance_klines()` function in schema_adapter.py
2. Add "klines": normalizer to binance normalizer map (line 21)
3. Add klines normalization in sse_server.py after line 895
4. Handle array response properly (wrap in {"klines": [...]} structure)

---

## Issue 3: Venue Parameter None

### Error Message
```
venue="N/A" or venue=None in responses
```

### Root Cause Analysis

**Location**: `mcp-gateway/mcp_gateway/adapters/unified_router.py:95`

**Current Implementation**:
```python
# Feature 014: Default venue to "binance" if not provided (FR-001, FR-002)
venue = arguments.get("venue", "binance")
```

**Good News**: This is ALREADY FIXED! The venue parameter already defaults to "binance" in unified_router.py!

**Verification Needed**:
- Check if test results still show venue=None
- If so, issue might be:
  1. Old test using different code path
  2. Error responses not including venue field
  3. Schema normalizer overwriting venue with None

**Location 2**: `mcp-gateway/mcp_gateway/adapters/schema_adapter.py:94-95`

```python
# Ensure venue is set
if "venue" not in normalized:
    normalized["venue"] = venue
```

This should ensure venue is always set. Need to verify this is working correctly.

**Fix Approach**:
1. Run test_sse_client.py to verify venue defaulting works
2. If still showing None, check error response paths
3. Ensure normalized responses don't override venue with None

---

## Verification Strategy

### Test Tools Comparison

**Working Tools** (for comparison):
- ✅ `market.get_ticker`: Uses `_normalize_binance_ticker()` - works perfectly
- ✅ `analytics.get_volume_profile`: Uses `_normalize_binance_volume_profile()` - returns raw (works)
- ✅ `analytics.get_orderbook_health`: Uses `_normalize_binance_orderbook_health()` - works

**Broken Tools**:
- ❌ `market.get_orderbook_l1`: Uses `_normalize_binance_orderbook_l1()` - **bids/asks not found**
- ❌ `market.get_klines`: **NO NORMALIZER** - list index error

### Next Steps

1. **T001 ✅ Complete**: Analyzed orderbook_l1 normalization logic
2. **T002 ✅ Complete**: Analyzed klines normalization (missing!)
3. **T003 ✅ Complete**: Venue parameter defaulting already implemented
4. **T004 - TODO**: Compare working vs broken tool responses
5. **T005 - Current**: Document findings in this file

### Implementation Order

**Phase 2 - User Story 1** (Fix orderbook_l1):
1. Add debug logging to see actual provider response
2. Fix normalizer to handle correct response structure
3. Test with ETHUSDT

**Phase 3 - User Story 2** (Fix klines):
1. Create `_normalize_binance_klines()` function
2. Add to normalizer registry
3. Update sse_server.py to call normalizer
4. Test with BTCUSDT

**Phase 4 - User Story 3** (Venue defaulting):
1. Verify venue="binance" appears in all responses
2. Might already be working - just needs verification

---

## Code References

- **Schema Adapter**: `mcp-gateway/mcp_gateway/adapters/schema_adapter.py`
  - Orderbook L1 normalizer: lines 168-236
  - Missing klines normalizer: should be added after line 301
  - Normalizer registry: line 20-47

- **SSE Server**: `mcp-gateway/mcp_gateway/sse_server.py`
  - Orderbook L1 normalization call: lines 679-689
  - Klines NO normalization: lines 895-897 (needs fix)

- **Unified Router**: `mcp-gateway/mcp_gateway/adapters/unified_router.py`
  - Venue defaulting: line 95 (already working!)
  - Venue injection into response: line 157

---

## Expected Outcomes After Fixes

- **Before**: 60% pass rate (3/5 tools working)
- **After US1**: 80% pass rate (4/5 tools - orderbook fixed)
- **After US2**: 100% pass rate (5/5 tools - klines fixed)
- **After US3**: 100% pass rate + clean venue="binance" in all responses

---

## FINAL RESULTS (2025-10-20)

### Test Results: 100% Pass Rate ✅

All 5 tools now working correctly:
1. ✅ market.get_ticker (BTCUSDT) - was working
2. ✅ market.get_orderbook_l1 (ETHUSDT) - **FIXED** in Phase 2
3. ✅ market.get_klines (BTCUSDT, 1h, 5 candles) - **FIXED** in Phase 3
4. ✅ analytics.get_volume_profile (BTCUSDT) - was working
5. ✅ analytics.get_orderbook_health (BTCUSDT) - was working

### Venue Parameter: Working ✅
All responses show `venue: "binance"` - Feature 014 implementation was already correct.

### Files Modified

1. **mcp-gateway/mcp_gateway/adapters/schema_adapter.py**
   - Fixed `_normalize_binance_orderbook_l1()` to extract from `best_bid`/`best_ask` fields
   - Created `_normalize_binance_klines()` to handle array responses
   - Registered klines normalizer

2. **mcp-gateway/mcp_gateway/adapters/unified_router.py**
   - Added type check before injecting metadata into array results
   - Fixed "list indices must be integers" error

3. **mcp-gateway/mcp_gateway/sse_server.py**
   - Added klines normalization call (removed "pass" placeholder)

### Root Causes Identified and Fixed

**Issue 1 - Orderbook L1**: Provider returns `OrderBookMetrics` struct with `best_bid`/`best_ask` string fields, not `bids`/`asks` arrays as expected by standard Binance API.

**Issue 2 - Klines**: Two bugs:
- Missing normalizer function (created `_normalize_binance_klines()`)
- Router tried to add metadata to array using dict syntax (`result["result"]["latency_ms"]`)

**Issue 3 - Venue**: Already working from Feature 014 implementation.
