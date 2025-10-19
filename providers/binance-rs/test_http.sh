#!/bin/bash
set -e

echo "=== T082: Testing HTTP Initialize and Session Management ==="
echo ""

# Step 1: Initialize
echo "Step 1: Initialize session"
INIT_RESPONSE=$(curl -s -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"initialize","id":1}')

echo "$INIT_RESPONSE" | jq '.' || echo "$INIT_RESPONSE"
SESSION_ID=$(echo "$INIT_RESPONSE" | jq -r '.result.sessionId // empty')

if [ -n "$SESSION_ID" ]; then
  echo "✅ Session ID: $SESSION_ID"
else
  echo "❌ No session ID"
  exit 1
fi

# Step 2: Tools list with session
echo ""
echo "Step 2: Test tools/list with session"
TOOLS=$(curl -s -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{"jsonrpc":"2.0","method":"tools/list","id":2}')

TOOL_COUNT=$(echo "$TOOLS" | jq '.result.tools | length')
echo "✅ Tools returned: $TOOL_COUNT"
echo "$TOOLS" | jq '.result.tools[0:3] | .[].name'

# Step 3: Without session
echo ""
echo "Step 3: Test without session (should accept)"
NO_SESSION=$(curl -s -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/list","id":3}')
echo "$NO_SESSION" | jq -r 'if .error then "❌ Error: " + .error.message else "✅ Success (tools/list works without session)" end'

echo ""
echo "=== T082 Complete ==="
