# MCP Gateway Manual Testing Guide

## ✅ Current Status

**Binance gRPC Provider:** ✅ Running on port 50053  
**MCP Gateway:** ✅ Verified connection (21 tools discovered)  
**Logs:** 
- Provider: `/tmp/grpc_provider.log`
- Gateway: `/tmp/mcp_gateway.log`

## Gateway Connection Log

```
2025-10-19 20:00:06,437 - INFO - Retrieved capabilities from binance-rs: 21 tools
2025-10-19 20:00:06,437 - INFO - Provider binance-rs: 21 tools, 1 resources, 2 prompts
2025-10-19 20:00:06,441 - INFO - MCP Gateway server started on stdio
```

✅ **Success!** The gateway successfully connected to the Binance provider and loaded all 21 tools.

## How MCP Gateway Works

The MCP Gateway uses **stdio** (standard input/output) for communication, which is designed for:
1. **Claude Desktop** integration
2. **MCP protocol clients** that communicate via JSON-RPC over stdio

It does NOT run as an HTTP server - that's what our Binance provider's HTTP transport is for.

## Architecture

```
┌─────────────────────────────────────────────┐
│         Claude Desktop / CLI                │
│                 ↓ (stdio)                   │
│            MCP Gateway                      │
│         (JSON-RPC over stdio)               │
│                 ↓ (gRPC)                    │
│       Binance Provider (Port 50053)         │
│                 ↓                           │
│          Binance REST/WebSocket API         │
└─────────────────────────────────────────────┘
```

## Manual Testing Options

### Option 1: Test with MCP stdio Protocol

```bash
cd /home/limerc/repos/ForgeTrade/mcp-trader/mcp-gateway

# Start gateway and send JSON-RPC request
echo '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}' | uv run python -m mcp_gateway.main
```

Expected response:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "protocolVersion": "2024-11-05",
    "capabilities": {...},
    "serverInfo": {"name": "mcp-gateway", "version": "0.1.0"}
  },
  "id": 1
}
```

### Option 2: Test with Interactive Session

```bash
# Start gateway in interactive mode
uv run python -m mcp_gateway.main

# Then send JSON-RPC requests line by line:

# 1. Initialize
{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}

# 2. List tools
{"jsonrpc":"2.0","method":"tools/list","id":2}

# 3. Call a tool
{"jsonrpc":"2.0","method":"tools/call","params":{"name":"binance.get_ticker","arguments":{"symbol":"BTCUSDT"}},"id":3}

# 4. Exit (Ctrl+D)
```

### Option 3: Use Claude Desktop

1. Configure Claude Desktop to use the MCP Gateway:

**~/.config/claude/mcp_servers.json**:
```json
{
  "mcpServers": {
    "binance-gateway": {
      "command": "uv",
      "args": [
        "run",
        "--directory",
        "/home/limerc/repos/ForgeTrade/mcp-trader/mcp-gateway",
        "python",
        "-m",
        "mcp_gateway.main"
      ]
    }
  }
}
```

2. Restart Claude Desktop
3. The 21 Binance tools should appear in Claude's tool list

### Option 4: Use Python MCP Client

```bash
# Install MCP client
uv pip install mcp

# Create test client
cat > test_client.py << 'PYTHON'
import asyncio
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client

async def main():
    server_params = StdioServerParameters(
        command="uv",
        args=[
            "run",
            "--directory",
            "/home/limerc/repos/ForgeTrade/mcp-trader/mcp-gateway",
            "python",
            "-m",
            "mcp_gateway.main"
        ]
    )
    
    async with stdio_client(server_params) as (read, write):
        async with ClientSession(read, write) as session:
            await session.initialize()
            
            # List all tools
            tools = await session.list_tools()
            print(f"Available tools: {len(tools.tools)}")
            for tool in tools.tools[:5]:
                print(f"  - {tool.name}: {tool.description}")
            
            # Call binance.get_ticker
            result = await session.call_tool(
                "binance.get_ticker",
                {"symbol": "BTCUSDT"}
            )
            print(f"\nBTC Price result: {result}")

asyncio.run(main())
PYTHON

# Run test client
uv run python test_client.py
```

## Verifying the Setup

### Check Provider Status
```bash
# Provider should be running
ps aux | grep binance-provider | grep -v grep

# Check provider logs
tail -20 /tmp/grpc_provider.log
```

Expected log output:
```
INFO Starting gRPC server on 0.0.0.0:50053
INFO   - 21 tools (16 base + 5 analytics)
INFO   - 4 resources (market, balances, trades, orders)
INFO   - 2 prompts (trading-analysis, portfolio-risk)
```

### Test Direct gRPC Connection
```bash
# If grpcurl is installed
grpcurl -plaintext localhost:50053 provider.v1.Provider/ListCapabilities
```

## Troubleshooting

### Issue: Gateway exits immediately
**Cause:** Normal behavior when no stdin input  
**Solution:** Use one of the testing methods above that provides stdin

### Issue: "Connection refused" errors
**Cause:** Provider not running  
**Solution:** 
```bash
cd /home/limerc/repos/ForgeTrade/mcp-trader/providers/binance-rs
RUST_LOG=info ./target/release/binance-provider --grpc --port 50053 &
```

### Issue: Tools not appearing in Claude Desktop
**Cause:** Configuration file incorrect  
**Solution:** Verify mcp_servers.json path and syntax

## Quick Start for Manual Testing

```bash
# 1. Ensure provider is running
cd /home/limerc/repos/ForgeTrade/mcp-trader/providers/binance-rs
RUST_LOG=info ./target/release/binance-provider --grpc --port 50053 > /tmp/provider.log 2>&1 &

# 2. Test gateway connection
cd /home/limerc/repos/ForgeTrade/mcp-trader/mcp-gateway
echo '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}' | uv run python -m mcp_gateway.main 2>&1 | jq '.'

# 3. List all tools
echo '{"jsonrpc":"2.0","method":"tools/list","id":2}' | uv run python -m mcp_gateway.main 2>&1 | grep -A 1000 '"jsonrpc"' | jq '.'
```

## Summary

✅ **MCP Gateway is working correctly!**

The gateway successfully:
- Connected to binance-rs provider on port 50053
- Discovered all 21 tools
- Started stdio server

To actually use it, you need to connect via:
- Claude Desktop (recommended)
- MCP Python client
- JSON-RPC over stdio

The gateway is NOT an HTTP server - use the Binance provider's HTTP transport (port 3000) for web/ChatGPT integration.
