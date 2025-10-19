# Feature Specification: ChatGPT MCP Connector Integration

**Feature Branch**: `005-specify-scripts-bash`
**Created**: 2025-10-19
**Status**: Draft
**Input**: User description: "сейчас не работает подключение к нашему mcp gateway через mcp. Вот прочитай доку https://platform.openai.com/docs/mcp и исправь эту проблему."

## Problem Statement

The current Binance MCP provider cannot connect to ChatGPT Developer Mode because:

1. **Transport incompatibility**: ChatGPT requires SSE (Server-Sent Events) transport, but our server only supports HTTP with JSON-RPC
2. **Tool schema mismatch**: ChatGPT expects exactly two tools (`search` and `fetch`) with specific response formats, while our server exposes 21 Binance-specific tools
3. **Response format differences**: ChatGPT expects MCP content arrays with specific structure, different from our current JSON-RPC responses

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Connect Binance Data to ChatGPT (Priority: P1)

A trader wants to use ChatGPT to ask questions about cryptocurrency market data from Binance and get AI-powered insights using the Binance MCP tools.

**Why this priority**: This is the core functionality - without SSE transport and proper tool mapping, ChatGPT cannot connect to the MCP server at all.

**Independent Test**: Can be fully tested by configuring the MCP connector in ChatGPT Developer Mode settings and successfully making a search query for "BTCUSDT price".

**Acceptance Scenarios**:

1. **Given** the MCP server is running with SSE transport, **When** a user adds the connector in ChatGPT settings using the SSE endpoint URL, **Then** ChatGPT successfully connects and lists available tools
2. **Given** the connector is added in ChatGPT, **When** a user asks "What is the current price of Bitcoin?", **Then** ChatGPT uses the `search` tool to find relevant market data
3. **Given** the `search` tool returns results, **When** ChatGPT needs detailed data, **Then** it calls the `fetch` tool to retrieve complete ticker information

---

### User Story 2 - Search Cryptocurrency Market Data (Priority: P1)

A user wants to search for cryptocurrency market information using natural language queries through ChatGPT.

**Why this priority**: The `search` tool is mandatory for ChatGPT connector integration and enables discovery of relevant market data.

**Independent Test**: Can be tested by calling the `search` tool directly with query "Bitcoin price" and verifying it returns properly formatted results.

**Acceptance Scenarios**:

1. **Given** a user query "Bitcoin trading volume", **When** the `search` tool is called, **Then** it returns a list of results including ticker data for BTC pairs
2. **Given** a search for "ETHUSDT", **When** the tool executes, **Then** results include document IDs, titles describing the data type, and URLs for citation
3. **Given** multiple matching symbols, **When** search is executed, **Then** results are ranked by relevance (e.g., exact matches first)

---

### User Story 3 - Fetch Detailed Market Information (Priority: P1)

A user needs complete details about a specific cryptocurrency or market data point after finding it through search.

**Why this priority**: The `fetch` tool completes the search-and-retrieve pattern required by ChatGPT, providing full data for analysis.

**Independent Test**: Can be tested by fetching a specific document ID (e.g., "ticker:BTCUSDT") and verifying complete ticker data is returned.

**Acceptance Scenarios**:

1. **Given** a document ID from search results, **When** the `fetch` tool is called with that ID, **Then** complete market data is returned with proper citation URL
2. **Given** an orderbook document ID, **When** fetch is executed, **Then** full orderbook data including bids, asks, and spread is returned
3. **Given** a non-existent document ID, **When** fetch is called, **Then** a clear error message is returned

---

### User Story 4 - Use Deep Research with Market Data (Priority: P2)

A trader wants to use ChatGPT's deep research feature to analyze market trends across multiple cryptocurrency pairs using Binance data.

**Why this priority**: Enables advanced use case of deep research, but basic connectivity (P1) must work first.

**Independent Test**: Can be tested by running a deep research query "Analyze Bitcoin vs Ethereum market trends" and verifying multiple tool calls are made.

**Acceptance Scenarios**:

1. **Given** a deep research query about market trends, **When** ChatGPT executes research, **Then** it makes multiple search and fetch calls to gather comprehensive data
2. **Given** orderbook analytics are needed, **When** fetch retrieves orderbook health data, **Then** the analysis includes liquidity and spread metrics
3. **Given** volume profile data is needed, **When** fetch is called for analytics, **Then** POC (Point of Control) and value area data is included

---

### Edge Cases

- What happens when the SSE connection is interrupted mid-query?
- How does the system handle searches that match no Binance trading pairs?
- What happens if Binance API rate limits are hit during a ChatGPT query?
- How are document IDs formatted to avoid conflicts between different data types (ticker vs orderbook vs analytics)?
- What happens when a user tries to search for expired kline/candlestick data?
- How does the system handle concurrent ChatGPT requests to the same MCP server?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST support SSE (Server-Sent Events) transport on a dedicated endpoint (e.g., `/sse/`)
- **FR-002**: System MUST implement a `search` tool that accepts a query string and returns results in ChatGPT-compatible format
- **FR-003**: System MUST implement a `fetch` tool that accepts a document ID and returns complete data in ChatGPT-compatible format
- **FR-004**: The `search` tool MUST return results as a JSON object with a `results` array containing objects with `id`, `title`, `text`, and `url` fields
- **FR-005**: The `fetch` tool MUST return a JSON object with `id`, `title`, `text`, `url`, and optional `metadata` fields
- **FR-006**: Both tools MUST return responses as MCP content arrays with `type: "text"` and JSON-encoded `text` field
- **FR-007**: System MUST map cryptocurrency queries to appropriate Binance tools (e.g., "Bitcoin price" → binance.get_ticker for BTCUSDT)
- **FR-008**: Document IDs MUST uniquely identify data sources (e.g., `ticker:BTCUSDT`, `orderbook:ETHUSDT`, `klines:BTCUSDT:1h`)
- **FR-009**: Search results MUST include snippet text (first 200 characters) from the full data
- **FR-010**: URLs in responses MUST point to the deployed MCP server endpoint for proper citation (e.g., `https://mcp-gateway.thevibe.trading/data/{id}`)
- **FR-011**: System MUST maintain existing HTTP JSON-RPC endpoint for backward compatibility
- **FR-012**: Search MUST support queries for symbol names, trading pairs, and market data types
- **FR-013**: System MUST support unauthenticated access for initial deployment (OAuth authentication will be added in a future iteration)
- **FR-014**: Fetch MUST retrieve real-time data from Binance API when called
- **FR-015**: System MUST log all search and fetch operations for debugging and monitoring

### Non-Functional Requirements

- **NFR-001**: SSE endpoint MUST handle long-lived connections (up to 1 hour) without timeout
- **NFR-002**: Search results MUST be returned within 2 seconds
- **NFR-003**: Fetch operations MUST complete within 3 seconds
- **NFR-004**: System MUST support at least 50 concurrent SSE connections
- **NFR-005**: Error messages MUST be clear and actionable for end users

### Key Entities

- **SearchResult**: Represents a single match from a search query
  - Key attributes: id (unique identifier), title (human-readable name), text (200-char snippet), url (citation link)
  - Used to provide overview of available data to ChatGPT

- **Document**: Complete data for a specific market data point
  - Key attributes: id (unique identifier), title (human-readable name), text (full data as JSON or formatted text), url (citation link), metadata (optional key-value pairs)
  - Document types include: ticker data, orderbook snapshot, klines/candlestick data, analytics results

- **MCPContent**: MCP protocol content item
  - Key attributes: type (always "text" for search/fetch), text (JSON-encoded payload)
  - Wrapper format required by MCP specification for tool responses

- **SSEConnection**: Long-lived server-sent events connection
  - Manages streaming communication with ChatGPT
  - Handles tool calls and responses over persistent connection

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can successfully add the MCP connector in ChatGPT Developer Mode settings within 1 minute
- **SC-002**: ChatGPT can discover and call both `search` and `fetch` tools without errors
- **SC-003**: Search queries return relevant results for 95% of valid cryptocurrency symbol queries
- **SC-004**: Deep research queries successfully retrieve and analyze data from multiple Binance tools
- **SC-005**: SSE connections remain stable for the duration of ChatGPT conversations (up to 30 minutes typical)
- **SC-006**: System handles 100 concurrent ChatGPT users without performance degradation
- **SC-007**: Error rates for tool calls are below 1% during normal operation
- **SC-008**: Documentation allows new users to connect to ChatGPT within 5 minutes

## Technical Context

### Current Architecture

- Binance provider runs in HTTP mode on port 3000
- Nginx reverse proxy at `https://mcp-gateway.thevibe.trading`
- 21 Binance-specific tools exposed (ticker, orderbook, trading, analytics)
- JSON-RPC protocol over HTTP POST to `/mcp` endpoint

### Required Changes

1. **Add SSE Transport Layer**
   - Implement `/sse/` endpoint for Server-Sent Events
   - Handle long-lived connections with proper keep-alive
   - Stream MCP messages using SSE format

2. **Implement Search/Fetch Adapter**
   - Map search queries to Binance tools (e.g., ticker search, symbol lookup)
   - Generate document IDs that reference specific Binance data
   - Format responses according to MCP content array specification

3. **Document ID Schema**
   - `ticker:{SYMBOL}` - e.g., `ticker:BTCUSDT`
   - `orderbook:{SYMBOL}` - e.g., `orderbook:ETHUSDT`
   - `orderbook_l1:{SYMBOL}` - Level 1 orderbook metrics
   - `orderbook_l2:{SYMBOL}` - Level 2 orderbook metrics
   - `klines:{SYMBOL}:{INTERVAL}` - e.g., `klines:BTCUSDT:1h`
   - `analytics:{TYPE}:{SYMBOL}` - e.g., `analytics:order_flow:BTCUSDT`

4. **Maintain Compatibility**
   - Keep existing HTTP JSON-RPC endpoint operational
   - Existing MCP clients continue to work unchanged
   - SSE transport is additive, not replacing

## Assumptions

- Users have access to ChatGPT Plus or Pro with Developer Mode enabled
- The MCP server URL (https://mcp-gateway.thevibe.trading) is publicly accessible
- SSL/TLS certificates are properly configured for HTTPS
- Binance API has no rate limiting issues for typical query volumes
- Search queries will primarily be in English
- Symbol names follow Binance conventions (e.g., BTCUSDT, ETHBTC)
- OAuth authentication can be added in a later iteration if needed

## Out of Scope

- Building a graphical UI for MCP server management
- Implementing write actions (trading operations) for ChatGPT
- Historical data analysis beyond current market state
- Multi-exchange support (only Binance for now)
- Advanced authentication beyond basic OAuth (can be added in future iterations)
- Custom data visualizations in ChatGPT responses
- Caching layer for frequently requested data
- Rate limiting per ChatGPT user

## Dependencies

- Python with FastMCP framework (or equivalent) for SSE transport implementation
- Existing Binance provider binary with all 21 tools
- HTTPS endpoint at mcp-gateway.thevibe.trading
- Systemd service for process management
- Nginx configuration for SSE proxy (requires special headers for streaming)

## Security Considerations

- All search queries may contain sensitive trading strategies - must not log query content
- OAuth tokens must be stored securely if authentication is implemented
- Rate limiting should be applied per connection to prevent abuse
- SSE connections should have maximum lifetime (1 hour) to prevent resource exhaustion
- Document IDs should not expose internal system information
- Error messages should not reveal system internals or API keys
