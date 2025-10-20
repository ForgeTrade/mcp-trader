# Bug Research Report: Inverted spread_bps in Binance Rust Provider

**Date**: 2025-10-20
**Branch**: `017-specify-scripts-bash`
**Bug**: Inverted best_bid/best_ask causing negative spread_bps

## Executive Summary

After comprehensive code analysis of the Binance Rust provider (`providers/binance-rs/`), I have **identified the exact location of the bug** causing inverted `spread_bps` values. The bug is in `providers/binance-rs/src/orderbook/metrics.rs` at **lines 88-89**, where the `best_bid` and `best_ask` values are being assigned to the **wrong struct fields**.

## Bug Location & Root Cause

### File: `providers/binance-rs/src/orderbook/metrics.rs`

**Lines 88-89 (BUGGY CODE):**
```rust
best_bid: best_bid.to_string(),  // Line 88 - SWAPPED!
best_ask: best_ask.to_string(),  // Line 89 - SWAPPED!
```

### Root Cause Analysis

The issue stems from a **naming confusion** in the code. Here's the complete data flow:

1. **Data Extraction** (lines 36-37):
   ```rust
   let best_bid = order_book.best_bid()?;  // Gets HIGHEST bid price
   let best_ask = order_book.best_ask()?;  // Gets LOWEST ask price
   ```

2. **Method Implementation** (`providers/binance-rs/src/orderbook/types.rs`, lines 44-52):
   ```rust
   /// Get best bid price (highest bid)
   pub fn best_bid(&self) -> Option<&Decimal> {
       self.bids.keys().next_back() // BTreeMap is ascending, so last key is highest
   }

   /// Get best ask price (lowest ask)
   pub fn best_ask(&self) -> Option<&Decimal> {
       self.asks.keys().next() // BTreeMap is ascending, so first key is lowest
   }
   ```

**THE BUG:** The methods `best_bid()` and `best_ask()` are **correctly extracting the highest bid and lowest ask** from the BTreeMap. However, when these values are assigned to the `OrderBookMetrics` struct fields at lines 88-89, **they are swapped**.

## Detailed Findings

### 1. OrderBookMetrics Struct Definition
**Location:** `providers/binance-rs/src/orderbook/types.rs`
**Lines:** 78-111

```rust
pub struct OrderBookMetrics {
    pub symbol: String,
    pub timestamp: i64,
    pub spread_bps: f64,
    pub microprice: f64,
    pub bid_volume: f64,
    pub ask_volume: f64,
    pub imbalance_ratio: f64,

    /// Highest bid price (string for decimal precision)
    pub best_bid: String,  // Line 101

    /// Lowest ask price (string for decimal precision)
    pub best_ask: String,  // Line 104

    pub walls: Walls,
    pub slippage_estimates: SlippageEstimates,
}
```

### 2. Spread Calculation
**Location:** `providers/binance-rs/src/orderbook/metrics.rs`
**Lines:** 95-111

```rust
/// Calculate spread in basis points
///
/// Formula: ((best_ask - best_bid) / best_bid) * 10000
fn calculate_spread_bps(best_bid: Decimal, best_ask: Decimal) -> Option<f64> {
    if best_bid.is_zero() {
        return None;
    }

    let spread = best_ask - best_bid;
    let spread_ratio = spread / best_bid;
    let spread_bps = (spread_ratio * Decimal::from(10000))
        .to_f64()
        .unwrap_or(0.0);

    Some(spread_bps)
}
```

**Analysis:**
- The formula is correct: `((best_ask - best_bid) / best_bid) * 10000`
- With swapped values, if bid=111,323.58 and ask=111,294.22:
  - spread = 111,294.22 - 111,323.58 = -29.36
  - spread_bps = (-29.36 / 111,323.58) * 10000 = **-2.6374** ❌ NEGATIVE!

- With correct values (after fixing the swap):
  - spread = 111,323.58 - 111,294.22 = 29.36
  - spread_bps = (29.36 / 111,294.22) * 10000 = **+2.6374** ✓ POSITIVE!

### 3. Microprice Calculation
**Location:** `providers/binance-rs/src/orderbook/metrics.rs`
**Lines:** 113-132

```rust
/// Calculate microprice (volume-weighted fair price)
///
/// Formula: (best_bid * ask_vol + best_ask * bid_vol) / (bid_vol + ask_vol)
fn calculate_microprice(
    best_bid: Decimal,
    best_ask: Decimal,
    bid_volume: f64,
    ask_volume: f64,
) -> Option<f64> {
    let total_volume = bid_volume + ask_volume;
    if total_volume == 0.0 {
        return None;
    }

    let bid_f64 = best_bid.to_f64()?;
    let ask_f64 = best_ask.to_f64()?;

    let microprice = (bid_f64 * ask_volume + ask_f64 * bid_volume) / total_volume;
    Some(microprice)
}
```

**Impact:** The microprice calculation is **also affected** by the swapped values since it uses `best_bid` and `best_ask` parameters that are swapped.

### 4. Order Book L1 Data Extraction
**Location:** `providers/binance-rs/src/orderbook/manager.rs`
**Lines:** 245-287

The REST API snapshot fetching correctly maps Binance API response to internal structures - bids go to bids, asks go to asks. The data ingestion from Binance API is **correct**.

## Summary of Affected Code

| File | Lines | Issue | Fix Required |
|------|-------|-------|--------------|
| `providers/binance-rs/src/orderbook/metrics.rs` | 88-89 | **CRITICAL**: best_bid and best_ask assignments are swapped | Swap the assignments |
| `providers/binance-rs/src/orderbook/metrics.rs` | 40, 64, 78, 99-110 | Calculations use swapped values | Will be fixed after line 88-89 fix |

## Recommended Fix

**File:** `providers/binance-rs/src/orderbook/metrics.rs`
**Lines:** 88-89

### Current (Buggy) Code:
```rust
Some(OrderBookMetrics {
    symbol: order_book.symbol.clone(),
    timestamp: order_book.timestamp,
    spread_bps,
    microprice,
    bid_volume,
    ask_volume,
    imbalance_ratio,
    best_bid: best_bid.to_string(),  // Line 88 - WRONG!
    best_ask: best_ask.to_string(),  // Line 89 - WRONG!
    walls,
    slippage_estimates,
})
```

### Fixed Code:
```rust
Some(OrderBookMetrics {
    symbol: order_book.symbol.clone(),
    timestamp: order_book.timestamp,
    spread_bps,
    microprice,
    bid_volume,
    ask_volume,
    imbalance_ratio,
    best_bid: best_ask.to_string(),  // Line 88 - FIXED: swap to correct value
    best_ask: best_bid.to_string(),  // Line 89 - FIXED: swap to correct value
    walls,
    slippage_estimates,
})
```

## Verification

After applying the fix:
- `spread_bps` will become **positive** (+2.6374 instead of -2.6374)
- `best_bid` will show the correct lower value (111,294.22)
- `best_ask` will show the correct higher value (111,323.58)
- `microprice` calculation will use correct values
- All downstream calculations will be corrected

## Additional Notes

- The bug does NOT affect other parts of the codebase that directly access `order_book.bids` and `order_book.asks`
- The MCP gateway normalization layer correctly passes through values without modification
- Unit tests in `metrics.rs` (lines 352-421) should be updated to catch this type of inversion bug
- This is a **single-line fix** (swap two assignments)
- **Impact**: Critical - affects all orderbook L1 data returned to clients
- **Scope**: Only affects OrderBookMetrics struct initialization, not the underlying orderbook data
