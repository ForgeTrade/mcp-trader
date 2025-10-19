# Binance Provider - Integration Guide

## Quick Start

### 1. Build the Provider

```bash
# Build all versions (base, orderbook, analytics)
./build.sh all

# Or build specific version:
./build.sh analytics  # Recommended for full features
```

### 2. Set Up Environment

```bash
# Required for authenticated operations
export BINANCE_API_KEY="your_api_key"
export BINANCE_API_SECRET="your_api_secret"

# Optional: Configure analytics storage
export ANALYTICS_DATA_PATH="./data/analytics"  # Default location

# Optional: Set log level
export RUST_LOG="info"  # trace, debug, info, warn, error
```

### 3. Run the Server

```bash
# Easy mode: Use the run script
./run-analytics.sh

# Or run manually:
cargo run --release --features "orderbook,orderbook_analytics" -- --grpc --port 50053
```

## Feature Flags

### Available Flags

| Flag | Tools | Description |
|------|-------|-------------|
| (none) | 13 | Base: market data, account, orders |
| `orderbook` | 16 | + L1/L2 orderbook depth analysis |
| `orderbook_analytics` | 21 | + Order flow, volume profile, anomalies |

**Note:** `orderbook_analytics` requires `orderbook` to be enabled.

### Build Examples

```bash
# Minimal build (13 tools)
cargo build --release

# With orderbook (16 tools)
cargo build --release --features "orderbook"

# Full analytics (21 tools) - RECOMMENDED
cargo build --release --features "orderbook,orderbook_analytics"
```

## Analytics Tools

### 1. Order Flow Analysis

**Tool:** `binance.get_order_flow`

**Parameters:**
```json
{
  "symbol": "BTCUSDT",
  "window_duration_secs": 60
}
```

**Returns:**
- Bid/ask flow rates (orders/sec)
- Net flow (bid - ask)
- Flow direction (STRONG_BUY, MODERATE_BUY, NEUTRAL, MODERATE_SELL, STRONG_SELL)
- Cumulative delta

### 2. Volume Profile

**Tool:** `binance.get_volume_profile`

**Parameters:**
```json
{
  "symbol": "ETHUSDT",
  "duration_hours": 24,
  "tick_size": "0.10"  // Optional
}
```

**Returns:**
- Volume histogram (price bins)
- Point of Control (POC) - highest volume price
- Value Area High/Low (VAH/VAL) - 70% volume boundaries
- Liquidity vacuum zones

### 3. Anomaly Detection

**Tool:** `binance.detect_market_anomalies`

**Parameters:**
```json
{
  "symbol": "BTCUSDT"
}
```

**Detects:**
- Quote stuffing (>500 updates/sec, <10% fills)
- Iceberg orders (>5x median refill rate)
- Flash crash risk (>80% depth loss, >10x spread widening)

### 4. Market Health

**Tool:** `binance.get_microstructure_health`

**Parameters:**
```json
{
  "symbol": "BTCUSDT"
}
```

**Returns:**
- Overall health score (0-100)
- Component scores:
  - Spread stability (25% weight)
  - Liquidity depth (35% weight)
  - Flow balance (25% weight)
  - Update rate (15% weight)

## Storage Configuration

### RocksDB Data

**Location:** `$ANALYTICS_DATA_PATH` (default: `./data/analytics`)

**Specifications:**
- Compression: Zstd
- Retention: 7 days (automatic cleanup)
- Size limit: 1GB hard limit
- Snapshots: 1-second intervals

**Manual Cleanup:**
```bash
# Remove old data
rm -rf ./data/analytics

# Storage will be recreated on next startup
```

## Testing

### Using cURL (via Python MCP Gateway)

```bash
# Example: Get order flow
curl -X POST http://localhost:8000/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "binance.get_order_flow",
      "arguments": {
        "symbol": "BTCUSDT",
        "window_duration_secs": 120
      }
    },
    "id": "1"
  }'
```

### Using grpcurl

```bash
# List services
grpcurl -plaintext localhost:50053 list

# Call tool (requires proto definitions)
grpcurl -plaintext -d '{
  "tool_name": "binance.get_order_flow",
  "payload": {"value": "{\"symbol\":\"BTCUSDT\",\"window_duration_secs\":60}"}
}' localhost:50053 provider.Provider/Invoke
```

## Troubleshooting

### Issue: "Analytics storage not initialized"

**Solution:** Ensure you're running with analytics features:
```bash
cargo run --features "orderbook,orderbook_analytics"
```

### Issue: "Insufficient historical data"

**Cause:** Not enough snapshots collected yet

**Solution:** Wait 1-2 minutes after startup for snapshot collection to begin

### Issue: RocksDB permissions error

**Solution:**
```bash
# Check data directory permissions
ls -la ./data/analytics

# Fix permissions if needed
chmod -R 755 ./data/analytics
```

### Issue: High memory usage

**Cause:** Large snapshot buffer

**Solution:** Reduce tracked symbols or clear old data:
```bash
# Clear analytics data
rm -rf ./data/analytics/*
```

## Production Deployment

### Systemd Service

```ini
[Unit]
Description=Binance MCP Provider (Analytics)
After=network.target

[Service]
Type=simple
User=binance
WorkingDirectory=/opt/binance-provider
Environment="BINANCE_API_KEY=your_key"
Environment="BINANCE_API_SECRET=your_secret"
Environment="ANALYTICS_DATA_PATH=/var/lib/binance-analytics"
Environment="RUST_LOG=info"
ExecStart=/opt/binance-provider/binance-provider --grpc --port 50053
Restart=on-failure
RestartSec=10s

[Install]
WantedBy=multi-user.target
```

### Docker

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --features "orderbook,orderbook_analytics"

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/binance-provider /usr/local/bin/
VOLUME ["/data"]
ENV ANALYTICS_DATA_PATH=/data
EXPOSE 50053
CMD ["binance-provider", "--grpc", "--port", "50053"]
```

## Performance Tuning

### RocksDB Tuning

Set via environment before startup:
```bash
# Increase write buffer (default: 64MB)
export ROCKSDB_WRITE_BUFFER_SIZE=128000000  # 128MB

# Disable compression for speed (not recommended)
export ROCKSDB_DISABLE_COMPRESSION=1
```

### Snapshot Interval

Default: 1 second per symbol

To modify, edit `capture_snapshot_task()` in `analytics/storage/snapshot.rs`:
```rust
let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(2)); // 2 seconds
```

## Next Steps

- [Specification](../../../specs/003-specify-scripts-bash/spec.md)
- [Architecture](../../../specs/003-specify-scripts-bash/research.md)
- [Data Model](../../../specs/003-specify-scripts-bash/data-model.md)
- [Contracts](../../../specs/003-specify-scripts-bash/contracts/)
