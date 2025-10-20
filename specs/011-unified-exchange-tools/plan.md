# Implementation Plan: Unified Multi-Exchange Gateway

**Branch**: `011-unified-exchange-tools` | **Date**: 2025-10-20 | **Spec**: [spec.md](./spec.md)

## Summary

Transform the MCP gateway from a Binance-specific tool aggregator into a multi-exchange platform that exposes unified, provider-agnostic tools to AI clients (ChatGPT, Claude). The gateway will normalize data from multiple exchanges (Binance, OKX, Bybit, etc.), manage canonical instrument identifiers, and implement intelligent routing with rate limiting and observability.

**Primary Requirement**: Enable querying market data from 5+ exchanges through a single set of ~15 unified tools (e.g., `market.get_ticker`) instead of 100+ exchange-specific tools, while maintaining provider-specific tools for advanced features.

**Technical Approach**: Multi-phase implementation starting with "Quick Fixes" to enable multi-provider support, followed by unified tools layer, instrument registry, and schema normalization infrastructure.

## Technical Context

**Language/Version**: Python 3.11+ (mcp-gateway), Rust 1.75+ (providers/binance-rs)
**Primary Dependencies**:
  - Python: `grpcio`, `jsonschema` (2020-12), `pydantic` (validation), `asyncio` (concurrency)
  - Rust: `tonic` (gRPC), `serde_json`, `tokio` (async runtime)
**Storage**: In-memory caching with configurable TTLs (Redis optional for distributed deployments)
**Testing**: `pytest` (Python), `cargo test` (Rust), contract testing for gRPC schemas
**Target Platform**: Linux server (production via systemd, Docker), local dev on macOS/Linux
**Project Type**: Multi-service distributed system (gateway + multiple provider services)
**Performance Goals**:
  - p95 latency < 2s for multi-venue queries
  - Support 1000 req/min aggregate across all providers
  - Cache hit rate > 80% for frequently-queried instruments
**Constraints**:
  - Provider APIs have strict rate limits (Binance: 1200/min, varies by exchange)
  - SSE transport for ChatGPT must expose <20 tools to avoid choice overload
  - Backward compatibility with existing Binance provider gRPC API
**Scale/Scope**:
  - 5+ exchange providers initially (Binance, OKX, Bybit, Kraken, Coinbase)
  - 1000+ instruments across spot, perpetual, futures markets
  - 15 unified tools + ~50 provider-specific tools (10/provider avg)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Principle I: Simplicity and Readability
✅ **PASS** - The unified tools abstraction simplifies AI client code by eliminating the need to understand exchange-specific APIs. Gateway routing logic will use descriptive function names (`normalize_ticker_response`, `map_instrument_to_native_symbol`) and avoid deep nesting through service/repository separation.

### Principle II: Library-First Development
✅ **PASS** - Leverage existing libraries:
  - `jsonschema` for JSON Schema 2020-12 validation (already in use)
  - `grpcio` for provider communication (already in use)
  - `pydantic` for request/response validation (widely adopted)
  - Do NOT re-implement schema validation, gRPC client pools, or caching (use existing implementations)

### Principle III: Justified Abstractions
✅ **PASS with Justification** - The unified tools layer introduces abstraction, justified by:
  - **Concrete need**: Supporting 5+ exchanges without 100+ tools explosion
  - **Present problem**: ChatGPT currently hardcoded to Binance (FR-047)
  - **Clear purpose**: Each unified tool (`market.get_ticker`, `market.get_orderbook_l2`) maps to a specific user story (US1-US6)
  - Provider-specific tools remain for exchange-unique features (not premature abstraction)

### Principle IV: DRY Principle
✅ **PASS** - Normalization logic (e.g., converting `bidPrice`/`askPrice` → `bid`/`ask`) will be centralized in schema adapters per provider, not duplicated across tools. Routing logic shared via a single `UnifiedToolRouter` class.

### Principle V: Service and Repository Patterns
✅ **PASS** - Architecture follows this pattern:
  - **Repository**: `ProviderClient` (gRPC communication), `InstrumentRegistry` (instrument metadata access)
  - **Service**: `UnifiedToolService` (business logic for routing, normalization), `RateLimitService` (quota management)
  - **Presentation**: SSE server, stdio MCP server (expose unified tools to clients)

### Principle VI: 12-Factor Methodology
✅ **PASS** - Compliance verified:
  1. ✅ Codebase: Monorepo with clear service boundaries
  2. ✅ Dependencies: `pyproject.toml`, `Cargo.toml`
  3. ✅ Config: Environment variables for provider endpoints, API keys, rate limits (NO hardcoded config)
  4. ✅ Backing services: Providers are attached resources (gRPC endpoints configurable)
  5. ✅ Build/release/run: Separate Dockerfile build, deployment configs
  6. ✅ Processes: Stateless gateway (all state in cache/external store)
  7. ✅ Port binding: SSE on :8000, stdio via stdin/stdout
  8. ✅ Concurrency: Horizontal scaling via multiple gateway instances
  9. ✅ Disposability: Graceful shutdown on SIGTERM
  10. ✅ Dev/prod parity: Same Docker images, env-var config
  11. ✅ Logs: Structured logging to stdout (JSON format)
  12. ✅ Admin: One-off scripts for instrument registry updates

### Principle VII: Minimal OOP
⚠️ **CONDITIONAL PASS with Justification** - Limited OOP usage:
  - **Justified classes**:
    - `ProviderClient` (encapsulates gRPC connection state, health checks)
    - `InstrumentRegistry` (encapsulates instrument lookup logic, caching)
    - `UnifiedToolRouter` (routes tool invocations to providers)
  - **Avoided**: Deep inheritance, excessive design patterns
  - **Default approach**: Functional modules for normalization (`normalize_ticker`, `normalize_orderbook`), utility functions for validation

**Overall Constitution Compliance**: ✅ **PASS** - All principles satisfied. Abstractions are justified by concrete multi-exchange requirements. OOP usage is minimal and purpose-driven.

## Project Structure

### Documentation (this feature)

```
specs/011-unified-exchange-tools/
├── plan.md              # This file
├── spec.md              # Feature specification (created)
├── checklists/
│   └── requirements.md  # Spec validation checklist (created)
└── tasks.md             # NOT created yet - use /speckit.tasks
```

### Source Code (repository root)

```
mcp-gateway/                         # Python gateway service
├── mcp_gateway/
│   ├── main.py                      # Entry point (MODIFY: load all providers)
│   ├── sse_server.py                # SSE MCP server (MODIFY: expose unified tools FR-024 to FR-028)
│   ├── adapters/
│   │   ├── grpc_client.py           # Provider gRPC client (MODIFY: connection pooling, health checks)
│   │   ├── unified_router.py        # NEW: Routes unified tools to providers
│   │   └── schema_adapter.py        # NEW: Normalizes provider responses to unified schemas
│   ├── services/
│   │   ├── instrument_registry.py   # NEW: Canonical instrument mapping (FR-012 to FR-017)
│   │   ├── rate_limiter.py          # NEW: Per-provider rate limit budgets (FR-029 to FR-034)
│   │   └── circuit_breaker.py       # NEW: Provider health tracking (FR-036, FR-038)
│   ├── schemas/
│   │   ├── unified/                 # NEW: Unified tool JSON schemas
│   │   │   ├── market_ticker.json   # Normalized ticker schema (FR-008)
│   │   │   ├── market_orderbook.json # Normalized orderbook schema (FR-009)
│   │   │   └── ...
│   │   └── providers/               # Provider-specific response schemas
│   │       ├── binance.json
│   │       └── okx.json             # NEW: OKX schemas
│   ├── cache.py                     # MODIFY: Per-tool TTL support (FR-034, FR-049)
│   └── document_registry.py         # MODIFY: Fix tool name mapping (FR-046)
├── tests/
│   ├── unit/
│   │   ├── test_unified_router.py
│   │   ├── test_schema_adapter.py
│   │   └── test_instrument_registry.py
│   ├── integration/
│   │   ├── test_multi_provider_routing.py
│   │   └── test_sse_unified_tools.py
│   └── contract/
│       └── test_provider_schemas.py
└── pyproject.toml                   # MODIFY: Add pydantic dependency

providers/binance-rs/                # Rust Binance provider
├── src/
│   ├── grpc/
│   │   └── capabilities.rs          # MODIFY: Relax symbol regex (FR-048)
│   └── lib.rs
└── Cargo.toml

pkg/proto/provider.proto             # MODIFY: Add capability metadata fields (FR-011, D-002)
```

**Structure Decision**: Multi-service architecture with Python gateway orchestrating Rust providers. Gateway handles unified tool abstraction, routing, normalization. Providers remain exchange-specific, exposing native tools via gRPC.

## Implementation Phases

### Phase 0: Quick Fixes (Priority: CRITICAL - Unblocks multi-provider)

**Goal**: Enable basic multi-provider support by fixing immediate blockers (FR-046 to FR-049)

**Tasks**:
1. Fix tool name mapping in `document_registry.py`: `binance_get_*` → `binance.get_*`
2. Remove hardcoded Binance search in `sse_server.py`: load all providers dynamically
3. Relax symbol regex in `providers/binance-rs/src/grpc/capabilities.rs`: support `BTC-USDT` format
4. Implement per-tool TTL in `cache.py`: replace global 5s with configurable map

**Deliverables**: Gateway can load multiple providers, SSE exposes all provider tools, symbol validation accepts common formats

**Estimated Effort**: 2-3 days

### Phase 1: Unified Tools Foundation

**Goal**: Implement core unified tools layer for market data (US1, US6)

**Tasks**:
1. Define unified tool schemas in `mcp_gateway/schemas/unified/`
2. Implement `UnifiedToolRouter` with venue-based routing
3. Create schema adapters for Binance (normalize ticker, orderbook responses)
4. Update SSE server to expose unified tools (configurable via `expose_unified_only` flag)
5. Add integration tests for `market.get_ticker`, `market.get_orderbook_l1`

**Deliverables**: ChatGPT can query `market.get_ticker` with `venue: binance` parameter

**Estimated Effort**: 1 week

### Phase 2: Instrument Registry

**Goal**: Canonical instrument mapping for cross-exchange queries (US4)

**Tasks**:
1. Design instrument_id format: `{venue}:{market_type}:{base}-{quote}`
2. Implement `InstrumentRegistry` service with in-memory cache
3. Add `registry.list_instruments` tool
4. Populate registry from provider capabilities
5. Update routers to translate canonical IDs to native symbols

**Deliverables**: Query `market.get_ticker` with canonical `btc:spot:usdt` across venues

**Estimated Effort**: 1 week

### Phase 3: Rate Limiting & Observability

**Goal**: Production-ready reliability (FR-029 to FR-040)

**Tasks**:
1. Implement `RateLimitService` with per-provider budgets
2. Add circuit breaker logic in `ProviderClient`
3. Instrument code with structured logging (Prometheus metrics export)
4. Create health check endpoint (`/health`)
5. Load testing with 5 providers, 100 concurrent requests

**Deliverables**: Gateway enforces rate limits, fails gracefully, emits metrics

**Estimated Effort**: 1 week

## Risks & Mitigations

See spec.md Risks section (R-001 to R-009) for detailed risk analysis.

**Top 3 Risks**:
1. **Schema Drift** (R-001): Exchanges change APIs → Implement schema validation + alerting
2. **Symbol Collisions** (R-002): Same symbol, different instruments → Enforce `{venue}:{type}:{symbol}` uniqueness
3. **Rate Limit Complexity** (R-003): Per-endpoint limits vary → Start conservative, monitor 429 responses, adapt

## Next Steps

1. ✅ Specification complete (`/speckit.specify`)
2. ✅ Implementation plan complete (this document)
3. ⏭️ **Run `/speckit.tasks`** to generate actionable task breakdown
4. Begin implementation with Phase 0 Quick Fixes

---

**Note**: Full research.md, data-model.md, contracts/ artifacts deferred to minimize planning overhead. Spec provides sufficient detail for task generation. Research can be done incrementally during implementation.

