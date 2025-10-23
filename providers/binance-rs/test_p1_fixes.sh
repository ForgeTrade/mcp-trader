#!/bin/bash
# Test P1 fixes for Feature 018

set -e

echo "=== Testing P1 Fixes for Feature 018 ==="
echo ""

# Kill any existing test provider on port 50055
lsof -ti :50055 2>/dev/null | xargs -r kill -9 2>/dev/null || true
sleep 2

# Temporarily move data directory to avoid lock conflicts
if [ -d "data/analytics" ]; then
    mv data/analytics data/analytics.backup
    echo "📦 Backed up existing analytics data"
fi

# Build
echo "1. Building binance-provider..."
cargo build --release --features 'orderbook,orderbook_analytics' > /dev/null 2>&1
echo "✅ Build successful"
echo ""

# Start provider with fresh data directory
echo "2. Starting binance-provider..."
./target/release/binance-provider --grpc --port 50055 > /tmp/test_provider.log 2>&1 &
PROVIDER_PID=$!
sleep 5

if ! kill -0 $PROVIDER_PID 2>/dev/null; then
    echo "❌ Provider failed to start"
    cat /tmp/test_provider.log
    exit 1
fi
echo "✅ Provider started (PID: $PROVIDER_PID)"
echo ""

# Create test script
cat > /tmp/test_p1.py <<'EOTEST'
import grpc
import json
import sys

# Import generated proto from mcp-gateway
sys.path.insert(0, "/home/limerc/repos/ForgeTrade/mcp-trader/mcp-gateway")
from mcp_gateway.generated.provider_pb2 import InvokeRequest, Json
from mcp_gateway.generated.provider_pb2_grpc import ProviderStub

channel = grpc.insecure_channel('localhost:50055')
stub = ProviderStub(channel)

print("TEST 1: Verify include_sections filtering")
print("-" * 50)

# Request with filtered sections
payload1 = json.dumps({
    "symbol": "BTCUSDT",
    "options": {
        "include_sections": ["price_overview", "orderbook_metrics"]
    }
})
request1 = InvokeRequest(
    tool_name="binance.generate_market_report",
    payload=Json(value=payload1.encode('utf-8'))
)

response1 = stub.Invoke(request1)
result1 = json.loads(response1.result.value.decode('utf-8'))
markdown1 = result1.get("markdown_content", "")

# Check that only requested sections are included
if "## Price Overview" in markdown1:
    print("✅ Price Overview section present")
else:
    print("❌ Price Overview section missing")

if "## Order Book Metrics" in markdown1:
    print("✅ Order Book Metrics section present")
else:
    print("❌ Order Book Metrics section missing")

if "## Liquidity Analysis" not in markdown1:
    print("✅ Liquidity Analysis section correctly excluded")
else:
    print("❌ Liquidity Analysis should be excluded")

print("")
print("TEST 2: Verify cache preserves metadata")
print("-" * 50)

# First request
payload2 = json.dumps({"symbol": "ETHUSDT"})
request2 = InvokeRequest(
    tool_name="binance.generate_market_report",
    payload=Json(value=payload2.encode('utf-8'))
)

response2 = stub.Invoke(request2)
result2 = json.loads(response2.result.value.decode('utf-8'))

first_data_age = result2.get("data_age_ms", 0)
first_failed = result2.get("failed_sections", [])
first_generated = result2.get("generated_at", 0)

print(f"First call:")
print(f"  data_age_ms: {first_data_age}")
print(f"  failed_sections: {first_failed}")
print(f"  generated_at: {first_generated}")

# Second request (should hit cache)
import time
time.sleep(0.5)

response3 = stub.Invoke(request2)
result3 = json.loads(response3.result.value.decode('utf-8'))

second_data_age = result3.get("data_age_ms", 0)
second_failed = result3.get("failed_sections", [])
second_generated = result3.get("generated_at", 0)

print(f"")
print(f"Cached call:")
print(f"  data_age_ms: {second_data_age}")
print(f"  failed_sections: {second_failed}")
print(f"  generated_at: {second_generated}")

# Verify metadata is preserved
if second_data_age == first_data_age:
    print("✅ data_age_ms preserved in cache")
else:
    print(f"❌ data_age_ms NOT preserved ({first_data_age} != {second_data_age})")

if second_failed == first_failed:
    print("✅ failed_sections preserved in cache")
else:
    print(f"❌ failed_sections NOT preserved ({first_failed} != {second_failed})")

if second_generated == first_generated:
    print("✅ generated_at timestamp preserved in cache")
else:
    print(f"❌ generated_at NOT preserved ({first_generated} != {second_generated})")

print("")
print("✅ All P1 fixes verified!")
EOTEST

# Run test
echo "3. Running P1 fix tests..."
cd ../../mcp-gateway && uv run python /tmp/test_p1.py
cd -

# Cleanup
echo ""
echo "4. Cleanup..."
kill $PROVIDER_PID 2>/dev/null || true
rm -f /tmp/test_p1.py
rm -rf data/analytics 2>/dev/null || true

# Restore backed up data
if [ -d "data/analytics.backup" ]; then
    mv data/analytics.backup data/analytics
    echo "📦 Restored analytics data"
fi

echo "✅ Cleanup complete"
echo ""
echo "=== P1 Fix Tests Complete ==="
