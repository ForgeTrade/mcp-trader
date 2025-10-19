#!/bin/bash
# MCP Gateway Test Script - Interactive JSON-RPC

echo "=== MCP Gateway Manual Test Interface ==="
echo ""
echo "Starting MCP Gateway..."

# Start gateway in background and capture its PID
uv run python -m mcp_gateway.main | tee /tmp/gateway_output.log &
GATEWAY_PID=$!

sleep 2

# Send initialize request
echo '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0.0"}},"id":1}' | uv run python -m mcp_gateway.main

wait $GATEWAY_PID
