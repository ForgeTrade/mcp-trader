# Quickstart: Testing Snapshot Persistence

**Feature**: 007-snapshot-persistence
**Date**: 2025-10-19
**Phase**: 1 (Design)

## Overview

This guide walks through testing the orderbook snapshot persistence feature locally. Follow these steps to verify that background snapshots are being collected and analytics tools can query historical data.

## Prerequisites

- Rust 1.75+ installed
- Local clone of `mcp-trader` repository
- Branch: `007-snapshot-persistence` checked out
- Optional: Binance API credentials (for live testing - testnet works too)

## Quick Test (5 Minutes)

### Step 1: Build the Provider

```bash
cd providers/binance-rs

# Build with analytics features enabled
cargo build --release --features 'orderbook,orderbook_analytics'
```

**Expected Output**:
```
   Compiling binance-provider v0.1.0
    Finished release [optimized] target(s) in 45.2s
```

### Step 2: Configure Environment

```bash
# Create .env file (optional for testnet)
cat > .env <<EOF
BINANCE_BASE_URL=https://testnet.binance.vision
ANALYTICS_DATA_PATH=./data/analytics-test
RUST_LOG=info
EOF
```

**Environment Variables**:
- `BINANCE_BASE_URL`: Use testnet for safe testing (no real funds)
- `ANALYTICS_DATA_PATH`: Temporary directory for RocksDB (will be created automatically)
- `RUST_LOG=info`: Show persistence logs

### Step 3: Start the Provider

```bash
# Run in grpc mode (default port 50053)
./target/release/binance-provider --grpc --port 50053
```

**Expected Initial Logs**:
```
INFO Analytics feature enabled - initializing RocksDB storage
INFO Analytics storage initialized at: ./data/analytics-test
INFO Binance provider starting in gRPC mode on port 50053
INFO Pre-subscribed to BTCUSDT for snapshot persistence
INFO Pre-subscribed to ETHUSDT for snapshot persistence
INFO WebSocket connected successfully symbol=BTCUSDT
INFO WebSocket connected successfully symbol=ETHUSDT
```

**What to Watch For**:
- ✅ "Pre-subscribed to BTCUSDT for snapshot persistence" (indicates eager subscription)
- ✅ "WebSocket connected successfully" (indicates live data feed active)
- ✅ No "TLS support not compiled in" errors (this was the previous bug)

### Step 4: Wait for Snapshots to Accumulate

```bash
# In another terminal, monitor logs
tail -f /tmp/binance-provider.log  # or wherever logs are going

# Wait 65 seconds (minimum for analytics tools to work)
sleep 65
```

**Expected Logs (every 1 second per symbol)**:
```
INFO Stored snapshot for BTCUSDT at timestamp 1737158400
INFO Stored snapshot for BTCUSDT at timestamp 1737158401
INFO Stored snapshot for ETHUSDT at timestamp 1737158400
INFO Stored snapshot for ETHUSDT at timestamp 1737158401
...
```

**Note**: If you see "Skipping snapshot: empty orderbook", it means the WebSocket hasn't received data yet. This is normal for the first 2-3 seconds.

### Step 5: Test Analytics Tools

Since the provider runs in gRPC mode, test analytics tools via the MCP gateway. In a third terminal:

```bash
cd ../../mcp-gateway

# Start MCP gateway (connects to gRPC provider at localhost:50053)
uv run python -m mcp_gateway.main
```

Then use Claude Code or ChatGPT to test:

**Test Command** (via ChatGPT or Claude):
```
Use the binance_get_order_flow tool with:
- symbol: BTCUSDT
- window_duration_secs: 60

Expected response: Order flow data with bid/ask pressure metrics
```

**Success Criteria**:
- ✅ No "Insufficient historical data" error
- ✅ Returns OrderFlow with bid_pressure, ask_pressure, net_pressure values
- ✅ Analysis includes "Captured X snapshots over 60-second window"

### Step 6: Verify RocksDB Storage

```bash
# Check that RocksDB directory was created and contains data
ls -lh data/analytics-test/

# Expected output: Several RocksDB SST files
# -rw-r--r-- 1 user user 4.2M Oct 19 22:15 000005.sst
# -rw-r--r-- 1 user user  16K Oct 19 22:15 MANIFEST-000004
```

**Verify Snapshot Count**:
```bash
# Count snapshots in last minute (requires rocksdb_dump or similar)
# Alternatively, check file size growth
watch -n 5 'ls -lh data/analytics-test/*.sst | tail -1'

# Expected: File size grows by ~60KB per minute (120 snapshots × 500 bytes)
```

## Extended Test (2 Hours) - Optional

### Test Long-Running Stability

```bash
# Run provider for 2 hours to verify stability
./target/release/binance-provider --grpc --port 50053 2>&1 | tee /tmp/persistence-test.log &

# Wait 2 hours
sleep 7200

# Verify snapshot accumulation
grep "Stored snapshot" /tmp/persistence-test.log | wc -l
# Expected: ~14,400 snapshots (2 symbols × 60 snapshots/min × 120 min)
```

### Test Analytics Tools with Long History

After 2 hours, test with longer time windows:

**Test get_volume_profile** (24-hour window - simulated with 2 hours):
```
Use binance_get_volume_profile with:
- symbol: BTCUSDT
- duration_hours: 2

Expected: Volume distribution histogram with POC/VAH/VAL metrics
```

## Troubleshooting

### Issue: "TLS support not compiled in" Error

**Symptom**:
```
WARN WebSocket connection failed: URL error: TLS support not compiled in
```

**Fix**:
This was the original bug. Ensure Cargo.toml line 49 has:
```toml
tokio-tungstenite = { version = "0.28", features = ["native-tls"] }
```

Then rebuild:
```bash
cargo clean
cargo build --release --features 'orderbook,orderbook_analytics'
```

---

### Issue: "Insufficient historical data" from Analytics Tools

**Symptom**:
```
Analytics tool returns: "Need at least 2 snapshots for 60 window (got 0)"
```

**Diagnosis**:
```bash
# Check if snapshots are being stored
grep "Stored snapshot" /tmp/binance-provider.log | tail -20

# Check if WebSocket is connected
grep "WebSocket connected" /tmp/binance-provider.log
```

**Fixes**:
1. If no "Stored snapshot" logs → Background task may not be spawned (implementation bug)
2. If no "WebSocket connected" logs → Network issue or Binance API down
3. If snapshots are being stored but analytics fail → RocksDB query bug

---

### Issue: Service Crashes on Startup

**Symptom**:
```
thread 'main' panicked at 'Failed to initialize analytics storage: Permission denied'
```

**Fix**:
```bash
# Check ANALYTICS_DATA_PATH permissions
mkdir -p ./data/analytics-test
chmod 755 ./data/analytics-test

# Ensure no other process has RocksDB lock
rm -f ./data/analytics-test/LOCK
```

---

### Issue: High CPU Usage

**Symptom**:
`top` shows binance-provider using >10% CPU continuously

**Diagnosis**:
```bash
# Check snapshot persistence rate
grep "Stored snapshot" /tmp/binance-provider.log | tail -100 | wc -l
# Should be ~2 per second (1/sec × 2 symbols)
```

**Expected**: Background task should use <1% CPU. If higher, there may be a performance regression.

---

## Verification Checklist

Use this checklist to verify the feature is working correctly:

- [ ] Service starts without errors
- [ ] "Pre-subscribed to BTCUSDT for snapshot persistence" log appears
- [ ] "Pre-subscribed to ETHUSDT for snapshot persistence" log appears
- [ ] WebSocket connections established within 5 seconds
- [ ] "Stored snapshot" logs appear every 1 second per symbol
- [ ] RocksDB directory created at ANALYTICS_DATA_PATH
- [ ] After 65 seconds, `get_order_flow` tool returns data (no "Insufficient data" error)
- [ ] After 2 minutes, `get_microstructure_health` tool returns composite score
- [ ] Service continues running for >10 minutes without crashes
- [ ] CPU usage <2%, memory usage <50MB for persistence task
- [ ] Live orderbook queries (orderbook_l1/l2) still work with <200ms latency

## Performance Benchmarks

### Expected Metrics (Local Development Machine)

| Metric | Expected Value | Measurement Method |
|--------|---------------|-------------------|
| Snapshot persistence rate | 58-60/min/symbol | `grep "Stored snapshot" log \| wc -l` ÷ minutes |
| RocksDB write latency | <10ms p99 | Check for "Failed to persist" errors (indicates timeouts) |
| Memory footprint | <10MB | `ps aux \| grep binance-provider` RSS column |
| CPU usage | <1% | `top -p $(pgrep binance-provider)` |
| Live orderbook latency | <200ms p99 | Test orderbook_l1 tool, check response time |

### Storage Growth

| Time Period | Expected Storage Size | Files |
|-------------|----------------------|-------|
| 1 minute | ~60 KB | 1-2 SST files |
| 1 hour | ~3.6 MB | 5-10 SST files |
| 1 day | ~86 MB | 20-50 SST files |
| 7 days | ~600 MB | 50-200 SST files (with compaction) |

**Note**: RocksDB automatically compacts data, so actual file count may be lower than estimated.

## Next Steps

After verifying locally:

1. **Run Integration Tests**: `cargo test --features 'orderbook,orderbook_analytics'`
2. **Deploy to Testnet**: Test on remote server with production-like environment
3. **Load Testing**: Increase to 10+ symbols, verify performance holds
4. **Production Deployment**: Deploy via `./infra/deploy-chatgpt.sh` to root@198.13.46.14

## Manual Testing Script

Save this as `test_persistence.sh` for automated testing:

```bash
#!/usr/bin/env bash
set -e

echo "=== Snapshot Persistence Feature Test ==="
echo ""

# 1. Build
echo "[1/6] Building binance-provider with analytics features..."
cd providers/binance-rs
cargo build --release --features 'orderbook,orderbook_analytics' 2>&1 | grep -E "(Compiling|Finished)"

# 2. Setup
echo ""
echo "[2/6] Setting up test environment..."
export ANALYTICS_DATA_PATH="./data/test-$(date +%s)"
export RUST_LOG=info
mkdir -p "$ANALYTICS_DATA_PATH"

# 3. Start provider
echo ""
echo "[3/6] Starting binance-provider in background..."
./target/release/binance-provider --grpc --port 50053 > /tmp/test-persistence.log 2>&1 &
PROVIDER_PID=$!
sleep 3

# 4. Verify startup
echo ""
echo "[4/6] Verifying service started..."
if ! kill -0 $PROVIDER_PID 2>/dev/null; then
    echo "ERROR: Provider failed to start"
    cat /tmp/test-persistence.log
    exit 1
fi

grep -q "Pre-subscribed to BTCUSDT" /tmp/test-persistence.log && echo "✅ BTCUSDT pre-subscribed" || echo "❌ BTCUSDT subscription missing"
grep -q "Pre-subscribed to ETHUSDT" /tmp/test-persistence.log && echo "✅ ETHUSDT pre-subscribed" || echo "❌ ETHUSDT subscription missing"

# 5. Wait for snapshots
echo ""
echo "[5/6] Waiting 70 seconds for snapshot accumulation..."
sleep 70

# 6. Verify snapshots
echo ""
echo "[6/6] Verifying snapshot persistence..."
SNAPSHOT_COUNT=$(grep -c "Stored snapshot" /tmp/test-persistence.log || echo 0)
echo "Total snapshots stored: $SNAPSHOT_COUNT"

if [ "$SNAPSHOT_COUNT" -ge 110 ]; then
    echo "✅ SUCCESS: Snapshot persistence working (expected ~120, got $SNAPSHOT_COUNT)"
else
    echo "❌ FAILURE: Insufficient snapshots (expected >=110, got $SNAPSHOT_COUNT)"
fi

# Cleanup
kill $PROVIDER_PID 2>/dev/null || true
rm -rf "$ANALYTICS_DATA_PATH"

echo ""
echo "=== Test Complete ==="
```

Run with: `bash test_persistence.sh`

---

**Quickstart Complete**: You can now test the snapshot persistence feature locally and verify that analytics tools receive historical data.
