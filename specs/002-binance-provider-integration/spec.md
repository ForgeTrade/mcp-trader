# Feature Specification: Binance Provider Integration

**Feature Branch**: `002-binance-provider-integration`
**Created**: 2025-10-18
**Status**: Draft
**Input**: User description: "а теперь интегрируй нового провайдера, используй вот этот mcp /home/limerc/repos/ForgeQuant/mcp-binance-rs и нужно превратить его в нашего провайдера"

## Overview

This feature integrates the existing mcp-binance-rs MCP server as a new provider in the MCP Gateway system. The Binance provider will expose cryptocurrency market data, account management, and trading capabilities through the gateway, enabling AI clients to access Binance's trading functionality through the Model Context Protocol.

## User Scenarios & Testing

### User Story 1 - Market Data Access (Priority: P1)

Users need to query real-time and historical cryptocurrency market data from Binance to make informed trading decisions or perform market analysis.

**Why this priority**: Market data is the foundation for all other trading features. Without access to prices, order books, and historical data, users cannot make informed decisions. This is the minimum viable feature that delivers immediate value.

**Independent Test**: Can be fully tested by querying ticker prices, order books, and candlestick data for any trading symbol (e.g., BTCUSDT) and verifying the data is current and accurate.

**Acceptance Scenarios**:

1. **Given** the Binance provider is running, **When** a user requests ticker data for BTCUSDT, **Then** the system returns current 24-hour price statistics including price, volume, and price change percentage
2. **Given** the Binance provider is connected, **When** a user requests order book data with a depth of 100 levels, **Then** the system returns bid and ask prices with quantities sorted by price
3. **Given** the Binance provider is accessible, **When** a user requests candlestick data for 1-hour intervals over the past day, **Then** the system returns OHLCV data for each interval
4. **Given** the Binance provider is available, **When** a user requests recent trades for ETHUSDT, **Then** the system returns a list of recent public trades with price, quantity, and timestamp

---

### User Story 2 - Account Information Retrieval (Priority: P2)

Users need to view their Binance account balances and trading history to monitor their portfolio and track trading performance.

**Why this priority**: After market data access, users need to see their current positions and account state. This enables portfolio monitoring and is required before executing trades. This story depends on API credential configuration.

**Independent Test**: Can be tested by configuring valid Binance API credentials and successfully retrieving account balance data and trade history.

**Acceptance Scenarios**:

1. **Given** valid Binance API credentials are configured, **When** a user requests account information, **Then** the system returns all asset balances with available and locked amounts
2. **Given** authenticated access to Binance, **When** a user requests trade history for BTCUSDT, **Then** the system returns a list of executed trades with price, quantity, commission, and timestamp
3. **Given** the user has an active Binance account, **When** API credentials are invalid or missing, **Then** the system returns a clear error message indicating authentication failure

---

### User Story 3 - Order Management (Priority: P3)

Users need to place, monitor, and cancel trading orders on Binance to execute their trading strategies.

**Why this priority**: Order execution is the final step in the trading workflow, built on top of market data and account access. This enables active trading but requires the foundation from P1 and P2 stories.

**Independent Test**: Can be tested by placing a small limit order, verifying it appears in open orders, then canceling it and confirming the cancellation.

**Acceptance Scenarios**:

1. **Given** valid API credentials and sufficient account balance, **When** a user places a limit buy order for BTCUSDT at a specified price and quantity, **Then** the system creates the order and returns an order ID
2. **Given** an existing open order, **When** a user requests to cancel the order by ID, **Then** the system cancels the order and confirms the cancellation
3. **Given** active and completed orders exist, **When** a user queries all open orders, **Then** the system returns only active orders with their current status
4. **Given** the user has order history, **When** a user requests all orders for BTCUSDT, **Then** the system returns both open and completed orders with full details

---

### User Story 4 - Real-Time Order Book Depth Analysis (Priority: P4)

Advanced users need real-time order book metrics and depth analysis to identify trading opportunities, detect market walls, and estimate slippage for large orders.

**Why this priority**: This is an advanced feature for sophisticated traders. While valuable, it's not essential for basic trading workflows and requires additional infrastructure (WebSocket connections).

**Independent Test**: Can be tested by enabling the orderbook feature, subscribing to depth updates for a symbol, and verifying that L1 metrics (spread, microprice) and L2 depth data are updated in real-time.

**Acceptance Scenarios**:

1. **Given** the orderbook feature is enabled, **When** a user requests L1 order book metrics for BTCUSDT, **Then** the system returns spread, mid-price, microprice, bid/ask imbalance, and detected walls
2. **Given** WebSocket depth tracking is active, **When** a user requests L2 order book depth with 20 levels, **Then** the system returns aggregated bid/ask levels with compact integer encoding for efficient transfer
3. **Given** order book subscriptions are running, **When** a user queries order book health status, **Then** the system reports connection status, last update timestamp, and data freshness for all tracked symbols

---

### Edge Cases

- What happens when Binance API rate limits are exceeded?
- How does the system handle network connectivity issues to Binance servers?
- What occurs when API credentials expire or are revoked during operation?
- How does the system respond when a user attempts to place an order with insufficient balance?
- What happens when a trading symbol is temporarily delisted or trading is halted?
- How does the system handle WebSocket disconnections for real-time order book data?
- What occurs when order book depth subscriptions exceed the maximum concurrent symbol limit (20)?

## Requirements

### Functional Requirements

#### Provider Integration (Core)

- **FR-001**: System MUST convert the mcp-binance-rs stdio MCP server into a gRPC provider that implements the provider.proto contract
- **FR-002**: System MUST expose all 16 tools from mcp-binance-rs through the gRPC provider interface
- **FR-003**: System MUST expose all 4 resources (market data, account balances, open orders) through the gRPC ReadResource RPC
- **FR-004**: System MUST expose all 2 prompts (trading analysis, portfolio risk) through the gRPC GetPrompt RPC
- **FR-005**: System MUST register the Binance provider in the gateway's providers.yaml configuration file

#### Tool Mapping (Market Data)

- **FR-006**: System MUST provide a tool to retrieve server time with client offset calculation
- **FR-007**: System MUST provide a tool to get 24-hour ticker statistics for any valid trading symbol
- **FR-008**: System MUST provide a tool to retrieve order book depth with configurable limit (default 100 levels)
- **FR-009**: System MUST provide a tool to fetch recent public trades with configurable limit
- **FR-010**: System MUST provide a tool to retrieve OHLCV candlestick data with configurable interval and limit
- **FR-011**: System MUST provide a tool to get current average price for a trading symbol

#### Tool Mapping (Account & Trading)

- **FR-012**: System MUST provide a tool to retrieve account information including all balances and permissions
- **FR-013**: System MUST provide a tool to fetch personal trade history for a symbol with pagination
- **FR-014**: System MUST provide a tool to place market and limit orders (BUY/SELL) with required order parameters
- **FR-015**: System MUST provide a tool to query order status by order ID and symbol
- **FR-016**: System MUST provide a tool to cancel an active order by order ID and symbol
- **FR-017**: System MUST provide a tool to list all open orders with optional symbol filtering
- **FR-018**: System MUST provide a tool to retrieve complete order history for a symbol

#### Tool Mapping (Order Book Depth - Optional)

- **FR-019**: System MUST provide a tool to get L1 order book metrics (spread, microprice, walls, imbalance) when orderbook feature is enabled
- **FR-020**: System MUST provide a tool to get L2 order book depth with 20 or 100 levels when orderbook feature is enabled
- **FR-021**: System MUST provide a tool to check order book service health and data freshness when orderbook feature is enabled

#### Configuration & Credentials

- **FR-022**: System MUST load Binance API credentials from environment variables (BINANCE_API_KEY, BINANCE_API_SECRET)
- **FR-023**: System MUST support configurable Binance base URL to enable testnet vs production switching
- **FR-024**: System MUST gracefully handle missing credentials for public endpoints (market data tools)
- **FR-025**: System MUST return clear authentication errors when credentials are required but not provided

#### Error Handling

- **FR-026**: System MUST convert Binance API errors to gRPC error responses with user-friendly messages
- **FR-027**: System MUST handle Binance rate limit errors and include retry-after information when available
- **FR-028**: System MUST validate tool parameters against JSON schemas before invoking Binance API
- **FR-029**: System MUST never expose sensitive data (API secrets, raw error traces) in error messages
- **FR-030**: System MUST return structured error responses for network failures, timeouts, and API unavailability

#### Protocol Translation

- **FR-031**: System MUST convert JSON Schema tool definitions from mcp-binance-rs to protobuf Json message format
- **FR-032**: System MUST serialize tool parameters and results as JSON bytes in the gRPC payload
- **FR-033**: System MUST convert MCP resource URIs (binance://market/btcusdt) to gRPC ResourceRequest format
- **FR-034**: System MUST convert MCP prompt parameter schemas to protobuf Json format in PromptRequest
- **FR-035**: System MUST maintain correlation IDs for distributed tracing across gateway and provider

### Key Entities

- **Binance Provider**: A gRPC service that wraps the mcp-binance-rs functionality, implementing the Provider service contract with ListCapabilities, Invoke, ReadResource, and GetPrompt RPCs
- **Trading Symbol**: A cryptocurrency trading pair identifier (e.g., BTCUSDT, ETHUSDT) used across all market data and trading tools
- **Tool Definition**: Metadata describing a tool's name, description, and JSON schema for input parameters (16 tools total)
- **Resource**: A URI-addressable data endpoint (binance://market/{symbol}, binance://account/balances) that returns formatted content
- **Prompt Template**: A parameterized template for generating AI prompts with variable substitution (trading_analysis, portfolio_risk)
- **API Credentials**: Binance API key and secret pair required for authenticated endpoints (account, trading, orders)
- **Order**: A trading order with attributes including symbol, side (BUY/SELL), type (LIMIT/MARKET), price, quantity, and status
- **Order Book**: Real-time market depth data with bid/ask levels, prices, quantities, and aggregated metrics
- **Candlestick**: OHLCV (Open, High, Low, Close, Volume) data for a specific time interval used in technical analysis

## Success Criteria

### Measurable Outcomes

- **SC-001**: Users can retrieve real-time market data (ticker, order book, trades) for any trading symbol with response times under 2 seconds
- **SC-002**: Users can successfully authenticate and retrieve account balances when valid API credentials are configured
- **SC-003**: Users can place, query, and cancel orders through the gateway with end-to-end latency under 3 seconds
- **SC-004**: The Binance provider successfully starts and registers all 16 tools with the gateway on initialization
- **SC-005**: Gateway can route tool invocations to the Binance provider and return results with proper JSON schema validation
- **SC-006**: Real-time order book metrics are updated with latency under 200ms when WebSocket feature is enabled
- **SC-007**: System gracefully handles Binance API errors and rate limits without exposing sensitive information
- **SC-008**: All 4 resource endpoints return properly formatted content when queried through the gateway
- **SC-009**: Both prompt templates successfully generate AI-ready prompts with correct parameter substitution

## Assumptions

- The mcp-binance-rs codebase is stable and maintained, using the rmcp SDK v0.8.1 for MCP protocol support
- Binance Testnet API is available and compatible with the production API contract for development and testing
- Users will provide their own Binance API credentials through environment variables
- The gRPC provider will run as a separate process, similar to the hello-go provider architecture
- JSON schema validation will occur at the gateway level before invoking provider tools
- The orderbook feature with WebSocket support is optional and can be enabled via feature flag compilation
- Network latency to Binance servers is reasonable (under 500ms for REST API calls)
- The Rust-based provider will be compiled to a native binary and executed as a subprocess by the gateway
