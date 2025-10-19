# Tasks: MCP Gateway System with Provider Orchestration

**Input**: Design documents from `/specs/001-specify-scripts-bash/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: Tests are NOT explicitly requested in the specification. This tasks list focuses on implementation and manual testing with MCP Inspector.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`
- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions
- **Gateway**: `mcp-gateway/mcp_gateway/`
- **Go Provider**: `providers/hello-go/`
- **Rust Provider**: `providers/hello-rs/`
- **Shared**: `pkg/proto/`, `pkg/schemas/`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure

- [ ] T001 Create directory structure: mcp-gateway/, providers/hello-go/, providers/hello-rs/, pkg/proto/, pkg/schemas/, infra/
- [ ] T002 Copy provider.proto from specs/001-specify-scripts-bash/contracts/ to pkg/proto/provider.proto
- [ ] T003 [P] Copy JSON schemas from specs/001-specify-scripts-bash/contracts/schemas/ to pkg/schemas/
- [ ] T004 [P] Initialize Python project in mcp-gateway/ with uv and pyproject.toml (dependencies: mcp, grpcio, grpcio-tools, jsonschema, pyyaml)
- [ ] T005 [P] Initialize Go module in providers/hello-go/ with go.mod (dependencies: google.golang.org/grpc, google.golang.org/protobuf)
- [ ] T006 [P] Initialize Rust project in providers/hello-rs/ with Cargo.toml (dependencies: tonic 0.9, prost 0.11, tokio 1.28)
- [ ] T007 [P] Create build.rs in providers/hello-rs/ for Tonic protobuf codegen
- [ ] T008 [P] Create Makefile with targets: run:gateway, run:hello-go, run:hello-rs, proto:gen, test
- [ ] T009 [P] Create infra/docker-compose.yml for NATS JetStream (optional)
- [ ] T010 Create providers.yaml template in mcp-gateway/ with hello-go and hello-rs configuration

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [ ] T011 Generate Python gRPC code from pkg/proto/provider.proto to mcp-gateway/mcp_gateway/generated/
- [ ] T012 [P] Generate Go gRPC code from pkg/proto/provider.proto to providers/hello-go/internal/pb/
- [ ] T013 [P] Generate Rust gRPC code via build.rs (auto-generated during cargo build)
- [ ] T014 Create mcp_gateway/validation.py module with SchemaValidator class for JSON Schema Draft 2020-12 validation
- [ ] T015 [P] Create mcp_gateway/providers_registry.py module with ProviderRegistry class for provider management
- [ ] T016 [P] Create mcp_gateway/adapters/grpc_client.py module with ProviderGRPCClient class (connection pooling: 15 channels per provider)
- [ ] T017 Create mcp_gateway/adapters/__init__.py empty file
- [ ] T018 Create mcp_gateway/__init__.py empty file

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - AI Client Tool Discovery (Priority: P1) 🎯 MVP

**Goal**: Enable AI clients to discover and invoke tools from at least one provider (hello-go) through the gateway

**Independent Test**: Connect MCP Inspector to gateway, verify "hello-go.echo.v1" and "hello-go.sum.v1" appear in tools list, invoke both successfully

### Implementation for User Story 1

#### Go Provider (hello-go) - Tool Implementation

- [ ] T019 [P] [US1] Create providers/hello-go/internal/tools/echo.go with Echo tool implementation (returns input message unchanged)
- [ ] T020 [P] [US1] Create providers/hello-go/internal/tools/sum.go with Sum tool implementation (calculates sum of number array)
- [ ] T021 [P] [US1] Create providers/hello-go/internal/capabilities/capabilities.go to return Capabilities with echo.v1 and sum.v1 tools
- [ ] T022 [US1] Create providers/hello-go/internal/server/server.go implementing Provider gRPC service (ListCapabilities, Invoke RPCs)
- [ ] T023 [US1] Create providers/hello-go/cmd/server/main.go as entry point (listen on :50051, register gRPC server)
- [ ] T024 [US1] Load echo.input.schema.json and sum.input.schema.json from pkg/schemas/ in capabilities.go

#### Gateway - Provider Discovery and Tool Proxying

- [ ] T025 [US1] Implement ProviderRegistry.load_providers() in providers_registry.py to read providers.yaml
- [ ] T026 [US1] Implement ProviderGRPCClient.list_capabilities() in adapters/grpc_client.py with 2.5s timeout
- [ ] T027 [US1] Implement ProviderRegistry.discover_capabilities() to call ListCapabilities on all providers and cache results
- [ ] T028 [US1] Implement SchemaValidator.validate() in validation.py using jsonschema.Draft202012Validator with validator caching
- [ ] T029 [US1] Create mcp_gateway/main.py with FastMCP server setup and stdio transport initialization
- [ ] T030 [US1] Implement @mcp.list_tools() handler in main.py to aggregate tools from all providers with provider name prefixes
- [ ] T031 [US1] Implement @mcp.call_tool() handler in main.py to route requests to providers via ProviderGRPCClient.invoke()
- [ ] T032 [US1] Add input payload validation in call_tool handler before forwarding to provider
- [ ] T033 [US1] Add output response validation in call_tool handler after receiving from provider (reject on schema mismatch per FR-011)
- [ ] T034 [US1] Add correlation ID generation and propagation in gRPC metadata for distributed tracing
- [ ] T035 [US1] Add structured logging with correlation IDs in main.py (log to stderr)
- [ ] T036 [US1] Implement 10MB payload size limit check in call_tool handler (reject oversized requests)
- [ ] T037 [US1] Add error handling for provider unavailability (fail-fast with clear error message)

**Checkpoint**: At this point, User Story 1 should be fully functional - gateway can discover and invoke tools from hello-go provider

---

## Phase 4: User Story 2 - Resource Access Through Gateway (Priority: P2)

**Goal**: Enable AI clients to read resources from providers using URI schemes

**Independent Test**: Request "hello://greeting" through MCP Inspector and receive "Hello, MCP" response

### Implementation for User Story 2

#### Go Provider (hello-go) - Resource Implementation

- [ ] T038 [P] [US2] Create providers/hello-go/internal/resources/greeting.go with ReadResource handler for "hello://greeting" URI
- [ ] T039 [US2] Update capabilities.go to include Resource with uri_scheme="hello" in Capabilities response
- [ ] T040 [US2] Implement ReadResource RPC method in server.go to route URI requests to appropriate resource handlers

#### Gateway - Resource Proxying

- [ ] T041 [US2] Implement @mcp.list_resources() handler in main.py to aggregate resources from all providers
- [ ] T042 [US2] Implement @mcp.read_resource() handler in main.py to route URI requests to providers via ProviderGRPCClient.read_resource()
- [ ] T043 [US2] Add ProviderGRPCClient.read_resource() method in adapters/grpc_client.py with 2.5s timeout
- [ ] T044 [US2] Add resource not found error handling in read_resource handler

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently

---

## Phase 5: User Story 3 - Prompt Template Discovery (Priority: P3)

**Goal**: Enable AI clients to discover and use prompt templates from providers

**Independent Test**: List prompts through MCP Inspector, invoke "hello-plan" with {"name": "Alice"}, receive personalized greeting

### Implementation for User Story 3

#### Go Provider (hello-go) - Prompt Implementation

- [ ] T045 [P] [US3] Create providers/hello-go/internal/prompts/hello_plan.go with GetPrompt handler for "hello-plan" template
- [ ] T046 [US3] Update capabilities.go to include Prompt with name="hello-plan" and args_schema for name parameter
- [ ] T047 [US3] Implement GetPrompt RPC method in server.go to route prompt requests to appropriate prompt handlers

#### Gateway - Prompt Proxying

- [ ] T048 [US3] Implement @mcp.list_prompts() handler in main.py to aggregate prompts from all providers
- [ ] T049 [US3] Implement @mcp.get_prompt() handler in main.py to route prompt requests to providers via ProviderGRPCClient.get_prompt()
- [ ] T050 [US3] Add ProviderGRPCClient.get_prompt() method in adapters/grpc_client.py with 2.5s timeout
- [ ] T051 [US3] Add prompt parameter validation against args_schema before forwarding

**Checkpoint**: All core MCP primitives (tools, resources, prompts) now functional

---

## Phase 6: User Story 4 - Multi-Provider Aggregation (Priority: P2)

**Goal**: Demonstrate language-agnostic provider contract by adding Rust provider with identical capabilities

**Independent Test**: Run both hello-go and hello-rs, verify MCP Inspector shows tools from both providers (hello-go.echo.v1, hello-rs.echo.v1)

### Implementation for User Story 4

#### Rust Provider (hello-rs) - Complete Provider

- [ ] T052 [P] [US4] Create providers/hello-rs/src/tools/echo.rs with echo tool implementation
- [ ] T053 [P] [US4] Create providers/hello-rs/src/tools/sum.rs with sum tool implementation
- [ ] T054 [P] [US4] Create providers/hello-rs/src/resources/greeting.rs with greeting resource handler
- [ ] T055 [P] [US4] Create providers/hello-rs/src/prompts/hello_plan.rs with hello-plan prompt handler
- [ ] T056 [US4] Create providers/hello-rs/src/capabilities.rs to build Capabilities response with all tools/resources/prompts
- [ ] T057 [US4] Create providers/hello-rs/src/service.rs implementing Provider gRPC service trait with all RPC methods
- [ ] T058 [US4] Create providers/hello-rs/src/main.rs as entry point (Tokio runtime, listen on [::1]:50052, register gRPC server)
- [ ] T059 [US4] Load JSON schemas from pkg/schemas/ in capabilities.rs using include_bytes! or std::fs

#### Gateway - Multi-Provider Support

- [ ] T060 [US4] Update providers.yaml to include hello-rs provider configuration (localhost:50052)
- [ ] T061 [US4] Verify provider name prefixing works correctly for multiple providers in list_tools handler
- [ ] T062 [US4] Add error handling for partial provider failures (exclude unavailable providers from capability list per FR-012)

**Checkpoint**: Language-agnostic provider contract validated - both Go and Rust providers work identically

---

## Phase 7: User Story 5 - Event Stream Consumption (Priority: P4) [OPTIONAL]

**Goal**: Enable providers to publish events to NATS and gateway to consume them

**Independent Test**: Publish CloudEvent from hello-go, verify gateway receives it within 2 seconds, restart gateway and verify event replay

### Implementation for User Story 5 (Optional)

#### Infrastructure

- [ ] T063 [P] [US5] Start NATS JetStream via docker-compose.yml
- [ ] T064 [P] [US5] Create NATS stream configuration for "hello.events" topic in infra/nats-setup/

#### Go Provider - Event Publishing

- [ ] T065 [P] [US5] Add NATS client dependency to providers/hello-go/go.mod
- [ ] T066 [P] [US5] Create providers/hello-go/internal/events/publisher.go with NATS JetStream publisher
- [ ] T067 [US5] Implement Stream RPC method in server.go to stream CloudEvents
- [ ] T068 [US5] Publish sample CloudEvent from echo tool execution

#### Gateway - Event Consumption

- [ ] T069 [US5] Add nats-py dependency to mcp-gateway/pyproject.toml
- [ ] T070 [US5] Create mcp_gateway/adapters/nats_client.py with JetStream durable consumer
- [ ] T071 [US5] Implement event subscription in main.py with durable consumer (at-least-once delivery)
- [ ] T072 [US5] Add event replay on gateway restart (durable consumer auto-replay)
- [ ] T073 [US5] Add structured logging for received events

**Checkpoint**: Full event streaming pipeline operational (optional MVP feature)

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [ ] T074 [P] Add perimeter authentication to gateway (client-gateway auth per FR-029) in main.py
- [ ] T075 [P] Add mTLS configuration option for gateway-provider communication in grpc_client.py
- [ ] T076 [P] Add OpenTelemetry instrumentation to gateway in main.py (metrics and distributed tracing per FR-014)
- [ ] T077 [P] Create docs/provider-contract.md explaining how to implement new providers
- [ ] T078 [P] Create docs/mcp-surface.md documenting naming conventions for capabilities
- [ ] T079 [P] Add README.md in repository root with quickstart instructions
- [ ] T080 Create .gitignore files for Python, Go, Rust artifacts
- [ ] T081 [P] Add logging configuration to control log levels via environment variables
- [ ] T082 [P] Add graceful shutdown handling in all components (SIGTERM/SIGINT)
- [ ] T083 Run complete end-to-end test per quickstart.md validation steps with MCP Inspector
- [ ] T084 Verify all 11 success criteria from spec.md are met (SC-001 through SC-011)
- [ ] T085 Code cleanup and refactoring for readability (Constitution Principle I)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion (especially T002 proto copy, T011-T013 codegen) - BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Foundational completion - No dependencies on other stories
- **User Story 2 (Phase 4)**: Depends on Foundational completion - Builds on US1 but independently testable
- **User Story 3 (Phase 5)**: Depends on Foundational completion - Builds on US1 but independently testable
- **User Story 4 (Phase 6)**: Depends on Foundational completion - Can run in parallel with US2/US3
- **User Story 5 (Phase 7)**: Depends on Foundational completion - Fully independent, optional
- **Polish (Phase 8)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - **MVP MILESTONE**
- **User Story 2 (P2)**: Can start after Foundational - Adds resource support to working tool system
- **User Story 3 (P3)**: Can start after Foundational - Adds prompt support to working tool system
- **User Story 4 (P2)**: Can start after Foundational - Validates multi-provider architecture
- **User Story 5 (P4)**: Can start after Foundational - Optional feature, fully independent

### Within Each User Story

- Provider implementation before gateway handlers
- Capability registration before RPC implementations
- Schema loading before validation logic
- Connection setup before request routing
- Error handling after happy path implementation

### Parallel Opportunities

**Phase 1 (Setup)**:
- T003-T007 can all run in parallel (different projects)
- T008-T010 can run in parallel (infrastructure files)

**Phase 2 (Foundational)**:
- T012-T013 can run in parallel (Go/Rust codegen)
- T014-T016 can run in parallel (different Python modules)

**Phase 3 (User Story 1)**:
- T019-T020 can run in parallel (different Go tool files)
- T021-T024 are sequential (depends on T019-T020)
- T025-T028 can run in parallel (different gateway modules)

**Between User Stories**:
- Once Foundational is complete, US2, US3, US4, US5 can ALL start in parallel if team capacity allows
- Different team members can own different stories

---

## Parallel Example: User Story 1

```bash
# Launch Provider work and Gateway work in parallel:

# Provider Thread:
Task T019: "Create echo.go tool"
Task T020: "Create sum.go tool"  # parallel with T019
Then T021-T024 sequentially

# Gateway Thread:
Task T025: "Implement load_providers()"
Task T026: "Implement list_capabilities()"  # parallel with T025
Task T027: "Implement discover_capabilities()"
Task T028: "Implement validate()"  # parallel with T025-T027
Then T029-T037 sequentially
```

---

## Parallel Example: Multi-Story Development

```bash
# After Foundational Phase completes, launch multiple stories:

# Developer A: User Story 1 (T019-T037)
# Developer B: User Story 4 (T052-T062) - Rust provider
# Developer C: User Story 2 (T038-T044) - Resources
# Developer D: User Story 3 (T045-T051) - Prompts

# All stories integrate independently without conflicts
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup → Directory structure, dependencies, protobuf
2. Complete Phase 2: Foundational → Codegen, base modules (CRITICAL!)
3. Complete Phase 3: User Story 1 → hello-go provider + gateway tool proxying
4. **STOP and VALIDATE**: Test with MCP Inspector
   - Verify tools appear: hello-go.echo.v1, hello-go.sum.v1
   - Invoke echo with {"message": "test"}
   - Invoke sum with {"numbers": [1,2,3]}
   - Verify <500ms latency, <3s discovery time
5. **MVP COMPLETE** - Deployable system with core value

### Incremental Delivery

1. **Foundation** (Phases 1-2): ~18 tasks → Protobuf contract + base infrastructure ready
2. **MVP** (Phase 3): +19 tasks → Tool discovery and invocation working → **DEMO/DEPLOY**
3. **Resources** (Phase 4): +7 tasks → Resource access added → **DEMO/DEPLOY**
4. **Prompts** (Phase 5): +7 tasks → Prompt templates added → **DEMO/DEPLOY**
5. **Multi-Provider** (Phase 6): +11 tasks → Rust provider validates architecture → **DEMO/DEPLOY**
6. **Events** (Phase 7): +11 tasks (optional) → Event streaming added → **DEMO/DEPLOY**
7. **Polish** (Phase 8): +12 tasks → Production-ready → **PRODUCTION DEPLOY**

Each phase adds incremental value without breaking previous functionality.

### Parallel Team Strategy

With 3-4 developers after Foundational phase completes:

**Week 1** (Foundation):
- All developers: Complete Setup + Foundational together (critical path)

**Week 2** (MVP):
- All developers: Focus on User Story 1 completion and validation
- Milestone: MVP demo with MCP Inspector

**Week 3** (Expansion):
- Developer A: User Story 2 (Resources)
- Developer B: User Story 4 (Rust provider)
- Developer C: User Story 3 (Prompts)
- Developer D: User Story 5 (Events) or Polish tasks

**Week 4** (Integration & Polish):
- All developers: Integration testing, polish, documentation
- Milestone: Production deployment

---

## Notes

- **[P] tasks** = different files, no dependencies - safe to parallelize
- **[Story] label** maps task to specific user story for traceability
- **Each user story independently completable** - can stop after any phase for demo
- **No tests explicitly requested** - validation via MCP Inspector per quickstart.md
- **Constitution compliance**: Simple procedural code, library-first approach, justified abstractions only
- **Commit strategy**: Commit after completing each logical phase or task group
- **Checkpoints**: Stop at any checkpoint to validate story works independently before proceeding
- **MVP = Phase 3 complete**: Delivers core value (tool discovery and invocation from one provider)
- **Avoid**: Cross-story dependencies, same-file conflicts, speculative features not in spec

---

## Task Count Summary

- **Phase 1 (Setup)**: 10 tasks
- **Phase 2 (Foundational)**: 8 tasks
- **Phase 3 (US1 - MVP)**: 19 tasks
- **Phase 4 (US2 - Resources)**: 7 tasks
- **Phase 5 (US3 - Prompts)**: 7 tasks
- **Phase 6 (US4 - Multi-Provider)**: 11 tasks
- **Phase 7 (US5 - Events, Optional)**: 11 tasks
- **Phase 8 (Polish)**: 12 tasks

**Total**: 85 tasks
**MVP (Phases 1-3)**: 37 tasks
**Parallel opportunities**: ~30 tasks marked [P]
**Independent stories**: 5 user stories can proceed in parallel after Foundational

---

## Success Criteria Mapping

| Criteria | Tasks | Validation |
|----------|-------|------------|
| SC-001: Discovery <3s | T026, T027, T030 | Measure ListCapabilities call time |
| SC-002: Invocation <500ms | T026, T031, T043, T049 | Measure tool call round-trip |
| SC-003: Multi-provider | T060-T062 | Verify both providers in tools list |
| SC-004: 100% valid success | T028, T032, T033 | Schema validation prevents failures |
| SC-005: MCP Inspector | T083 | Manual validation per quickstart.md |
| SC-006: Graceful degradation | T037, T062 | Test with one provider down |
| SC-007: Correlation IDs | T034, T035 | Check logs for correlation_id |
| SC-008: <1 day provider | T052-T059 | Rust provider as proof |
| SC-009: Clean startup | T008-T010, T083 | Run docker compose + make targets |
| SC-010: Event <2s | T070-T072 | Measure NATS delivery time |
| SC-011: Event replay | T071, T072 | Restart gateway, verify replay |
