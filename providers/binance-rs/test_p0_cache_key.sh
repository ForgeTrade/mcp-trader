#!/bin/bash
# Test P0 fix: Verify cache keys include options

set -e

echo "=== Testing P0 Fix: Cache Key Includes Options ==="
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
cat > /tmp/test_p0_cache.py <<'EOTEST'
import grpc
import json
import sys

# Import generated proto from mcp-gateway
sys.path.insert(0, "/home/limerc/repos/ForgeTrade/mcp-trader/mcp-gateway")
from mcp_gateway.generated.provider_pb2 import InvokeRequest, Json
from mcp_gateway.generated.provider_pb2_grpc import ProviderStub

channel = grpc.insecure_channel('localhost:50055')
stub = ProviderStub(channel)

print("TEST: Verify different options create different cache entries")
print("-" * 70)

# Request 1: Partial report with only price_overview section
print("\n1. Request BTCUSDT with only price_overview section")
payload1 = json.dumps({
    "symbol": "BTCUSDT",
    "options": {
        "include_sections": ["price_overview"]
    }
})
request1 = InvokeRequest(
    tool_name="binance.generate_market_report",
    payload=Json(value=payload1.encode('utf-8'))
)

response1 = stub.Invoke(request1)
result1 = json.loads(response1.result.value.decode('utf-8'))
markdown1 = result1.get("markdown_content", "")

# Count sections in first response
has_price = "## Price Overview" in markdown1
has_orderbook = "## Order Book Metrics" in markdown1
has_liquidity = "## Liquidity Analysis" in markdown1

print(f"   Price Overview: {'✅' if has_price else '❌'}")
print(f"   Order Book Metrics: {'❌ (expected)' if not has_orderbook else '⚠️  (unexpected)'}")
print(f"   Liquidity Analysis: {'❌ (expected)' if not has_liquidity else '⚠️  (unexpected)'}")

if has_price and not has_orderbook and not has_liquidity:
    print("   ✅ Correct: Only requested section present")
    test1_pass = True
else:
    print("   ❌ Wrong sections returned")
    test1_pass = False

# Request 2: Full report with all sections (default options)
import time
time.sleep(0.2)

print("\n2. Request BTCUSDT with default options (all sections)")
payload2 = json.dumps({"symbol": "BTCUSDT"})
request2 = InvokeRequest(
    tool_name="binance.generate_market_report",
    payload=Json(value=payload2.encode('utf-8'))
)

response2 = stub.Invoke(request2)
result2 = json.loads(response2.result.value.decode('utf-8'))
markdown2 = result2.get("markdown_content", "")

# Count sections in second response
has_price2 = "## Price Overview" in markdown2
has_orderbook2 = "## Order Book Metrics" in markdown2
has_liquidity2 = "## Liquidity Analysis" in markdown2

print(f"   Price Overview: {'✅' if has_price2 else '❌'}")
print(f"   Order Book Metrics: {'✅' if has_orderbook2 else '❌'}")
print(f"   Liquidity Analysis: {'✅' if has_liquidity2 else '❌'}")

if has_price2 and has_orderbook2 and has_liquidity2:
    print("   ✅ Correct: All sections present")
    test2_pass = True
else:
    print("   ❌ Missing sections in full report")
    test2_pass = False

# Request 3: Same as request 1 (should hit cache)
time.sleep(0.2)

print("\n3. Request BTCUSDT with price_overview again (should hit cache)")
response3 = stub.Invoke(request1)
result3 = json.loads(response3.result.value.decode('utf-8'))
markdown3 = result3.get("markdown_content", "")

# Should match the first partial report, not the full one
has_orderbook3 = "## Order Book Metrics" in markdown3
if not has_orderbook3:
    print("   ✅ Cached partial report returned correctly")
    test3_pass = True
else:
    print("   ❌ Wrong cached report returned (got full report instead of partial)")
    test3_pass = False

# Summary
print("\n" + "=" * 70)
if test1_pass and test2_pass and test3_pass:
    print("✅ All P0 Cache Key Tests Passed!")
    print("")
    print("Verified:")
    print("  - Different options create separate cache entries")
    print("  - Cached reports respect original options")
    print("  - No cache pollution between option combinations")
else:
    print("❌ Some tests failed")
    sys.exit(1)
EOTEST

# Run test
echo "3. Running P0 cache key tests..."
cd ../../mcp-gateway && uv run python /tmp/test_p0_cache.py
cd -

# Cleanup
echo ""
echo "4. Cleanup..."
kill $PROVIDER_PID 2>/dev/null || true
rm -f /tmp/test_p0_cache.py
rm -rf data/analytics 2>/dev/null || true

# Restore backed up data
if [ -d "data/analytics.backup" ]; then
    mv data/analytics.backup data/analytics
    echo "📦 Restored analytics data"
fi

echo "✅ Cleanup complete"
echo ""
echo "=== P0 Cache Key Tests Complete ==="
