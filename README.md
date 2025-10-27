# MCP Gateway System

[![Build and Push](https://github.com/forgequant/mcp-gateway/actions/workflows/build-and-push.yml/badge.svg)](https://github.com/forgequant/mcp-gateway/actions/workflows/build-and-push.yml)

Model Context Protocol (MCP) gateway for orchestrating multiple capability providers.

## Breaking Changes

**⚠️ Version 0.2.0 (2025-10-24) - System Now Read-Only**

This system has been transformed from a hybrid read/write trading client into a **read-only market data analysis tool**. All order management functionality has been removed.

**What Changed**:
- ❌ All order placement, cancellation, and query methods removed
- ❌ Account information and trade history retrieval removed
- ✅ Unified market data report API introduced
- ✅ All market data analysis features preserved and enhanced

See [CHANGELOG.md](CHANGELOG.md) for complete migration guide.

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
            └→ binance-rs (Rust provider) - Unified market data reports
```

### Production Deployment

**ChatGPT MCP Server** (SSE transport):
- **URL**: https://mcp-gateway.thevibe.trading/sse/
- **Status**: ✅ Live in production
- **Version**: 0.2.0 (Read-only market data analysis)
- **Features**: Unified market intelligence reports with advanced analytics
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
- **Status**: ✅ **Fully Implemented** (Read-only market data analysis)
- **Version**: 0.2.0

**Capabilities**:
- **1 Primary Tool**: 🆕 **Unified Market Data Report**
  - `generate_market_report` - Comprehensive market intelligence combining 8+ data sections
    - Price Overview (24h statistics with trend indicators)
    - Order Book Metrics (spread, microprice, imbalance)
    - Liquidity Analysis (walls, volume profile, vacuums)
    - Market Anomalies (detection with severity badges)
    - Microstructure Health (health scores and component status)
    - Data Health Status (WebSocket connectivity, freshness)
    - Performance: 60s caching, <500ms cold generation, <3ms cached

- **11 Individual Analytics Tools** (legacy, superseded by unified report):
  - `get_ticker` - 24h price statistics
  - `orderbook_l1` / `orderbook_l2` - Order book snapshots
  - `get_recent_trades` - Trade history
  - `get_order_flow` - Bid/ask pressure tracking
  - `get_volume_profile` - Volume distribution (POC/VAH/VAL)
  - `detect_market_anomalies` - Unusual pattern detection
  - `get_microstructure_health` - Market health scoring
  - `get_liquidity_vacuums` - Low-volume price zone detection
  - `orderbook_health` - Real-time order book health metrics

- **1 Resource**: Markdown-formatted data snapshot
  - `binance://market/{SYMBOL}` - Real-time market summary

- **1 Prompt**: AI-ready analysis template
  - `trading-analysis` - Market analysis with live context

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
You: "Generate a comprehensive market report for Bitcoin"
Claude: [Uses binance.generate_market_report tool]
Response:
# Market Data Report - BTCUSDT
Generated: 2025-10-24 12:00:00 UTC 🟢

## Price Overview
- Current Price: $106,841.00 📈
- 24h Change: +0.43% (+$459.00)
- 24h High/Low: $107,200.00 / $105,800.00
- Volume: 1,234.56 BTC ($131.8M)

## Order Book Metrics
- Spread: 0.01% (1.0 bps)
- Microprice: $106,840.50
- Imbalance: 58% bid-heavy 📊

## Liquidity Analysis
- Buy Wall: $106,500 💪 Strong (125.5 BTC)
- Volume Profile POC: $106,750
[... 5 more sections ...]

You: "Show me ETHUSDT with just price and liquidity sections"
Claude: [Uses generate_market_report with custom options]
Response: [Markdown report with only requested sections]

You: "Analyze trading opportunities for Bitcoin"
Claude: [Uses trading-analysis prompt with live market data]
Response: [Detailed analysis combining report data with AI insights]
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

The MCP gateway is now available for ChatGPT via SSE transport. The unified market data report provides comprehensive market intelligence!

**Setup for ChatGPT**:
1. Enable **Developer Mode** in ChatGPT (Plus/Pro required)
2. Go to **Settings** → **MCP Servers** → **Add Server**
3. Configure the SSE endpoint:
   - **Server URL**: `https://mcp-gateway.thevibe.trading/sse/`
   - **Transport**: SSE (Server-Sent Events)

**Example Usage**:
```
You: "Generate a full market report for BTCUSDT"
ChatGPT: [Uses binance_generate_market_report tool]
Response: [Complete markdown report with 8 sections: price, orderbook,
          liquidity, anomalies, health, etc. - cached for 60s]

You: "Show me just the price and liquidity analysis for ETHUSDT"
ChatGPT: [Uses binance_generate_market_report with options]
Request: {"symbol": "ETHUSDT", "options": {"include_sections": ["price_overview", "liquidity_analysis"]}}
Response: [Customized report with only requested sections]

You: "What's the order book health for BTCUSDT?"
ChatGPT: [Uses binance_orderbook_health tool - legacy individual tool]
Response: [Quick health metrics snapshot]
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

## API Reference

### Unified Market Data Report

**Tool**: `generate_market_report`

Generates comprehensive market intelligence reports combining 8+ data sections with smart caching and customization options.

**Parameters**:
```python
{
  "symbol": str,              # Required: Trading pair (e.g., "BTCUSDT", "ETHUSDT")
  "venue": str,               # Optional: Exchange venue (default: "binance")
  "options": {                # Optional: Report customization
    "include_sections": list[str],  # Sections to include (default: all)
    "volume_window_hours": int,     # Volume analysis window (default: 24)
    "orderbook_levels": int         # Order book depth (default: 20)
  }
}
```

**Available Sections**:
- `"price_overview"` - 24h price statistics with trend indicators
- `"orderbook_metrics"` - Spread, microprice, bid-ask imbalance
- `"liquidity_analysis"` - Liquidity walls, volume profile, vacuums
- `"market_microstructure"` - Order flow analysis (placeholder)
- `"market_anomalies"` - Anomaly detection with severity badges
- `"microstructure_health"` - Health scores and component status
- `"data_health"` - WebSocket connectivity and data freshness

**Response**:
```python
{
  "markdown_content": str,     # Complete formatted report
  "symbol": str,               # Trading pair
  "generated_at": int,         # Unix timestamp (milliseconds)
  "data_age_ms": int,          # Data freshness indicator
  "failed_sections": list[str],# Sections that failed to generate
  "generation_time_ms": int    # Report generation time
}
```

**Examples**:

Full report (all sections):
```python
await client.generate_market_report(symbol="BTCUSDT")
```

Custom report (specific sections):
```python
await client.generate_market_report(
    symbol="ETHUSDT",
    options={
        "include_sections": ["price_overview", "liquidity_analysis"],
        "volume_window_hours": 48,
        "orderbook_levels": 50
    }
)
```

**Performance**:
- Cold generation: <500ms
- Cached retrieval: <3ms
- Cache TTL: 60 seconds
- Graceful degradation for missing data sources

**Migration from v0.1.0**:

If you were using individual tools:
```python
# BEFORE (v0.1.0 - no longer works)
ticker = await client.get_ticker(symbol="BTCUSDT")
account = await client.get_account()
trades = await client.get_my_trades(symbol="BTCUSDT")

# AFTER (v0.2.0 - unified report)
report = await client.generate_market_report(symbol="BTCUSDT")
# Access report.markdown_content for formatted output
```

See [CHANGELOG.md](CHANGELOG.md) for complete migration guide.

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
│   └── binance-rs/      # Binance market data provider (read-only)
│       └── src/
│           ├── report/       # Unified report generator 🆕
│           └── orderbook/
│               └── analytics/  # Advanced analytics module
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
- [ChatGPT Integration Guide](providers/binance-rs/CHATGPT_INTEGRATION.md) - SSE setup for ChatGPT
- [Integration Testing](providers/binance-rs/INTEGRATION_TESTS_COMPLETE.md) - Test results

### Gateway & Deployment
- [Manual Testing Guide](mcp-gateway/MANUAL_TESTING_GUIDE.md) - Testing SSE endpoints
- [Deploy Script](infra/deploy-chatgpt.sh) - Production deployment automation
- [Deployment Summary](DEPLOYMENT_P1_FIXES.md) - Latest production deployment

### Specifications
- **Feature 018: Unified Market Data Report** 🆕
  - [Specification](specs/018-market-data-report/spec.md)
  - [Implementation Plan](specs/018-market-data-report/plan.md)
  - [Task Breakdown](specs/018-market-data-report/tasks.md)
  - [Quickstart Guide](specs/018-market-data-report/quickstart.md)
- Feature 002: Binance Provider Integration
  - [Specification](specs/002-binance-provider-integration/spec.md)
  - [Implementation Plan](specs/002-binance-provider-integration/plan.md)
  - [Task Breakdown](specs/002-binance-provider-integration/tasks.md)

### Changelog
- [CHANGELOG.md](CHANGELOG.md) - Version history and migration guides

## License

See LICENSE file.
