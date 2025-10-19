# ChatGPT Integration Guide

## Overview

The HTTP/JSON-RPC 2.0 transport we implemented enables **direct ChatGPT integration** via Custom GPTs with Actions (OpenAI's plugin system).

## Integration Architecture

```
┌──────────────────────────────────────────────────────┐
│                 ChatGPT (Custom GPT)                 │
│                         ↓                            │
│               Actions (HTTP/JSON-RPC)                │
│                         ↓                            │
│          Binance Provider (Port 3000)                │
│           - HTTP/JSON-RPC 2.0 endpoint               │
│           - Session management                       │
│           - 21 MCP tools                             │
└──────────────────────────────────────────────────────┘
```

## Why HTTP Transport Works for ChatGPT

✅ **JSON-RPC 2.0**: Standard protocol ChatGPT Actions understand  
✅ **REST API**: POST /mcp endpoint compatible with GPT Actions  
✅ **Session Management**: UUID-based sessions for stateful interactions  
✅ **OpenAPI Compatible**: Can generate OpenAPI spec from our schemas  

## Setup Guide

### Step 1: Start HTTP Server

```bash
# Production deployment
./target/release/binance-provider --http --port 3000

# Or with environment variables
export BINANCE_API_KEY="your_key"
export BINANCE_API_SECRET="your_secret"
./target/release/binance-provider --http
```

### Step 2: Create OpenAPI Specification

ChatGPT Actions require an OpenAPI 3.0 spec. Here's a minimal example:

```yaml
openapi: 3.0.0
info:
  title: Binance MCP Provider
  version: 0.1.0
  description: 21 Binance trading tools via MCP protocol

servers:
  - url: https://your-server.com
    description: Production server

paths:
  /mcp:
    post:
      summary: Execute MCP JSON-RPC 2.0 requests
      operationId: mcpRpc
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              properties:
                jsonrpc:
                  type: string
                  enum: ["2.0"]
                method:
                  type: string
                  enum:
                    - initialize
                    - tools/list
                    - tools/call
                params:
                  type: object
                id:
                  oneOf:
                    - type: string
                    - type: number
              required:
                - jsonrpc
                - method
      responses:
        '200':
          description: Successful response
          headers:
            Mcp-Session-Id:
              schema:
                type: string
                format: uuid
          content:
            application/json:
              schema:
                type: object
                properties:
                  jsonrpc:
                    type: string
                  result:
                    type: object
                  error:
                    type: object
                  id:
                    oneOf:
                      - type: string
                      - type: number
```

### Step 3: Configure Custom GPT

1. Go to **ChatGPT → Explore → Create a GPT**
2. In **Configure** tab:
   - Name: "Binance Trading Assistant"
   - Description: "Access 21 Binance trading tools including advanced analytics"
   - Instructions: "You are a cryptocurrency trading assistant..."

3. In **Actions** tab:
   - Click "Create new action"
   - Paste the OpenAPI spec above
   - Set authentication: **API Key** or **None** (if public)
   - Add session header: `Mcp-Session-Id`

4. Add example tools in instructions:
```
Available tools:
- binance.get_ticker: Get 24h price statistics
- binance.get_order_flow: Analyze bid/ask pressure
- binance.get_volume_profile: Generate volume distribution
- binance.detect_market_anomalies: Find suspicious activity
- binance.get_liquidity_vacuums: Identify stop-loss zones
... (all 21 tools)
```

### Step 4: Test Integration

In your Custom GPT chat:

```
User: "What's the current BTC price?"

ChatGPT calls: binance.get_ticker with symbol=BTCUSDT

User: "Analyze the order flow for ETH in the last 60 seconds"

ChatGPT calls: binance.get_order_flow with symbol=ETHUSDT, window_duration_secs=60
```

## Alternative: Direct API Integration

If you don't want to use Custom GPTs, you can expose our HTTP endpoint as a standalone API:

### Generate OpenAPI Spec from Tool Schemas

```bash
# Create a script to extract schemas from capabilities
curl -s -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"initialize","id":1}' | jq -r '.result.sessionId' > /tmp/session

SESSION=$(cat /tmp/session)

curl -s -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -H "Mcp-Session-Id: $SESSION" \
  -d '{"jsonrpc":"2.0","method":"tools/list","id":2}' | jq '.result.tools'
```

## Session Management for ChatGPT

ChatGPT Actions are **stateless** by default, so session management works like this:

1. **First request (initialize)**: ChatGPT calls without session
   - Server returns `sessionId` in response
   - ChatGPT stores session ID in conversation context

2. **Subsequent requests**: ChatGPT includes session header
   - `Mcp-Session-Id: <uuid>`
   - Server validates and extends timeout (30 minutes)

3. **Session expiry**: After 30 minutes of inactivity
   - Server returns error: "Session expired"
   - ChatGPT automatically re-initializes

## MCP Gateway vs Direct HTTP

### Option 1: MCP Gateway (Claude)
```
Claude Desktop → MCP Gateway (gRPC) → Binance Provider (Port 50053)
```
**Use case:** Claude Desktop, Claude API integration

### Option 2: Direct HTTP (ChatGPT, Web Apps)
```
ChatGPT/Browser → Binance Provider HTTP (Port 3000)
```
**Use case:** ChatGPT Custom GPTs, web applications, debugging

### Option 3: Hybrid (Both)
```
┌─────────────────┐
│ Claude Desktop  │ ──→ gRPC (Port 50053)
└─────────────────┘           ↓
                         Binance Provider
┌─────────────────┐           ↓
│    ChatGPT      │ ──→ HTTP (Port 3000)
└─────────────────┘
```
**Use case:** Multi-platform deployment

## Production Deployment for ChatGPT

### 1. Deploy with HTTPS

ChatGPT requires HTTPS for Actions. Use nginx or Caddy:

```nginx
# /etc/nginx/sites-available/binance-mcp
server {
    listen 443 ssl http2;
    server_name api.yourdomain.com;

    ssl_certificate /etc/letsencrypt/live/api.yourdomain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/api.yourdomain.com/privkey.pem;

    location /mcp {
        proxy_pass http://127.0.0.1:3000/mcp;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header Mcp-Session-Id $http_mcp_session_id;
    }
}
```

### 2. Authentication Options

**Option A: No Auth (Public Demo)**
- No additional configuration needed
- Rate limit with nginx

**Option B: API Key**
```bash
# Add API key validation in nginx
if ($http_authorization != "Bearer your_secret_key") {
    return 401;
}
```

**Option C: OAuth 2.0**
- Implement OAuth provider
- Configure in Custom GPT Actions

### 3. Rate Limiting

```nginx
limit_req_zone $binary_remote_addr zone=mcp:10m rate=10r/s;

location /mcp {
    limit_req zone=mcp burst=20 nodelay;
    proxy_pass http://127.0.0.1:3000/mcp;
}
```

## Example ChatGPT Conversation

```
User: "What's the current Bitcoin price and analyze the market health"

ChatGPT:
1. Calls binance.get_ticker(symbol="BTCUSDT")
   → Price: $108,731.20
   
2. Calls binance.get_microstructure_health(symbol="BTCUSDT")
   → Composite Score: 87/100
   → Status: Healthy
   → Spread stability: 92/100
   → Liquidity depth: 85/100

Response: "Bitcoin is currently trading at $108,731.20. The market 
microstructure health score is 87/100 (Healthy), indicating stable 
spreads and good liquidity depth..."
```

## Limitations & Workarounds

### 1. **ChatGPT doesn't maintain MCP state**
**Workaround:** Use session IDs in HTTP headers

### 2. **OpenAPI spec size limits**
**Workaround:** Create multiple Custom GPTs (Market Data, Analytics, Trading)

### 3. **Response timeout (30s)**
**Workaround:** Implement async operations with polling

## Testing Your Integration

```bash
# 1. Start server
./target/release/binance-provider --http --port 3000

# 2. Test with curl (simulating ChatGPT)
SESSION=$(curl -s -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"initialize","id":1}' | jq -r '.result.sessionId')

# 3. Call a tool
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -H "Mcp-Session-Id: $SESSION" \
  -d '{
    "jsonrpc":"2.0",
    "method":"tools/call",
    "params":{
      "name":"binance.get_ticker",
      "arguments":{"symbol":"BTCUSDT"}
    },
    "id":2
  }' | jq '.'
```

## Conclusion

✅ **Yes, MCP Gateway can connect to ChatGPT!**

The HTTP transport we implemented is specifically designed for this use case:
- JSON-RPC 2.0 protocol (ChatGPT compatible)
- REST API endpoint (POST /mcp)
- Session management (UUID-based)
- All 21 tools accessible

**Recommendation:** Use **Direct HTTP integration** for ChatGPT Custom GPTs, as it's simpler than routing through MCP Gateway (which is Claude-specific).
