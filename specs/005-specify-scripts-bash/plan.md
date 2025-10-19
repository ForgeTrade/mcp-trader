# Implementation Plan: ChatGPT MCP Connector Integration

**Feature Branch**: `005-specify-scripts-bash`
**Created**: 2025-10-19
**Status**: Draft

## Plan Overview

This plan implements SSE (Server-Sent Events) transport and ChatGPT-compatible search/fetch tools for the Binance MCP provider, enabling integration with ChatGPT Developer Mode.

**Core Changes**:
1. Add SSE transport endpoint at `/sse/`
2. Implement `search` and `fetch` tools with ChatGPT-compatible response format
3. Create search/fetch adapter layer to map queries to 21 existing Binance tools
4. Maintain backward compatibility with existing HTTP JSON-RPC endpoint

---

## Phase 0: Research & Investigation

### Research Tasks

**R0.1: FastMCP Framework Investigation**
- **Goal**: Understand how to implement SSE transport using FastMCP
- **What to research**:
  - FastMCP SSE transport API and configuration
  - How to run FastMCP alongside existing HTTP JSON-RPC server
  - SSE endpoint routing and nginx proxy requirements
  - Long-lived connection management (keep-alive, timeouts)
- **Output**: Document FastMCP SSE setup patterns and code examples

**R0.2: ChatGPT MCP Response Format Analysis**
- **Goal**: Understand exact response format ChatGPT expects
- **What to research**:
  - MCP content array structure: `[{type: "text", text: JSON-encoded-string}]`
  - search tool response schema (id, title, text, url fields)
  - fetch tool response schema (id, title, text, url, metadata fields)
  - Error response format for ChatGPT
- **Output**: Document response format examples and validation rules

**R0.3: Document ID Schema Design**
- **Goal**: Design unique, parseable document IDs for all Binance data types
- **What to research**:
  - How to encode symbol + data type in ID (e.g., `ticker:BTCUSDT`, `orderbook:ETHUSDT`)
  - How to handle time-based data (klines with intervals: `klines:BTCUSDT:1h`)
  - How to map document IDs back to specific Binance tool calls
  - ID collision prevention across 21 tool types
- **Output**: Complete document ID schema specification

**R0.4: Search Query → Binance Tool Mapping**
- **Goal**: Design algorithm to map natural language search queries to Binance tools
- **What to research**:
  - Symbol extraction from queries (e.g., "Bitcoin price" → BTCUSDT)
  - Data type detection (price → ticker, orderbook → orderbook_l1, volume → volume_profile)
  - Ranking strategy for multiple matches
  - Handling ambiguous queries (e.g., "BTC" could match BTCUSDT, BTCETH, etc.)
- **Output**: Query parsing and tool selection algorithm

**R0.5: Nginx SSE Proxy Configuration**
- **Goal**: Understand nginx configuration needed for SSE streaming
- **What to research**:
  - Required nginx headers for SSE (Connection, proxy_buffering, chunked_transfer_encoding)
  - Timeout settings for long-lived SSE connections
  - How to route `/sse/` separately from `/mcp` endpoint
  - Testing SSE connection with curl or similar tools
- **Output**: Nginx configuration requirements and test commands

### Research Completion Criteria
- [ ] FastMCP SSE code example created and tested locally
- [ ] Complete document ID schema documented with all 21 tool types mapped
- [ ] Query parsing algorithm tested with 20+ sample queries
- [ ] Nginx SSE configuration tested with long-lived connection (5+ minutes)

---

## Phase 1: Design

### Design Tasks

**D1.1: Search/Fetch Adapter Architecture**
- **Goal**: Design the adapter layer between ChatGPT tools and Binance tools
- **Decisions needed**:
  - Where does adapter live? (Python FastMCP service? Rust extension? Separate service?)
  - How to call existing Binance gRPC tools from adapter?
  - Caching strategy for frequently requested data (ticker, orderbook)
  - Error handling and retry logic
- **Output**: Architecture diagram and component interfaces

**D1.2: Data Model for Search Results**
- **Goal**: Define data structures for search and fetch responses
- **Decisions needed**:
  - Python dataclasses or Pydantic models?
  - How to serialize Binance tool responses into search result snippets (200 char limit)
  - How to format full data for fetch responses (JSON vs formatted text)
  - URL structure for citations (e.g., `https://mcp-gateway.thevibe.trading/data/ticker:BTCUSDT`)
- **Output**: Data model definitions with examples

**D1.3: API Contracts**
- **Goal**: Define exact API contracts for search and fetch tools
- **Decisions needed**:
  - search tool parameters: query (string), limit (optional int)?
  - fetch tool parameters: document_id (string)
  - Response schemas matching ChatGPT requirements
  - Error codes and messages
- **Output**: OpenAPI/JSON Schema definitions for both tools

**D1.4: Deployment Architecture**
- **Goal**: Plan how to deploy SSE service alongside existing HTTP service
- **Decisions needed**:
  - Run FastMCP as separate systemd service or integrate into binance-provider?
  - Port allocation (use 3001 for SSE? Or same port 3000 with path routing?)
  - Process management (separate service vs single service with both transports)
  - Logging and monitoring strategy
- **Output**: Deployment diagram and systemd service specifications

**D1.5: Backward Compatibility Strategy**
- **Goal**: Ensure existing MCP clients continue working
- **Decisions needed**:
  - Keep HTTP JSON-RPC at `/mcp` endpoint unchanged
  - Add SSE at new `/sse/` endpoint
  - Both endpoints expose same 21 Binance tools? Or SSE only exposes search/fetch?
  - Version negotiation if needed
- **Output**: Compatibility matrix and migration guide

### Design Completion Criteria
- [ ] All architectural decisions documented and reviewed
- [ ] Data models defined with validation rules
- [ ] API contracts defined with example requests/responses
- [ ] Deployment plan validated against production constraints

---

## Phase 2: Implementation

### Implementation Tasks

**I2.1: FastMCP SSE Service Setup**
- **Goal**: Create FastMCP service with SSE transport
- **Steps**:
  1. Create new Python package `mcp-gateway-sse`
  2. Install FastMCP and dependencies
  3. Implement SSE server with `/sse/` endpoint
  4. Add health check endpoint
  5. Configure logging and error handling
- **Files**: `mcp-gateway-sse/main.py`, `pyproject.toml`, `README.md`
- **Tests**: SSE connection test, long-lived connection test (10 min)

**I2.2: Implement Search Tool**
- **Goal**: Implement `search` tool with query parsing and result formatting
- **Steps**:
  1. Create search tool handler in FastMCP
  2. Implement query parser (extract symbols, data types)
  3. Call appropriate Binance gRPC tools
  4. Format results as search result array
  5. Return MCP content array with JSON-encoded results
- **Files**: `mcp-gateway-sse/tools/search.py`
- **Tests**: Unit tests for query parsing, integration test with Binance provider

**I2.3: Implement Fetch Tool**
- **Goal**: Implement `fetch` tool with document ID parsing and data retrieval
- **Steps**:
  1. Create fetch tool handler in FastMCP
  2. Implement document ID parser (parse type and parameters)
  3. Map document ID to Binance gRPC tool call
  4. Format full data with metadata
  5. Return MCP content array with JSON-encoded document
- **Files**: `mcp-gateway-sse/tools/fetch.py`
- **Tests**: Unit tests for ID parsing, integration tests for all 21 tool types

**I2.4: Implement Document ID Registry**
- **Goal**: Create registry mapping document IDs to Binance tool calls
- **Steps**:
  1. Define DocumentID dataclass with type and parameters
  2. Implement ID serialization/deserialization
  3. Create registry mapping ID patterns to tool call functions
  4. Add validation for all 21 tool types
- **Files**: `mcp-gateway-sse/document_registry.py`
- **Tests**: Unit tests for all document ID types

**I2.5: Implement Binance gRPC Client**
- **Goal**: Create client to call existing Binance gRPC provider
- **Steps**:
  1. Copy proto definitions from binance-rs
  2. Generate Python gRPC stubs
  3. Implement client with connection pooling
  4. Add retry logic and error handling
  5. Implement caching layer for ticker/orderbook (5 second TTL)
- **Files**: `mcp-gateway-sse/grpc_client.py`, `protos/*.proto`
- **Tests**: Integration tests with running Binance provider

**I2.6: Update Nginx Configuration**
- **Goal**: Configure nginx to proxy SSE endpoint
- **Steps**:
  1. Add location block for `/sse/` in nginx config
  2. Set SSE-specific headers (proxy_buffering off, chunked encoding)
  3. Configure long timeouts (3600s)
  4. Test SSE streaming through nginx
- **Files**: `infra/nginx-mcp-gateway.conf`
- **Tests**: Manual SSE connection test through nginx

**I2.7: Create Systemd Service for SSE Gateway**
- **Goal**: Deploy SSE gateway as systemd service
- **Steps**:
  1. Create systemd service file for mcp-gateway-sse
  2. Configure environment variables and working directory
  3. Set restart policy and logging
  4. Test service start/stop/restart
- **Files**: `infra/mcp-gateway-sse.service`
- **Tests**: Service deployment and health check

**I2.8: Update Deployment Scripts**
- **Goal**: Update deployment automation for SSE gateway
- **Steps**:
  1. Add SSE gateway to deploy.sh and deploy-quick.sh
  2. Install Python dependencies on server (uv)
  3. Deploy SSE service alongside Binance provider
  4. Update nginx and restart services
- **Files**: `infra/deploy.sh`, `infra/deploy-quick.sh`
- **Tests**: Full deployment to staging/production

### Implementation Completion Criteria
- [ ] SSE endpoint accessible at `https://mcp-gateway.thevibe.trading/sse/`
- [ ] search tool returns valid results for 20+ test queries
- [ ] fetch tool retrieves data for all 21 Binance tool types
- [ ] SSE connections remain stable for 30+ minutes
- [ ] Nginx properly proxies SSE with correct headers
- [ ] Services start automatically on server boot

---

## Phase 3: Testing & Validation

### Testing Tasks

**T3.1: ChatGPT Integration Test**
- **Goal**: Verify ChatGPT can connect and use the MCP connector
- **Steps**:
  1. Add connector in ChatGPT Developer Mode settings
  2. Test search query: "What is the current price of Bitcoin?"
  3. Verify fetch is called for detailed data
  4. Test deep research query with multiple tool calls
  5. Verify citations show correct URLs
- **Success**: ChatGPT successfully executes queries and displays results

**T3.2: Load Testing**
- **Goal**: Verify system handles 50+ concurrent connections
- **Steps**:
  1. Create load test script (50 concurrent SSE connections)
  2. Send search/fetch requests continuously for 10 minutes
  3. Monitor CPU, memory, connection count
  4. Verify no timeouts or errors
- **Success**: System maintains <2s response times under load

**T3.3: Error Handling Test**
- **Goal**: Verify graceful error handling for edge cases
- **Tests**:
  - Invalid document ID → clear error message
  - Binance API rate limit → retry with backoff
  - SSE connection drop → automatic reconnect
  - Invalid search query → empty results with helpful message
  - Non-existent symbol → error with suggestion
- **Success**: All errors return user-friendly messages

**T3.4: Backward Compatibility Test**
- **Goal**: Verify existing HTTP JSON-RPC clients still work
- **Steps**:
  1. Test existing test scripts (test_http_tools.sh, test_21_tools.sh)
  2. Verify all 21 tools still callable via HTTP
  3. Verify no performance degradation
- **Success**: All existing tests pass without modification

### Testing Completion Criteria
- [ ] ChatGPT connector successfully added and tested
- [ ] Load test passes with 50 concurrent connections
- [ ] All error scenarios handled gracefully
- [ ] Backward compatibility verified with existing tests

---

## Phase 4: Documentation & Deployment

### Documentation Tasks

**D4.1: ChatGPT Connection Guide**
- **Goal**: Document how to connect ChatGPT to the MCP server
- **Content**:
  - Step-by-step guide with screenshots
  - Example queries to try
  - Troubleshooting common issues
  - Rate limits and best practices
- **File**: `docs/CHATGPT_INTEGRATION.md`

**D4.2: API Reference**
- **Goal**: Document search and fetch tool APIs
- **Content**:
  - Tool descriptions and parameters
  - Response format with examples
  - Document ID schema reference
  - Error codes and messages
- **File**: `docs/API_REFERENCE.md`

**D4.3: Deployment Runbook**
- **Goal**: Document deployment and operations procedures
- **Content**:
  - Server setup requirements
  - Deployment steps (manual and automated)
  - Monitoring and logging
  - Troubleshooting guide
- **File**: `docs/DEPLOYMENT.md`

### Deployment Tasks

**DP4.1: Deploy to Production**
- **Goal**: Deploy SSE gateway to production server
- **Steps**:
  1. Run deploy-quick.sh to deploy SSE gateway
  2. Verify health check endpoint responds
  3. Test SSE connection through nginx
  4. Monitor logs for errors
- **Success**: Service running and accessible

**DP4.2: Post-Deployment Verification**
- **Goal**: Verify production deployment works end-to-end
- **Steps**:
  1. Connect ChatGPT to production endpoint
  2. Run smoke tests (3-5 queries)
  3. Check logs for errors
  4. Monitor performance metrics
- **Success**: ChatGPT successfully retrieves market data

### Documentation & Deployment Completion Criteria
- [ ] ChatGPT integration guide published
- [ ] API reference complete with examples
- [ ] Production deployment successful
- [ ] Post-deployment verification passed

---

## Risk Assessment

### High Risk
- **SSE connection stability**: Long-lived connections may have issues with nginx/network
  - Mitigation: Extensive timeout testing, implement reconnection logic
- **Query parsing accuracy**: Natural language queries may not map correctly to Binance tools
  - Mitigation: Build comprehensive test suite with 100+ queries, implement fuzzy matching

### Medium Risk
- **Binance API rate limits**: ChatGPT may trigger rate limits with many requests
  - Mitigation: Implement caching layer, add rate limit monitoring
- **Performance under load**: 50+ concurrent connections may strain system
  - Mitigation: Load testing, optimize gRPC connection pooling

### Low Risk
- **Backward compatibility**: New SSE endpoint may affect existing clients
  - Mitigation: Keep endpoints separate, comprehensive testing

---

## Dependencies

### External Dependencies
- FastMCP Python framework (for SSE transport)
- Python gRPC libraries (to call Binance provider)
- uv (Python package manager)
- Existing Binance provider gRPC service (must be running)

### Service Dependencies
- Binance provider must be running on port 50053 (gRPC mode)
- Nginx must be configured for SSE streaming
- HTTPS certificates must be valid

---

## Rollback Plan

If production deployment fails:
1. Stop mcp-gateway-sse.service: `systemctl stop mcp-gateway-sse`
2. Revert nginx config: `rm /etc/nginx/sites-enabled/mcp-gateway-sse`
3. Reload nginx: `systemctl reload nginx`
4. Verify existing HTTP JSON-RPC endpoint still works
5. Review logs to identify root cause

---

## Success Metrics

- ChatGPT connector successfully added within 1 minute
- Search results relevant for 95%+ of queries
- Fetch retrieves data for all 21 tool types
- SSE connections stable for 30+ minute conversations
- <2s response time for search queries
- <3s response time for fetch queries
- Error rate <1% during normal operation
- 50+ concurrent connections supported

---

## Timeline Estimate

- Phase 0 (Research): 4-6 hours
- Phase 1 (Design): 3-4 hours
- Phase 2 (Implementation): 12-16 hours
- Phase 3 (Testing): 4-6 hours
- Phase 4 (Documentation & Deployment): 3-4 hours

**Total**: 26-36 hours (3-5 days)

---

## Agent Context Updates

After research phase, update agent context with:
- FastMCP SSE transport patterns and examples
- Document ID schema specification
- Query parsing algorithm details
- Nginx SSE proxy configuration
- gRPC client implementation patterns
