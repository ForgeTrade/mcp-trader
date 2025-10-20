# Quickstart: Testing Trade Stream Persistence

**Feature**: 008-trade-stream-persistence
**Date**: 2025-10-19
**Phase**: 1 (Design)

## Overview

This guide walks through testing the trade stream persistence feature locally to verify that trades are being collected from Binance aggTrade WebSocket and analytics tools can query historical data.

## Prerequisites

- Rust 1.75+ installed
- Local clone of `mcp-trader` repository
- Branch: `008-trade-stream-persistence` checked out
- Feature 007 (snapshot persistence) already deployed

## Quick Test (10 Minutes)

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
BINANCE_BASE_URL=https://api.binance.com
ANALYTICS_DATA_PATH=./data/analytics-test
RUST_LOG=info
EOF
```

**Environment Variables**:
- `BINANCE_BASE_URL`: Use production API (trade streams are public, no credentials needed)
- `ANALYTICS_DATA_PATH`: Temporary directory for RocksDB (will be created automatically)
- `RUST_LOG=info`: Show trade collection logs

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
INFO Pre-subscribed to BTCUSDT for snapshot persistence (Feature 007)
INFO Pre-subscribed to ETHUSDT for snapshot persistence (Feature 007)
INFO Starting trade stream collection for BTCUSDT
INFO Starting trade stream collection for ETHUSDT
INFO Trade WebSocket connected successfully symbol=BTCUSDT
INFO Trade WebSocket connected successfully symbol=ETHUSDT
```

**What to Watch For**:
- ✅ "Starting trade stream collection for BTCUSDT/ETHUSDT" (indicates trade persistence task spawned)
- ✅ "Trade WebSocket connected successfully" (indicates aggTrade stream active)
- ✅ No "TLS support not compiled in" errors (uses same WebSocket setup as Feature 007)

### Step 4: Wait for Trades to Accumulate

```bash
# In another terminal, monitor logs
tail -f /tmp/binance-provider.log  # or wherever logs are going

# Wait 65 seconds (minimum for analytics tools to work)
sleep 65
```

**Expected Logs (every 1-2 seconds)**:
```
INFO Stored 87 trades for BTCUSDT at timestamp 1760903627000
INFO Stored 92 trades for ETHUSDT at timestamp 1760903627000
INFO Stored 105 trades for BTCUSDT at timestamp 1760903628000
INFO Stored 88 trades for ETHUSDT at timestamp 1760903628000
...
```

**Trade Rate Expectations**:
- BTCUSDT: 60-600 trades/minute during active market hours
- ETHUSDT: 60-600 trades/minute during active market hours
- Lower rates overnight (10-60 trades/minute)

**Note**: If you see "No trades received in last second" warnings, this is normal during very quiet periods (e.g., 3-5 AM UTC).

### Step 5: Test Analytics Tools

Since the provider runs in gRPC mode, test analytics tools via the MCP gateway. In a third terminal:

```bash
cd ../../mcp-gateway

# Start MCP gateway (connects to gRPC provider at localhost:50053)
uv run python -m mcp_gateway.main
```

Then use Claude Code or ChatGPT to test:

**Test Command 1** (get_volume_profile):
```
Use the binance_get_volume_profile tool with:
- symbol: BTCUSDT
- duration_hours: 1

Expected response: Volume distribution histogram with POC/VAH/VAL metrics
```

**Success Criteria**:
- ✅ No "Insufficient historical data" or "need ≥1000 trades" errors
- ✅ Returns VolumeProfile with price levels and volume distribution
- ✅ Analysis includes POC (Point of Control), VAH (Value Area High), VAL (Value Area Low)

**Test Command 2** (get_liquidity_vacuums):
```
Use the binance_get_liquidity_vacuums tool with:
- symbol: ETHUSDT
- duration_hours: 1

Expected response: Low-volume price zones identified
```

**Success Criteria**:
- ✅ No "Insufficient historical data" errors
- ✅ Returns list of LiquidityVacuum entries with price ranges and volume metrics
- ✅ Analysis includes zones with significantly below-average volume

### Step 6: Verify RocksDB Storage

```bash
# Check that RocksDB directory was created and contains trade data
ls -lh data/analytics-test/

# Expected output: RocksDB SST files plus trade data
# -rw-r--r-- 1 user user 6.2M Oct 19 22:15 000025.sst (larger than Feature 007 snapshots)
# -rw-r--r-- 1 user user  16K Oct 19 22:15 MANIFEST-000024
```

**Verify Trade Count**:
```bash
# Count trade batches in last minute
# (Approximate: grep logs for "Stored N trades" messages)
grep "Stored.*trades" /tmp/binance-provider.log | tail -60 | wc -l

# Expected: ~120 log entries per minute (2 symbols × 60 batches/min)
```

**Storage Growth**:
```bash
# Watch storage grow over time
watch -n 5 'du -sh data/analytics-test/'

# Expected growth rate: ~250KB per minute for 2 symbols
# (100 trades/sec × 2 symbols × 75 bytes/trade × 60 sec / 1000 = ~900KB/min raw, ~250KB/min MessagePack compressed)
```

## Extended Test (2 Hours) - Optional

### Test Long-Running Stability

```bash
# Run provider for 2 hours to verify stability
./target/release/binance-provider --grpc --port 50053 2>&1 | tee /tmp/persistence-test.log &

# Wait 2 hours
sleep 7200

# Verify trade accumulation
grep "Stored.*trades" /tmp/persistence-test.log | wc -l
# Expected: ~14,400 log entries (2 symbols × 60 batches/min × 120 min)
```

### Test Analytics Tools with Long History

After 2 hours, test with longer time windows:

**Test get_volume_profile with 2-hour window**:
```
Use binance_get_volume_profile with:
- symbol: BTCUSDT
- duration_hours: 2

Expected: Volume distribution histogram covering 2-hour period with POC/VAH/VAL
```

**Test get_liquidity_vacuums with 2-hour window**:
```
Use binance_get_liquidity_vacuums with:
- symbol: ETHUSDT
- duration_hours: 2

Expected: Liquidity vacuum zones identified across 2-hour period
```

## Troubleshooting

### Issue: "Trade WebSocket connection failed"

**Symptom**:
```
ERROR Trade WebSocket connection failed: URL error: TLS support not compiled in
```

**Fix**:
Ensure Cargo.toml line 49 has native-tls feature:
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
Analytics tool returns: "Need at least 1000 trades for volume profile (got 0)"
```

**Diagnosis**:
```bash
# Check if trades are being stored
grep "Stored.*trades" /tmp/binance-provider.log | tail -20

# Check if trade WebSocket is connected
grep "Trade WebSocket connected" /tmp/binance-provider.log
```

**Fixes**:
1. If no "Stored N trades" logs → Trade persistence task may not be spawned (implementation bug)
2. If no "Trade WebSocket connected" logs → Network issue or Binance API down
3. If trades are being stored but analytics fail → RocksDB query bug or wrong key format

---

### Issue: Service Crashes on Trade Deserialization

**Symptom**:
```
thread 'tokio-runtime-worker' panicked at 'Failed to parse trade message'
```

**Fix**:
```bash
# Check logs for malformed trade messages
grep "Failed to parse aggTrade message" /tmp/binance-provider.log

# Verify WebSocket endpoint is correct
# Should be: wss://stream.binance.com/ws/btcusdt@aggTrade (lowercase symbol)
```

Expected behavior: Service should log error and skip malformed message, NOT crash.

---

### Issue: High Storage Growth

**Symptom**:
Storage grows faster than expected (>1 GB/day instead of 10-15 MB/day)

**Diagnosis**:
```bash
# Check trade rate
grep "Stored.*trades" /tmp/binance-provider.log | tail -100 | \
  awk '{sum += $3} END {print sum/100 " trades/batch avg"}'

# Should be: 50-150 trades/batch during active hours
```

**Fixes**:
1. If >500 trades/batch consistently → Extremely volatile market (normal, wait for calming)
2. If storage continues growing → Check retention cleanup is running (should run hourly)

---

## Verification Checklist

Use this checklist to verify the feature is working correctly:

- [ ] Service starts without errors
- [ ] "Starting trade stream collection for BTCUSDT" log appears
- [ ] "Starting trade stream collection for ETHUSDT" log appears
- [ ] Trade WebSocket connections established within 5 seconds
- [ ] "Stored N trades" logs appear every 1-2 seconds (N varies by market activity)
- [ ] RocksDB directory contains trade data (key prefix: `trades:`)
- [ ] After 65 seconds, `get_volume_profile` tool returns data (no "Insufficient data" error)
- [ ] After 2 minutes, `get_liquidity_vacuums` tool returns vacuum zones
- [ ] Service continues running for >10 minutes without crashes
- [ ] CPU usage <2%, memory usage <50MB for trade collection
- [ ] Live orderbook queries (orderbook_l1/l2) still work with <200ms latency (Feature 007 unaffected)

## Performance Benchmarks

### Expected Metrics (Local Development Machine)

| Metric | Expected Value | Measurement Method |
|--------|---------------|-------------------|
| Trade collection rate | 60-600/min/symbol | Count "Stored N trades" logs × N |
| RocksDB write latency | <10ms p99 | Check for "Failed to persist" errors (indicates timeouts) |
| Memory footprint | <50MB | `ps aux \| grep binance-provider` RSS column |
| CPU usage | <2% | `top -p $(pgrep binance-provider)` |
| Query time (1h window) | <1s | Test get_volume_profile with duration_hours=1, measure response time |
| Query time (24h window) | <3s | Test get_volume_profile with duration_hours=24, measure response time |

### Storage Growth

| Time Period | Expected Storage Size | Batches Stored |
|-------------|----------------------|----------------|
| 1 minute | ~250 KB | 120 (2 symbols × 60 sec) |
| 1 hour | ~15 MB | 7,200 |
| 1 day | ~350 MB | 172,800 |
| 7 days | ~2.5 GB | 1,209,600 (with compaction: ~2 GB) |

**Note**: RocksDB automatically compacts data, so actual storage may be 20-30% less than raw estimate.

**Comparison to Feature 007**:
- Feature 007 (orderbook snapshots): ~600 MB for 7 days
- Feature 008 (trade stream): ~2 GB for 7 days
- Combined: ~2.6 GB for 7 days (manageable on production server with 50GB+ disk)

## Next Steps

After verifying locally:

1. **Run Integration Tests**: `cargo test --features 'orderbook,orderbook_analytics' trade_`
2. **Load Testing**: Let service run for 24 hours, verify storage growth and query performance
3. **Production Deployment**: Deploy via `./infra/deploy-chatgpt.sh` to root@198.13.46.14
4. **Post-Deployment Validation**: Monitor logs, test analytics tools via ChatGPT

## Manual Testing Script

Save this as `test_trade_persistence.sh` for automated testing:

```bash
#!/usr/bin/env bash
set -e

echo "=== Trade Stream Persistence Feature Test ==="
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
./target/release/binance-provider --grpc --port 50053 > /tmp/test-trade-persistence.log 2>&1 &
PROVIDER_PID=$!
sleep 5

# 4. Verify startup
echo ""
echo "[4/6] Verifying service started..."
if ! kill -0 $PROVIDER_PID 2>/dev/null; then
    echo "ERROR: Provider failed to start"
    cat /tmp/test-trade-persistence.log
    exit 1
fi

grep -q "Starting trade stream collection for BTCUSDT" /tmp/test-trade-persistence.log && echo "✅ BTCUSDT trade stream started" || echo "❌ BTCUSDT stream missing"
grep -q "Starting trade stream collection for ETHUSDT" /tmp/test-trade-persistence.log && echo "✅ ETHUSDT trade stream started" || echo "❌ ETHUSDT stream missing"

# 5. Wait for trades
echo ""
echo "[5/6] Waiting 70 seconds for trade accumulation..."
sleep 70

# 6. Verify trades
echo ""
echo "[6/6] Verifying trade persistence..."
TRADE_BATCHES=$(grep -c "Stored.*trades" /tmp/test-trade-persistence.log || echo 0)
echo "Total trade batches stored: $TRADE_BATCHES"

if [ "$TRADE_BATCHES" -ge 110 ]; then
    echo "✅ SUCCESS: Trade persistence working (expected ~120, got $TRADE_BATCHES)"
else
    echo "❌ FAILURE: Insufficient trade batches (expected >=110, got $TRADE_BATCHES)"
fi

# Cleanup
kill $PROVIDER_PID 2>/dev/null || true
rm -rf "$ANALYTICS_DATA_PATH"

echo ""
echo "=== Test Complete ==="
```

Run with: `bash test_trade_persistence.sh`

---

**Quickstart Complete**: You can now test the trade stream persistence feature locally and verify that analytics tools receive historical trade data.
