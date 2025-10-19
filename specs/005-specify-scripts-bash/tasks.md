# Tasks: ChatGPT MCP Connector Integration

**Input**: Design documents from `/specs/005-specify-scripts-bash/`
**Prerequisites**: plan.md, spec.md

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`
- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3, US4)
- Include exact file paths in descriptions

## Path Conventions
- Python package: `mcp-gateway/` (new FastMCP SSE service)
- Existing provider: `providers/binance-rs/` (Rust gRPC provider)
- Infrastructure: `infra/` (nginx, systemd services)
- Documentation: `docs/`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure

- [ ] T001 Create mcp-gateway Python package directory structure (mcp-gateway/, mcp-gateway/tools/, mcp-gateway/protos/)
- [ ] T002 Initialize Python project with uv in mcp-gateway/pyproject.toml
- [ ] T003 [P] Add FastMCP and gRPC dependencies to mcp-gateway/pyproject.toml
- [ ] T004 [P] Copy proto definitions from providers/binance-rs/proto/ to mcp-gateway/protos/
- [ ] T005 [P] Create .gitignore for Python in mcp-gateway/.gitignore

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [ ] T006 Generate Python gRPC stubs from proto files in mcp-gateway/protos/ using protoc
- [ ] T007 Implement Binance gRPC client with connection pooling in mcp-gateway/grpc_client.py
- [ ] T008 [P] Add retry logic and error handling to gRPC client in mcp-gateway/grpc_client.py
- [ ] T009 [P] Implement caching layer (5 second TTL) for ticker/orderbook in mcp-gateway/cache.py
- [ ] T010 Create Document ID registry with serialization/deserialization in mcp-gateway/document_registry.py
- [ ] T011 Implement document ID schema for all 21 Binance tool types in mcp-gateway/document_registry.py
- [ ] T012 [P] Add validation for document ID types in mcp-gateway/document_registry.py
- [ ] T013 Create FastMCP SSE server scaffold in mcp-gateway/main.py
- [ ] T014 [P] Configure logging and error handling in mcp-gateway/main.py
- [ ] T015 [P] Add health check endpoint to FastMCP server in mcp-gateway/main.py

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Connect Binance Data to ChatGPT (Priority: P1) 🎯 MVP

**Goal**: Enable ChatGPT to connect to the MCP server via SSE transport and discover available tools

**Independent Test**: Add connector in ChatGPT Developer Mode settings using SSE endpoint URL, verify ChatGPT successfully connects and lists available tools

### Implementation for User Story 1

- [ ] T016 [US1] Implement SSE transport endpoint at /sse/ in mcp-gateway/main.py
- [ ] T017 [US1] Configure SSE connection handling with long-lived connection support in mcp-gateway/main.py
- [ ] T018 [US1] Add SSE keep-alive mechanism (heartbeat) in mcp-gateway/main.py
- [ ] T019 [US1] Update nginx configuration to add /sse/ location block in infra/nginx-mcp-gateway.conf
- [ ] T020 [US1] Configure SSE-specific nginx headers (proxy_buffering off, chunked encoding) in infra/nginx-mcp-gateway.conf
- [ ] T021 [US1] Set long timeouts (3600s) for SSE connections in infra/nginx-mcp-gateway.conf
- [ ] T022 [US1] Create systemd service file for mcp-gateway SSE service in infra/mcp-gateway-sse.service
- [ ] T023 [US1] Configure environment variables and working directory in infra/mcp-gateway-sse.service
- [ ] T024 [US1] Update deploy.sh to deploy mcp-gateway SSE service in infra/deploy.sh
- [ ] T025 [US1] Update deploy-quick.sh to deploy mcp-gateway SSE service in infra/deploy-quick.sh

**Checkpoint**: At this point, ChatGPT should successfully connect via SSE and discover tools

---

## Phase 4: User Story 2 - Search Cryptocurrency Market Data (Priority: P1)

**Goal**: Enable users to search for cryptocurrency market information using natural language queries through ChatGPT

**Independent Test**: Call the `search` tool directly with query "Bitcoin price" and verify it returns properly formatted results with document IDs, titles, and URLs

### Implementation for User Story 2

- [ ] T026 [US2] Create query parser to extract symbols and data types in mcp-gateway/tools/query_parser.py
- [ ] T027 [P] [US2] Implement symbol extraction (e.g., "Bitcoin price" → BTCUSDT) in mcp-gateway/tools/query_parser.py
- [ ] T028 [P] [US2] Implement data type detection (price → ticker, orderbook → orderbook_l1) in mcp-gateway/tools/query_parser.py
- [ ] T029 [US2] Implement search result ranking strategy in mcp-gateway/tools/query_parser.py
- [ ] T030 [US2] Create search tool handler in mcp-gateway/tools/search.py
- [ ] T031 [US2] Implement tool-to-gRPC mapping for search queries in mcp-gateway/tools/search.py
- [ ] T032 [US2] Format search results as MCP content array with JSON-encoded results in mcp-gateway/tools/search.py
- [ ] T033 [US2] Generate document IDs for search results (ticker:BTCUSDT, etc.) in mcp-gateway/tools/search.py
- [ ] T034 [US2] Create 200-character snippets from full data in mcp-gateway/tools/search.py
- [ ] T035 [US2] Add citation URLs (https://mcp-gateway.thevibe.trading/data/{id}) in mcp-gateway/tools/search.py
- [ ] T036 [US2] Register search tool with FastMCP server in mcp-gateway/main.py
- [ ] T037 [US2] Add error handling for no results and invalid queries in mcp-gateway/tools/search.py

**Checkpoint**: At this point, ChatGPT can search for market data and receive formatted results

---

## Phase 5: User Story 3 - Fetch Detailed Market Information (Priority: P1)

**Goal**: Enable users to retrieve complete details about specific cryptocurrency or market data points after finding them through search

**Independent Test**: Fetch a specific document ID (e.g., "ticker:BTCUSDT") and verify complete ticker data is returned with proper citation URL

### Implementation for User Story 3

- [ ] T038 [US3] Create document ID parser to extract type and parameters in mcp-gateway/tools/document_parser.py
- [ ] T039 [US3] Implement ID-to-tool mapping for all 21 tool types in mcp-gateway/tools/document_parser.py
- [ ] T040 [US3] Create fetch tool handler in mcp-gateway/tools/fetch.py
- [ ] T041 [US3] Implement document retrieval by calling appropriate Binance gRPC tool in mcp-gateway/tools/fetch.py
- [ ] T042 [US3] Format full data with metadata for fetch responses in mcp-gateway/tools/fetch.py
- [ ] T043 [US3] Return MCP content array with JSON-encoded document in mcp-gateway/tools/fetch.py
- [ ] T044 [US3] Add citation URL to fetch responses in mcp-gateway/tools/fetch.py
- [ ] T045 [US3] Register fetch tool with FastMCP server in mcp-gateway/main.py
- [ ] T046 [US3] Add error handling for non-existent document IDs in mcp-gateway/tools/fetch.py
- [ ] T047 [P] [US3] Implement fetch for ticker data (ticker:SYMBOL) in mcp-gateway/tools/fetch.py
- [ ] T048 [P] [US3] Implement fetch for orderbook data (orderbook:SYMBOL, orderbook_l1:SYMBOL, orderbook_l2:SYMBOL) in mcp-gateway/tools/fetch.py
- [ ] T049 [P] [US3] Implement fetch for klines data (klines:SYMBOL:INTERVAL) in mcp-gateway/tools/fetch.py
- [ ] T050 [P] [US3] Implement fetch for analytics data (analytics:TYPE:SYMBOL) in mcp-gateway/tools/fetch.py

**Checkpoint**: At this point, ChatGPT can fetch complete market data for any discovered document

---

## Phase 6: User Story 4 - Use Deep Research with Market Data (Priority: P2)

**Goal**: Enable traders to use ChatGPT's deep research feature to analyze market trends across multiple cryptocurrency pairs

**Independent Test**: Run a deep research query "Analyze Bitcoin vs Ethereum market trends" and verify multiple search and fetch calls are made successfully

### Implementation for User Story 4

- [ ] T051 [US4] Enhance search to support multiple symbols in single query in mcp-gateway/tools/search.py
- [ ] T052 [US4] Optimize fetch for batch analytics requests in mcp-gateway/tools/fetch.py
- [ ] T053 [US4] Add volume profile data support in document registry in mcp-gateway/document_registry.py
- [ ] T054 [US4] Add orderbook health analytics in document registry in mcp-gateway/document_registry.py
- [ ] T055 [US4] Implement fetch for volume_profile analytics in mcp-gateway/tools/fetch.py
- [ ] T056 [US4] Implement fetch for orderbook_health analytics in mcp-gateway/tools/fetch.py
- [ ] T057 [US4] Add metadata fields for POC and value area in analytics responses in mcp-gateway/tools/fetch.py
- [ ] T058 [US4] Test deep research with multi-step queries (search → multiple fetches) manually

**Checkpoint**: All user stories should now be independently functional

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories, deployment, and documentation

- [ ] T059 [P] Create ChatGPT connection guide with screenshots in docs/CHATGPT_INTEGRATION.md
- [ ] T060 [P] Document search and fetch tool APIs with examples in docs/API_REFERENCE.md
- [ ] T061 [P] Create deployment runbook in docs/DEPLOYMENT.md
- [ ] T062 [P] Add example queries to documentation in docs/CHATGPT_INTEGRATION.md
- [ ] T063 [P] Add troubleshooting guide in docs/CHATGPT_INTEGRATION.md
- [ ] T064 Deploy mcp-gateway SSE service to production server using infra/deploy-quick.sh
- [ ] T065 Verify health check endpoint responds on production
- [ ] T066 Test SSE connection through nginx on production
- [ ] T067 Add connector in ChatGPT Developer Mode (production smoke test)
- [ ] T068 Run 3-5 smoke test queries in ChatGPT with production endpoint
- [ ] T069 [P] Add logging for all search operations in mcp-gateway/tools/search.py
- [ ] T070 [P] Add logging for all fetch operations in mcp-gateway/tools/fetch.py
- [ ] T071 Monitor production logs for errors after deployment
- [ ] T072 Verify existing HTTP JSON-RPC endpoint still works (backward compatibility test)
- [ ] T073 Run existing test scripts (test_http_tools.sh, test_21_tools.sh) to verify compatibility
- [ ] T074 Create load test script for 50 concurrent SSE connections in tests/load_test.sh
- [ ] T075 Run load test and verify <2s response times
- [ ] T076 [P] Add rate limiting monitoring in mcp-gateway/main.py
- [ ] T077 [P] Document security considerations in docs/SECURITY.md
- [ ] T078 Code cleanup and refactoring across mcp-gateway/
- [ ] T079 Update main project README.md with SSE endpoint information

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-6)**: All depend on Foundational phase completion
  - User Story 1 (P1): SSE transport and ChatGPT connection
  - User Story 2 (P1): Search functionality (DEPENDS on US1 for SSE)
  - User Story 3 (P1): Fetch functionality (DEPENDS on US1 for SSE, works with US2)
  - User Story 4 (P2): Deep research (DEPENDS on US2 and US3)
- **Polish (Phase 7)**: Depends on all P1 user stories being complete (US1, US2, US3)

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P1)**: DEPENDS on User Story 1 (needs SSE transport to work)
- **User Story 3 (P1)**: DEPENDS on User Story 1 (needs SSE transport to work)
- **User Story 4 (P2)**: DEPENDS on User Story 2 AND 3 (uses both search and fetch)

### Within Each User Story

- User Story 1: SSE endpoint → nginx config → systemd service → deployment scripts
- User Story 2: Query parser → search handler → registration → error handling
- User Story 3: Document parser → fetch handler → registration → all document types
- User Story 4: Enhanced search → batch fetch → analytics support

### Parallel Opportunities

- **Phase 1 (Setup)**: T003, T004, T005 can run in parallel
- **Phase 2 (Foundational)**: T008, T009, T012, T014, T015 can run in parallel (within their groups)
- **User Story 2**: T027, T028 can run in parallel (different aspects of query parsing)
- **User Story 3**: T047, T048, T049, T050 can run in parallel (different document types)
- **Phase 7 (Polish)**: T059, T060, T061, T062, T063, T069, T070, T076, T077 can run in parallel (documentation and logging)

---

## Parallel Example: User Story 2

```bash
# Launch query parser components in parallel:
Task T027: "Implement symbol extraction in mcp-gateway/tools/query_parser.py"
Task T028: "Implement data type detection in mcp-gateway/tools/query_parser.py"
```

## Parallel Example: User Story 3

```bash
# Launch all document type implementations in parallel:
Task T047: "Implement fetch for ticker data in mcp-gateway/tools/fetch.py"
Task T048: "Implement fetch for orderbook data in mcp-gateway/tools/fetch.py"
Task T049: "Implement fetch for klines data in mcp-gateway/tools/fetch.py"
Task T050: "Implement fetch for analytics data in mcp-gateway/tools/fetch.py"
```

---

## Implementation Strategy

### MVP First (User Stories 1, 2, 3 - All P1)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1 (SSE connection)
4. Complete Phase 4: User Story 2 (Search functionality)
5. Complete Phase 5: User Story 3 (Fetch functionality)
6. **STOP and VALIDATE**: Test ChatGPT integration end-to-end
7. Deploy to production (Phase 7 tasks T064-T068)

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 → Test SSE connection independently → Verify in ChatGPT
3. Add User Story 2 → Test search independently → Verify in ChatGPT
4. Add User Story 3 → Test fetch independently → Verify in ChatGPT (MVP COMPLETE!)
5. Add User Story 4 → Test deep research → Deploy
6. Polish and documentation

### Sequential Strategy (Recommended)

Given the dependencies between user stories:

1. Complete Phase 1 + Phase 2 (Setup + Foundational)
2. Complete Phase 3 (User Story 1 - SSE transport) - REQUIRED for others
3. Complete Phase 4 + Phase 5 in sequence or parallel (User Story 2 + 3 both need US1)
4. Complete Phase 6 (User Story 4 - needs US2 and US3)
5. Complete Phase 7 (Polish and deploy)

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- User stories have dependencies: US2 and US3 both require US1, US4 requires US2+US3
- MVP = User Stories 1, 2, and 3 complete (all P1)
- Binance provider must be running in gRPC mode on port 50053
- Nginx must proxy both /mcp (existing) and /sse/ (new) endpoints
- Backward compatibility: existing HTTP JSON-RPC at /mcp must continue working
- Document IDs follow schema: ticker:SYMBOL, orderbook:SYMBOL, klines:SYMBOL:INTERVAL, analytics:TYPE:SYMBOL
- All responses use MCP content array format: [{type: "text", text: JSON-encoded-string}]
- No authentication for initial deployment (OAuth can be added later)
