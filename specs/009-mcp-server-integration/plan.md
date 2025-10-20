# Implementation Plan: MCP Server Integration

**Branch**: `009-mcp-server-integration` | **Date**: 2025-10-20 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/009-mcp-server-integration/spec.md`

## Summary

Integrate Model Context Protocol (MCP) server capabilities from mcp-binance-rs into the Binance provider. Enables AI agents to connect via stdio (local) or SSE (remote HTTPS), access data through MCP resources, and use guided analysis prompts. Implementation uses rmcp SDK 0.8.1 with procedural macros for routing, session management for SSE, and markdown-formatted responses.

## Technical Context

**Language/Version**: Rust 1.75 (Edition 2021)
**Primary Dependencies**: rmcp 0.8.1, axum 0.8.6, tokio 1.48, schemars 1.0.4
**Storage**: In-memory session management (no persistent storage)
**Testing**: cargo test with MCP protocol integration tests
**Target Platform**: Linux server (primary), macOS/Windows (stdio only)
**Project Type**: Single binary with multiple transport modes
**Performance Goals**: stdio <2s init, SSE 50 sessions <500ms P95, resources <200ms P95
**Constraints**: SSE 30s timeout, max 50 concurrent sessions, no session recovery
**Scale/Scope**: 21 MCP tools, 5 prompts, 6+ resources

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

✅ **I. Simplicity**: Macro-driven routing, standard patterns, no deep nesting
✅ **II. Library-First**: rmcp SDK, axum, tokio (all battle-tested)
✅ **III. Justified Abstractions**: SessionManager, ResourceUri, TransportMode all needed
✅ **IV. DRY**: Reuses existing tools, orderbook manager, error converters
⚠️ **V. Service/Repository**: JUSTIFIED - no persistence needed for ephemeral sessions
✅ **VI. 12-Factor**: All 12 factors compliant
✅ **VII. Minimal OOP**: Only BinanceServer and SessionManager structs (justified)

**Verdict**: ✅ PASS

## Project Structure

### Documentation (this feature)

```
specs/009-mcp-server-integration/
├── plan.md              # This file
├── research.md          # MCP protocol patterns from mcp-binance-rs
├── data-model.md        # MCP entities (Session, Resource, Prompt)
├── quickstart.md        # Setup guide for stdio and SSE modes
└── checklists/
    └── requirements.md  # Validation checklist (completed)
```

### Source Code (repository root)

```
providers/binance-rs/
├── src/
│   ├── mcp/                    # NEW: MCP server implementation
│   │   ├── mod.rs              # Module exports
│   │   ├── server.rs           # BinanceServer + ServerHandler
│   │   ├── handler.rs          # Tool and prompt handlers
│   │   ├── resources.rs        # Resource URI parsing
│   │   ├── prompts.rs          # Prompt definitions
│   │   └── types.rs            # Parameter types
│   ├── transport/              # NEW: Transport layer
│   │   ├── mod.rs              # Transport mode selection
│   │   ├── stdio.rs            # Stdio transport setup
│   │   └── sse/                # SSE transport implementation
│   │       ├── mod.rs
│   │       ├── server.rs       # HTTP server + heartbeat
│   │       ├── session.rs      # SessionManager
│   │       ├── handlers.rs     # HTTP endpoints
│   │       └── types.rs        # ConnectionId, SessionMetadata
│   ├── grpc/                   # EXISTING: gRPC provider
│   ├── orderbook/              # EXISTING: Reused
│   ├── binance/                # EXISTING: API client
│   ├── config/                 # EXISTING: Config management
│   ├── error.rs                # EXISTING: Extended for MCP
│   ├── lib.rs                  # Module exports
│   └── main.rs                 # Entry point (transport selection)
├── tests/
│   ├── mcp/                    # NEW: MCP protocol tests
│   │   ├── stdio_test.rs
│   │   ├── sse_test.rs
│   │   ├── resources_test.rs
│   │   └── prompts_test.rs
│   └── integration/            # EXISTING
└── Cargo.toml                  # Add rmcp, axum, schemars
```

**Structure Decision**: Single project with modular architecture. New `src/mcp/` and `src/transport/` modules contain MCP-specific logic. Existing modules (orderbook, binance client) reused without modification.

## Complexity Tracking

*No violations requiring justification.* The partial deviation from Service/Repository patterns is an appropriate design choice given lack of persistent storage.

