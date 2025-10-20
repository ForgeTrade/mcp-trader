# Feature Specification: Fix Inverted Spread BPS Bug

**Feature Branch**: `017-specify-scripts-bash`
**Created**: 2025-10-20
**Status**: Draft
**Input**: User description: "fix the bug with inverted spread_bps"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Correct Best Bid/Ask Ordering (Priority: P1)

When querying top-of-book orderbook data for any trading pair, the system must always return best_bid < best_ask, resulting in positive spread_bps values. This is a fundamental market data integrity requirement.

**Why this priority**: This is critical P1 because inverted bid/ask data corrupts all downstream trading decisions, analytics, and risk calculations. Negative spreads are mathematically impossible in real markets and indicate a data integrity bug.

**Independent Test**: Can be fully tested by calling `market.get_orderbook_l1` for BTCUSDT and verifying that best_bid < best_ask and spread_bps > 0 in all responses.

**Acceptance Scenarios**:

1. **Given** BTCUSDT orderbook data from Binance, **When** querying market.get_orderbook_l1, **Then** best_bid price must be strictly less than best_ask price
2. **Given** any valid trading pair orderbook, **When** calculating spread_bps, **Then** spread_bps must be positive (> 0)
3. **Given** orderbook data with bid=111323.58 and ask=111294.22, **When** processing in provider, **Then** system must detect and correct the inversion before returning to gateway
4. **Given** 100 consecutive orderbook requests, **When** analyzing all responses, **Then** 100% must have correct bid/ask ordering with zero inversions

---

### Edge Cases

- What happens when market is locked (bid == ask)?
  - spread_bps should be exactly 0.0, not negative
  - Both microprice and imbalance calculations remain valid

- How does system handle extreme volatility with rapid price changes?
  - Each snapshot must maintain bid < ask invariant
  - If provider receives inverted data from exchange, must reject or correct it

- What if exchange API returns genuinely inverted data due to exchange bug?
  - Provider must validate and reject malformed data
  - Return error rather than propagating invalid data downstream

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST ensure best_bid price is strictly less than best_ask price in all OrderBookMetrics calculations
- **FR-002**: System MUST calculate spread_bps as ((best_ask - best_bid) / microprice) * 10000, which must always result in positive values for valid orderbooks
- **FR-003**: System MUST validate bid/ask ordering in the Binance provider's OrderBookMetrics calculation before returning data to gateway
- **FR-004**: System MUST NOT swap or invert best_bid and best_ask values during any stage of data processing
- **FR-005**: System MUST log warning and reject orderbook data if bid >= ask condition is detected from exchange API
- **FR-006**: System MUST calculate microprice as weighted average: (best_bid * ask_size + best_ask * bid_size) / (bid_size + ask_size), using correctly ordered bid/ask values

### Key Entities *(include if feature involves data)*

- **OrderBook**: Top-of-book market data containing best bid (highest buy price) and best ask (lowest sell price), with sizes at each level
- **OrderBookMetrics**: Aggregated orderbook analytics including spread_bps, microprice, and imbalance_ratio - must be calculated using correctly ordered bid/ask data

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of orderbook L1 responses must have best_bid < best_ask (zero tolerance for inversions)
- **SC-002**: 100% of orderbook L1 responses must have spread_bps > 0 for non-locked markets (zero tolerance for negative spreads)
- **SC-003**: Microprice calculation must fall between best_bid and best_ask: best_bid <= microprice <= best_ask
- **SC-004**: Reduce inverted orderbook data incidents from current rate (100% observed in production sample) to 0% after fix

## Technical Context

### Current Behavior (Bug)

Production data observed on 2025-10-20:
```
Best bid: 111,323.58  ← HIGHER (incorrect)
Best ask: 111,294.22  ← LOWER (incorrect)
Spread (bps): -2.6374 ← NEGATIVE (impossible)
```

### Expected Behavior (After Fix)

```
Best bid: 111,294.22  ← LOWER (correct)
Best ask: 111,323.58  ← HIGHER (correct)
Spread (bps): +2.6374 ← POSITIVE (correct)
```

### Root Cause Location

Based on investigation in `/home/limerc/repos/ForgeTrade/mcp-trader/specs/016-fix-sse-schema-bugs/research.md`:

- **NOT in MCP Gateway**: The schema normalization layer (`mcp-gateway/mcp_gateway/adapters/schema_adapter.py:168-229`) correctly passes through best_bid and best_ask without modification
- **IN Binance Provider**: The bug is in the upstream Binance Rust provider's OrderBookMetrics calculation logic (likely in `providers/binance-rs` codebase)
- **Specific Issue**: The provider is swapping bid and ask values somewhere in the orderbook metrics calculation

### Dependencies

- **Binance Provider**: gRPC service on port 50053
- **MCP Gateway**: SSE server on port 3001
- **Production Deployment**: mcp-gateway.thevibe.trading (198.13.46.14)

### Assumptions

1. The exchange API (Binance) returns correctly ordered orderbook data
2. The inversion happens during OrderBookMetrics calculation in the provider
3. The MCP gateway normalization layer (Feature 016) is working correctly
4. This is a pre-existing bug in the provider, not introduced by recent changes
