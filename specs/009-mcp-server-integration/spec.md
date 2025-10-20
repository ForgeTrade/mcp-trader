# Feature Specification: MCP Server Integration

**Feature Branch**: `009-mcp-server-integration`
**Created**: 2025-10-20
**Status**: Draft
**Input**: User description: "Integrate MCP server features from mcp-binance-rs project into binance provider"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - MCP Protocol Support via Stdio (Priority: P1)

AI agents can connect to the Binance provider using the Model Context Protocol (MCP) via stdio transport, enabling direct integration with Claude Desktop and other MCP-compatible clients.

**Why this priority**: This is the foundation for MCP integration. Without proper MCP protocol support, none of the other features (SSE, resources, prompts) will work. This delivers immediate value by enabling Claude Desktop integration.

**Independent Test**: Can be fully tested by configuring Claude Desktop to connect via stdio, listing available tools, and executing a simple tool like `get_ticker`. Success means the provider responds to MCP protocol messages and executes tools correctly.

**Acceptance Scenarios**:

1. **Given** the Binance provider is running in stdio mode, **When** an MCP client sends an `initialize` request, **Then** the server responds with proper capabilities (tools, prompts, resources) and version information
2. **Given** an initialized MCP connection, **When** the client calls `tools/list`, **Then** all available Binance tools are returned in MCP format
3. **Given** an initialized MCP connection, **When** the client calls `tools/call` with valid parameters, **Then** the tool executes and returns results in MCP format
4. **Given** an initialized MCP connection, **When** the client sends invalid tool parameters, **Then** proper MCP error responses are returned with descriptive messages

---

### User Story 2 - SSE Transport for Remote Access (Priority: P2)

Operators can deploy the Binance provider to a remote server and access it via HTTPS using Server-Sent Events (SSE) transport, enabling secure remote connections from anywhere.

**Why this priority**: SSE transport enables remote deployment scenarios and cloud hosting. While valuable, the core functionality works with stdio (P1), making this an enhancement for production deployments.

**Independent Test**: Can be tested by starting the provider with SSE transport enabled, connecting via HTTPS from a remote client, and executing tools. Success means the provider handles SSE connections, manages sessions, and routes requests correctly.

**Acceptance Scenarios**:

1. **Given** the provider is running with SSE transport enabled, **When** a client sends GET request to `/mcp/sse`, **Then** an SSE connection is established and a connection ID is returned
2. **Given** an active SSE connection, **When** the client sends POST request to `/mcp/message` with connection ID header, **Then** MCP messages are processed and responses are streamed via SSE
3. **Given** multiple concurrent SSE connections, **When** each client sends independent requests, **Then** sessions are isolated and responses are routed to correct clients
4. **Given** an SSE connection idle for 30 seconds, **When** no new messages are sent, **Then** the session times out and resources are cleaned up

---

### User Story 3 - MCP Resources for Efficient Data Access (Priority: P3)

AI agents can read frequently-accessed market data through MCP resources (e.g., `binance://market/btcusdt`, `binance://account/balances`), reducing latency and improving performance for repeated queries.

**Why this priority**: Resources optimize data access patterns but are not essential for basic functionality. Agents can still get data via tool calls, making this a performance optimization.

**Independent Test**: Can be tested by calling `resources/list` to see available resources, then reading a resource like `binance://market/btcusdt`. Success means resources return properly formatted data (markdown for market data, tables for balances).

**Acceptance Scenarios**:

1. **Given** an initialized MCP connection, **When** the client calls `resources/list`, **Then** all available Binance resources are returned (market data, account balances, open orders)
2. **Given** the resource list is available, **When** the client reads `binance://market/btcusdt`, **Then** real-time market data is returned in markdown format
3. **Given** API credentials are configured, **When** the client reads `binance://account/balances`, **Then** current account balances are returned in markdown table format
4. **Given** API credentials are configured, **When** the client reads `binance://orders/open`, **Then** active orders are returned in markdown table format

---

### User Story 4 - MCP Prompts for Trading Analysis (Priority: P3)

AI agents can use pre-defined prompts (e.g., `trading_analysis`, `portfolio_risk`, `market_microstructure_analysis`) to receive guided analysis workflows with structured recommendations.

**Why this priority**: Prompts enhance the AI experience but are not required for core functionality. All analysis can still be done through direct tool calls, making this a UX enhancement.

**Independent Test**: Can be tested by listing prompts, then requesting a prompt like `trading_analysis` with a symbol parameter. Success means the prompt returns a structured analysis message that guides the AI to use appropriate tools.

**Acceptance Scenarios**:

1. **Given** an initialized MCP connection, **When** the client calls `prompts/list`, **Then** all available analysis prompts are returned with descriptions
2. **Given** the prompt list is available, **When** the client requests `trading_analysis` prompt with symbol "BTCUSDT", **Then** a structured prompt message is returned guiding comprehensive market analysis
3. **Given** API credentials are configured, **When** the client requests `portfolio_risk` prompt, **Then** a structured prompt message is returned guiding portfolio risk assessment
4. **Given** orderbook_analytics feature is enabled, **When** the client requests `market_microstructure_analysis` prompt, **Then** a structured prompt message is returned guiding advanced microstructure analysis

---

### User Story 5 - Shuttle.dev Cloud Deployment (Priority: P4)

DevOps engineers can deploy the Binance provider to Shuttle.dev cloud platform with automatic HTTPS, secret management, and zero SSL configuration.

**Why this priority**: Shuttle integration is specific to one deployment platform. The provider can run anywhere with SSE transport (P2), making Shuttle support a convenience feature for a specific hosting choice.

**Independent Test**: Can be tested by running `shuttle deploy`, configuring secrets, and verifying the deployed service responds to SSE requests. Success means the provider runs on Shuttle with automatic HTTPS and secret injection.

**Acceptance Scenarios**:

1. **Given** Shuttle.dev account and CLI are configured, **When** operator runs `shuttle deploy`, **Then** the provider is deployed with automatic HTTPS endpoint
2. **Given** secrets are configured in Shuttle, **When** the provider starts, **Then** Binance API credentials are loaded from secret store
3. **Given** the provider is deployed on Shuttle, **When** a client connects to the HTTPS endpoint, **Then** SSE transport works over TLS
4. **Given** multiple deployments exist, **When** operator runs `shuttle deploy`, **Then** the new version replaces the old deployment with zero downtime

---

### Edge Cases

- What happens when an SSE client disconnects abruptly (network failure, browser close)?
  - Session manager detects disconnection and cleans up resources within 30 seconds
  - WebSocket connections to Binance are not shared across sessions, so cleanup is isolated

- What happens when the connection limit (50 concurrent SSE sessions) is reached?
  - New connection attempts receive 503 Service Unavailable with retry-after header
  - Clients should implement exponential backoff and retry logic

- What happens when Shuttle secrets are not configured?
  - Provider starts with warning logs but continues operation
  - Authenticated tools (account, trading) return error responses indicating missing credentials
  - Public tools (market data) continue to work normally

- What happens when a client sends MCP messages to an expired session ID?
  - Server returns 404 Not Found error with descriptive message
  - Client must re-establish SSE connection to get new session ID

- What happens when multiple clients try to manage the same orderbook subscription?
  - Orderbook manager uses reference counting to share WebSocket connections
  - Subscription is maintained as long as at least one session needs it
  - Last session to unsubscribe triggers WebSocket cleanup

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Provider MUST implement the MCP ServerHandler trait from rmcp SDK version 0.8.1
- **FR-002**: Provider MUST support stdio transport for local MCP connections
- **FR-003**: Provider MUST support SSE transport for remote HTTPS MCP connections
- **FR-004**: Provider MUST handle MCP protocol version 2024-11-05
- **FR-005**: Provider MUST advertise three capabilities: tools, prompts, and resources
- **FR-006**: Provider MUST implement tools/list endpoint returning all Binance tools in MCP format
- **FR-007**: Provider MUST implement tools/call endpoint executing Binance operations
- **FR-008**: Provider MUST implement resources/list endpoint returning available data resources
- **FR-009**: Provider MUST implement resources/read endpoint returning resource contents in markdown
- **FR-010**: Provider MUST implement prompts/list endpoint returning available analysis prompts
- **FR-011**: Provider MUST implement prompts/get endpoint returning prompt messages with parameters
- **FR-012**: Provider MUST manage SSE session lifecycle with 30-second idle timeout
- **FR-013**: Provider MUST limit concurrent SSE connections to 50 sessions
- **FR-014**: Provider MUST support Shuttle.dev deployment with automatic HTTPS and secret management
- **FR-015**: Provider MUST support command-line flag to choose transport mode (stdio or SSE)
- **FR-016**: Provider MUST validate connection IDs for SSE requests and return 404 for invalid sessions
- **FR-017**: Provider MUST return proper MCP error codes for validation failures, rate limits, and auth errors
- **FR-018**: Provider MUST clean up WebSocket subscriptions when SSE sessions end
- **FR-019**: Provider MUST load Binance API credentials from environment variables or Shuttle secrets
- **FR-020**: Provider MUST format resource data as markdown (text/markdown MIME type)

### Key Entities *(include if feature involves data)*

- **MCP Session**: Represents an active connection between client and provider via SSE transport. Contains connection ID, creation timestamp, last activity time, and associated WebSocket subscriptions.

- **MCP Resource**: Represents a data endpoint accessible via URI (e.g., `binance://market/btcusdt`). Contains URI, name, description, MIME type, and rendering logic to format data as markdown.

- **MCP Prompt**: Represents a guided analysis workflow template (e.g., `trading_analysis`, `portfolio_risk`). Contains name, description, parameters, and template messages that guide AI through specific analysis patterns.

- **Transport Mode**: Represents the communication protocol used (stdio or SSE). Determines how MCP messages are received and responses are sent. Configured at startup via command-line flags.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Claude Desktop can connect to the provider via stdio transport and execute all tools within 2 seconds of connection
- **SC-002**: Provider handles 50 concurrent SSE connections without exceeding 500ms P95 response latency
- **SC-003**: SSE sessions timeout and clean up resources within 35 seconds of last activity (30s timeout + 5s cleanup)
- **SC-004**: Shuttle deployment completes in under 5 minutes from `shuttle deploy` command to HTTPS endpoint availability
- **SC-005**: Resource reads return formatted markdown data in under 200ms P95 latency
- **SC-006**: Prompt requests return structured guidance messages in under 100ms
- **SC-007**: Provider maintains 99.9% uptime on Shuttle.dev platform over 30-day period
- **SC-008**: MCP protocol errors include actionable recovery suggestions that reduce retry failures by 80%

## Assumptions

- **AS-001**: The rmcp SDK version 0.8.1 provides stable MCP protocol implementation with no breaking changes expected
- **AS-002**: SSE transport is sufficient for remote access; WebSocket transport is not required
- **AS-003**: 50 concurrent SSE sessions is adequate for expected usage patterns
- **AS-004**: 30-second idle timeout balances resource usage with user experience
- **AS-005**: Shuttle.dev platform provides reliable infrastructure with <0.1% downtime
- **AS-006**: Markdown formatting is acceptable for all resource data (no need for JSON, CSV, or other formats)
- **AS-007**: Existing orderbook WebSocket implementation can be reused without modification
- **AS-008**: Existing tool implementations can be wrapped with MCP protocol layer without changes to business logic
- **AS-009**: Environment variable naming conventions (BINANCE_API_KEY, BINANCE_SECRET_KEY) are consistent across deployment methods

## Dependencies

- **DEP-001**: Requires rmcp crate version 0.8.1 for MCP SDK functionality
- **DEP-002**: Requires axum crate for HTTP server (SSE endpoints)
- **DEP-003**: Requires shuttle-runtime and shuttle-axum for Shuttle.dev deployment
- **DEP-004**: Requires existing orderbook manager for WebSocket subscription management
- **DEP-005**: Requires existing tool implementations (market data, analytics, trading)

## Out of Scope

- **OOS-001**: WebSocket transport for MCP (only stdio and SSE are supported)
- **OOS-002**: Authentication/authorization beyond Binance API credentials (no user auth for MCP access)
- **OOS-003**: Rate limiting per MCP session (rate limiting is global per Binance API limits)
- **OOS-004**: Persistent session recovery after server restart
- **OOS-005**: Resource subscriptions with push notifications (resources are pull-only)
- **OOS-006**: Custom prompt templates defined by users (only pre-defined prompts)
- **OOS-007**: Multi-exchange support (only Binance in this feature)
- **OOS-008**: Metrics/observability for MCP-specific events (general logging only)

