# Binance Provider

MCP provider for Binance cryptocurrency trading with advanced order book analytics and dual transport (gRPC + HTTP/JSON-RPC).

## Overview

This provider exposes Binance trading functionality through both gRPC and HTTP transports, implementing the MCP protocol for AI-powered trading analysis and execution. Features real-time order book analytics with RocksDB time-series storage.

## Features

- **21 Tools**: Market data, trading, orderbook analysis, and **5 advanced analytics tools**
- **4 Resources**: Market data, account balances, trade history, open orders
- **2 Prompts**: Trading analysis, portfolio risk assessment
- **Dual Transport**: gRPC (binary protocol) and HTTP (JSON-RPC 2.0)
- **Advanced Analytics**: Order flow, volume profile, anomaly detection, microstructure health, liquidity mapping
- **Time-Series Storage**: RocksDB with MessagePack compression (70% size reduction)

## Quick Start

### Prerequisites

- Rust 1.75+
- Protocol Buffers compiler (protoc) - for gRPC
- Binance API credentials (for authenticated endpoints)

### Build

```bash
# Full build with all features (default)
cargo build --release

# Minimal build (no analytics, no HTTP)
cargo build --release --no-default-features --features websocket

# With specific features
cargo build --release --features "orderbook,orderbook_analytics,http_transport"
```

### Configuration

Create `.env` file (copy from `.env.example`):

```bash
cp .env.example .env
# Edit .env with your credentials
```

Or set environment variables:

```bash
export BINANCE_API_KEY="your_api_key"
export BINANCE_API_SECRET="your_api_secret"
export ANALYTICS_DATA_PATH="./data/analytics"
export RUST_LOG="info"
```

### Run

```bash
# gRPC mode (default, port 50053)
./target/release/binance-provider --grpc

# HTTP mode (port 3000)
./target/release/binance-provider --http

# Custom ports
./target/release/binance-provider --mode http --port 8080
./target/release/binance-provider --mode grpc --port 50053

# With all features enabled
cargo run --release -- --http
```

## Transport Modes

### gRPC (Binary Protocol)
- High-performance binary protocol
- Default port: **50053**
- Best for: MCP Gateway integration, high-throughput scenarios
- Protocol: Tonic/gRPC

### HTTP (JSON-RPC 2.0)
- Web-compatible REST API
- Default port: **3000**
- Best for: Web applications, ChatGPT integration, debugging
- Protocol: JSON-RPC 2.0 over HTTP with session management
- Endpoint: `POST /mcp`

#### HTTP Session Management
- 30-minute idle timeout
- 50 concurrent session limit
- Header: `Mcp-Session-Id` (UUID)

#### HTTP Examples

```bash
# Initialize session
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"initialize","id":1}'

# Returns: {"jsonrpc":"2.0","result":{...,"sessionId":"<uuid>"},"id":1}

# List tools (with session)
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -H "Mcp-Session-Id: <uuid>" \
  -d '{"jsonrpc":"2.0","method":"tools/list","id":2}'

# Call analytics tool
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -H "Mcp-Session-Id: <uuid>" \
  -d '{
    "jsonrpc":"2.0",
    "method":"tools/call",
    "params":{
      "name":"binance.get_order_flow",
      "arguments":{"symbol":"BTCUSDT","window_duration_secs":60}
    },
    "id":3
  }'
```

## Tools (21 total)

### Market Data (Public) - 6 tools
1. `binance.get_ticker` - 24-hour ticker statistics
2. `binance.get_orderbook` - Market depth (bids/asks)
3. `binance.get_recent_trades` - Recent public trades
4. `binance.get_klines` - OHLCV candlestick data
5. `binance.get_exchange_info` - Exchange trading rules
6. `binance.get_avg_price` - Current average price

### Account (Authenticated) - 2 tools
7. `binance.get_account` - Account balances and permissions
8. `binance.get_my_trades` - Personal trade history

### Trading (Authenticated) - 5 tools
9. `binance.place_order` - Create BUY/SELL orders
10. `binance.cancel_order` - Cancel active order
11. `binance.get_order` - Query order status
12. `binance.get_open_orders` - List active orders
13. `binance.get_all_orders` - Complete order history

### OrderBook Analysis - 3 tools
14. `binance.orderbook_l1` - L1 metrics (spread, microprice, imbalance)
15. `binance.orderbook_l2` - L2 depth (20 or 100 levels)
16. `binance.orderbook_health` - WebSocket service health

### Advanced Analytics (Feature: `orderbook_analytics`) - 5 tools

#### 17. `binance.get_order_flow` - Bid/Ask Pressure Tracking
Analyzes order flow dynamics over configurable time windows (10-300 seconds).

**Parameters:**
- `symbol`: Trading pair (e.g., "BTCUSDT")
- `window_duration_secs`: Analysis window (default: 60, range: 10-300)

**Returns:**
- `bid_flow_rate`: Bid orders per second
- `ask_flow_rate`: Ask orders per second
- `net_flow`: Bid flow - ask flow
- `flow_direction`: STRONG_BUY, MODERATE_BUY, NEUTRAL, MODERATE_SELL, STRONG_SELL
- `cumulative_delta`: Running sum of buy volume - sell volume

**Example:**
```bash
curl -X POST http://localhost:3000/mcp \
  -H "Mcp-Session-Id: <uuid>" \
  -d '{
    "jsonrpc":"2.0",
    "method":"tools/call",
    "params":{
      "name":"binance.get_order_flow",
      "arguments":{"symbol":"ETHUSDT","window_duration_secs":30}
    },
    "id":1
  }'
```

#### 18. `binance.get_volume_profile` - Volume Distribution Histogram
Generates volume profile with POC (Point of Control), VAH/VAL (Value Area High/Low - 70% volume boundaries).

**Parameters:**
- `symbol`: Trading pair
- `duration_hours`: Time period (default: 24, range: 1-168)
- `tick_size`: Optional custom bin size

**Returns:**
- `histogram`: Volume bins sorted by price
- `point_of_control`: Price with highest volume
- `value_area_high/low`: 70% volume boundaries
- `total_volume`: Sum of all bin volumes

**Example:**
```bash
curl -X POST http://localhost:3000/mcp \
  -H "Mcp-Session-Id: <uuid>" \
  -d '{
    "jsonrpc":"2.0",
    "method":"tools/call",
    "params":{
      "name":"binance.get_volume_profile",
      "arguments":{"symbol":"SOLUSDT","duration_hours":4}
    },
    "id":2
  }'
```

#### 19. `binance.detect_market_anomalies` - Market Manipulation Detection
Detects quote stuffing, iceberg orders, and flash crash risk.

**Parameters:**
- `symbol`: Trading pair

**Returns:** Array of anomalies with:
- `anomaly_type`: QuoteStuffing, IcebergOrder, FlashCrashRisk
- `severity`: Low, Medium, High, Critical
- `description`: Human-readable explanation
- `recommendation`: Suggested action

**Example:**
```bash
curl -X POST http://localhost:3000/mcp \
  -H "Mcp-Session-Id: <uuid>" \
  -d '{
    "jsonrpc":"2.0",
    "method":"tools/call",
    "params":{
      "name":"binance.detect_market_anomalies",
      "arguments":{"symbol":"BTCUSDT"}
    },
    "id":3
  }'
```

#### 20. `binance.get_microstructure_health` - Market Health Scoring
Returns composite 0-100 health score with component breakdowns.

**Parameters:**
- `symbol`: Trading pair

**Returns:**
- `composite_score`: 0-100 weighted health score
- `spread_stability_score` (25%): Bid-ask spread consistency
- `liquidity_depth_score` (35%): Order book depth adequacy
- `flow_balance_score` (25%): Bid/ask flow equilibrium
- `update_rate_score` (15%): Quote update frequency
- `health_status`: Healthy, Degraded, Poor, Critical
- `warnings`: Active issues
- `recommendations`: Suggested actions

#### 21. `binance.get_liquidity_vacuums` - Stop-Loss Placement Guidance
Identifies price zones with <20% of median volume for optimal stop placement.

**Parameters:**
- `symbol`: Trading pair
- `duration_hours`: Lookback period (default: 24, range: 1-168)

**Returns:** Array of liquidity vacuums with:
- `vacuum_id`: Unique identifier (UUID)
- `price_range_low/high`: Vacuum zone boundaries
- `volume_deficit_pct`: Percentage below median (>80% = high severity)
- `expected_impact`: Low, Medium, High, Critical
- `detection_timestamp`: When detected

**Usage:**
- For **long positions**: Place stops **below** vacuum zones
- For **short positions**: Place stops **above** vacuum zones

**Example:**
```bash
curl -X POST http://localhost:3000/mcp \
  -H "Mcp-Session-Id: <uuid>" \
  -d '{
    "jsonrpc":"2.0",
    "method":"tools/call",
    "params":{
      "name":"binance.get_liquidity_vacuums",
      "arguments":{"symbol":"ETHUSDT","duration_hours":12}
    },
    "id":4
  }'
```

## Analytics Storage

### RocksDB Configuration
- **Path**: `ANALYTICS_DATA_PATH` (default: `./data/analytics`)
- **Format**: MessagePack binary encoding (70% size reduction vs JSON)
- **Retention**: 7 days, 1GB hard limit
- **Snapshot Interval**: 1 second
- **Key Format**: `"{symbol}:{timestamp}"` for efficient prefix scans

### Storage Initialization
Analytics storage is automatically initialized when running with `orderbook_analytics` feature:

```bash
# Initialize storage on startup
cargo run --features orderbook,orderbook_analytics -- --grpc

# Custom storage path
export ANALYTICS_DATA_PATH=/var/lib/binance-analytics
cargo run --features orderbook,orderbook_analytics -- --http
```

## Resources

- `binance://market/{symbol}` - Market data (e.g., btcusdt)
- `binance://account/balances` - Account balances table
- `binance://account/trades` - Trade history
- `binance://orders/open` - Open orders table

## Prompts

- `trading_analysis` - Market analysis with trading recommendations
- `portfolio_risk` - Portfolio risk assessment

## Feature Flags

```toml
[features]
default = ["orderbook", "http-api", "websocket", "orderbook_analytics", "http_transport"]
orderbook = ["tokio-tungstenite", "rust_decimal", "governor"]
websocket = ["tokio-tungstenite"]
http-api = []
orderbook_analytics = ["orderbook", "rocksdb", "statrs", "rmp-serde", "uuid"]
http_transport = ["axum", "tower", "tower-http", "uuid"]
```

**Build Configurations:**

```bash
# Full build (21 tools, both transports)
cargo build --release

# Minimal build (13 tools, gRPC only)
cargo build --release --no-default-features --features websocket

# Analytics only (no HTTP)
cargo build --release --no-default-features --features orderbook,orderbook_analytics

# HTTP only (no analytics)
cargo build --release --no-default-features --features http_transport
```

## Architecture

```
┌─────────────────────────────────────────┐
│         Transport Layer                 │
│  ┌──────────────┐   ┌─────────────────┐│
│  │ gRPC (Tonic) │   │ HTTP (Axum)     ││
│  │ Port: 50053  │   │ Port: 3000      ││
│  │ Binary Proto │   │ JSON-RPC 2.0    ││
│  └──────┬───────┘   └────────┬────────┘│
└─────────┼──────────────────┬─┘
          │                  │
          v                  v
┌─────────────────────────────────────────┐
│       MCP Tool Routing Layer            │
│  - Capabilities Discovery               │
│  - Tool Invocation                      │
│  - Resource URI Handling                │
│  - Prompt Template Generation           │
└─────────────────┬───────────────────────┘
                  │
    ┌─────────────┼─────────────┐
    v             v             v
┌────────┐  ┌──────────┐  ┌─────────────┐
│Market  │  │ Trading  │  │  Analytics  │
│Data    │  │ Tools    │  │  Engine     │
│Tools   │  │          │  │             │
└───┬────┘  └────┬─────┘  └──────┬──────┘
    │            │                │
    v            v                v
┌─────────────────────────────────────────┐
│       Binance API Client (reqwest)      │
│  - REST API                             │
│  - WebSocket Streams                    │
│  - HMAC-SHA256 Signing                  │
└─────────────┬───────────────────────────┘
              │
              v
    ┌──────────────────────┐
    │  RocksDB Storage     │
    │  (Analytics)         │
    │  - OrderBook         │
    │    Snapshots         │
    │  - Time-Series Data  │
    └──────────────────────┘
```

## Development

### Testing

```bash
# Run unit tests
cargo test

# Run with all features
cargo test --all-features

# Test specific feature
cargo test --features orderbook_analytics
```

### Testing gRPC with grpcurl

```bash
# List capabilities
grpcurl -plaintext -import-path ../../pkg/proto -proto provider.proto \
  localhost:50053 provider.v1.Provider/ListCapabilities

# Get order flow analytics
grpcurl -plaintext -import-path ../../pkg/proto -proto provider.proto \
  -d '{"tool_name":"binance.get_order_flow","payload":{"value":"eyJzeW1ib2wiOiJCVENVU0RUIiwid2luZG93X2R1cmF0aW9uX3NlY3MiOjYwfQ=="},"correlation_id":"test-001"}' \
  localhost:50053 provider.v1.Provider/Invoke
```

### Debugging

```bash
# Verbose logging
export RUST_LOG="debug,binance_provider=trace"
./target/release/binance-provider --http --port 3000

# Analytics-specific logging
export RUST_LOG="info,binance_provider::orderbook::analytics=debug"
./target/release/binance-provider --grpc
```

### Generate Protobuf Code

Protobuf stubs are auto-generated during `cargo build` via `build.rs`.

## Performance

- **gRPC**: ~1ms latency for tool invocation
- **HTTP**: ~2-5ms latency for JSON-RPC calls
- **Analytics Storage**: 70% compression ratio, sub-200ms query times
- **WebSocket**: Real-time order book updates (<100ms)

## Production Deployment

```bash
# Build optimized binary
cargo build --release

# Run with production settings
export RUST_LOG=info
export ANALYTICS_DATA_PATH=/var/lib/binance-analytics
export BINANCE_API_KEY="<production_key>"
export BINANCE_API_SECRET="<production_secret>"

# Start HTTP server
./target/release/binance-provider --http --port 3000

# Or gRPC server
./target/release/binance-provider --grpc --port 50053
```

### systemd Service Example

```ini
[Unit]
Description=Binance MCP Provider
After=network.target

[Service]
Type=simple
User=binance
WorkingDirectory=/opt/binance-provider
EnvironmentFile=/opt/binance-provider/.env
ExecStart=/opt/binance-provider/binance-provider --http --port 3000
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

## License

See repository root LICENSE file.
