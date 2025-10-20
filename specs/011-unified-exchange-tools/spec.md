# Feature Specification: Unified Multi-Exchange Gateway

**Feature Branch**: `011-unified-exchange-tools`
**Created**: 2025-10-20
**Status**: Draft
**Input**: User description: "Multi-exchange gateway with unified tools layer, instrument registry, normalized schemas, and provider-agnostic routing for scalable cross-exchange operations"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - AI Client Queries Market Data Across Exchanges (Priority: P1)

An AI client (ChatGPT, Claude, or other LLM) needs to retrieve ticker data for Bitcoin without knowing which exchange to query. The system automatically routes the request to available exchanges and returns normalized data in a consistent format.

**Why this priority**: This is the core value proposition - enabling exchange-agnostic queries. Without this, the unified tools layer provides no benefit over direct provider access.

**Independent Test**: Can be fully tested by querying a unified tool like `market.get_ticker` with parameters `{instrument: "BTC-USDT"}` and verifying the response contains normalized fields (mid, spread_bps, bid, ask, volume) regardless of which exchange handles the request.

**Acceptance Scenarios**:

1. **Given** ChatGPT is connected via SSE and Binance provider is available, **When** ChatGPT invokes `market.get_ticker` with `{venue: "binance", instrument: "BTCUSDT"}`, **Then** the gateway returns normalized ticker data with fields: mid, spread_bps, bid, ask, volume, timestamp
2. **Given** multiple providers (Binance, OKX) are registered, **When** AI client invokes `market.get_ticker` without specifying venue, **Then** the gateway routes to any available provider and returns normalized data
3. **Given** Binance returns native format with "bidPrice"/"askPrice", **When** gateway processes the response, **Then** output is normalized to standard "bid"/"ask" fields
4. **Given** AI client queries non-existent instrument, **When** `market.get_ticker` is invoked with invalid instrument, **Then** gateway returns structured error with available alternatives

---

### User Story 2 - Add New Exchange Provider Without Breaking Existing Clients (Priority: P1)

A developer adds support for OKX exchange by deploying a new OKX provider. Existing AI clients automatically gain access to OKX market data through the same unified tools, without any client-side changes or redeployment.

**Why this priority**: Scalability is a core requirement. If adding exchanges requires client updates, the architecture fails its primary goal of managing 5+ exchanges.

**Independent Test**: Deploy an OKX provider with native `okx.get_ticker` tool, verify that gateway automatically exposes `market.get_ticker` with `venue: "okx"` parameter, and existing SSE clients see the new venue option without reconnecting.

**Acceptance Scenarios**:

1. **Given** gateway is running with Binance provider only, **When** OKX provider is registered via gRPC, **Then** gateway capability list includes both `binance.get_ticker` and `okx.get_ticker` as provider-specific tools
2. **Given** OKX provider publishes capabilities with native tool names, **When** gateway processes them, **Then** unified tool `market.get_ticker` accepts `venue: "okx"` and routes correctly
3. **Given** OKX returns symbol format "BTC-USDT" (with hyphen), **When** gateway normalizes the instrument, **Then** internal instrument_id format is consistent (e.g., `okx:spot:BTC-USDT`)
4. **Given** ChatGPT connected before OKX was added, **When** capabilities refresh occurs, **Then** ChatGPT sees updated tool list including OKX without reconnecting

---

### User Story 3 - Query Orderbook Depth Across Multiple Exchanges (Priority: P2)

A trading analyst wants to compare liquidity across Binance and OKX by fetching L2 orderbook data for the same instrument from both exchanges simultaneously and receiving normalized depth metrics.

**Why this priority**: Cross-exchange analysis is a key use case enabled by unified tools. This validates that normalization works for complex data structures (orderbook levels), not just simple scalars.

**Independent Test**: Invoke `market.get_orderbook_l2` with `{instrument: "BTC-USDT", venues: ["binance", "okx"], depth: 10}` and verify response contains normalized bid/ask levels with price/quantity/cumulative fields for both venues.

**Acceptance Scenarios**:

1. **Given** both Binance and OKX providers are available, **When** `market.get_orderbook_l2` is invoked with multiple venues, **Then** gateway fans out requests in parallel and aggregates responses
2. **Given** Binance returns bids as `[price, qty]` arrays and OKX returns `{price, size}` objects, **When** gateway normalizes responses, **Then** both are transformed to consistent schema: `{price: float, quantity: float, cumulative: float}`
3. **Given** OKX request fails but Binance succeeds, **When** parallel fetch completes, **Then** response includes successful Binance data and structured error for OKX (partial success, not total failure)
4. **Given** orderbook data is fetched, **When** response is returned, **Then** each venue includes metadata: timestamp, latency_ms, exchange name

---

### User Story 4 - Resolve Instrument Symbols Across Exchange Formats (Priority: P2)

An AI client queries data for "Bitcoin perpetual futures" without knowing exchange-specific symbol formats (Binance: "BTCUSDT", OKX: "BTC-USDT-SWAP", Bybit: "BTCUSD"). The instrument registry resolves the canonical instrument to exchange-native symbols.

**Why this priority**: Symbol mapping is critical for multi-exchange support. Without it, clients must know each exchange's naming conventions, defeating the purpose of abstraction.

**Independent Test**: Query `market.get_ticker` with canonical `instrument_id: "btc:perp:usdt"` and verify gateway correctly maps to Binance "BTCUSDT", OKX "BTC-USDT-SWAP", etc., based on target venue.

**Acceptance Scenarios**:

1. **Given** instrument registry has mapping for BTC perpetual, **When** `market.get_ticker` is invoked with canonical `instrument_id: "btc:perp:usdt"` and `venue: "binance"`, **Then** gateway translates to native "BTCUSDT" before invoking Binance provider
2. **Given** same canonical instrument, **When** `venue: "okx"` is specified, **Then** gateway translates to "BTC-USDT-SWAP"
3. **Given** user provides exchange-specific symbol (e.g., "BTC-USDT"), **When** no venue is specified, **Then** gateway infers venue from symbol format or returns ambiguity error
4. **Given** instrument metadata includes tick_size and lot_size, **When** instrument is resolved, **Then** response includes venue-specific constraints (e.g., Binance min_qty: 0.001, OKX min_qty: 0.01)

---

### User Story 5 - Place Orders Through Unified Interface (Priority: P3)

A trading bot places a limit order to buy Bitcoin on the best available exchange. The gateway routes the order to the specified venue using normalized order parameters (side, quantity, price) and returns a standardized order confirmation.

**Why this priority**: Order execution validates that unified tools work for write operations, not just read-only market data. Lower priority because current focus is data retrieval; trading features can be added incrementally.

**Independent Test**: Invoke `order.place` with `{venue: "binance", instrument: "BTCUSDT", side: "buy", quantity: 0.01, price: 50000, order_type: "limit"}` and verify gateway translates to Binance native order API format, submits, and returns normalized order confirmation with order_id, status, filled_quantity.

**Acceptance Scenarios**:

1. **Given** authenticated session for Binance account, **When** `order.place` is invoked with normalized parameters, **Then** gateway translates to Binance REST API format and submits order
2. **Given** order is successfully placed, **When** response is received, **Then** gateway returns normalized confirmation: `{order_id, venue, instrument, status, filled_quantity, average_price, timestamp}`
3. **Given** venue returns exchange-specific error (e.g., "insufficient balance"), **When** error occurs, **Then** gateway normalizes to standard error code (e.g., `INSUFFICIENT_FUNDS`) with original message in metadata
4. **Given** unified tool `order.place` is invoked, **When** user lacks authentication for specified venue, **Then** gateway returns `AUTHENTICATION_REQUIRED` error before attempting order placement

---

### User Story 6 - SSE Gateway Exposes Only Unified Tools to ChatGPT (Priority: P1)

ChatGPT connects to the SSE gateway and receives a curated list of 10-20 unified tools (market.*, analytics.*) instead of 100+ provider-specific tools. Provider-specific tools are hidden unless explicitly enabled in configuration.

**Why this priority**: This directly addresses the "tool explosion" problem. Without filtering, ChatGPT receives too many tools and makes poor choices. This is essential for usability.

**Independent Test**: Connect ChatGPT via SSE and verify tools list contains only unified tools (e.g., `market.get_ticker`, `market.get_klines`) plus explicitly whitelisted provider tools (if configured). Verify Binance-specific tools like `binance.get_funding_rate` are NOT exposed by default.

**Acceptance Scenarios**:

1. **Given** SSE gateway is configured with `expose_unified_only: true`, **When** ChatGPT connects and requests capabilities, **Then** only unified tools are returned (no `binance.*` or `okx.*` tools)
2. **Given** configuration includes `expose_provider_tools: ["binance.get_funding_rate"]`, **When** capabilities are requested, **Then** both unified tools AND whitelisted provider tools are exposed
3. **Given** 5 providers are registered (Binance, OKX, Bybit, Kraken, Coinbase), **When** ChatGPT sees tool list, **Then** total tools count is ~15 unified tools, not 100+ (20 tools × 5 providers)
4. **Given** unified tool `market.get_ticker` requires `venue` parameter, **When** ChatGPT inspects tool schema, **Then** venue parameter has enum constraint listing available venues: ["binance", "okx", "bybit", ...]

---

### Edge Cases

- **What happens when a provider goes offline during a request?**
  Gateway should detect provider unavailability via gRPC health checks, return structured error for that venue, and allow fallback to other providers if configured. Partial failures should not block other venue responses in multi-venue queries.

- **How does the system handle symbol format collisions?**
  For example, if Binance uses "BTC-USD" (spot) and OKX uses "BTC-USD" (perpetual), the canonical instrument_id must include market type: `binance:spot:BTC-USD` vs `okx:perp:BTC-USD`. Gateway enforces uniqueness via `{venue}:{market_type}:{symbol}` format.

- **What if provider returns data in unexpected format (schema breaking change)?**
  Gateway validation layer checks response against expected schema for that provider version. If validation fails, gateway logs error with provider details, returns structured error to client, and optionally marks provider as unhealthy until manual intervention.

- **How to handle rate limiting across providers?**
  Gateway implements per-provider rate limit budgets (e.g., Binance: 1200 req/min, OKX: 600 req/min). When limit is approached, gateway queues requests or returns `RATE_LIMIT_EXCEEDED` error instead of forwarding to provider. This prevents provider API bans.

- **What happens when multiple providers support the same instrument but return different prices?**
  For read queries, gateway returns data from requested venue(s). If no venue is specified and multiple are available, gateway uses default routing strategy (e.g., lowest latency, round-robin, or explicit priority list in config). Price discrepancies are expected; aggregation/arbitrage detection is a separate analytics concern.

- **How does gateway handle large orderbook data (1000+ levels)?**
  Unified tool `market.get_orderbook_l2` includes `depth` parameter (default: 10, max: 100). Gateway validates depth limits and truncates responses if provider returns more levels. For deep orderbooks, clients should use provider-specific tools or streaming subscriptions (future feature).

- **What if venue parameter is omitted for a tool that requires it?**
  Gateway checks if parameter is required in unified tool schema. If venue is required but missing, gateway returns validation error: `MISSING_REQUIRED_PARAMETER: venue`. If venue is optional, gateway selects default venue from configuration or returns error if no default is configured.

- **How to handle instruments with no cross-exchange equivalent?**
  Provider-specific instruments (e.g., Binance leveraged tokens, Polymarket prediction markets) are NOT mapped to canonical instrument_ids. These remain accessible only via provider-specific tools (e.g., `binance.get_leveraged_token_info`). Canonical registry only includes instruments with cross-exchange equivalents.

## Requirements *(mandatory)*

### Functional Requirements

#### Unified Tools Layer

- **FR-001**: Gateway MUST expose a set of 10-15 unified tools covering core market data operations: `market.get_ticker`, `market.get_orderbook_l1`, `market.get_orderbook_l2`, `market.get_klines`, `market.get_trades`, `analytics.get_volume_profile`, `analytics.get_market_anomalies`, `analytics.get_liquidity_vacuums`
- **FR-002**: Each unified tool MUST accept `venue` parameter (string, enum of registered venues) and `instrument` parameter (string, canonical instrument_id or exchange-specific symbol)
- **FR-003**: Unified tools MUST route requests to the appropriate provider's native tool based on `venue` parameter and instrument mapping
- **FR-004**: Gateway MUST preserve provider-specific namespaced tools (e.g., `binance.get_funding_rate`, `okx.get_option_chain`) for features not covered by unified tools
- **FR-005**: Provider-specific tools MUST NOT be exposed to SSE clients by default unless explicitly whitelisted in configuration

#### Schema Normalization

- **FR-006**: All unified tools MUST return responses conforming to standardized JSON schemas defined in a schema registry
- **FR-007**: Gateway MUST transform provider native responses to unified schema format, including field renaming (e.g., `bidPrice` → `bid`, `askPrice` → `ask`)
- **FR-008**: Normalized ticker schema MUST include mandatory fields: `mid` (float), `spread_bps` (float), `bid` (float), `ask` (float), `volume` (float), `timestamp` (ISO 8601 string)
- **FR-009**: Normalized orderbook schema MUST include: `bids` (array of `{price, quantity, cumulative}`), `asks` (same structure), `venue` (string), `instrument` (string), `timestamp`, `latency_ms` (integer)
- **FR-010**: Schema definitions MUST support versioning (e.g., `market.get_ticker.v1`) to allow breaking changes without disrupting existing clients
- **FR-011**: Provider capability schemas MUST include metadata fields: `tags` (array of strings), `auth_required` (boolean), `stability` (enum: stable/beta/deprecated), `rate_limit_group` (string)

#### Instrument Registry

- **FR-012**: Gateway MUST maintain an Instrument Registry mapping canonical instrument identifiers to exchange-specific symbols
- **FR-013**: Canonical instrument_id format MUST be `{venue}:{market_type}:{base}-{quote}` (e.g., `binance:spot:BTC-USDT`, `okx:perp:ETH-USDT`)
- **FR-014**: Instrument Registry MUST store metadata for each instrument: `tick_size`, `lot_size`, `min_notional`, `contract_size` (for derivatives), `settlement_currency`, `expiry` (for futures/options), `option_type` (call/put), `strike_price`
- **FR-015**: Gateway MUST support reverse lookup: given exchange-specific symbol (e.g., "BTCUSDT"), resolve to canonical instrument_id(s)
- **FR-016**: Instrument Registry MUST be dynamically populated by querying provider capabilities or loading from cache/database (not hardcoded lists of symbols)
- **FR-017**: Gateway MUST support querying available instruments via dedicated tool: `registry.list_instruments` with filters: `{venue, market_type, base_currency, quote_currency}`

#### Routing and Aliases

- **FR-018**: Gateway MUST implement a routing layer that maps unified tool names to provider native tools based on venue and instrument
- **FR-019**: Routing configuration MUST support alias definitions: `market.get_ticker` → `{provider}.{native_tool}` (e.g., `market.get_ticker` with `venue: binance` → `binance.get_ticker`)
- **FR-020**: Gateway MUST support multi-venue fan-out: when `venues` parameter is an array, gateway executes requests to all specified venues in parallel
- **FR-021**: For multi-venue requests, gateway MUST aggregate results and return array of responses with per-venue status (success/error) and data
- **FR-022**: Gateway MUST implement fallback routing: if primary venue fails, retry with secondary venue (if configured in routing rules)
- **FR-023**: Routing rules MUST be configurable per-tool (e.g., ticker data can fallback to backup venue, but order placement cannot)

#### SSE Gateway for ChatGPT/AI Clients

- **FR-024**: SSE gateway MUST load all registered providers at startup (not hardcoded to Binance)
- **FR-025**: SSE gateway MUST construct unified tools list from all provider capabilities and expose via MCP capabilities endpoint
- **FR-026**: SSE gateway MUST support configuration flag `expose_unified_only` (boolean, default: true) to hide provider-specific tools from AI clients
- **FR-027**: SSE gateway MUST support configuration option `expose_provider_tools` (array of tool name patterns) to selectively expose provider-specific tools (e.g., `["binance.get_funding_rate"]`)
- **FR-028**: When AI client invokes unified tool, SSE gateway MUST route through unified routing layer (not direct provider invocation)

#### Performance and Rate Limiting

- **FR-029**: Gateway MUST implement per-provider rate limit budgets configurable via config file (e.g., `binance_rate_limit: 1200/min`)
- **FR-030**: Gateway MUST implement per-category rate limits: separate budgets for `market_data`, `account_data`, and `orders` categories
- **FR-031**: When rate limit is exceeded, gateway MUST return structured error `RATE_LIMIT_EXCEEDED` with retry-after timestamp
- **FR-032**: Gateway MUST implement request queuing with configurable queue depth (default: 100 requests per provider)
- **FR-033**: Gateway MUST support batch RPC operations via new proto message `InvokeBatch` accepting array of tool invocations and returning array of results
- **FR-034**: Gateway caching layer MUST support per-tool TTL configuration (e.g., ticker: 1s, orderbook: 0.5s, instrument metadata: 5min)

#### Observability and Reliability

- **FR-035**: Gateway MUST emit metrics for each provider: `request_count`, `error_count`, `latency_p50_ms`, `latency_p99_ms`, `rate_limit_hits`
- **FR-036**: Gateway MUST implement circuit breaker per provider: after N consecutive failures, mark provider unhealthy and skip routing for cooldown period
- **FR-037**: Gateway MUST log all provider errors with structured context: `{provider, tool, request_id, error_code, error_message, timestamp}`
- **FR-038**: Gateway MUST implement health check endpoint exposing per-provider status: `{provider, healthy, last_success, last_error, consecutive_failures}`
- **FR-039**: Gateway MUST periodically refresh provider capabilities (configurable interval, default: 60s) to detect new tools or schema changes
- **FR-040**: Capability refresh MUST be hot-reloadable: updated capabilities take effect without restarting gateway

#### Security and Context

- **FR-041**: Gateway MUST classify tools as `public` (market data) or `private` (account/orders) based on `auth_required` flag in capability schema
- **FR-042**: SSE gateway MUST NOT expose private tools to AI clients unless configuration flag `expose_private_tools: true` is explicitly set
- **FR-043**: When private tool is invoked, gateway MUST validate authentication credentials (API key, session token) before forwarding to provider
- **FR-044**: Gateway MUST support per-venue authentication: different API keys for each provider stored in secure configuration (env vars, secrets manager)
- **FR-045**: Gateway MUST sanitize error messages returned to AI clients: remove sensitive details (API keys, internal IPs, stack traces) while preserving actionable error information

#### Quick Fixes (Critical for Next Phase)

- **FR-046**: Document registry `DOC_TYPE_TO_TOOL` mapping MUST use consistent naming: change `binance_get_*` → `binance.get_*` to match capability tool names
- **FR-047**: SSE server MUST remove hardcoded Binance provider search and instead load all available providers dynamically
- **FR-048**: Provider symbol regex validation MUST be relaxed to accept multiple formats: `^[A-Z0-9]+$` (Binance "BTCUSDT"), `^[A-Z0-9]+-[A-Z0-9]+$` (OKX "BTC-USDT"), `^[A-Z0-9]+-[A-Z0-9]+-[A-Z]+$` (OKX "BTC-USDT-SWAP")
- **FR-049**: Gateway cache layer MUST support per-tool TTL overrides (not single global TTL): ticker 1s, orderbook 0.5s, klines 5s, instrument_metadata 5min

### Key Entities

- **Unified Tool**: Represents a provider-agnostic operation (e.g., `market.get_ticker`). Attributes: name, input schema (JSON Schema), output schema, required parameters (venue, instrument), optional parameters, supported venues.

- **Provider Tool**: Represents a native tool exposed by a specific provider (e.g., `binance.get_ticker`). Attributes: provider_id, tool_name, input schema, output schema, metadata (tags, auth_required, stability, rate_limit_group).

- **Canonical Instrument**: Represents a trading instrument in a normalized format. Attributes: instrument_id (`{venue}:{market_type}:{base}-{quote}`), base_currency, quote_currency, market_type (spot/perp/future/option), venue, metadata (tick_size, lot_size, contract_size, settlement, expiry, option_type).

- **Exchange Symbol Mapping**: Maps canonical instrument_ids to exchange-specific symbols. Attributes: instrument_id, venue, native_symbol, is_active (boolean).

- **Provider**: Represents a registered exchange provider. Attributes: provider_id, name, gRPC endpoint, health_status (healthy/unhealthy), last_health_check, capabilities (array of Provider Tools), rate_limit_budget, authentication_config.

- **Routing Rule**: Defines how unified tools map to provider tools. Attributes: unified_tool_name, target_provider_tool, venue_filter, fallback_provider_tool, enable_multi_venue (boolean).

- **Schema Version**: Tracks versions of input/output schemas for unified tools. Attributes: tool_name, version (e.g., "v1"), schema_definition (JSON Schema), deprecated (boolean), migration_notes.

- **Rate Limit Budget**: Tracks API quota usage per provider. Attributes: provider_id, category (market_data/account/orders), requests_per_minute, current_usage, reset_timestamp.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: AI clients can query market data across 5+ exchanges using a single unified tool name, without knowing exchange-specific APIs or symbol formats
- **SC-002**: Adding a new exchange provider requires zero changes to existing AI client integrations (ChatGPT, Claude, etc.)
- **SC-003**: Gateway normalizes responses from 100% of supported exchanges to consistent schema, enabling cross-exchange data comparison without client-side transformations
- **SC-004**: SSE-connected ChatGPT sees a tool list of 15-20 unified tools instead of 100+ provider-specific tools when 5 exchanges are registered
- **SC-005**: Multi-venue queries (e.g., fetch ticker from Binance and OKX simultaneously) return aggregated results in under 2 seconds at p95 latency
- **SC-006**: Gateway correctly maps canonical instrument identifiers to exchange-specific symbols for spot, perpetual, futures, and options markets across all supported venues
- **SC-007**: Gateway enforces per-provider rate limits with 99% accuracy, preventing API quota exhaustion or provider bans
- **SC-008**: System handles provider failures gracefully: when one exchange is down, queries to other exchanges succeed with partial success status
- **SC-009**: Gateway emits structured metrics enabling operators to identify performance bottlenecks per provider within 30 seconds of anomaly detection
- **SC-010**: Private tools (order placement, account queries) are never exposed to AI clients unless explicitly enabled in configuration

## Assumptions

- **A-001**: All provider gRPC services implement the standard Provider API contract (`ListCapabilities`, `Invoke`, `ReadResource`) defined in `pkg/proto/provider.proto`
- **A-002**: Exchange providers return data in a consistent format per provider (i.e., Binance always uses same field names for ticker data), though formats differ across providers
- **A-003**: Instrument metadata (tick_size, lot_size, etc.) can be fetched from exchange REST APIs or included in provider capabilities; gateway does not need to scrape web pages
- **A-004**: AI clients (ChatGPT, Claude) support MCP protocol over SSE transport and can handle JSON Schema tool definitions
- **A-005**: Gateway has sufficient memory to cache instrument registry for 1000+ instruments across 5+ exchanges (estimated ~10MB)
- **A-006**: Network latency between gateway and provider gRPC services is under 50ms p95 (providers are colocated or on low-latency network)
- **A-007**: Exchange API rate limits are documented and stable; gateway can rely on published limits (e.g., Binance 1200 req/min) without dynamic adjustment
- **A-008**: Canonical instrument_id format `{venue}:{market_type}:{base}-{quote}` is sufficient for spot, perpetual, and futures. Options may require extended format like `{venue}:option:{underlying}-{strike}-{expiry}-{type}` (documented separately if needed)
- **A-009**: Provider-specific tools that are not mapped to unified tools remain accessible for advanced use cases; unified tools cover 80% of common operations
- **A-010**: Authentication credentials (API keys) for private tools are stored securely in environment variables or secrets manager, not in gateway code or config files committed to version control

## Out of Scope

The following are explicitly excluded from this feature:

- **WebSocket streaming subscriptions**: Real-time orderbook/trade streams are not included in this phase. Unified tools are request-response only.
- **Cross-exchange order routing/smart order routing**: Gateway does not automatically split orders across venues or optimize execution. Order placement via `order.place` is single-venue only.
- **Historical data storage/backtesting**: Gateway does not persist market data or provide historical query APIs. Clients must handle their own data storage.
- **Arbitrage detection/alerting**: While analytics tools like `get_volume_profile` are included, the gateway does not implement trading logic or generate trade signals.
- **User authentication/authorization beyond API keys**: Gateway assumes AI clients are trusted or authenticated at the SSE transport layer. Fine-grained user permissions (e.g., user A can only access Binance, user B can access all exchanges) are not implemented.
- **Exchange-specific advanced features**: Leveraged tokens (Binance), dual investment products, liquidity mining, staking, etc., are accessible only via provider-specific tools, not unified tools.
- **Frontend UI for monitoring/configuration**: Gateway exposes metrics and health endpoints, but does not include a web dashboard. Operators use external tools (Grafana, Prometheus) for observability.

## Dependencies

- **D-001**: Existing gRPC Provider API contract (`pkg/proto/provider.proto`) must remain stable or support backward-compatible extensions for new metadata fields
- **D-002**: Provider implementations (binance-rs, future okx/bybit providers) must implement capability metadata fields: `tags`, `auth_required`, `stability`, `rate_limit_group`
- **D-003**: JSON Schema 2020-12 validation library must support schema versioning and migration (current gateway uses `jsonschema` library)
- **D-004**: SSE MCP server must support dynamic tool registration (capability list can change without restart)
- **D-005**: Gateway must have access to exchange REST APIs for instrument metadata (as fallback if not provided by providers)
- **D-006**: Deployment environment must support environment variables or secrets manager for storing per-venue API keys
- **D-007**: Monitoring stack (Prometheus/Grafana or equivalent) must be available to consume gateway metrics
- **D-008**: gRPC channel pool implementation must support health checks and circuit breaker patterns (may require upgrading `grpc_client.py` library)

## Risks and Mitigations

### High-Impact Risks

- **R-001: Schema Drift**: Exchange providers change API response formats without notice, breaking normalization.
  **Mitigation**: Implement schema validation on provider responses; log warnings when validation fails; cache last-known-good schema version; add provider version tracking to capabilities.

- **R-002: Instrument Symbol Collisions**: Different exchanges use same symbol for different instruments (e.g., "BTC-USD" spot vs perpetual).
  **Mitigation**: Canonical instrument_id includes venue and market_type prefix; enforce uniqueness constraints in registry; validate instrument metadata consistency.

- **R-003: Rate Limit Complexity**: Each exchange has different rate limit rules (per-IP, per-API-key, per-endpoint), making unified budgeting impractical.
  **Mitigation**: Start with conservative global limits per provider; implement per-category limits (market_data, account, orders); add adaptive rate limiting in future phase based on 429 response monitoring.

- **R-004: Performance Overhead**: Normalization and routing add latency, making gateway slower than direct provider access.
  **Mitigation**: Implement aggressive caching with per-tool TTLs; use connection pooling; profile and optimize hot paths; provide direct provider tools as escape hatch for latency-sensitive clients.

### Medium-Impact Risks

- **R-005: Provider Unavailability**: Exchange downtime or API issues cause unified tools to fail.
  **Mitigation**: Implement circuit breakers; support fallback routing to backup venues; return partial success for multi-venue queries; expose per-provider health status.

- **R-006: Config Complexity**: Managing routing rules, rate limits, and whitelists across 5+ exchanges becomes operationally burdensome.
  **Mitigation**: Provide sensible defaults; use config templates per exchange; implement config validation at gateway startup; document config schema thoroughly.

- **R-007: Versioning Challenges**: Updating unified tool schemas breaks existing clients.
  **Mitigation**: Use semantic versioning for tools (e.g., `market.get_ticker.v1`, `v2`); support multiple versions simultaneously; provide migration guides; deprecate old versions gradually.

### Low-Impact Risks

- **R-008: Documentation Drift**: Unified tool documentation falls out of sync with implementation.
  **Mitigation**: Generate documentation from JSON schemas; include schema examples in API docs; automate schema validation in CI/CD.

- **R-009: Test Coverage Gaps**: Testing all combinations of providers × tools × error scenarios is prohibitively expensive.
  **Mitigation**: Focus integration tests on critical paths (P1/P2 user stories); use contract testing for provider schemas; implement chaos engineering tests for failure scenarios.

## Notes

- This specification focuses on read-only market data operations (tickers, orderbooks, klines, analytics) as the primary use case. Order execution (`order.place`, `order.cancel`) is included as P3 priority but requires additional design work around error handling, order state tracking, and authentication.

- The canonical instrument_id format `{venue}:{market_type}:{base}-{quote}` is suitable for spot and perpetual markets. Futures and options may require extended formats to include expiry, strike, and option type. This can be addressed during implementation planning.

- The specification intentionally does not prescribe specific technologies (databases for caching, metrics backends, etc.) to allow flexibility in implementation. These decisions should be made during the planning phase based on existing infrastructure.

- Schema normalization rules (field mappings, unit conversions) should be documented separately in a Schema Normalization Guide. This spec defines the requirement for normalization but not the exhaustive field-by-field mappings for every provider.

- The "quick fixes" (FR-046 to FR-049) address immediate issues preventing multi-provider support. These should be implemented first before tackling the broader unified tools architecture.

