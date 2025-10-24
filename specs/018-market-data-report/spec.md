# Feature Specification: Unified Market Data Report

**Feature Branch**: `018-market-data-report`
**Created**: 2025-10-23
**Status**: Draft
**Input**: User description: "Удалить все методы управления ордерами и объединить методы рыночных данных в единый метод отчетности"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Access Comprehensive Market Intelligence Report (Priority: P1)

A trader needs to quickly assess the current market conditions for a specific trading pair by requesting a single comprehensive report that combines pricing, liquidity, order book health, and microstructure analysis.

**Why this priority**: This is the core value proposition of the feature - consolidating multiple data sources into one actionable report saves time and provides better decision-making context than viewing isolated metrics.

**Independent Test**: Can be fully tested by calling the unified report method with a single symbol parameter (e.g., "BTCUSDT") and receiving a markdown-formatted report with all market data sections populated. Success is measured by report completeness and accuracy.

**Acceptance Scenarios**:

1. **Given** a trader wants to analyze BTCUSDT market conditions, **When** they request the unified market data report with symbol "BTCUSDT", **Then** they receive a markdown report containing current price, 24h statistics, order book metrics, spread analysis, liquidity profile, and market health indicators.

2. **Given** the market data report is generated for ETHUSDT, **When** the trader views the report, **Then** they can immediately identify: current price trend, bid-ask spread quality, order book depth, major liquidity zones, potential anomalies, and overall market health status.

3. **Given** a trader needs to compare multiple trading pairs, **When** they request reports for BTCUSDT and ETHUSDT consecutively, **Then** each report is generated within 3 seconds and presents data in a consistent markdown format enabling easy comparison.

---

### User Story 2 - Identify Trading Risks Through Anomaly Detection (Priority: P2)

A trader needs to be warned about abnormal market conditions such as quote stuffing, flash crash risks, or iceberg orders that could affect trade execution quality.

**Why this priority**: Safety and risk awareness is critical for protecting capital, but it's secondary to basic market data access. This builds on P1 by adding advanced analytics.

**Independent Test**: Can be tested by requesting the market report during various market conditions and verifying that detected anomalies are clearly highlighted in a dedicated "Market Anomalies" section with severity levels and recommendations.

**Acceptance Scenarios**:

1. **Given** the market is experiencing quote stuffing activity, **When** a trader requests the market data report, **Then** the report includes a "Market Anomalies" section showing the anomaly type, severity level (Low/Medium/High/Critical), affected price levels, and specific recommendations.

2. **Given** no market anomalies are detected, **When** a trader requests the market data report, **Then** the "Market Anomalies" section shows "No anomalies detected" with a timestamp confirming the last analysis time.

3. **Given** multiple anomalies exist simultaneously, **When** the report is generated, **Then** anomalies are sorted by severity (Critical first) and each includes actionable recommendations.

---

### User Story 3 - Analyze Liquidity for Order Placement (Priority: P2)

A trader needs to identify optimal price levels for placing stop-loss or limit orders by understanding where liquidity vacuums exist and where major liquidity walls are positioned.

**Why this priority**: This addresses the "how to act on market data" question, helping traders make better tactical decisions about order placement. It's P2 because it requires P1 (basic market data) to be useful.

**Independent Test**: Can be tested by requesting the market report and verifying the presence of a "Liquidity Analysis" section that shows liquidity walls, volume profile with POC/VAH/VAL levels, and identified liquidity vacuum zones with price ranges.

**Acceptance Scenarios**:

1. **Given** a trader wants to identify safe stop-loss levels, **When** they request the market data report, **Then** the report includes a "Liquidity Analysis" section showing liquidity vacuum zones with price ranges and expected impact levels.

2. **Given** the order book has significant buy/sell walls, **When** the report is generated, **Then** the liquidity walls are clearly identified with their price levels and volumes, indicating potential support/resistance zones.

3. **Given** a trader needs to understand volume distribution, **When** they view the volume profile section of the report, **Then** they see the Point of Control (POC), Value Area High (VAH), and Value Area Low (VAL) price levels calculated over a configurable time window.

---

### User Story 4 - Monitor Order Book Health and Data Quality (Priority: P3)

A trader needs to verify that the market data being presented is reliable and up-to-date before making trading decisions, especially when using automated strategies.

**Why this priority**: While important for data quality assurance, this is less critical than the actual market insights. It's a supporting feature that enhances trust but doesn't directly drive trading decisions.

**Independent Test**: Can be tested by requesting the market report and checking the "Data Health Status" section which shows websocket connectivity, last update timestamp age, and overall health status (Healthy/Degraded/Poor/Critical).

**Acceptance Scenarios**:

1. **Given** all market data feeds are operating normally, **When** a trader requests the market data report, **Then** the "Data Health Status" section shows status as "Healthy" with green indicators and recent update timestamps.

2. **Given** the websocket connection is disconnected, **When** the report is generated, **Then** the health status shows "Degraded" or "Critical" with a warning message explaining the data freshness issue.

3. **Given** a trader wants to verify data recency, **When** they view the report header, **Then** the report includes a generation timestamp and indicates how old the underlying data is (e.g., "Data as of: 2025-10-23 14:32:15 UTC, Age: 245ms").

---

### Edge Cases

- **No Data Available**: What happens when market data is temporarily unavailable for a requested symbol? The report should clearly indicate missing sections with explanatory messages rather than failing completely.

- **Stale Data**: How does the system handle situations where cached data is older than acceptable thresholds? The report should include data freshness warnings and recommend refreshing.

- **Invalid Symbol**: What happens when a user requests a report for a non-existent or unsupported trading pair? The system should return a clear error message listing supported symbols.

- **Partial Feature Availability**: How does the report behave when advanced analytics features (orderbook_analytics) are disabled at compile time? The report should gracefully omit those sections and include a note explaining limited feature availability.

- **Rate Limiting**: What happens when the exchange rate limits are hit during report generation? The system should use cached data where available and indicate which sections are stale.

- **Concurrent Requests**: How does the system handle multiple simultaneous report requests for different symbols? Each request should be independent and not block others.

- **Extremely Volatile Markets**: How does the report handle flash crash scenarios where data changes rapidly? The report should capture a consistent snapshot with a precise timestamp.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST remove all order management methods from public APIs including: place_order, cancel_order, get_order, get_open_orders, get_all_orders, get_account, get_my_trades, and WebSocket listen key management methods (create_listen_key, keepalive_listen_key, close_listen_key).

- **FR-002**: System MUST consolidate all market data retrieval methods into a single unified reporting method named `generate_market_report()`.

- **FR-003**: The unified report method MUST accept a trading symbol parameter (e.g., "BTCUSDT") and return a comprehensive markdown-formatted document.

- **FR-004**: The markdown report MUST include the following sections in this order:
  1. **Report Header** - Symbol, generation timestamp, data age indicator
  2. **Price Overview** - Current price, 24h change, 24h high/low, volume
  3. **Order Book Metrics** - Spread (bps), microprice, bid/ask volume, imbalance ratio
  4. **Liquidity Analysis** - Major walls, volume profile (POC/VAH/VAL), liquidity vacuums
  5. **Market Microstructure** - Order flow direction, bid/ask flow rates, net flow
  6. **Market Anomalies** - Detected anomalies with severity and recommendations
  7. **Microstructure Health** - Composite health score, component scores, warnings
  8. **Data Health Status** - Websocket connectivity, last update age, overall status

- **FR-005**: Each report section MUST degrade gracefully if data is unavailable, showing "[Data Unavailable]" or similar placeholder with an explanatory note rather than failing the entire report.

- **FR-006**: The report MUST include visual indicators in markdown format (tables, lists, emoji indicators for status levels) to enhance readability.

- **FR-007**: The unified method MUST accept optional parameters to customize report content:
  - `include_sections`: List of section names to include (default: all)
  - `volume_window_hours`: Time window for volume profile calculation (default: 24h)
  - `orderbook_levels`: Number of order book levels to include in depth analysis (default: 20)

- **FR-008**: System MUST provide clear error messages when the requested symbol is invalid, unsupported, or when data cannot be retrieved.

- **FR-009**: The unified method MUST execute efficiently by parallelizing independent data fetches (ticker, orderbook, analytics) and complete in <5s for cold cache requests and <3s for cached requests.

- **FR-010**: System MUST remove all gRPC tool handlers related to order management: `binance.place_order`, `binance.cancel_order`, `binance.get_order`, `binance.get_open_orders`, `binance.get_all_orders`, `binance.get_account`, `binance.get_my_trades`.

- **FR-011**: System MUST expose the new unified reporting method through both the MCP handler layer and gRPC tool layer for backward compatibility with existing integrations.

- **FR-012**: Authentication infrastructure (API key handling, signature generation, credential management) MUST be preserved in the codebase for potential future use with authenticated read-only endpoints. Only the order management methods themselves should be removed, not the underlying authentication mechanisms.

- **FR-013**: System MUST maintain existing market data streaming capabilities (WebSocket subscriptions for ticker, orderbook, trades) as these support real-time data for report generation.

### Key Entities

- **Market Data Report**: A comprehensive markdown-formatted document containing aggregated market intelligence for a specific trading symbol. Key attributes include: target symbol, generation timestamp, data freshness indicators, and structured sections for price, liquidity, order flow, anomalies, and health metrics.

- **Report Section**: A named component of the market report with specific data requirements and formatting. Sections can be independently generated, included/excluded via parameters, and gracefully degrade if data is unavailable.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Traders can obtain a complete market intelligence report for any supported symbol in under 5 seconds on first request and under 3 seconds on subsequent requests within the cache window.

- **SC-002**: The unified report method reduces the number of API calls required to gather comprehensive market data from 8-12 individual method calls to a single method call, improving efficiency by at least 80%.

- **SC-003**: Generated markdown reports are human-readable and can be rendered correctly in standard markdown viewers without formatting errors.

- **SC-004**: Report generation succeeds even when up to 30% of data sources are temporarily unavailable, with clear indicators showing which sections have missing data.

- **SC-005**: The codebase is reduced by removing all order management functionality, resulting in at least 500 lines of code removal from client.rs, grpc/tools.rs, and mcp/handler.rs combined.

- **SC-006**: 100% of removed order management methods have no remaining references in the codebase (verified by compilation success and grep searches).

- **SC-007**: The markdown report includes actionable insights that enable traders to make informed decisions without needing to interpret raw data (e.g., "Strong buy pressure detected", "Critical anomaly: Flash crash risk at $42,150").

- **SC-008**: Report generation handles at least 10 concurrent requests for different symbols without performance degradation or data corruption.

## Assumptions

- **Architecture Assumption**: The system will maintain the existing layered architecture (Binance client → MCP/gRPC handlers → Gateway) but simplify it by removing order execution pathways. Authentication infrastructure will be preserved for future use.

- **Data Source Assumption**: All market data continues to be sourced from Binance REST API and WebSocket streams as currently implemented. No new data providers are required.

- **Markdown Format Assumption**: Markdown is the standard output format because it's human-readable, widely supported, and can be easily consumed by LLMs, documentation systems, and terminal renderers. Alternative formats (JSON, HTML) are not required for the initial implementation.

- **Caching Strategy Assumption**: The system will leverage existing caching mechanisms (SnapshotStorage, OrderBookManager) to improve report generation performance. Cache invalidation strategies remain unchanged.

- **Feature Flags Assumption**: Advanced analytics sections (anomaly detection, microstructure health) depend on compile-time feature flags (`orderbook_analytics`). Reports will gracefully handle the absence of these features.

- **Single Symbol Focus Assumption**: The unified report method generates reports for one symbol at a time. Multi-symbol comparison reports are out of scope for this feature.

- **Read-Only Transformation Assumption**: This feature transforms the system from a hybrid read/write trading client into a read-only market data analysis tool. All write operations (trading) are being intentionally removed.

## Out of Scope

- **Multi-Symbol Comparison Reports**: Generating side-by-side comparison reports for multiple trading pairs in a single call.

- **Report Persistence**: Saving generated reports to disk or database for historical analysis.

- **Custom Report Templates**: Allowing users to define custom report layouts or section ordering beyond the include_sections parameter.

- **Real-Time Report Updates**: Streaming live updates to an existing report as market conditions change (reports are static snapshots).

- **Alternative Output Formats**: Generating reports in JSON, XML, HTML, or PDF formats (only markdown is supported).

- **Backtesting Data**: Including historical candlestick data or trade history in the report beyond the current 24h ticker statistics.

- **Portfolio Context**: Integrating user-specific context like existing positions, P&L, or risk exposure (all order management is removed).

- **Alert Configuration**: Setting up automated alerts based on report metrics (e.g., "notify me when spread exceeds 50 bps").

- **API Gateway Changes**: Modifications to the Python MCP Gateway layer beyond updating tool registration to expose the new unified method.

## Dependencies

- **Existing Binance Client Library**: The unified report method will call existing market data retrieval functions (get_ticker, get_orderbook, get_orderbook_metrics, etc.) as building blocks.

- **OrderBook Manager**: Real-time order book data and metrics depend on the OrderBookManager maintaining active WebSocket connections and snapshot storage.

- **Analytics Subsystem**: Advanced report sections require the orderbook_analytics feature to be enabled at compile time.

- **Markdown Rendering**: Users need markdown-capable viewers (terminal renderers, IDEs, documentation platforms) to properly display formatted reports.

## Risks

- **Performance Degradation**: Aggregating 8+ data sources in a single method call could exceed acceptable latency thresholds if not properly parallelized.
  - *Mitigation*: Implement concurrent data fetching using async/await patterns and set timeout limits per data source.

- **Cache Coherency**: Different data sources may have different update frequencies, leading to inconsistent snapshots where price data is fresh but order book data is stale.
  - *Mitigation*: Include data timestamps for each section and calculate maximum age deviation in the report header.

- **Breaking Changes**: Removing all order management methods will break any existing integrations or scripts that depend on those endpoints.
  - *Mitigation*: Clearly document breaking changes, provide migration guide, and consider versioning the API if backward compatibility is critical.

- **Incomplete Reports**: If multiple data sources fail simultaneously, the report may become too sparse to be useful.
  - *Mitigation*: Define minimum data requirements (e.g., must have at least ticker and basic orderbook) and fail fast with clear error messages if these are unavailable.

