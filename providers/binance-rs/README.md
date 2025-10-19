# Binance Provider

gRPC provider for Binance cryptocurrency trading integration with MCP Gateway.

## Overview

This provider exposes Binance trading functionality through a gRPC interface that implements the `provider.proto` contract. It wraps the existing mcp-binance-rs MCP server functionality while adding gRPC transport.

## Features

- **16 Tools**: Market data, account management, order execution, orderbook analysis
- **4 Resources**: Market data, account balances, trade history, open orders
- **2 Prompts**: Trading analysis, portfolio risk assessment
- **Dual Mode**: Supports both stdio MCP and gRPC server modes

## Quick Start

### Prerequisites

- Rust 1.75+
- Protocol Buffers compiler (protoc)
- Binance API credentials (for authenticated endpoints)

### Build

```bash
# With orderbook feature (default)
cargo build --release

# Without orderbook feature
cargo build --release --no-default-features
```

### Configuration

Set environment variables:

```bash
export BINANCE_API_KEY="your_api_key"
export BINANCE_API_SECRET="your_api_secret"
export BINANCE_BASE_URL="https://testnet.binance.vision"
export RUST_LOG="info"
```

### Run

```bash
# gRPC mode (for MCP Gateway)
./target/release/binance-provider --grpc --port 50053

# stdio MCP mode (standalone)
./target/release/binance-provider
```

## Testing with grpcurl

```bash
# List capabilities
grpcurl -plaintext -import-path ../../pkg/proto -proto provider.proto \
  localhost:50053 provider.v1.Provider/ListCapabilities

# Get ticker data
grpcurl -plaintext -import-path ../../pkg/proto -proto provider.proto \
  -d '{"tool_name":"binance.get_ticker","payload":{"value":"eyJzeW1ib2wiOiJCVENVU0RUIn0="},"correlation_id":"test-001"}' \
  localhost:50053 provider.v1.Provider/Invoke

# Read market resource
grpcurl -plaintext -import-path ../../pkg/proto -proto provider.proto \
  -d '{"uri":"binance://market/BTCUSDT","correlation_id":"test-002"}' \
  localhost:50053 provider.v1.Provider/ReadResource
```

## Tools

### Market Data (Public)
1. `get_server_time` - Server time synchronization
2. `get_ticker` - 24-hour ticker statistics
3. `get_order_book` - Market depth (bids/asks)
4. `get_recent_trades` - Recent public trades
5. `get_klines` - OHLCV candlestick data
6. `get_average_price` - Current average price

### Account (Authenticated)
7. `get_account_info` - Account balances and permissions
8. `get_account_trades` - Personal trade history

### Trading (Authenticated)
9. `place_order` - Create BUY/SELL orders
10. `get_order` - Query order status
11. `cancel_order` - Cancel active order
12. `get_open_orders` - List active orders
13. `get_all_orders` - Complete order history

### OrderBook (Optional Feature)
14. `get_orderbook_metrics` - L1 metrics (spread, microprice, walls)
15. `get_orderbook_depth` - L2 depth (20 or 100 levels)
16. `get_orderbook_health` - Service health status

## Resources

- `binance://market/{symbol}` - Market data (e.g., btcusdt)
- `binance://account/balances` - Account balances table
- `binance://account/trades` - Trade history
- `binance://orders/open` - Open orders table

## Prompts

- `trading_analysis` - Market analysis with trading recommendations
- `portfolio_risk` - Portfolio risk assessment

## Architecture

```
gRPC Server (Tonic)
    ├── Capabilities Discovery
    ├── Tool Invocation Routing
    ├── Resource URI Handling
    └── Prompt Template Generation
         ↓
    MCP Tool Handlers (rmcp SDK)
         ↓
    Binance API Client (reqwest)
         ↓
    Binance REST API / WebSocket
```

## Development

### Generate Protobuf Code

Protobuf stubs are auto-generated during `cargo build` via `build.rs`.

### Testing

```bash
# Run unit tests
cargo test

# Run with all features
cargo test --all-features
```

### Debugging

```bash
# Verbose logging
export RUST_LOG="debug,binance_provider=trace"
./target/release/binance-provider --grpc --port 50052
```

## License

See repository root LICENSE file.
