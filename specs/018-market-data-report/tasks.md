# Tasks: Unified Market Data Report

**Input**: Design documents from `/specs/018-market-data-report/`
**Prerequisites**: plan.md (✓), spec.md (✓), research.md (✓), data-model.md (✓), contracts/ (✓)

**Tests**: Tests are NOT explicitly requested in the specification. Unit tests and integration tests are included for quality assurance but are marked as optional enhancements.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`
- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3, US4)
- Include exact file paths in descriptions

## Path Conventions
- **Rust provider**: `providers/binance-rs/src/`
- **Python gateway**: `mcp-gateway/mcp_gateway/`
- **Tests**: `providers/binance-rs/tests/`
- **Contracts**: `providers/binance-rs/proto/`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and module structure setup

- [ ] T001 Create report module structure in providers/binance-rs/src/report/ with mod.rs, generator.rs, formatter.rs, sections.rs
- [ ] T002 [P] Create test directory structure in providers/binance-rs/tests/integration/report_generation.rs and tests/unit/report/
- [ ] T003 [P] Copy gRPC contract from specs/018-market-data-report/contracts/market-report.proto to providers/binance-rs/proto/
- [ ] T004 Update providers/binance-rs/Cargo.toml to add report module and any new dependencies (if needed)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [ ] T005 Define ReportOptions struct in providers/binance-rs/src/report/mod.rs with validation methods
- [ ] T006 [P] Define MarketReport struct in providers/binance-rs/src/report/mod.rs with metadata fields
- [ ] T007 [P] Define ReportSection internal struct in providers/binance-rs/src/report/mod.rs for section building
- [ ] T008 Implement ReportCache struct with TTL-based in-memory caching in providers/binance-rs/src/report/mod.rs
- [ ] T009 [P] Implement markdown formatting utilities (build_table, build_list, build_section_header) in providers/binance-rs/src/report/formatter.rs
- [ ] T010 Update providers/binance-rs/build.rs to add market-report.proto to the prost protobuf compilation list
- [ ] T011 Generate Rust code from market-report.proto by running cargo build

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Access Comprehensive Market Intelligence Report (Priority: P1) 🎯 MVP

**Goal**: Implement the core unified report generation method that consolidates all market data into a single markdown report with 7-8 sections

**Independent Test**: Call generate_report("BTCUSDT", default_options) and verify markdown output contains all 8 sections: Report Header, Price Overview, Order Book Metrics, Liquidity Analysis, Market Microstructure, Market Anomalies, Microstructure Health, Data Health Status

### Implementation for User Story 1

- [ ] T012 [P] [US1] Implement build_report_header() function with visual indicators for data freshness (Fresh/Recent/Stale) in providers/binance-rs/src/report/sections.rs (symbol, timestamp, data age, cache status)
- [ ] T013 [P] [US1] Implement build_price_overview_section() function in providers/binance-rs/src/report/sections.rs using Ticker24hr data
- [ ] T014 [P] [US1] Implement build_orderbook_metrics_section() function with markdown tables and visual indicators in providers/binance-rs/src/report/sections.rs using OrderBookMetrics data
- [ ] T015 [P] [US1] Implement build_liquidity_analysis_section() function in providers/binance-rs/src/report/sections.rs using OrderBookDepth, Walls, VolumeProfile
- [ ] T016 [P] [US1] Implement build_microstructure_section() function in providers/binance-rs/src/report/sections.rs using OrderFlowSnapshot data
- [ ] T017 [P] [US1] Implement build_anomalies_section() function with severity emoji indicators (🔴/🟡/🟢) in providers/binance-rs/src/report/sections.rs (conditional compilation for orderbook_analytics feature)
- [ ] T018 [P] [US1] Implement build_health_section() function in providers/binance-rs/src/report/sections.rs using MicrostructureHealth data (conditional compilation)
- [ ] T019 [P] [US1] Implement build_data_health_section() function with status emoji indicators (✅/⚠️/❌) in providers/binance-rs/src/report/sections.rs using OrderBookHealth data
- [ ] T020 [US1] Implement ReportGenerator::new() constructor in providers/binance-rs/src/report/generator.rs with dependency injection
- [ ] T021 [US1] Implement parallel data fetching logic using tokio::join! in ReportGenerator::fetch_all_data() method in providers/binance-rs/src/report/generator.rs
- [ ] T022 [US1] Implement ReportGenerator::build_sections() method to assemble all sections with graceful error handling in providers/binance-rs/src/report/generator.rs
- [ ] T023 [US1] Implement ReportGenerator::generate_report() public API method with caching and options handling in providers/binance-rs/src/report/generator.rs
- [ ] T024 [US1] Add report generation method to gRPC service implementation in providers/binance-rs/src/grpc/tools.rs calling ReportGenerator
- [ ] T025 [US1] Add report generation method to MCP handler in providers/binance-rs/src/mcp/handler.rs calling ReportGenerator
- [ ] T026 [US1] Update Python MCP gateway fetch.py to handle "report:SYMBOL" document ID format in mcp-gateway/mcp_gateway/tools/fetch.py
- [ ] T027 [US1] Update Python gRPC client proxy to call GenerateMarketReport RPC in mcp-gateway/mcp_gateway/adapters/grpc_client.py

**Checkpoint**: At this point, User Story 1 should be fully functional - traders can request unified reports via gRPC or MCP and receive complete markdown output

---

## Phase 4: User Story 2 - Identify Trading Risks Through Anomaly Detection (Priority: P2)

**Goal**: Enhance the Market Anomalies section to provide clear severity indicators, sorting, and actionable recommendations for detected anomalies

**Independent Test**: Request report during various market conditions and verify "Market Anomalies" section displays detected anomalies sorted by severity with recommendations, or shows "No anomalies detected" message

### Implementation for User Story 2

- [ ] T028 [US2] Enhance build_anomalies_section() to format anomalies with severity badges (Critical/High/Medium/Low) in providers/binance-rs/src/report/sections.rs
- [ ] T029 [US2] Add anomaly sorting logic by severity (Critical first) in build_anomalies_section() in providers/binance-rs/src/report/sections.rs
- [ ] T030 [US2] Add recommendation formatting with actionable guidance per anomaly type in providers/binance-rs/src/report/sections.rs
- [ ] T031 [US2] Add "No anomalies detected" message with timestamp when anomalies array is empty in providers/binance-rs/src/report/sections.rs
- [ ] T032 [US2] Add anomaly description enrichment with affected price levels and detection context in providers/binance-rs/src/report/sections.rs

**Checkpoint**: Anomaly detection section now provides comprehensive risk warnings with clear severity and actionable recommendations

---

## Phase 5: User Story 3 - Analyze Liquidity for Order Placement (Priority: P2)

**Goal**: Enhance the Liquidity Analysis section to prominently display liquidity walls, POC/VAH/VAL indicators, and liquidity vacuum zones

**Independent Test**: Request report and verify "Liquidity Analysis" section shows identified liquidity walls with price/volume, volume profile with POC/VAH/VAL levels, and liquidity vacuum zones with price ranges and impact levels

### Implementation for User Story 3

- [ ] T033 [US3] Enhance build_liquidity_analysis_section() to format liquidity walls table with price, volume, and support/resistance indicators in providers/binance-rs/src/report/sections.rs
- [ ] T034 [US3] Add volume profile visualization with POC/VAH/VAL prominently displayed in providers/binance-rs/src/report/sections.rs
- [ ] T035 [US3] Add liquidity vacuums table with price ranges, volume deficit %, and expected impact levels in providers/binance-rs/src/report/sections.rs
- [ ] T036 [US3] Add volume window duration display (e.g., "24h Volume Profile") respecting ReportOptions.volume_window_hours in providers/binance-rs/src/report/sections.rs
- [ ] T037 [US3] Add visual indicators (emoji or symbols) for strong/moderate/weak walls and high/medium/low impact zones in providers/binance-rs/src/report/sections.rs

**Checkpoint**: Liquidity analysis section now provides comprehensive guidance for order placement decisions

---

## Phase 6: User Story 4 - Monitor Order Book Health and Data Quality (Priority: P3)

**Goal**: Enhance the Data Health Status section and report header to prominently display data freshness, connectivity status, and health indicators

**Independent Test**: Request report and verify header shows generation timestamp and data age, "Data Health Status" section displays WebSocket connectivity, service status (Healthy/Degraded/Critical), and last update age

### Implementation for User Story 4

- [ ] T038 [US4] Enhance build_report_header() to display data freshness indicator with color-coded status (Fresh <1s, Recent <5s, Stale >5s) in providers/binance-rs/src/report/sections.rs
- [ ] T039 [US4] Enhance build_data_health_section() to format health status with visual indicators (✅/⚠️/❌) in providers/binance-rs/src/report/sections.rs
- [ ] T040 [US4] Add WebSocket connectivity status display in data health section in providers/binance-rs/src/report/sections.rs
- [ ] T041 [US4] Add active symbols count and last update age display in data health section in providers/binance-rs/src/report/sections.rs
- [ ] T042 [US4] Add degradation warnings when data age exceeds thresholds (warn >5s, critical >30s) in providers/binance-rs/src/report/sections.rs
- [ ] T043 [US4] Add report footer with generation time, feature build info, and cache status in providers/binance-rs/src/report/sections.rs

**Checkpoint**: All user stories (US1-US4) are now complete - the unified report provides comprehensive market intelligence with risk warnings, liquidity guidance, and data quality assurance

---

## Phase 7: Code Removal (Order Management Cleanup)

**Purpose**: Remove all order management methods and gRPC/MCP handlers as specified in requirements

**⚠️ WARNING**: This phase contains breaking changes - existing clients using order management will fail after this phase

- [ ] T044 [P] Remove create_order(), cancel_order(), query_order(), get_open_orders(), get_all_orders(), get_my_trades() methods from providers/binance-rs/src/binance/client.rs
- [ ] T045 [P] Remove create_listen_key(), keepalive_listen_key(), close_listen_key() WebSocket user data stream methods from providers/binance-rs/src/binance/client.rs
- [ ] T046 [P] Remove get_account() method from providers/binance-rs/src/binance/client.rs
- [ ] T047 [P] Remove handle_place_order(), handle_cancel_order(), handle_get_order(), handle_get_open_orders(), handle_get_all_orders(), handle_get_my_trades(), handle_get_account() from providers/binance-rs/src/grpc/tools.rs
- [ ] T048 [P] Remove corresponding order management tool registrations from MCP handler in providers/binance-rs/src/mcp/handler.rs
- [ ] T049 [P] Remove Order, AccountInfo, MyTrade type definitions from providers/binance-rs/src/binance/types.rs IF no longer referenced (check compilation)
- [ ] T050 [P] Remove order management gRPC proxy methods from mcp-gateway/mcp_gateway/adapters/grpc_client.py
- [ ] T051 Verify no remaining references to removed methods by running: grep -r "place_order\|cancel_order\|get_account\|get_my_trades" providers/ mcp-gateway/
- [ ] T052 Verify authentication infrastructure (auth.rs) is preserved and still compiles in providers/binance-rs/src/binance/auth.rs
- [ ] T053 Run cargo build to verify no compilation errors after method removal

**Checkpoint**: All order management code removed - system is now read-only market data analysis tool

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories and final quality enhancements

- [ ] T054 [P] Add unit tests for markdown formatting utilities in providers/binance-rs/tests/unit/report/formatter_test.rs
- [ ] T055 [P] Add unit tests for individual section builders (price, orderbook, liquidity, etc.) in providers/binance-rs/tests/unit/report/sections_test.rs
- [ ] T056 [P] Add unit tests for ReportCache TTL behavior in providers/binance-rs/tests/unit/report/cache_test.rs
- [ ] T057 Add integration test for full report generation with real data and validate invalid symbol error handling in providers/binance-rs/tests/integration/report_generation.rs
- [ ] T058 [P] Add integration test for graceful degradation covering edge cases: missing data sources, stale data warnings, and rate limiting fallback in providers/binance-rs/tests/integration/report_generation.rs
- [ ] T059 [P] Add integration test for feature flag handling (orderbook_analytics disabled) covering partial feature availability edge case in providers/binance-rs/tests/integration/report_generation.rs
- [ ] T060 [P] Add integration test for concurrent report generation (10+ simultaneous requests) covering concurrent requests edge case and volatile market snapshots in providers/binance-rs/tests/integration/report_generation.rs
- [ ] T061 [P] Add Python integration test for MCP gateway fetch("report:BTCUSDT") in mcp-gateway/tests/test_report_integration.py
- [ ] T062 [P] Update CHANGELOG.md with breaking changes notice and migration guide at repository root
- [ ] T063 [P] Update README.md with new unified report API examples at repository root
- [ ] T064 [P] Add inline documentation comments to all public ReportGenerator methods in providers/binance-rs/src/report/generator.rs
- [ ] T065 Performance profiling: Measure report generation time and optimize if >5s for cold requests
- [ ] T066 Performance profiling: Verify cache hit latency meets <3s requirement
- [ ] T067 Code review: Verify all constitution principles are satisfied (simplicity, DRY, minimal OOP)
- [ ] T068 Run cargo clippy and fix any warnings in providers/binance-rs/
- [ ] T069 Run cargo fmt to format all Rust code in providers/binance-rs/
- [ ] T070 Run pytest and fix any failures in mcp-gateway/
- [ ] T071 Validate quickstart.md examples by manually executing each code sample from specs/018-market-data-report/quickstart.md

**Checkpoint**: Feature complete, tested, documented, and ready for production deployment

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup (Phase 1) completion - BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Foundational (Phase 2) - MVP feature
- **User Story 2 (Phase 4)**: Depends on User Story 1 (Phase 3) completion - enhances anomalies section
- **User Story 3 (Phase 5)**: Depends on User Story 1 (Phase 3) completion - enhances liquidity section
- **User Story 4 (Phase 6)**: Depends on User Story 1 (Phase 3) completion - enhances health section
- **Code Removal (Phase 7)**: Can start after User Story 1 (Phase 3) - independent of US2/US3/US4
- **Polish (Phase 8)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Core report generation - REQUIRED for MVP
  - Depends on: Foundational phase only
  - Blocks: US2, US3, US4 (they enhance sections created in US1)

- **User Story 2 (P2)**: Enhanced anomaly detection display
  - Depends on: US1 (enhances build_anomalies_section created in US1)
  - Blocks: None (independent enhancement)

- **User Story 3 (P2)**: Enhanced liquidity analysis display
  - Depends on: US1 (enhances build_liquidity_analysis_section created in US1)
  - Blocks: None (independent enhancement)

- **User Story 4 (P3)**: Enhanced data health monitoring
  - Depends on: US1 (enhances header and data health section created in US1)
  - Blocks: None (independent enhancement)

### Within Each User Story

**User Story 1 (Core Report)**:
1. Section builders (T012-T019) can run in parallel [P]
2. ReportGenerator constructor (T020) depends on section builders
3. Data fetching logic (T021) can be done in parallel with section builders
4. Assembly logic (T022-T023) depends on both section builders and data fetching
5. gRPC/MCP integration (T024-T027) depends on ReportGenerator being complete

**User Story 2-4 (Enhancements)**:
- All tasks within US2, US3, US4 modify different sections
- US2 (T028-T032), US3 (T033-T037), US4 (T038-T043) can run in parallel

**Code Removal (Phase 7)**:
- All removal tasks (T044-T050) can run in parallel [P]
- Verification tasks (T051-T053) must run sequentially after removals

**Polish (Phase 8)**:
- Most tasks marked [P] can run in parallel (different test files, docs)
- Performance profiling (T065-T066) should run after all features complete
- Validation (T071) should be last

### Parallel Opportunities

- **Setup Phase**: T002, T003 can run in parallel (different directories)
- **Foundational Phase**: T005, T006, T007 can run in parallel (struct definitions), T009 can run in parallel with T008
- **User Story 1**: T012-T019 (all section builders) can run in parallel, T024-T027 (integrations) can run after core is done
- **User Stories 2, 3, 4**: Can all run in parallel if team capacity allows (different sections)
- **Code Removal**: T044-T050 can all run in parallel (different files/methods)
- **Polish**: T054-T061 (tests), T062-T064 (docs), T068-T070 (linting) can run in parallel

---

## Parallel Example: User Story 1

```bash
# After Foundational phase completes, launch all section builders together:
Task T012: "Implement build_report_header()"
Task T013: "Implement build_price_overview_section()"
Task T014: "Implement build_orderbook_metrics_section()"
Task T015: "Implement build_liquidity_analysis_section()"
Task T016: "Implement build_microstructure_section()"
Task T017: "Implement build_anomalies_section()"
Task T018: "Implement build_health_section()"
Task T019: "Implement build_data_health_section()"

# After section builders complete, implement generator logic:
Task T020: "Implement ReportGenerator::new()"
Task T021: "Implement parallel data fetching with tokio::join!"
Task T022: "Implement build_sections() method"
Task T023: "Implement generate_report() public API"

# After generator complete, add integrations in parallel:
Task T024: "Add gRPC service method"
Task T025: "Add MCP handler method"
Task T026: "Update Python fetch.py"
Task T027: "Update Python gRPC client"
```

---

## Parallel Example: Enhancement Stories (After US1)

```bash
# User Stories 2, 3, 4 can run in parallel (different teams/developers):

# Developer A: User Story 2 (Anomaly Detection)
Task T028: "Enhance anomalies section with severity badges"
Task T029: "Add anomaly sorting by severity"
Task T030: "Add recommendation formatting"
Task T031: "Add no anomalies message"
Task T032: "Add anomaly description enrichment"

# Developer B: User Story 3 (Liquidity Analysis)
Task T033: "Enhance liquidity walls table"
Task T034: "Add volume profile visualization"
Task T035: "Add liquidity vacuums table"
Task T036: "Add volume window display"
Task T037: "Add visual indicators"

# Developer C: User Story 4 (Data Health)
Task T038: "Enhance header with freshness indicator"
Task T039: "Enhance health section with visual indicators"
Task T040: "Add WebSocket status display"
Task T041: "Add active symbols display"
Task T042: "Add degradation warnings"
Task T043: "Add report footer"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001-T004)
2. Complete Phase 2: Foundational (T005-T011) - **CRITICAL GATE**
3. Complete Phase 3: User Story 1 (T012-T027)
4. **STOP and VALIDATE**:
   - Test: `generate_report("BTCUSDT", default)`
   - Verify: Markdown output with all 7+ sections
   - Performance: Check <5s cold, <3s cached
5. **OPTIONAL**: Deploy MVP for early feedback

**MVP Scope**: At this point you have a working unified market data report generator. Traders can request comprehensive reports via gRPC or MCP. This is production-ready.

### Incremental Delivery

1. **Foundation** (Phases 1-2): Setup + Foundational → Foundation ready (T001-T011)
2. **MVP** (Phase 3): Add User Story 1 → Test independently → Deploy/Demo (T012-T027)
3. **Enhanced Anomalies** (Phase 4): Add User Story 2 → Test anomaly section → Deploy (T028-T032)
4. **Enhanced Liquidity** (Phase 5): Add User Story 3 → Test liquidity section → Deploy (T033-T037)
5. **Enhanced Health** (Phase 6): Add User Story 4 → Test health monitoring → Deploy (T038-T043)
6. **Code Cleanup** (Phase 7): Remove order management → Test no breakage in reports → Deploy (T044-T053)
7. **Production Ready** (Phase 8): Polish, tests, docs → Final validation → Production release (T054-T071)

Each story adds value without breaking previous stories.

### Parallel Team Strategy

With multiple developers after Foundation (Phase 2) is complete:

**Sprint 1: MVP (Foundation + US1)**
- Entire team: Complete Setup (Phase 1) and Foundational (Phase 2) together
- Entire team: Complete User Story 1 (Phase 3) together - critical path

**Sprint 2: Enhancements (US2, US3, US4)**
- Developer A: User Story 2 (anomaly enhancements)
- Developer B: User Story 3 (liquidity enhancements)
- Developer C: User Story 4 (health monitoring enhancements)
- Stories complete independently in parallel

**Sprint 3: Cleanup & Polish**
- Developer A: Code Removal (Phase 7)
- Developer B + C: Tests and Documentation (Phase 8)

---

## Task Summary

### Total Tasks: 71

**By Phase**:
- Phase 1 (Setup): 4 tasks
- Phase 2 (Foundational): 7 tasks (BLOCKING)
- Phase 3 (User Story 1): 16 tasks ⭐ MVP
- Phase 4 (User Story 2): 5 tasks
- Phase 5 (User Story 3): 5 tasks
- Phase 6 (User Story 4): 6 tasks
- Phase 7 (Code Removal): 10 tasks ⚠️ BREAKING
- Phase 8 (Polish): 18 tasks

**By Story**:
- US1 (Access Comprehensive Report): 16 tasks - Core functionality
- US2 (Identify Trading Risks): 5 tasks - Enhances anomalies section
- US3 (Analyze Liquidity): 5 tasks - Enhances liquidity section
- US4 (Monitor Data Health): 6 tasks - Enhances health monitoring
- Infrastructure (Setup + Foundation): 11 tasks
- Code Removal: 10 tasks
- Polish & Testing: 18 tasks

**Parallel Opportunities Identified**:
- Phase 1: 2 parallel tasks (T002, T003)
- Phase 2: 4 parallel tasks (T005-T007, T009)
- Phase 3: 8 parallel section builders (T012-T019), 4 parallel integrations (T024-T027)
- Phase 4-6: All 16 enhancement tasks can run in parallel (US2, US3, US4)
- Phase 7: 7 parallel removal tasks (T044-T050)
- Phase 8: 13 parallel polish tasks

**MVP Scope**: Phases 1-3 (T001-T027) = 27 tasks

**Full Feature Scope**: All 71 tasks

---

## Notes

- [P] tasks = different files, no dependencies within phase
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- **Breaking Change Warning**: Phase 7 removes all order management - communicate to stakeholders before deployment
- Authentication infrastructure (auth.rs) is explicitly preserved per FR-012
- Feature flag `orderbook_analytics` must be handled gracefully in sections (US1 T017, T018)
- Cache TTL is 60 seconds per research.md decision
- Performance targets: <5s cold, <3s cached (validate in T065-T066)
