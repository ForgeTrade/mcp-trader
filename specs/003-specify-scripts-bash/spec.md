# Feature Specification: Advanced Order Book Analytics Integration

**Feature Branch**: `003-specify-scripts-bash`
**Created**: 2025-10-19
**Status**: Draft
**Input**: User description: "Advanced Order Book Analytics Integration - Order Flow, Volume Profile, Market Microstructure from mcp-binance-rs"

## Clarifications

### Session 2025-10-19

- Q: What are the minimum and maximum allowed values for `window_duration_secs` in the order flow analysis? → A: Min: 10s, Max: 300s (5 minutes)
- Q: Which statistical approach should be used for the 95% confidence threshold in iceberg detection? → A: Z-score method (standard deviations from mean)
- Q: How should the http_transport, orderbook, and orderbook_analytics feature flags depend on each other? → A: http_transport independent of analytics (can expose any tools)
- Q: What method should be used to calculate the 24-hour average spread for abnormal spread widening detection? → A: Rolling 24h moving average (sliding window)
- Q: What should happen when RocksDB storage approaches or exceeds the 1GB threshold for 20 symbols? → A: Hard limit - enforce 1GB maximum, fail new writes with clear error

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Order Flow Analysis for Trade Timing (Priority: P1)

Algorithmic traders using the MCP gateway need real-time **order flow dynamics** (bid/ask pressure rates) to identify optimal entry and exit points. They require instant visibility into buying and selling pressure changes to make time-sensitive trading decisions through natural language queries to AI agents.

**Why this priority**: Order flow is the most critical real-time indicator of market sentiment. Without it, AI-powered trading assistants cannot provide actionable momentum shift signals, making this the foundation of intelligent trade timing.

**Independent Test**: Can be fully tested by requesting "Show me order flow for BTCUSDT over the last 60 seconds" through Claude Code and verifying the response includes bid_flow_rate, ask_flow_rate, net_flow, and flow_direction indicators with clear "buying pressure increasing" or "selling pressure dominant" interpretations.

**Acceptance Scenarios**:

1. **Given** BTCUSDT orderbook WebSocket is connected via binance-rs provider, **When** AI agent requests order flow metrics for last 60 seconds, **Then** system returns bid flow rate (orders/sec), ask flow rate (orders/sec), net flow (bid - ask), flow direction indicator (STRONG_BUY, MODERATE_BUY, NEUTRAL, MODERATE_SELL, STRONG_SELL), and cumulative delta

2. **Given** order flow shows high bid flow rate (>100 orders/min), **When** AI agent analyzes the data, **Then** response highlights "strong buying pressure detected" with timestamp of pressure surge and actionable recommendation

3. **Given** sudden spike in order cancellations on bid side (>50 orders/min removed), **When** orderbook updates, **Then** system detects "liquidity withdrawal event" and flags it in order flow analysis with severity indicator

---

### User Story 2 - Volume Profile for Support/Resistance Discovery (Priority: P2)

Technical analysts using AI assistants need **volume distribution across price levels** to identify high-volume nodes (support/resistance zones) that traditional price charts miss. They ask questions like "Where are the key support levels for Ethereum?" and expect intelligent analysis based on actual traded volume.

**Why this priority**: Volume profile reveals institutional accumulation zones and critical support/resistance levels that significantly improve trade entry/exit decisions and risk management for swing traders.

**Independent Test**: Can be tested by asking "Generate volume profile for ETHUSDT over the last 24 hours" and verifying the response correctly identifies Point of Control (highest volume price), Value Area High/Low (70% volume boundaries), and highlights liquidity vacuum zones.

**Acceptance Scenarios**:

1. **Given** 24 hours of ETHUSDT aggregated trade data from Binance, **When** user requests volume profile through AI agent, **Then** system returns histogram showing volume distribution, Point of Control (POC), Value Area High (VAH), Value Area Low (VAL), and total volume metrics

2. **Given** volume profile shows POC at $3,500 with 45% of volume, **When** current price approaches $3,500, **Then** AI agent highlights "approaching high-volume support zone at POC ($3,500)" with trade implications

3. **Given** low-volume node (liquidity vacuum) between $3,550-$3,580 (volume <20% of median), **When** AI analyzes profile, **Then** response flags "liquidity vacuum detected - expect rapid price movement through $3,550-$3,580 zone" with stop loss placement guidance

---

### User Story 3 - Market Microstructure Anomaly Detection (Priority: P3)

Risk managers and professional traders need AI-powered **anomaly detection** (quote stuffing, iceberg orders, flash crash precursors) to avoid executing trades during manipulated or unstable market conditions. They rely on natural language queries like "Is BTCUSDT market healthy right now?" to receive instant risk assessments.

**Why this priority**: Protects traders from losses during HFT manipulation and abnormal volatility events. Essential for institutional-grade risk management and maintaining fair trading conditions.

**Independent Test**: Can be tested by requesting "Check for market anomalies in BTCUSDT" and verifying the system correctly detects quote stuffing (simulated via high update rate), iceberg orders (via refill pattern analysis), and flash crash precursors with actionable recommendations.

**Acceptance Scenarios**:

1. **Given** normal market conditions (<100 orderbook updates/sec), **When** quote stuffing occurs (>500 updates/sec with <10% trade fill rate), **Then** system detects "quote stuffing - potential HFT manipulation" with High severity and recommends "delay order execution, widen limit order spreads"

2. **Given** large iceberg order detected at bid level (refill rate >5x median after fills), **When** AI agent analyzes orderbook patterns, **Then** response highlights "suspected iceberg order at $106,400 - institutional accumulation likely" with confidence score and implications for price support

3. **Given** sudden liquidity drain (>80% of top 20 levels removed in <1 second) with spread widening (>10x average), **When** flash crash risk emerges, **Then** system triggers "flash crash risk - extreme caution advised" with Critical severity and recommends halting new trades

4. **Given** composite microstructure health score calculation, **When** user requests "Is market healthy?", **Then** system returns score 0-100 with components (spread stability, liquidity depth, flow balance, update rate) and interpretation: Excellent (80-100), Good (60-79), Fair (40-59), Poor (20-39), Critical (0-19)

---

### User Story 4 - Liquidity Mapping and Smart Order Placement (Priority: P4)

Advanced traders need **liquidity vacuum identification** and **absorption event detection** to optimize stop loss placement and identify institutional activity. They ask "Where should I place stops for SOLUSDT?" and expect analysis of liquidity gaps and whale accumulation zones.

**Why this priority**: Prevents stop hunting by avoiding low-liquidity zones and helps identify institutional order flow for trade confirmation. Enhances execution quality and reduces slippage risk.

**Independent Test**: Can be tested by requesting "Map liquidity for SOLUSDT" and verifying the system identifies vacuum zones (price ranges with <20% median volume), large order walls, and provides stop loss placement recommendations.

**Acceptance Scenarios**:

1. **Given** historical orderbook snapshot analysis over 30 minutes, **When** user requests liquidity mapping, **Then** system returns identified vacuum zones with severity (Critical/High/Medium), price ranges, volume deficit percentages, and expected impact (rapid price discovery)

2. **Given** absorption event detected (large bid at $144.00 absorbing 250 SOL of market sells without price movement), **When** AI analyzes patterns, **Then** response flags "whale accumulation detected at $144.00 - strong support level" with absorbed volume and refill count

3. **Given** vacuum zone identified at $145.50-$148.20 (85% below median volume), **When** user asks about stop placement, **Then** system recommends "avoid placing stops in $145.50-$148.20 vacuum zone - use $143.80 or $148.50 instead" with risk explanation

---

### User Story 5 - ChatGPT MCP Integration via Streamable HTTP (Priority: P5)

ChatGPT users and plugin developers need to connect ChatGPT to the binance-rs provider using the official MCP Streamable HTTP transport protocol (March 2025 specification) through a single `/mcp` endpoint without complex handshake workflows.

**Why this priority**: Enables ChatGPT integration which dramatically expands user base beyond Claude Code users. Lower priority than core analytics but high value for ecosystem adoption.

**Independent Test**: Can be tested by configuring ChatGPT connector with `/mcp` endpoint URL and executing tool calls through ChatGPT interface, verifying JSON-RPC 2.0 responses with proper `Mcp-Session-Id` session management.

**Acceptance Scenarios**:

1. **Given** no prior connection, **When** ChatGPT sends POST `/mcp` with `initialize` method, **Then** provider creates session and returns `Mcp-Session-Id` header enabling subsequent requests

2. **Given** valid `Mcp-Session-Id` header from initialization, **When** ChatGPT sends POST `/mcp` with `tools/list` method, **Then** provider returns JSON-RPC response listing all tools with schemas in MCP content array format

3. **Given** active session, **When** ChatGPT sends POST `/mcp` with `tools/call` method for `binance.detect_market_anomalies`, **Then** provider executes analytics and returns results compatible with ChatGPT's content rendering

4. **Given** missing or invalid `Mcp-Session-Id` header, **When** ChatGPT sends non-initialize request, **Then** provider returns appropriate JSON-RPC error (code -32002 for missing, -32001 for invalid) with HTTP 400/404 status

---

### Edge Cases

- What happens when Streamable HTTP session expires mid-conversation? (Client receives HTTP 404 with JSON-RPC error code -32001, must re-initialize with new `initialize` request)
- How does system handle rapid concurrent initialize requests from multiple clients? (Each gets unique session ID up to 50 concurrent session limit, 503 error when limit exceeded)
- What if ChatGPT client sends tools/call without prior initialize? (HTTP 400 with JSON-RPC error code -32002: Missing Mcp-Session-Id header)

- What happens when WebSocket connection drops during order flow calculation? (System marks data as stale, shows "degraded" health status, resumes calculation on reconnection without data loss)
- How does system handle volume profile requests for low-liquidity pairs with <100 trades/day? (Returns "insufficient data for reliable analysis - minimum 1000 trades required for 24h profile" with current trade count)
- What if iceberg detection triggers false positives on legitimate market maker activity? (Uses 95% confidence threshold - only flags patterns with >95% statistical certainty, includes confidence score in response)
- How to distinguish between natural spread widening and flash crash precursor? (Compares current spread vs 24h rolling average - flags only if >10x wider AND accompanied by >80% liquidity drain)
- What if RocksDB time-series storage fills disk space? (Automatic 7-day retention with rolling deletion, compression enabled, monitoring with disk usage alerts at 80% capacity)
- How does system handle concurrent requests for analytics on 20+ symbols? (Rate limiting with queue, maximum 20 concurrent symbol tracking inherited from existing orderbook feature, clear error message when limit exceeded)

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST integrate existing order flow analysis from mcp-binance-rs into binance-rs provider, exposing `binance.get_order_flow` MCP tool through gateway with parameters: symbol (required), window_duration_secs (optional, default 60, range 10-300)

- **FR-002**: System MUST calculate order flow metrics (bid_flow_rate, ask_flow_rate, net_flow, cumulative_delta) over configurable time windows (10-300 seconds) by aggregating orderbook snapshot changes from RocksDB time-series storage captured at 1-second intervals

- **FR-003**: System MUST determine flow direction using thresholds: STRONG_BUY (bid flow >2x ask flow), MODERATE_BUY (1.2-2x), NEUTRAL (0.8-1.2), MODERATE_SELL (0.5-0.8), STRONG_SELL (<0.5)

- **FR-004**: System MUST integrate volume profile generation from mcp-binance-rs, exposing `binance.get_volume_profile` MCP tool with parameters: symbol (required), duration_hours (optional, default 24, range 1-168), tick_size (optional, auto-calculated if omitted)

- **FR-005**: System MUST generate volume profile histogram using adaptive tick-based binning (exchange tick size × 10, or price_range/100 if larger) with Point of Control (POC = max volume bin), Value Area High/Low (VAH/VAL = 70% volume boundaries)

- **FR-006**: System MUST connect to Binance aggregated trade stream (wss://stream.binance.com:9443/ws/<symbol>@aggTrade) for volume profile data collection with exponential backoff reconnection (1s, 2s, 4s, 8s, max 60s)

- **FR-007**: System MUST integrate anomaly detection from mcp-binance-rs, exposing `binance.detect_market_anomalies` MCP tool with parameters: symbol (required), window_duration_secs (optional, default 60)

- **FR-008**: System MUST detect quote stuffing by monitoring orderbook update rate (>500 updates/sec threshold) vs trade fill rate (<10% threshold) and flag with severity (Medium if 500-750 updates/sec, High if 750-1000, Critical if >1000)

- **FR-009**: System MUST identify iceberg orders by tracking price level refill rates (>5x median refill rate after fills = suspected iceberg) with 95% confidence threshold using z-score statistical analysis (refill rate z-score > 1.96 indicates iceberg)

- **FR-010**: System MUST monitor flash crash precursors: liquidity drain (>80% depth loss in <1s), abnormal spread widening (>10x rolling 24h moving average spread), high cancellation rate (>90% of updates are cancels)

- **FR-011**: System MUST integrate microstructure health scoring, exposing `binance.get_microstructure_health` MCP tool returning 0-100 composite score from: spread_stability (25% weight), liquidity_depth (35%), flow_balance (25%), update_rate (15%)

- **FR-012**: System MUST integrate liquidity vacuum detection, exposing `binance.get_liquidity_vacuums` MCP tool identifying price ranges where volume <20% of median with severity classifications and stop loss placement recommendations

- **FR-013**: System MUST implement RocksDB-backed time-series storage for orderbook snapshots captured at 1-second intervals with 7-day retention, prefix-scan queries, MessagePack serialization, and Zstd compression (hard limit: 1GB maximum for 20 symbols - fail new writes with "storage_limit_exceeded" error when reached)

- **FR-014**: System MUST provide absorption event detection via `binance.get_order_flow` output, identifying large orders (>5x median size) repeatedly absorbing market pressure without price movement

- **FR-015**: System MUST add feature flag `orderbook_analytics` to Cargo.toml extending `orderbook` feature, gating all analytics tools behind compilation flag for optional deployment

- **FR-016**: Gateway MUST expose all 5 new analytics tools through MCP protocol with proper JSON schema definitions, parameter validation, and error handling consistent with existing binance tools

- **FR-017**: System MUST return comprehensive error messages for analytics failures including: "insufficient_historical_data" (need N more snapshots), "websocket_disconnected" (trade stream unavailable), "storage_error" (RocksDB failure), "rate_limit_exceeded" (too many concurrent requests), "storage_limit_exceeded" (1GB hard limit reached, oldest data must be purged)

- **FR-018**: System MUST integrate Streamable HTTP transport from mcp-binance-rs, exposing POST `/mcp` endpoint for all MCP JSON-RPC 2.0 requests (initialize, tools/list, tools/call) following March 2025 MCP specification

- **FR-019**: System MUST implement session management using `Mcp-Session-Id` header for request validation - first `initialize` request creates session and returns header, subsequent requests require valid header for authentication

- **FR-020**: System MUST support up to 50 concurrent Streamable HTTP sessions with automatic session timeout after 30 minutes of inactivity and graceful cleanup of expired sessions

- **FR-021**: System MUST return proper JSON-RPC 2.0 error responses with HTTP status codes: 400 (missing session header, code -32002), 404 (invalid session, code -32001), 503 (session limit exceeded, code -32000)

- **FR-022**: System MUST add feature flag `http_transport` to Cargo.toml (separate from `http-api` REST endpoints and independent of `orderbook_analytics`), gating Streamable HTTP behind optional compilation - http_transport can expose any tool set (base tools, analytics, or both)

- **FR-023**: System MUST maintain backward compatibility with existing Python MCP gateway (gRPC mode) when `http_transport` feature is disabled - both transports can coexist via feature flags

- **FR-024**: System MUST support dual-mode operation where provider can run as either gRPC server (for Python gateway) OR Streamable HTTP server (for direct AI client access) based on command-line flags

- **FR-025**: System MUST reuse all existing tool implementations (market data, orderbook, analytics) for Streamable HTTP transport without code duplication - transport layer is abstraction only

### Key Entities

- **OrderFlowSnapshot**: Represents order flow state over time window - includes symbol, time_window_start/end, window_duration_secs, bid_flow_rate (orders/sec), ask_flow_rate (orders/sec), net_flow (bid - ask), flow_direction (enum), cumulative_delta (running buy - sell volume)

- **VolumeProfile**: Histogram of traded volume across price levels - includes symbol, histogram (array of VolumeBin), bin_size (adaptive tick-based), POC (Point of Control price), VAH/VAL (Value Area High/Low), total_volume, time_period

- **VolumeBin**: Single price level in volume profile - includes price_level (Decimal), volume (Decimal), trade_count (u64), percentage_of_total (f64)

- **MarketMicrostructureAnomaly**: Detected abnormal behavior - includes anomaly_id (UUID), anomaly_type (QuoteStuffing, IcebergOrder, FlashCrashRisk), severity (Low/Medium/High/Critical), confidence (0.0-1.0), timestamp, affected_price_levels, description, recommended_action

- **MicrostructureHealthScore**: Composite market health assessment - includes score (0-100), components (spread_stability, liquidity_depth, flow_balance, update_rate each 0-100), interpretation (Excellent/Good/Fair/Poor/Critical), timestamp, symbol

- **LiquidityVacuum**: Low-volume price zone - includes vacuum_id (UUID), symbol, price_range_low/high (Decimal), volume_deficit_pct (f64), severity (Medium/High/Critical), expected_impact (enum: FastMovement, ModerateMovement), detection_timestamp

- **AbsorptionEvent**: Large order absorbing pressure - includes event_id (UUID), symbol, price_level (Decimal), absorbed_volume (Decimal), refill_count (u32), suspected_entity (MarketMaker, Whale, Unknown), direction (Accumulation, Distribution), timestamp

- **StreamableHttpSession**: Active MCP client connection via Streamable HTTP - includes session_id (UUID from `Mcp-Session-Id` header), client_metadata (IP, user-agent), created_at, last_activity_timestamp, expires_at (30min timeout)

- **McpJsonRpcMessage**: JSON-RPC 2.0 formatted MCP message - includes jsonrpc ("2.0"), method (initialize/tools/list/tools/call), params (object), id (request identifier), result/error (response fields)

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: AI agents can identify order flow direction changes within 5 seconds of pressure shift occurring through natural language queries like "What's the BTCUSDT order flow?"

- **SC-002**: Volume profile calculations complete in under 500ms for 24-hour data window on major pairs (BTCUSDT, ETHUSDT) when requested via MCP tools

- **SC-003**: Quote stuffing detection achieves >95% precision (false positive rate <5%) based on statistical threshold validation

- **SC-004**: Iceberg order detection identifies suspected institutional orders with >95% confidence scores in real-time analysis

- **SC-005**: Flash crash risk alerts trigger when precursors detected (liquidity drain + spread widening + cancellation spike) providing early warning through health monitoring

- **SC-006**: Microstructure health score calculations complete in <200ms and correlate with market stability based on historical validation

- **SC-007**: System processes >1000 orderbook updates/second without dropping analytics calculations or degrading existing orderbook tool performance

- **SC-008**: Liquidity vacuum detection identifies price zones prone to rapid movement with actionable stop loss placement guidance

- **SC-009**: RocksDB time-series storage maintains <200ms query latency for historical snapshot retrieval supporting 60-300 second flow analysis windows

- **SC-010**: All 5 new analytics tools integrate seamlessly with existing 16 binance-rs tools in MCP gateway without breaking changes

- **SC-011**: Claude Code users can ask complex analytics questions like "Is BTCUSDT market healthy and where are the liquidity vacuums?" receiving comprehensive multi-tool analysis in natural language responses

- **SC-012**: Feature flag `orderbook_analytics` allows optional compilation - system builds and runs correctly with and without analytics features enabled

- **SC-013**: Streamable HTTP `/mcp` endpoint responds to ChatGPT initialize requests within 500ms under normal load (<10 concurrent sessions)

- **SC-014**: Provider maintains 100% tool compatibility between gRPC (Python gateway) and Streamable HTTP (direct AI client) transports - same tools, same behavior, same response formats

- **SC-015**: Provider handles at least 50 concurrent Streamable HTTP sessions without degradation in response time or accuracy

- **SC-016**: ChatGPT successfully connects to provider and executes all 21 tools (16 base + 5 analytics) without requiring custom adapters or middleware

- **SC-017**: Session management completes in <50ms (session creation, validation, cleanup) contributing minimal overhead to tool execution latency

## Assumptions *(mandatory)*

- Existing binance-rs provider in mcp-trader/providers/binance-rs follows similar Rust project structure as mcp-binance-rs, enabling direct code integration with minimal refactoring
- mcp-binance-rs analytics code (src/orderbook/analytics/) is production-ready and well-tested with 100% pass rate as documented in their README
- WebSocket orderbook stream from Binance provides <100ms update latency sufficient for order flow analysis granularity
- Aggregated trade stream (@aggTrade) from Binance delivers complete volume data for accurate profile generation with 60-80% data reduction vs raw trades
- Quote stuffing threshold (500 updates/sec, <10% fill rate) is calibrated for cryptocurrency markets based on mcp-binance-rs validation
- RocksDB embedded database is acceptable dependency for MCP gateway deployment (no external database required, embedded storage is production-ready)
- AI agents (Claude Code) can interpret complex analytics responses (flow direction enums, health scores, anomaly types) and provide natural language explanations to users
- 7-day retention for orderbook snapshots provides sufficient historical context for flow analysis with 1GB hard storage limit for 20 symbols (enforced via write failures when exceeded)
- Users deploying with `orderbook_analytics` feature understand performance implications (additional CPU for analytics calculations, disk I/O for RocksDB)
- Feature flags are independent: `http_transport` can be enabled without `orderbook_analytics` (exposing base 16 tools via HTTP), and `orderbook_analytics` can be enabled without `http_transport` (analytics via gRPC only)
- Existing MCP gateway in mcp-trader/mcp-gateway can handle 5 additional tools without architectural changes to gRPC client pooling
- Streamable HTTP transport from mcp-binance-rs is production-ready and tested with ChatGPT connectors
- March 2025 MCP Streamable HTTP specification is stable and won't require breaking changes
- ChatGPT and other MCP clients follow JSON-RPC 2.0 specification strictly (no proprietary extensions required)
- HTTP server deployment (HTTPS, secrets management) is handled by deployment platform of user's choice (Docker, Kubernetes, cloud platforms, etc.)
- 50 concurrent sessions is reasonable default limit for in-memory session storage

## Dependencies *(mandatory)*

- **Existing Feature 002 (Binance Provider Integration)**: Requires binance-rs provider with orderbook feature enabled, WebSocket infrastructure, OrderBook types, BinanceClient, and 16 existing tools
- **mcp-binance-rs Repository**: Source of analytics implementation to be integrated - includes complete src/orderbook/analytics/ module with flow.rs, profile.rs, anomaly.rs, health.rs, storage/, types.rs, tools.rs
- **Binance Trade Stream API**: Requires wss://stream.binance.com:9443/ws/<symbol>@aggTrade for aggregated trades (distinct from depth stream used by existing orderbook feature)
- **RocksDB Dependency**: Requires rocksdb = "0.23.0" crate for time-series snapshot storage (new dependency not in current binance-rs)
- **Statistical Analysis**: Requires statrs = "0.18.0" crate for percentile calculations, rolling averages, standard deviation in anomaly detection
- **Serialization Libraries**: Requires rmp-serde = "1.3.0" (MessagePack) and uuid = "1.11" (event IDs) as new dependencies
- **Protobuf Updates**: Requires adding 5 new tool definitions to pkg/proto/provider.proto and regenerating bindings for gateway integration
- **JSON Schemas**: Requires creating pkg/schemas/ files for 5 new tools (get_order_flow, get_volume_profile, detect_market_anomalies, get_microstructure_health, get_liquidity_vacuums)
- **Streamable HTTP Transport**: Requires src/transport/sse/ module from mcp-binance-rs with session management, JSON-RPC routing, and MCP protocol handlers
- **Axum HTTP Framework**: Requires axum = "0.8+" for Streamable HTTP transport endpoints (POST `/mcp`)
- **Session Storage**: Requires in-memory HashMap for session management (50 concurrent limit, 30min timeout, no persistence needed)

## Scope *(mandatory)*

### In Scope
- Integration of complete analytics module from mcp-binance-rs/src/orderbook/analytics/ into mcp-trader/providers/binance-rs/src/orderbook/analytics/
- 5 new MCP tools: get_order_flow, get_volume_profile, detect_market_anomalies, get_microstructure_health, get_liquidity_vacuums
- RocksDB time-series storage for 1-second orderbook snapshots with 7-day retention
- Binance aggregated trade stream connection for volume profile data
- Feature flag `orderbook_analytics` for optional compilation
- Gateway integration with proper tool routing and JSON schema validation
- Comprehensive error handling for analytics-specific failures
- Documentation updates (README.md, tool descriptions, usage examples)
- Integration of Streamable HTTP transport from mcp-binance-rs/src/transport/sse/ into mcp-trader/providers/binance-rs/src/transport/
- POST `/mcp` endpoint for JSON-RPC 2.0 MCP protocol (initialize, tools/list, tools/call methods)
- Session management with `Mcp-Session-Id` header (50 concurrent sessions, 30min timeout)
- Feature flag `http_transport` for optional Streamable HTTP compilation
- Dual-mode operation (gRPC for Python gateway OR Streamable HTTP for direct AI clients)
- ChatGPT connector compatibility testing and documentation
- Session cleanup automation and health monitoring endpoints
- Environment variable configuration for HTTP server (HOST, PORT, BINANCE_API_KEY, etc.)

### Out of Scope
- Historical backtesting engine (analytics are real-time only, no replay of past market conditions)
- Automated trading signals or execution logic (analytics provide data, no algorithmic trading automation)
- Cross-exchange analytics comparison (single exchange focus on Binance only)
- Machine learning-based anomaly detection (uses statistical thresholds, no ML models)
- Custom visualization or charting (returns raw JSON data, UI rendering is client responsibility via Claude Code)
- Futures-specific microstructure analysis (funding rates, basis, open interest - spot markets only)
- Integration with other providers (hello-go, hello-rs) - analytics specific to binance-rs only
- Performance optimization of existing orderbook feature (scope is additive features only)
- Migration of existing time-series data (fresh RocksDB instance, no historical data import)
- WebSocket transport protocol (only gRPC and Streamable HTTP supported)
- Persistent session storage across provider restarts (sessions are in-memory only)
- Load balancing across multiple provider instances (single-instance deployment)
- Custom authentication beyond Binance API credentials (session ID is only auth mechanism)
- Streaming SSE responses for long-running operations (Streamable HTTP uses request-response only)
- DELETE `/mcp` endpoint for explicit session termination (sessions expire via timeout)
- Migration of Python MCP gateway to Streamable HTTP (gateway remains gRPC-only, transport is provider-side feature)

## Non-Functional Requirements *(optional)*

### Performance
- Order flow calculations complete within 100ms of WebSocket update to maintain real-time responsiveness
- Volume profile generation handles 100,000+ trades without memory overflow or excessive CPU usage
- Anomaly detection processes 1000+ orderbook updates/second without dropping calculations or increasing latency
- RocksDB snapshot queries complete in <200ms for time-range scans supporting 60-300 second flow analysis windows
- Total end-to-end latency for analytics tools via MCP gateway <500ms (including gRPC overhead)

### Scalability
- Support concurrent analytics on up to 20 trading pairs (aligned with existing orderbook feature limit)
- RocksDB time-series storage retains 7 days at 1-second intervals (86,400 snapshots/day × 20 pairs = 12M snapshots, ~500MB-1GB with Zstd compression)
- Aggregated trade stream connections scale to 20 concurrent symbols with exponential backoff reconnection

### Reliability
- Gracefully handle WebSocket disconnections (both depth and aggTrade) without losing in-progress calculations
- Provide clear data staleness indicators when analytics calculated from outdated snapshots (>5 seconds old)
- RocksDB write failures trigger graceful degradation (continue orderbook functionality, disable analytics)
- Automatic background cleanup of RocksDB keys older than 7 days to prevent unbounded disk growth

### Security
- No new authentication requirements (reuses existing Binance API credentials from environment)
- RocksDB storage file permissions restricted to process owner only (0600)
- No sensitive data stored in RocksDB snapshots (only price/volume data, no account information)

## Constitution Compliance *(mandatory)*

- **Security-First**: ✅ No new authentication surface area - reuses existing Binance WebSocket connection security model
- **Auto-Generation Priority**: ✅ Integration work is primarily code adaptation, not generation - leveraging proven implementation from mcp-binance-rs
- **Modular Architecture**: ✅ Feature gated behind `orderbook_analytics` flag, fully optional deployment, no impact when disabled
- **Type Safety**: ✅ All analytics types (OrderFlowSnapshot, VolumeProfile, anomaly enums) use strong Rust typing with validation
- **MCP Protocol Compliance**: ✅ All 5 new tools follow existing JSON Schema patterns, protobuf definitions, and gateway routing conventions
- **Async-First Design**: ✅ All analytics calculations async/await, non-blocking on WebSocket threads, uses Tokio runtime
- **Machine-Optimized Development**: ✅ Specification follows /speckit.specify workflow with independently testable user stories and measurable success criteria
