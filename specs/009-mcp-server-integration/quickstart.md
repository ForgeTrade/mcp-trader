# Quick Start Guide: MCP Server Integration

**Feature**: 009-mcp-server-integration
**Date**: 2025-10-20

## Overview

This guide shows how to set up and test the MCP server integration in both stdio (local) and SSE (remote) modes.

## Prerequisites

- Rust 1.75+ installed
- Binance API credentials (testnet recommended)
- Claude Desktop (for stdio testing) or curl (for SSE testing)

## Option 1: Stdio Mode (Local - Claude Desktop)

### 1. Build the Provider

```bash
cd providers/binance-rs
cargo build --release --features "mcp_server,orderbook,orderbook_analytics"
```

### 2. Configure Claude Desktop

Edit `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "binance": {
      "command": "/path/to/binance-provider",
      "args": ["--stdio"],
      "env": {
        "BINANCE_API_KEY": "your_testnet_api_key",
        "BINANCE_SECRET_KEY": "your_testnet_secret_key",
        "BINANCE_BASE_URL": "https://testnet.binance.vision",
        "RUST_LOG": "info"
      }
    }
  }
}
```

### 3. Restart Claude Desktop

```bash
# macOS
killall Claude
open -a Claude
```

### 4. Verify Connection

In Claude Desktop, check for the 🔌 icon. Click it to see "binance" server listed.

Try these commands:
- "What's the current Bitcoin price?"
- "Show me orderbook metrics for BTCUSDT"
- "List available market resources"

---

## Option 2: SSE Mode (Remote)

### 1. Build with SSE Support

```bash
cd providers/binance-rs
cargo build --release --features "mcp_server,sse,orderbook,orderbook_analytics"
```

### 2. Start SSE Server

```bash
export BINANCE_API_KEY="your_testnet_api_key"
export BINANCE_SECRET_KEY="your_testnet_secret_key"
export BINANCE_BASE_URL="https://testnet.binance.vision"
export RUST_LOG="info"

./target/release/binance-provider --sse --port 8000
```

### 3. Test SSE Handshake

```bash
# Get session ID
curl -X POST http://localhost:8000/mcp/sse \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}'
```

Response includes `Mcp-Session-Id` header - save this value.

### 4. Test Tool Call

```bash
SESSION_ID="<session-id-from-above>"

curl -X POST http://localhost:8000/mcp/message \
  -H "Content-Type: application/json" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/call",
    "params": {
      "name": "get_ticker",
      "arguments": {"symbol": "BTCUSDT"}
    }
  }'
```

### 5. Test Resource Read

```bash
curl -X POST http://localhost:8000/mcp/message \
  -H "Content-Type: application/json" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{
    "jsonrpc": "2.0",
    "id": 3,
    "method": "resources/read",
    "params": {
      "uri": "binance://market/btcusdt"
    }
  }'
```

---

## Testing Tools

### List Available Tools

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/list",
  "params": {}
}
```

### List Available Resources

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "resources/list",
  "params": {}
}
```

### List Available Prompts

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "prompts/list",
  "params": {}
}
```

### Call a Prompt

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "prompts/get",
  "params": {
    "name": "trading_analysis",
    "arguments": {
      "symbol": "BTCUSDT",
      "strategy": "balanced"
    }
  }
}
```

---

## Troubleshooting

### Stdio Mode

**Problem**: "binance" server not appearing in Claude Desktop

**Solutions**:
1. Check logs: `tail -f ~/Library/Logs/Claude/mcp-server-binance.log`
2. Verify binary path in config is absolute
3. Ensure RUST_LOG is set to "info" or "debug"
4. Restart Claude Desktop completely

**Problem**: Tools execute but return errors

**Solutions**:
1. Check Binance API credentials are correct
2. Verify using testnet URL
3. Check logs for rate limit errors

### SSE Mode

**Problem**: Connection timeout after 30 seconds

**Solutions**:
- This is expected behavior
- Re-establish connection via new POST /mcp/sse request
- Implement keep-alive pings from client

**Problem**: 503 Service Unavailable

**Solutions**:
- Max 50 concurrent connections reached
- Wait for stale sessions to timeout (30s)
- Check server logs for connection count

**Problem**: 404 Not Found on /mcp/message

**Solutions**:
- Session ID expired or invalid
- Re-establish connection via POST /mcp/sse
- Ensure Mcp-Session-Id header is sent correctly

---

## Next Steps

After verifying the integration works:

1. Run full test suite: `cargo test --features "mcp_server,sse,orderbook_analytics"`
2. Deploy to production (see Shuttle deployment guide if using cloud)
3. Monitor logs for errors and performance metrics
4. Configure additional MCP clients (ChatGPT, other AI agents)

---

**Support**: See `plan.md` for architecture details and `data-model.md` for entity definitions.
