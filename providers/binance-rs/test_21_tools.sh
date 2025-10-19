#!/bin/bash
set -e

echo "Starting HTTP server..."
RUST_LOG=info ./target/release/binance-provider --http --port 3000 > /tmp/http_server2.log 2>&1 &
SERVER_PID=$!
sleep 3

echo "Initializing session..."
SESSION_ID=$(curl -s -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"initialize","id":1}' | jq -r '.result.sessionId')

echo "Session ID: $SESSION_ID"
echo ""

echo "Fetching tools list..."
TOOLS=$(curl -s -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{"jsonrpc":"2.0","method":"tools/list","id":2}')

TOOL_COUNT=$(echo "$TOOLS" | jq '.result.tools | length')
echo "✅ Total tools available: $TOOL_COUNT"
echo ""
echo "All tools:"
echo "$TOOLS" | jq -r '.result.tools[].name' | sort
echo ""

if [ "$TOOL_COUNT" -eq 21 ]; then
  echo "✅ SUCCESS: All 21 tools registered!"
else
  echo "⚠️  WARNING: Expected 21 tools, got $TOOL_COUNT"
fi

kill $SERVER_PID 2>/dev/null
