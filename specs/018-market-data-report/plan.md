# Implementation Plan: Unified Market Data Report

**Branch**: `018-market-data-report` | **Date**: 2025-10-23 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/018-market-data-report/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

This feature removes all order management functionality from the mcp-trader system and consolidates multiple market data retrieval methods into a single unified reporting method that generates comprehensive markdown-formatted market intelligence reports. The unified method (`generate_market_report()`) will aggregate data from existing sources (ticker, orderbook, analytics) and present it in 8 structured sections covering price, liquidity, anomalies, and health metrics. The implementation focuses on refactoring the existing Rust-based Binance provider and gRPC/MCP handlers to remove authentication-dependent trading methods while preserving market data streaming capabilities and the authentication infrastructure for future use.

## Technical Context

**Language/Version**: Rust 1.75+ (providers/binance-rs), Python 3.11+ (mcp-gateway)
**Primary Dependencies**:
- Rust: tokio (async runtime), reqwest (HTTP), tungstenite (WebSocket), serde (serialization), prost (gRPC)
- Python: mcp (Model Context Protocol), grpc, pydantic (validation)

**Storage**: In-memory caching (SnapshotStorage, OrderBookManager), Redis-compatible cache layer (existing)
**Testing**: cargo test (Rust unit/integration), pytest (Python), contract testing for gRPC interfaces
**Target Platform**: Linux server (Docker containers, Kubernetes deployment)
**Project Type**: Distributed microservices (Rust provider services + Python MCP gateway)
**Performance Goals**:
- Report generation: <5s first request, <3s cached requests
- Concurrent handling: 10+ simultaneous symbol requests
- WebSocket update latency: <200ms

**Constraints**:
- Must maintain backward compatibility with MCP protocol
- Authentication infrastructure must be preserved (not removed)
- Feature-gated analytics (orderbook_analytics) must degrade gracefully
- Breaking change: All order management endpoints will be removed

**Scale/Scope**:
- Code removal: ~500-800 lines across client.rs, grpc/tools.rs, mcp/handler.rs
- New code: ~400-600 lines for unified report generator and markdown formatter
- Affected services: binance-rs provider, mcp-gateway
- Active symbols: 50-100 trading pairs, 1000+ req/day expected

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Initial Constitution Review (Pre-Phase 0)

**I. Simplicity and Readability**: ✅ PASS
- The feature simplifies the API surface by removing 10 order management methods and consolidating 8+ market data methods into 1 unified method
- Markdown report generation uses clear, descriptive section builders
- No complex nested logic introduced - parallel data fetching with clear error handling

**II. Library-First Development**: ✅ PASS
- Leveraging existing Rust ecosystem libraries (tokio for async, serde for serialization)
- No custom implementations needed - all functionality uses existing battle-tested crates
- Markdown formatting can use existing `comrak` or simple string building

**III. Justified Abstractions**: ✅ PASS
- No new abstractions introduced - feature removes complexity
- Existing abstractions (BinanceClient, OrderBookManager) are reused
- Report generator is a concrete implementation, not an abstract interface

**IV. DRY Principle**: ✅ PASS
- Consolidating 8+ similar method calls into 1 unified method eliminates duplication in client code
- Markdown section builders will be reusable functions, not duplicated per section
- Error handling patterns will be extracted to shared utilities

**V. Service and Repository Patterns**: ✅ PASS
- Existing architecture already uses these patterns:
  - BinanceClient acts as repository for market data
  - OrderBookManager/SnapshotStorage provide data access layer
  - Report generator will be a service orchestrating these repositories
- No pattern violations introduced

**VI. 12-Factor Methodology**: ✅ PASS
- Configuration remains in environment variables (API endpoints, cache settings)
- Stateless report generation - no session state
- Logs to stdout/stderr (existing pattern maintained)
- No config stored in code
- Deployment model unchanged (Docker + Kubernetes)

**VII. Minimal Object-Oriented Programming**: ✅ PASS
- Rust implementation uses struct/impl pattern (necessary for the language)
- No inheritance hierarchies introduced
- Report generator is a simple struct with methods, not OOP abstraction layers
- Procedural approach for markdown formatting (functions, not classes)

### Gate Decision: ✅ PROCEED TO PHASE 0

All seven constitution principles are satisfied. This feature actually **reduces** complexity by removing code and consolidating interfaces. No violations requiring justification.

## Project Structure

### Documentation (this feature)

```
specs/018-market-data-report/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
│   └── market-report.proto  # gRPC service definition
├── checklists/
│   └── requirements.md  # Specification quality checklist (already created)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```
providers/binance-rs/
├── src/
│   ├── binance/
│   │   ├── client.rs            # [MODIFY] Remove order management methods
│   │   ├── types.rs             # [MODIFY] Remove Order, AccountInfo types (if unused)
│   │   └── auth.rs              # [PRESERVE] Keep authentication infrastructure
│   │
│   ├── grpc/
│   │   └── tools.rs             # [MODIFY] Remove order management tool handlers
│   │
│   ├── mcp/
│   │   └── handler.rs           # [MODIFY] Remove order management MCP tools
│   │
│   ├── orderbook/
│   │   ├── manager.rs           # [REUSE] Existing orderbook management
│   │   ├── tools.rs             # [REUSE] L1/L2 metrics tools
│   │   └── analytics/
│   │       └── tools.rs         # [REUSE] Anomaly detection, volume profile
│   │
│   └── report/                  # [NEW] Market report generator module
│       ├── mod.rs               # Module exports
│       ├── generator.rs         # Main report generation logic
│       ├── formatter.rs         # Markdown formatting utilities
│       └── sections.rs          # Individual section builders
│
└── tests/
    ├── integration/
    │   └── report_generation.rs # [NEW] End-to-end report tests
    └── unit/
        └── report/              # [NEW] Unit tests for report module
            ├── generator_test.rs
            └── formatter_test.rs

mcp-gateway/mcp_gateway/
├── tools/
│   ├── fetch.py                 # [MODIFY] Update to expose unified report
│   └── search.py                # [MINIMAL] Update document registry
│
└── adapters/
    └── grpc_client.py           # [MODIFY] Remove order management proxies
```

**Structure Decision**:

This is a **distributed microservices** architecture with Rust provider services and Python MCP gateway. The implementation will:

1. **Modify existing Rust modules**: Remove order management code from `binance/client.rs`, `grpc/tools.rs`, and `mcp/handler.rs`
2. **Add new Rust module**: Create `report/` module for unified market intelligence report generation
3. **Preserve authentication**: Keep `binance/auth.rs` intact for future authenticated read-only endpoints
4. **Reuse analytics**: Leverage existing `orderbook/` modules for advanced metrics
5. **Update Python gateway**: Modify MCP tool registration to expose new unified report method

## Complexity Tracking

*Fill ONLY if Constitution Check has violations that must be justified*

**No violations detected.** All constitution principles are satisfied. This section intentionally left empty.

