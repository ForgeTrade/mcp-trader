# Implementation Plan: Fix Inverted Spread BPS Bug

**Feature Branch**: `017-specify-scripts-bash`
**Created**: 2025-10-20
**Status**: Planning Complete

## Overview

This plan outlines the implementation approach for fixing the inverted spread_bps bug in the Binance Rust provider, where best_bid and best_ask values are swapped in the OrderBookMetrics struct initialization.

## Technical Architecture

### Component Affected

**Binance Rust Provider** (`providers/binance-rs/`)
- **Language**: Rust
- **Role**: Provides orderbook data and analytics to MCP gateway via gRPC
- **Bug Location**: `src/orderbook/metrics.rs` lines 88-89

### Data Flow

```
Binance API → OrderBook struct → OrderBookMetrics calculation → gRPC response → MCP Gateway → SSE clients
                                          ↑
                                   BUG IS HERE (metrics.rs:88-89)
```

### Root Cause

The bug is a simple **variable swap** in struct initialization:

**Current (Buggy)**:
```rust
best_bid: best_bid.to_string(),  // Line 88 - assigns highest value to best_bid field
best_ask: best_ask.to_string(),  // Line 89 - assigns lowest value to best_ask field
```

**Problem**: The local variables `best_bid` and `best_ask` contain the **correct** values (highest bid, lowest ask), but they're being assigned to the **wrong** struct fields.

**Fix**:
```rust
best_bid: best_ask.to_string(),  // Line 88 - assign lowest value to best_bid field
best_ask: best_bid.to_string(),  // Line 89 - assign highest value to best_ask field
```

## Implementation Strategy

### Phase 1: Code Fix

**File to Modify**: `providers/binance-rs/src/orderbook/metrics.rs`

#### Change 1: Swap best_bid and best_ask assignments (lines 88-89)

**Before**:
```rust
Some(OrderBookMetrics {
    symbol: order_book.symbol.clone(),
    timestamp: order_book.timestamp,
    spread_bps,
    microprice,
    bid_volume,
    ask_volume,
    imbalance_ratio,
    best_bid: best_bid.to_string(),  // Line 88
    best_ask: best_ask.to_string(),  // Line 89
    walls,
    slippage_estimates,
})
```

**After**:
```rust
Some(OrderBookMetrics {
    symbol: order_book.symbol.clone(),
    timestamp: order_book.timestamp,
    spread_bps,
    microprice,
    bid_volume,
    ask_volume,
    imbalance_ratio,
    best_bid: best_ask.to_string(),  // Line 88 - FIXED: swap assignment
    best_ask: best_bid.to_string(),  // Line 89 - FIXED: swap assignment
    walls,
    slippage_estimates,
})
```

**Rationale**: This single swap corrects the field assignment so that:
- `best_bid` field gets the lowest price (from `best_ask` variable)
- `best_ask` field gets the highest price (from `best_bid` variable)

### Phase 2: Testing

#### Test 1: Local Integration Test

**Tool**: `test_sse_client.py`

**Test Case**: Orderbook L1 for BTCUSDT
```python
# Expected behavior after fix:
response = call_tool("market.get_orderbook_l1", {"instrument": "BTCUSDT"})
assert response["best_bid"] < response["best_ask"]  # bid must be lower
assert response["spread_bps"] > 0  # spread must be positive
```

**Success Criteria**:
- `best_bid` < `best_ask` (e.g., 111,294.22 < 111,323.58)
- `spread_bps` > 0 (e.g., +2.6374 instead of -2.6374)
- `microprice` between bid and ask: `best_bid <= microprice <= best_ask`

#### Test 2: Multiple Symbol Verification

Test with multiple trading pairs to ensure fix works across all orderbooks:
- BTCUSDT
- ETHUSDT
- BNBUSDT

#### Test 3: Production Validation

After deployment:
- Monitor production logs for negative spread_bps values
- Verify all orderbook L1 responses have positive spreads
- Check 100 consecutive requests for 100% correct bid/ask ordering

### Phase 3: Deployment

#### Step 1: Build & Test Locally
```bash
cd providers/binance-rs
cargo build --release
cargo test --release
```

#### Step 2: Restart Provider
```bash
# Kill old process
ps aux | grep binance-provider | grep -v grep | awk '{print $2}' | xargs -r kill

# Start new binary
./target/release/binance-provider --grpc --port 50053 > /tmp/binance-provider.log 2>&1 &
```

#### Step 3: Integration Test
```bash
cd /home/limerc/repos/ForgeTrade/mcp-trader
uv run --directory mcp-gateway python test_sse_client.py
```

#### Step 4: Deploy to Production

**Production Server**: 198.13.46.14 (mcp-gateway.thevibe.trading)

```bash
# On production server:
cd /home/limerc/repos/ForgeTrade/mcp-trader
git pull origin master
cd providers/binance-rs
cargo build --release
systemctl restart binance-provider  # Or equivalent service management
systemctl restart mcp-gateway
```

## Risk Assessment

### Low Risk

This bugfix is **extremely low risk** because:

1. **Single-line change**: Only 2 assignments are swapped
2. **No algorithmic changes**: Formula calculations remain unchanged
3. **No new dependencies**: Uses existing variables
4. **Localized impact**: Only affects OrderBookMetrics struct initialization
5. **Easy rollback**: Can revert by re-swapping the two lines

### Affected Systems

- ✅ **Binance Provider**: Direct fix location
- ✅ **MCP Gateway**: Receives corrected data (no code changes needed)
- ✅ **SSE Clients**: Receive corrected data automatically
- ❌ **Other Providers**: Not affected (bug is Binance-specific)
- ❌ **Database/Persistence**: Not affected (orderbook metrics are real-time only)

## Verification Checklist

After deployment, verify:

- [ ] `spread_bps` is positive for all orderbook L1 requests
- [ ] `best_bid` < `best_ask` in 100% of responses
- [ ] `microprice` is between bid and ask
- [ ] No negative spread_bps values in logs
- [ ] Production metrics show 0% error rate for inverted orderbooks

## Rollback Plan

If issues are detected:

```bash
git revert <commit-hash>
cd providers/binance-rs
cargo build --release
systemctl restart binance-provider
```

**Time to rollback**: < 5 minutes

## Success Metrics

- **Before Fix**: 100% of orderbook L1 responses have inverted bid/ask (negative spread)
- **After Fix**: 100% of orderbook L1 responses have correct bid/ask (positive spread)
- **Target**: Zero tolerance for negative spreads in production

## Dependencies

### Build Dependencies
- Rust toolchain (already installed)
- Cargo (already available)

### Runtime Dependencies
- No new dependencies added
- Existing gRPC communication unchanged

### External Dependencies
- Binance API (no changes required)
- MCP Gateway (no changes required)

## Timeline Estimate

- **Code Fix**: 2 minutes (swap two lines)
- **Local Testing**: 5 minutes (build + test)
- **Production Deployment**: 10 minutes (build + restart services)
- **Total**: ~20 minutes from start to production

## Notes

- This is a **pure bugfix** with no feature additions
- The fix corrects a pre-existing bug, not a regression
- No database migrations or schema changes required
- No API contract changes (response structure unchanged, just values corrected)
- Compatible with all existing clients
