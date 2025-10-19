# MCP Gateway System

Model Context Protocol (MCP) gateway for orchestrating multiple capability providers.

## Overview

This system implements a distributed MCP architecture where a Python gateway orchestrates multiple language-specific providers via gRPC. Each provider exposes tools, resources, and prompts through a unified interface.

## Architecture

```
AI Clients
    ├→ Claude Code (STDIO transport)
    │       ↓
    │   MCP Gateway (Python)
    │       ├→ hello-go (Go provider) - Demo tools
    │       ├→ hello-rs (Rust provider) - Demo tools
    │       └→ binance-rs (Rust provider) - Cryptocurrency trading
    │
    └→ ChatGPT (SSE transport) 🆕
            ↓
        SSE Gateway (Python)
            └→ binance-rs (Rust provider) - All 21 tools
```

### Production Deployment

**ChatGPT MCP Server** (SSE transport):
- **URL**: https://mcp-gateway.thevibe.trading/sse/
- **Status**: ✅ Live in production
- **Features**: Exposes all 21 Binance tools directly to ChatGPT
- **Transport**: Server-Sent Events (SSE) - required for ChatGPT integration
- **Documentation**: See [ChatGPT Integration Guide](providers/binance-rs/CHATGPT_INTEGRATION.md)

## Providers

### hello-go
- **Language**: Go
- **Port**: 50051
- **Tools**: echo.v1, sum.v1 (demo tools)
- **Status**: ✅ Implemented

### hello-rs
- **Language**: Rust
- **Port**: 50052
- **Tools**: (To be implemented)
- **Status**: 🚧 Planned

### binance-rs ⭐ **Production Ready**
- **Language**: Rust
- **Port**: 50053
- **Status**: ✅ **Fully Implemented** (All features complete)

**Capabilities**:
- **21 Tools**: Cryptocurrency trading & advanced analytics
  - 🔸 Market data (6 tools): Real-time ticker, order book, trades, klines, exchange info
  - 🔸 Account management (2 tools): Live balances, trade history
  - 🔸 Order execution (5 tools): Place, cancel, query orders
  - 🔸 OrderBook analysis (3 tools): WebSocket-powered L1/L2 metrics, health monitoring
  - 🔸 **Advanced Analytics (5 tools)**: 🆕
    - `get_recent_trades` - Recent trades with aggregation
    - `get_volume_profile` - Volume distribution analysis
    - `detect_market_anomalies` - Unusual market activity detection
    - `get_microstructure_health` - Spread, slippage, fragmentation metrics
    - `get_liquidity_vacuums` - Price level gap detection
- **4 Resources**: Markdown-formatted data snapshots (with **LIVE data**)
  - `binance://market/{SYMBOL}` - Real-time market summary
  - `binance://account/balances` - Current account balances
  - `binance://account/trades/{SYMBOL}` - Recent trade history
  - `binance://orders/{STATUS}` - Order tracking
- **2 Prompts**: AI-ready analysis templates (with **LIVE data**)
  - `trading-analysis` - Market analysis with real prices/volumes
  - `portfolio-risk` - Portfolio assessment with actual holdings

**Features**:
- ✅ Live WebSocket order book streaming (sub-200ms latency)
- ✅ Real-time market data from Binance API
- ✅ Lazy initialization & connection pooling
- ✅ Progressive disclosure (L1 → L2 depth on demand)
- ✅ Support for 20 concurrent symbol subscriptions
- ✅ Testnet & Production modes

See [providers/binance-rs/README.md](providers/binance-rs/README.md) for complete documentation.

## Quick Start

### Using with Claude Code 🤖

The MCP gateway integrates seamlessly with Claude Code. Once configured, you can ask natural language questions:

```
You: "What's the current Bitcoin price?"
Claude: [Uses binance.get_ticker tool]
Response: BTC is currently trading at $106,841.00, up 0.43% in the last 24h

You: "Show me the ETHUSDT order book metrics"
Claude: [Uses binance.orderbook_l1 tool via WebSocket]
Response: ETH/USDT spread: 2.5 bps, microprice: $4,125.50, imbalance: 62% bid

You: "Analyze trading opportunities for Bitcoin on the 1-hour timeframe"
Claude: [Uses trading-analysis prompt with live market data]
Response: [Detailed analysis with real prices, volumes, and order book depth]
```

**Setup for Claude Code**:
```bash
# Configure MCP server (already done in .claude/settings.json)
claude mcp add --transport stdio mcp-gateway -- \
  bash -c "cd mcp-gateway && uv run python -m mcp_gateway.main"

# Start your session
claude  # MCP gateway starts automatically
```

### Using with ChatGPT 🤖 🆕

The MCP gateway is now available for ChatGPT via SSE transport. All 21 Binance tools are accessible!

**Setup for ChatGPT**:
1. Enable **Developer Mode** in ChatGPT (Plus/Pro required)
2. Go to **Settings** → **MCP Servers** → **Add Server**
3. Configure the SSE endpoint:
   - **Server URL**: `https://mcp-gateway.thevibe.trading/sse/`
   - **Transport**: SSE (Server-Sent Events)

**Example Usage**:
```
You: "What's the current BTC price and order book health?"
ChatGPT: [Uses binance_get_ticker and binance_orderbook_l1 tools]
Response: Bitcoin is at $106,841.00 (+0.43%). Order book spread: 0.01%,
          bid-ask imbalance: 58% bid-heavy, microprice: $106,840.50

You: "Analyze volume profile for ETHUSDT"
ChatGPT: [Uses binance_get_volume_profile tool]
Response: [Detailed volume distribution analysis with POC, VAH, VAL]

You: "Detect any market anomalies for BTCUSDT"
ChatGPT: [Uses binance_detect_market_anomalies tool]
Response: [Analysis of unusual order book patterns, volume spikes, etc.]
```

See [CHATGPT_INTEGRATION.md](providers/binance-rs/CHATGPT_INTEGRATION.md) for detailed setup guide.

### Manual Setup

### Prerequisites

- Python 3.11+ with uv
- Go 1.21+ (for hello-go provider)
- Rust 1.75+ (for binance-rs provider)
- Protocol Buffers compiler (protoc)

### Build All Providers

```bash
# Generate protobuf stubs
make proto-gen

# Build Go provider
cd providers/hello-go && go build -o bin/hello-go cmd/server/main.go

# Build Rust providers
make build-binance
```

### Run the System

**Terminal 1 - Start providers**:
```bash
# hello-go
make run-hello-go

# binance-rs (in another terminal)
make run-binance
```

**Terminal 2 - Start gateway**:
```bash
make run-gateway
```

### Configuration

Configure providers in `mcp-gateway/providers.yaml`:

```yaml
providers:
  - name: hello-go
    type: grpc
    address: localhost:50051
    enabled: true

  - name: binance-rs
    type: grpc
    address: localhost:50053
    enabled: true
    metadata:
      description: "Binance cryptocurrency trading provider"
      features: ["orderbook"]
```

For binance-rs, also set environment variables:
```bash
export BINANCE_API_KEY="your_api_key"
export BINANCE_API_SECRET="your_api_secret"
export BINANCE_BASE_URL="https://testnet.binance.vision"
```

## Testing

```bash
# Run all tests
make test

# Test individual provider
cd providers/binance-rs && cargo test
```

## Development

### Adding a New Provider

1. Create `providers/{name}/` directory
2. Implement `provider.proto` gRPC service
3. Add to `providers.yaml`
4. Update Makefile with build/run targets

### Protocol Contract

All providers implement `/pkg/proto/provider.proto`:

- `ListCapabilities` - Discover tools/resources/prompts
- `Invoke` - Execute tools
- `ReadResource` - Fetch resources
- `GetPrompt` - Generate prompts
- `Stream` - Event streaming (optional)

## Project Structure

```
mcp-trader/
├── mcp-gateway/          # Python MCP gateway
│   ├── mcp_gateway/
│   │   ├── main.py      # FastMCP server (STDIO transport)
│   │   ├── sse_server.py # SSE server for ChatGPT 🆕
│   │   ├── adapters/    # gRPC clients
│   │   ├── tools/       # Search & fetch tools 🆕
│   │   └── validation.py
│   └── providers.yaml   # Provider configuration
├── providers/
│   ├── hello-go/        # Go demo provider
│   ├── hello-rs/        # Rust demo provider
│   └── binance-rs/      # Binance trading provider (21 tools)
│       └── src/
│           └── orderbook/
│               └── analytics/  # Advanced analytics module 🆕
├── infra/               # Production deployment 🆕
│   ├── deploy-chatgpt.sh        # ChatGPT SSE deployment script
│   ├── binance-provider.service # Systemd service
│   ├── mcp-gateway-sse.service  # SSE gateway service
│   └── nginx-mcp-gateway.conf   # NGINX reverse proxy
├── pkg/
│   ├── proto/           # Shared protobuf contracts
│   └── schemas/         # JSON schemas
└── Makefile
```

## Documentation

### Binance Provider
- [Binance Provider Guide](providers/binance-rs/README.md) - Complete provider documentation
- [ChatGPT Integration Guide](providers/binance-rs/CHATGPT_INTEGRATION.md) - SSE setup for ChatGPT 🆕
- [Integration Testing](providers/binance-rs/INTEGRATION_TESTS_COMPLETE.md) - Test results

### Gateway & Deployment
- [Manual Testing Guide](mcp-gateway/MANUAL_TESTING_GUIDE.md) - Testing SSE endpoints 🆕
- [Deploy Script](infra/deploy-chatgpt.sh) - Production deployment automation 🆕

### Specifications
- [Specification](specs/002-binance-provider-integration/spec.md)
- [Implementation Plan](specs/002-binance-provider-integration/plan.md)
- [Task Breakdown](specs/002-binance-provider-integration/tasks.md)

## License

See LICENSE file.
