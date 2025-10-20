# Feature Specification: Fix SSE Schema Normalization Bugs

**Feature Branch**: `016-fix-sse-schema-bugs`
**Created**: 2025-10-20
**Status**: Draft
**Input**: User description: "заведи bugfix на issues после тестирования которое выше" (Create bugfix for issues found during testing above)

## Problem Statement

During SSE gateway testing with the Binance provider, two critical tools are failing due to schema normalization errors:

1. **market.get_orderbook_l1**: Returns error "Normalization failed for binance.orderbook_l1: Invalid orderbook: missing bids or asks"
2. **market.get_klines**: Returns error "Provider binance failed to execute binance.get_klines: list indices must be integers or slices, not str"

Additionally, the venue parameter defaults to `None` instead of `"binance"`, causing normalization failures across multiple tools.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Orderbook Data Access (Priority: P1)

Users (AI clients like ChatGPT) need to query real-time orderbook metrics via the SSE gateway to make trading decisions and market analysis.

**Why this priority**: Orderbook data is fundamental for trading analysis. Without it, users cannot assess market depth, spread, or liquidity - core requirements for any trading application.

**Independent Test**: Can be fully tested by calling `market.get_orderbook_l1` via SSE with ETHUSDT and verifying it returns valid bid/ask/spread data instead of an error.

**Acceptance Scenarios**:

1. **Given** SSE client is connected to gateway, **When** user calls `market.get_orderbook_l1` for ETHUSDT with venue="binance", **Then** system returns valid orderbook data with best_bid, best_ask, spread_bps, microprice, and imbalance
2. **Given** SSE client is connected to gateway, **When** user calls `market.get_orderbook_l1` for BTCUSDT without specifying venue (defaults to binance), **Then** system returns valid orderbook data without normalization errors
3. **Given** orderbook has valid bids and asks, **When** normalization occurs, **Then** schema adapter correctly extracts bid/ask arrays from provider response

---

### User Story 2 - Historical Candlestick Data Access (Priority: P1)

Users need to retrieve historical price candles (klines) to analyze price trends, calculate indicators, and backtest strategies.

**Why this priority**: Klines are essential for technical analysis and strategy development. Equal priority to orderbook as both are core market data types.

**Independent Test**: Can be fully tested by calling `market.get_klines` via SSE with BTCUSDT, interval="1h", limit=5 and verifying it returns 5 valid candlesticks.

**Acceptance Scenarios**:

1. **Given** SSE client is connected to gateway, **When** user calls `market.get_klines` for BTCUSDT with interval="1h" and limit=5, **Then** system returns array of 5 candlesticks with open/high/low/close/volume data
2. **Given** klines response from provider, **When** normalization occurs, **Then** schema adapter correctly parses array response without attempting string-keyed access
3. **Given** user specifies venue="binance" explicitly, **When** klines are requested, **Then** venue is correctly passed to provider and normalization succeeds

---

### User Story 3 - Venue Parameter Defaulting (Priority: P2)

Users should be able to call market tools without explicitly specifying venue="binance" every time, as binance is the default and only supported venue currently.

**Why this priority**: Improves user experience by reducing boilerplate, but not blocking core functionality if users specify venue explicitly.

**Independent Test**: Can be fully tested by calling any market.* tool without venue parameter and verifying it defaults to "binance" and executes successfully.

**Acceptance Scenarios**:

1. **Given** user calls `market.get_ticker` without venue parameter, **When** request is processed, **Then** system defaults venue to "binance" and returns valid ticker data
2. **Given** venue defaults to "binance", **When** normalization occurs, **Then** schema adapter successfully finds binance normalizer instead of returning "No normalizer available for venue 'None'"
3. **Given** venue is None in normalized response, **When** displayed to user, **Then** response shows venue="binance" not venue="N/A"

---

### Edge Cases

- What happens when orderbook response has empty bids array but valid asks array?
- What happens when orderbook response has null/undefined bids or asks fields?
- How does klines normalization handle different array structures (array of arrays vs array of objects)?
- What happens when venue parameter is explicitly set to None/null?
- How does system handle provider responses with unexpected field names?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST correctly parse orderbook responses from binance provider containing bids and asks arrays
- **FR-002**: System MUST extract best_bid and best_ask from orderbook data structure without normalization errors
- **FR-003**: System MUST parse klines array responses using integer indices, not string keys
- **FR-004**: System MUST default venue parameter to "binance" when not explicitly specified by user
- **FR-005**: Schema normalization MUST handle binance provider response formats for both orderbook_l1 and get_klines tools
- **FR-006**: System MUST validate presence of required fields (bids, asks) before attempting normalization
- **FR-007**: Error messages MUST clearly indicate which field is missing or malformed in provider response
- **FR-008**: Normalized responses MUST include venue field set to actual venue name ("binance"), not None or "N/A"

### Key Entities

- **OrderbookL1 Response**: Contains best_bid, best_ask, spread_bps, microprice, imbalance, timestamp, venue, and venue_symbol fields
- **Klines Response**: Contains array of candlestick objects, each with open, high, low, close, volume, and timestamp
- **Venue Parameter**: String identifier for exchange (currently only "binance" supported), should default to "binance" when omitted

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: market.get_orderbook_l1 tool returns valid data without errors for 100% of requests to supported trading pairs
- **SC-002**: market.get_klines tool returns correct number of candlesticks without schema errors for 100% of valid requests
- **SC-003**: 100% of market.* tools execute successfully when venue parameter is omitted (defaulting to binance)
- **SC-004**: Error rate for schema normalization failures reduces to 0% for orderbook_l1 and get_klines tools
- **SC-005**: SSE gateway test suite shows 100% pass rate (currently 60% - 3/5 tools working)
- **SC-006**: All tool responses include correct venue="binance" field, 0% show venue=None or venue="N/A"

## Assumptions

- Binance provider is already returning data in correct format (evidenced by ticker and volume_profile working correctly)
- Issue is isolated to schema normalization layer, not provider communication
- Venue "binance" is the only supported venue currently, so defaulting to it is safe
- Test environment uses same SSE transport and MCP protocol as production
- Orderbook responses from provider always include timestamp and symbol fields
- Klines responses are arrays (not paginated or chunked differently)

## Dependencies

- Existing schema adapter in `mcp-gateway/mcp_gateway/adapters/schema_adapter.py`
- Binance provider normalization logic
- UnifiedToolRouter in `mcp-gateway/mcp_gateway/adapters/unified_router.py`
- SSE server maintaining current tool interface contracts

## Out of Scope

- Adding support for additional venues beyond binance
- Changing tool signatures or adding new parameters
- Performance optimization of normalization logic
- Adding new market data tools
- Modifying Binance provider response format
- Changes to gRPC protocol or provider communication
