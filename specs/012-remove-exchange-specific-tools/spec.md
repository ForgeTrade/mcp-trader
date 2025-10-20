# Feature Specification: Remove Exchange-Specific Tools

**Feature Branch**: `012-remove-exchange-specific-tools`
**Created**: 2025-10-20
**Status**: Draft
**Input**: User requirement: "Заменить все Binance-specific методы (binance.get_ticker, binance.orderbook_l1, etc.) унифицированными методами с параметром venue. Удалить все exchange-specific tools из API, оставить только unified tools."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Developer Uses Unified API Only (Priority: P1)

A developer integrating with the MCP gateway wants to fetch ticker data from any exchange using a single unified method, without needing to know exchange-specific method names.

**Why this priority**: This is the core value proposition - API simplification and exchange-agnostic design. Without this, the gateway doesn't achieve its primary goal of abstraction.

**Independent Test**: Developer can list available tools and see ONLY unified methods like `market.get_ticker`, `market.get_orderbook_l1`, etc. No exchange-specific methods (binance.*, okx.*) are visible in the tool listing.

**Acceptance Scenarios**:

1. **Given** the MCP gateway is running with Binance and OKX providers, **When** a developer calls `/list_tools`, **Then** they see only unified tools (`market.get_ticker`, `market.get_orderbook_l1`, `market.get_orderbook_l2`, `market.get_klines`) and NO provider-specific tools (no `binance.get_ticker`, `binance.orderbook_l1`, etc.)

2. **Given** the developer wants to fetch BTCUSDT ticker from Binance, **When** they call `market.get_ticker(venue="binance", instrument="BTCUSDT")`, **Then** they receive normalized ticker data with bid, ask, mid, spread_bps

3. **Given** the developer wants to fetch BTCUSDT ticker from OKX, **When** they call `market.get_ticker(venue="okx", instrument="BTC-USDT")`, **Then** they receive the same normalized schema as Binance

4. **Given** the developer tries to call `binance.get_ticker`, **When** they invoke this tool, **Then** they receive an error "Unsupported tool" or "Tool not found"

---

### User Story 2 - Multi-Exchange Comparison (Priority: P2)

A trading analyst wants to compare orderbook data across multiple exchanges using identical method calls, differing only by the venue parameter.

**Why this priority**: Enables cross-exchange analysis without learning multiple APIs. This is a key use case for professional traders.

**Independent Test**: Analyst can call the same method (`market.get_orderbook_l1`) multiple times with different venue parameters and receive responses in identical normalized formats.

**Acceptance Scenarios**:

1. **Given** the analyst wants to compare Binance and OKX orderbooks, **When** they call `market.get_orderbook_l1(venue="binance", instrument="BTCUSDT")` and `market.get_orderbook_l1(venue="okx", instrument="BTC-USDT")`, **Then** both responses have identical schema structure (bid_price, ask_price, bid_quantity, ask_quantity, mid, spread_bps)

2. **Given** the analyst needs to fetch klines from multiple venues, **When** they iterate over venues calling `market.get_klines(venue=v, instrument=i, interval="1h")`, **Then** all responses conform to the same normalized schema

---

### User Story 3 - New Exchange Integration (Priority: P3)

A developer adds a new exchange provider (e.g., Coinbase) to the gateway. The unified API automatically exposes this exchange through existing unified tools without requiring client code changes.

**Why this priority**: Demonstrates extensibility and forward compatibility of the unified design.

**Independent Test**: After adding a new provider, clients can immediately use `market.get_ticker(venue="coinbase", instrument="BTC-USD")` without any gateway API changes.

**Acceptance Scenarios**:

1. **Given** a new provider "coinbase" is registered in providers.yaml, **When** the gateway initializes, **Then** the tool listing still shows ONLY unified tools (no coinbase.* tools added)

2. **Given** the new provider supports ticker data, **When** a developer calls `market.get_ticker(venue="coinbase", instrument="BTC-USD")`, **Then** the request is routed correctly and returns normalized data

3. **Given** the tool metadata is queried, **When** developer requests `market.get_ticker` metadata, **Then** the response includes "coinbase" in the list of available_venues

---

### Edge Cases

- What happens when a client attempts to call a previously-available provider-specific tool like `binance.get_ticker`?
  - System should return clear error: "Tool not found. Use unified tool market.get_ticker with venue parameter instead."

- How does the system handle providers that register tools with custom names not matching the unified pattern?
  - UnifiedToolRouter should have explicit mapping for supported unified tools. Custom tools that don't map are NOT exposed.

- What if a provider supports a tool that has no unified equivalent yet?
  - Provider-specific tool remains hidden. Feature can be exposed later by adding new unified tool definition.

- How are clients migrated from old provider-specific API to unified API?
  - Breaking change requires version bump and migration guide. Recommendation: Gateway version 2.0.0 with clear migration documentation.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The gateway MUST expose ONLY unified tools in the `/list_tools` response (e.g., `market.get_ticker`, `market.get_orderbook_l1`, `market.get_orderbook_l2`, `market.get_klines`)

- **FR-002**: The gateway MUST NOT expose provider-specific tools in the `/list_tools` response (e.g., NO `binance.get_ticker`, `binance.orderbook_l1`, `okx.get_ticker`, etc.)

- **FR-003**: All unified tools MUST require a `venue` parameter that specifies which exchange to query (e.g., "binance", "okx", "coinbase")

- **FR-004**: All unified tools MUST use normalized parameter names (e.g., `instrument` instead of `symbol`) to abstract exchange-specific terminology

- **FR-005**: The gateway MUST return normalized response schemas for all unified tool calls, regardless of which provider/venue is used

- **FR-006**: The `UnifiedToolRouter` MUST continue to function, routing unified tool calls to the appropriate provider-specific gRPC methods internally

- **FR-007**: When a client attempts to invoke a provider-specific tool (e.g., `binance.get_ticker`), the gateway MUST return a clear error indicating the tool is not available and suggesting the unified alternative

- **FR-008**: Tool metadata (via `get_tool_metadata()`) MUST list all available venues for each unified tool (e.g., `market.get_ticker` metadata shows `available_venues: ["binance", "okx"]`)

- **FR-009**: The gateway MUST dynamically update available venues based on which providers are configured and healthy, without changing the unified tool names

- **FR-010**: The implementation MUST maintain backward compatibility with the internal gRPC provider protocol (providers still expose provider-specific tools internally, but gateway filters them from external API)

### Key Entities

- **Unified Tool**: A tool exposed by the gateway with a generic name (e.g., `market.get_ticker`) that accepts a `venue` parameter to route to any exchange

- **Provider-Specific Tool**: A tool with exchange-specific naming (e.g., `binance.get_ticker`) that is used internally for gRPC routing but NOT exposed in the external API

- **Venue**: A string identifier for an exchange provider (e.g., "binance", "okx") used as a parameter in unified tools

- **Tool Listing**: The response from `/list_tools` endpoint, which must contain ONLY unified tools after this feature

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: The `/list_tools` endpoint response contains ZERO tools with provider-specific prefixes (binance.*, okx.*, etc.) - only unified tools (market.*, trade.*)

- **SC-002**: Developers can query ticker data from all available exchanges using the single method `market.get_ticker` with different venue parameters

- **SC-003**: The tool count exposed to clients is reduced from 21+ provider-specific tools to approximately 4-6 unified tools (market.get_ticker, market.get_orderbook_l1, market.get_orderbook_l2, market.get_klines, plus any trade.* tools)

- **SC-004**: All unified tool responses conform to normalized schemas defined in SchemaAdapter, with consistent field names across all venues

- **SC-005**: Adding a new exchange provider requires ZERO changes to the exposed tool API - only the available_venues list grows

- **SC-006**: Existing integration tests for UnifiedToolRouter and SchemaAdapter continue to pass without modification

- **SC-007**: Client attempting to use old provider-specific tool receives actionable error message within 100ms
