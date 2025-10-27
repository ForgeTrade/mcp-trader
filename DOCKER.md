# Docker Deployment Guide

Docker Compose configuration for running the MCP Trader stack.

## Architecture

```
┌─────────────────────┐
│   mcp-gateway       │
│  (SSE Server)       │
│   Port: 3001        │
└──────────┬──────────┘
           │ gRPC
           ▼
┌─────────────────────┐
│ binance-provider    │
│  (Rust gRPC)        │
│   Port: 50053       │
└─────────────────────┘
```

## Quick Start

### 1. Configuration

Create `.env` file in `providers/binance-rs/`:

```bash
cp providers/binance-rs/.env.example providers/binance-rs/.env
# Edit with your Binance API credentials
```

Or set environment variables:

```bash
export BINANCE_API_KEY="your_api_key"
export BINANCE_API_SECRET="your_api_secret"
```

### 2. Build and Run

```bash
# Build and start all services
docker compose up -d

# View logs
docker compose logs -f

# View specific service logs
docker compose logs -f binance-provider
docker compose logs -f mcp-gateway
```

### 3. Verify

```bash
# Check service health
docker compose ps

# Test binance provider (gRPC)
grpcurl -plaintext localhost:50053 list

# Test mcp-gateway (SSE)
curl http://localhost:3001/sse
```

## Services

### binance-provider

**Image**: Custom Rust build
**Port**: 50053 (gRPC)
**Features**:
- Real-time order book analytics
- RocksDB time-series storage
- 12 market data tools
- WebSocket + REST fallback

**Volumes**:
- `binance-analytics`: Persistent RocksDB data

### mcp-gateway

**Image**: Custom Python build
**Port**: 3001 (SSE)
**Features**:
- MCP protocol over SSE
- Multi-provider orchestration
- Tool routing and validation

## Common Commands

```bash
# Start services
docker compose up -d

# Stop services
docker compose down

# Restart a service
docker compose restart binance-provider

# Rebuild after code changes
docker compose build
docker compose up -d

# View resource usage
docker compose stats

# Clean up everything (including volumes)
docker compose down -v

# Scale services (if needed)
docker compose up -d --scale binance-provider=2
```

## Development

### Hot Reload (Local Development)

For development with hot reload, use local builds instead:

```bash
# Terminal 1: Run Rust provider locally
cd providers/binance-rs
cargo run --release -- --grpc

# Terminal 2: Run Python gateway locally
cd mcp-gateway
python -m mcp_gateway.sse_server
```

### Custom Configuration

Override default settings via environment variables in `compose.yml`:

```yaml
environment:
  - RUST_LOG=debug          # Rust logging level
  - MCP_SSE_PORT=3002       # Custom SSE port
  - ANALYTICS_DATA_PATH=/custom/path
```

## Troubleshooting

### Provider won't start

```bash
# Check logs
docker compose logs binance-provider

# Common issues:
# 1. Missing API credentials → Check .env file
# 2. Port conflict → Change ports in compose.yml
# 3. Build failure → Check Rust version (1.75+)
```

### Gateway can't connect to provider

```bash
# Verify provider is running
docker compose ps binance-provider

# Check network connectivity
docker compose exec mcp-gateway ping binance-provider

# Verify gRPC endpoint
docker compose exec mcp-gateway grpcurl -plaintext binance-provider:50053 list
```

### Analytics data persistence

Analytics data is stored in the `binance-analytics` volume:

```bash
# Inspect volume
docker volume inspect mcp-trader_binance-analytics

# Backup volume
docker run --rm -v mcp-trader_binance-analytics:/data -v $(pwd):/backup \
  alpine tar czf /backup/analytics-backup.tar.gz -C /data .

# Restore volume
docker run --rm -v mcp-trader_binance-analytics:/data -v $(pwd):/backup \
  alpine tar xzf /backup/analytics-backup.tar.gz -C /data
```

## Production Considerations

### Security

1. **API Credentials**: Use Docker secrets instead of .env files:

```yaml
secrets:
  binance_api_key:
    external: true
  binance_api_secret:
    external: true

services:
  binance-provider:
    secrets:
      - binance_api_key
      - binance_api_secret
```

2. **Network Isolation**: Expose only necessary ports:

```yaml
ports:
  - "127.0.0.1:3001:3001"  # Bind to localhost only
```

### Resource Limits

```yaml
services:
  binance-provider:
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 2G
        reservations:
          cpus: '1'
          memory: 1G
```

### Monitoring

```yaml
# Add Prometheus metrics exporter
services:
  prometheus:
    image: prom/prometheus
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
    ports:
      - "9090:9090"
```

## Integration with ChatGPT

The SSE endpoint is compatible with ChatGPT MCP integration:

1. **Endpoint**: `http://localhost:3001/sse`
2. **Protocol**: MCP over SSE (Server-Sent Events)
3. **Tools**: All binance tools exposed via unified routing

See ChatGPT MCP documentation for integration details.
