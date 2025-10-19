# Quickstart Guide: Advanced Order Book Analytics & Streamable HTTP Transport

**Feature**: 003-specify-scripts-bash
**Date**: 2025-10-19
**Audience**: Developers implementing or testing this feature

---

## Prerequisites

### System Requirements

- **Rust**: 1.75+ (check with `rustc --version`)
- **cargo**: Included with Rust toolchain
- **protobuf compiler**: For gRPC code generation
  ```bash
  # Ubuntu/Debian
  sudo apt-get install protobuf-compiler

  # macOS
  brew install protobuf
  ```
- **RocksDB system dependencies** (for analytics feature):
  ```bash
  # Ubuntu/Debian
  sudo apt-get install librocksdb-dev clang

  # macOS
  brew install rocksdb
  ```

### Binance API Credentials

- **Testnet** (recommended for development):
  - Sign up at https://testnet.binance.vision/
  - Create API key and secret
  - No real funds required

- **Production**:
  - Real Binance account required
  - Create API key at https://www.binance.com/en/my/settings/api-management
  - **Use with caution** - analytics tools are read-only but account security is critical

### Environment Setup

Create `.env` file in project root:

```bash
# Required for all modes
BINANCE_API_KEY=your_api_key_here
BINANCE_SECRET_KEY=your_secret_key_here

# Optional for HTTP mode
HOST=0.0.0.0
PORT=8080

# Optional for debugging
RUST_LOG=debug  # or trace, info, warn, error
```

**Load environment**:
```bash
source .env  # bash/zsh
export $(cat .env | xargs)  # alternative
```

---

## Feature Flag Compilation

The project uses Cargo feature flags for modular builds (from research.md decision #6).

### Feature Hierarchy

```toml
[features]
default = ["websocket", "orderbook"]
websocket = ["tokio-tungstenite"]
orderbook = ["websocket"]
orderbook_analytics = ["orderbook", "rocksdb", "statrs", "rmp-serde", "uuid"]
http_transport = ["axum", "tower", "tower-http"]
grpc = ["tonic", "prost"]
```

### Build Variants

**1. Full Build (All Features)**
```bash
cd providers/binance-rs
cargo build --release --all-features
```
*Includes: gRPC, HTTP transport, base tools, analytics tools*

**2. Minimal Build (Base Tools Only)**
```bash
cargo build --release --no-default-features --features grpc
```
*Includes: gRPC mode only, 16 base tools, no analytics*

**3. Analytics via gRPC (No HTTP)**
```bash
cargo build --release --features "grpc,orderbook_analytics"
```
*Includes: gRPC mode, 16 base tools + 5 analytics tools*

**4. HTTP Without Analytics**
```bash
cargo build --release --features "http_transport"
```
*Includes: HTTP mode, 16 base tools only (for ChatGPT access to base tools)*

**5. Full HTTP Analytics Build**
```bash
cargo build --release --features "http_transport,orderbook_analytics"
```
*Includes: HTTP mode with all 21 tools*

### Verify Build

```bash
# Check binary exists
ls -lh target/release/binance-provider

# Check linked dependencies
ldd target/release/binance-provider  # Linux
otool -L target/release/binance-provider  # macOS
```

---

## Running the Provider

### Mode Selection

The binary supports dual-mode operation via `--mode` flag (from research.md decision #7):

```bash
./target/release/binance-provider --mode <grpc|http> [OPTIONS]
```

### gRPC Mode (Default, for Python Gateway)

```bash
# Default port 50053
./target/release/binance-provider --mode grpc

# Custom port
./target/release/binance-provider --mode grpc --port 50055

# Custom host (bind to specific interface)
./target/release/binance-provider --mode grpc --host 127.0.0.1 --port 50053
```

**When to use**:
- Existing Python MCP gateway integration
- Multi-provider setups (hello-go, hello-rs, binance-rs)
- Production deployments with established gRPC infrastructure

### HTTP Mode (for Direct AI Client Access)

```bash
# Default port 8080
./target/release/binance-provider --mode http

# Custom port
./target/release/binance-provider --mode http --port 3000

# Production (bind to localhost, use reverse proxy for HTTPS)
./target/release/binance-provider --mode http --host 127.0.0.1 --port 8080
```

**When to use**:
- ChatGPT MCP connector integration
- Direct Claude Code access without Python gateway
- Browser-based MCP clients

### Environment Variable Overrides

```bash
# Override defaults without command-line flags
HOST=127.0.0.1 PORT=8080 ./target/release/binance-provider --mode http
```

---

## Testing Analytics Tools

### Prerequisites for Analytics

1. **Build with analytics feature**:
   ```bash
   cargo build --release --features "orderbook_analytics"
   ```

2. **Start provider** (choose mode):
   ```bash
   # gRPC mode
   ./target/release/binance-provider --mode grpc --port 50053

   # HTTP mode
   ./target/release/binance-provider --mode http --port 8080
   ```

3. **Wait for data accumulation**:
   - Order flow requires 10-300 seconds of snapshots (from clarifications Q1)
   - Volume profile requires aggregated trade data (24h recommended)
   - Anomaly detection requires baseline metrics (~5 minutes)

### Testing via gRPC (Python Gateway)

*Requires existing Python MCP gateway from Feature 002*

```bash
# Assuming Python gateway is running on localhost:50052
# and forwards to binance-provider on localhost:50053

# Test order flow (via Claude Code or direct MCP client)
# Natural language: "Show me order flow for BTCUSDT over the last 60 seconds"

# Or direct JSON-RPC call:
{
  "method": "tools/call",
  "params": {
    "name": "binance.get_order_flow",
    "arguments": {
      "symbol": "BTCUSDT",
      "window_duration_secs": 60
    }
  }
}
```

### Testing via HTTP (Direct cURL)

**1. Initialize Session**
```bash
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "initialize",
    "params": {
      "protocolVersion": "2024-11-05",
      "capabilities": {"tools": {}},
      "clientInfo": {"name": "curl-test", "version": "1.0"}
    },
    "id": "init-1"
  }' \
  -i  # Show headers to get Mcp-Session-Id
```

**Expected Response**:
```
HTTP/1.1 200 OK
Mcp-Session-Id: 550e8400-e29b-41d4-a716-446655440000
Content-Type: application/json

{"jsonrpc":"2.0","result":{...},"id":"init-1"}
```

**Extract session ID**:
```bash
SESSION_ID=$(curl -s -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"initialize","params":{},"id":"1"}' \
  -D - | grep -i mcp-session-id | awk '{print $2}' | tr -d '\r')

echo "Session ID: $SESSION_ID"
```

**2. List Available Tools**
```bash
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/list",
    "params": {},
    "id": "list-1"
  }' | jq '.result.tools[].name'
```

**Expected Output** (if analytics enabled):
```
"binance.get_ticker"
"binance.get_order_flow"
"binance.get_volume_profile"
"binance.detect_market_anomalies"
"binance.get_microstructure_health"
"binance.get_liquidity_vacuums"
... (16 more base tools)
```

**3. Test Order Flow Tool**
```bash
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -H "Mcp-Session-Id: $SESSION_ID" \
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
    "id": "call-1"
  }' | jq '.result.content[0].text | fromjson'
```

**Expected Response** (after 120s of data):
```json
{
  "symbol": "BTCUSDT",
  "time_window_start": "2025-10-19T14:30:00Z",
  "time_window_end": "2025-10-19T14:32:00Z",
  "window_duration_secs": 120,
  "bid_flow_rate": 38.5,
  "ask_flow_rate": 22.3,
  "net_flow": 16.2,
  "flow_direction": "ModerateBuy",
  "cumulative_delta": 1850.3
}
```

**Error if insufficient data**:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "content": [{
      "type": "text",
      "text": "Error: insufficient_historical_data - Need 60 more snapshots for 120s window analysis"
    }],
    "isError": true
  },
  "id": "call-1"
}
```

**4. Test Volume Profile**
```bash
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "binance.get_volume_profile",
      "arguments": {
        "symbol": "ETHUSDT",
        "duration_hours": 1
      }
    },
    "id": "call-2"
  }' | jq '.result.content[0].text | fromjson | {poc: .point_of_control, vah: .value_area_high, val: .value_area_low}'
```

**5. Test Anomaly Detection**
```bash
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "binance.detect_market_anomalies",
      "arguments": {
        "symbol": "BTCUSDT",
        "window_duration_secs": 60
      }
    },
    "id": "call-3"
  }' | jq '.result.content[0].text | fromjson | .anomalies'
```

---

## Monitoring and Health Checks

### RocksDB Storage

**Location**: `./data/orderbook_snapshots/` (default)

**Check size**:
```bash
du -sh ./data/orderbook_snapshots/
```

**Verify under 1GB limit** (from clarifications Q5):
```bash
SIZE=$(du -b ./data/orderbook_snapshots/ | cut -f1)
if [ $SIZE -gt 1073741824 ]; then
  echo "WARNING: Storage exceeds 1GB limit ($SIZE bytes)"
else
  echo "Storage OK: $(numfmt --to=iec $SIZE)"
fi
```

**Manual cleanup** (if needed):
```bash
rm -rf ./data/orderbook_snapshots/
# Provider will recreate on next start
```

### Session Count (HTTP Mode)

**Check logs** for session activity:
```bash
# Enable debug logging
RUST_LOG=debug ./target/release/binance-provider --mode http

# Look for log lines:
# [INFO] HTTP session created: 550e8400-e29b-41d4-a716-446655440000
# [DEBUG] Active sessions: 12/50
```

### Health Endpoint (HTTP Mode)

```bash
curl http://localhost:8080/health | jq
```

**Response**:
```json
{
  "status": "healthy",
  "active_sessions": 3,
  "max_sessions": 50,
  "uptime_seconds": 3600
}
```

### Performance Monitoring

**Enable trace logging** for detailed timings:
```bash
RUST_LOG=trace ./target/release/binance-provider --mode http 2>&1 | grep -E "(order_flow|volume_profile|anomaly)"
```

**Expected latencies** (from success criteria):
- Order flow calculation: <100ms (SC-001)
- Volume profile generation: <500ms for 24h (SC-002)
- Anomaly detection: <200ms (SC-006)
- Session management: <50ms (SC-017)

---

## ChatGPT Integration

### ChatGPT MCP Connector Setup

1. **Deploy provider with HTTPS**:
   ```bash
   # Use reverse proxy (nginx, Traefour, Cloudflare Tunnel)
   # Example with Cloudflare Tunnel:
   cloudflared tunnel --url http://localhost:8080
   ```

2. **Get public HTTPS URL**:
   ```
   https://your-tunnel.trycloudflare.com
   ```

3. **Configure ChatGPT connector** (hypothetical - exact format may vary):
   ```json
   {
     "schema_version": "v1",
     "name_for_model": "binance_analytics",
     "description_for_model": "Real-time cryptocurrency order flow, volume profile, and anomaly detection",
     "api": {
       "type": "mcp_streamable_http",
       "url": "https://your-tunnel.trycloudflare.com/mcp"
     }
   }
   ```

4. **Test in ChatGPT**:
   - "Show me order flow for Bitcoin over the last 2 minutes"
   - "What's the Point of Control for Ethereum in the last 24 hours?"
   - "Are there any market anomalies in BTCUSDT right now?"

---

## Troubleshooting

### Build Errors

**Error**: `error: failed to run custom build command for rocksdb`

**Solution**: Install RocksDB system dependencies:
```bash
# Ubuntu/Debian
sudo apt-get install librocksdb-dev clang

# macOS
brew install rocksdb
```

**Error**: `feature orderbook_analytics is not defined`

**Solution**: Check Cargo.toml has feature definitions (should be in place after T006)

### Runtime Errors

**Error**: `Missing Mcp-Session-Id header` (HTTP mode)

**Solution**: Initialize session first with `initialize` method, then use returned session ID in header

**Error**: `insufficient_historical_data - Need N more snapshots`

**Solution**: Wait for data accumulation. Order flow requires 10-300s of snapshots (from clarifications Q1).

**Error**: `storage_limit_exceeded - 1GB hard limit reached`

**Solution**: Automatic cleanup should handle this, but manual cleanup:
```bash
rm -rf ./data/orderbook_snapshots/
# Restart provider
```

**Error**: `Session limit exceeded` (HTTP mode)

**Solution**: 50 concurrent sessions max. Close old sessions or increase timeout (currently 30min from FR-020).

### WebSocket Connection Issues

**Error**: `websocket_disconnected - Orderbook stream unavailable`

**Solution**: Check network connectivity to Binance:
```bash
curl -s https://api.binance.com/api/v3/ping
# Should return: {}

# Test WebSocket (requires websocat):
websocat wss://stream.binance.com:9443/ws/btcusdt@depth
```

**Error**: Reconnection loops (exponential backoff)

**Solution**: Check logs for backoff timing (should be 1s, 2s, 4s, 8s, max 60s from research.md):
```bash
grep -i "reconnect" logs.txt
# [DEBUG] Reconnecting in 2s (attempt 2)
# [DEBUG] Reconnecting in 4s (attempt 3)
```

---

## Development Tips

### Hot Reloading

Use `cargo watch` for development:
```bash
cargo install cargo-watch

# Rebuild on file changes
cargo watch -x 'build --features orderbook_analytics'

# Run tests on changes
cargo watch -x 'test --features orderbook_analytics'
```

### Unit Testing

```bash
# Run all tests
cargo test --all-features

# Run analytics tests only
cargo test --features orderbook_analytics analytics

# Run with output
cargo test --features orderbook_analytics -- --nocapture
```

### Integration Testing

```bash
# Run integration tests (requires live Binance connection)
cargo test --test integration --features orderbook_analytics

# Specific test
cargo test --test integration test_order_flow_calculation
```

### Debug Logging

```bash
# Trace level (very verbose)
RUST_LOG=trace ./target/release/binance-provider --mode http

# Debug level (moderate)
RUST_LOG=debug ./target/release/binance-provider --mode http

# Module-specific logging
RUST_LOG=binance_provider::analytics=trace,binance_provider::transport=debug ./target/release/binance-provider --mode http
```

---

## Next Steps

After successfully testing:

1. ✅ **Verify all 5 analytics tools** work (order flow, volume profile, anomalies, health, vacuums)
2. ✅ **Check storage stays under 1GB** for 20 symbols × 7 days
3. ✅ **Test dual-mode operation** (gRPC and HTTP) with same build
4. ✅ **Benchmark performance** against success criteria (SC-001 to SC-017)
5. ✅ **Document any deviations** from plan.md or spec.md
6. ⏸️ **Proceed to `/speckit.tasks`** for task generation (Phase 2)

---

## Reference Documentation

- **Specification**: `specs/003-specify-scripts-bash/spec.md`
- **Implementation Plan**: `specs/003-specify-scripts-bash/plan.md`
- **Data Models**: `specs/003-specify-scripts-bash/data-model.md`
- **API Contracts**: `specs/003-specify-scripts-bash/contracts/`
- **Research Decisions**: `specs/003-specify-scripts-bash/research.md`
- **HTTP Transport Contract**: `specs/003-specify-scripts-bash/contracts/streamable_http_mcp.md`

---

**Quickstart Version**: 1.0
**Last Updated**: 2025-10-19
