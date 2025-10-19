#!/bin/bash
set -e

echo "=== T086: Testing HTTP Tools via curl ==="
echo ""

# Get session
SESSION_ID=$(curl -s -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"initialize","id":1}' | jq -r '.result.sessionId')

echo "Session ID: $SESSION_ID"
echo ""

# Test 1: Get ticker (public API)
echo "Test 1: binance.get_ticker (BTCUSDT)"
TICKER=$(curl -s -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{
    "jsonrpc":"2.0",
    "method":"tools/call",
    "params":{
      "name":"binance.get_ticker",
      "arguments":{"symbol":"BTCUSDT"}
    },
    "id":2
  }')

echo "$TICKER" | jq -r 'if .result then "✅ Success - Price: " + (.result.content[0].text | fromjson | .lastPrice) else "❌ Error: " + (.error.message // "unknown") end'
echo ""

# Test 2: Get orderbook L1
echo "Test 2: binance.orderbook_l1 (ETHUSDT)"
OB_L1=$(curl -s -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{
    "jsonrpc":"2.0",
    "method":"tools/call",
    "params":{
      "name":"binance.orderbook_l1",
      "arguments":{"symbol":"ETHUSDT"}
    },
    "id":3
  }')

echo "$OB_L1" | jq -r 'if .result then "✅ Success - Spread: " + (.result.content[0].text | fromjson | .spread) else "❌ Error: " + (.error.message // "unknown") end'
echo ""

# Test 3: List all tools
echo "Test 3: List all tools"
TOOLS=$(curl -s -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{"jsonrpc":"2.0","method":"tools/list","id":4}')

TOOL_COUNT=$(echo "$TOOLS" | jq '.result.tools | length')
echo "✅ Total tools available: $TOOL_COUNT"
echo ""
echo "Available tools:"
echo "$TOOLS" | jq -r '.result.tools[].name' | sort

echo ""
echo "=== T086 Complete ==="
