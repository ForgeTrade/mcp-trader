# Quickstart Guide: MCP Gateway System

**Date**: 2025-10-18
**Audience**: Developers implementing the MCP Gateway system

This guide provides step-by-step instructions for setting up and running the MCP Gateway system locally.

## Prerequisites

### Required Software

- **Python 3.11+** with `uv` package manager
- **Go 1.21+** for hello-go provider
- **Rust 1.75+** with Cargo for hello-rs provider
- **Protocol Buffers compiler** (`protoc` 3.x)
- **Docker & Docker Compose** (optional, for NATS JetStream)
- **Make** utility

### Install System Dependencies

**macOS**:
```bash
brew install python@3.11 go rust protobuf
pip install uv
```

**Ubuntu/Debian**:
```bash
sudo apt update
sudo apt install python3.11 golang rustc cargo protobuf-compiler
curl -LsSf https://astral.sh/uv/install.sh | sh
```

---

## Project Setup

### 1. Clone Repository and Navigate to Project

```bash
cd /path/to/mcp-trader
git checkout 001-specify-scripts-bash
```

### 2. Generate Protobuf Code

From repository root:

```bash
# Create pkg/proto directory if it doesn't exist
mkdir -p pkg/proto

# Copy contract
cp specs/001-specify-scripts-bash/contracts/provider.proto pkg/proto/

# Generate Python code
python -m grpc_tools.protoc \
  -I pkg/proto \
  --python_out=mcp-gateway/mcp_gateway/generated \
  --grpc_python_out=mcp-gateway/mcp_gateway/generated \
  pkg/proto/provider.proto

# Generate Go code
protoc -I pkg/proto \
  --go_out=providers/hello-go \
  --go-grpc_out=providers/hello-go \
  pkg/proto/provider.proto

# Generate Rust code (handled by build.rs in hello-rs)
# No manual step needed - Tonic build script runs during cargo build
```

---

## Running the System

### Step 1: Start NATS JetStream (Optional - for event streaming)

```bash
docker compose -f infra/docker-compose.yml up -d
```

Verify NATS is running:
```bash
docker logs mcp-trader-nats
```

### Step 2: Start Providers

**Terminal 1 - Go Provider**:
```bash
cd providers/hello-go
go run cmd/server/main.go
```

Expected output:
```
2025/10/18 12:00:00 Provider hello-go listening on :50051
```

**Terminal 2 - Rust Provider**:
```bash
cd providers/hello-rs
cargo run
```

Expected output:
```
Provider hello-rs listening on [::1]:50052
```

### Step 3: Configure Gateway

Edit `mcp-gateway/providers.yaml`:
```yaml
providers:
  - name: hello-go
    type: grpc
    address: localhost:50051
    enabled: true

  - name: hello-rs
    type: grpc
    address: localhost:50052
    enabled: true
```

### Step 4: Start MCP Gateway

**Terminal 3 - Gateway**:
```bash
cd mcp-gateway
uv sync  # Install dependencies
uv run python -m mcp_gateway.main
```

Expected output:
```
INFO:mcp_gateway:Gateway started, discovering providers...
INFO:mcp_gateway:Discovered 2 tools from hello-go
INFO:mcp_gateway:Discovered 2 tools from hello-rs
INFO:mcp_gateway:Gateway ready, listening on stdio
```

---

## Testing with MCP Inspector

### Install MCP Inspector

```bash
npx @modelcontextprotocol/inspector@latest
```

**Important**: Use version ≥ 0.14.1 due to security vulnerability in earlier versions.

### Connect Inspector to Gateway

1. Run MCP Inspector: `npx @modelcontextprotocol/inspector`
2. Select transport: **stdio**
3. Enter command: `uv run python -m mcp_gateway.main` (from mcp-gateway directory)
4. Click **Connect**

### Verify Capabilities

In Inspector UI, you should see:

**Tools**:
- `hello-go.echo.v1`
- `hello-go.sum.v1`
- `hello-rs.echo.v1`
- `hello-rs.sum.v1`

**Resources**:
- `hello://greeting` (from both providers)

**Prompts**:
- `hello-plan`

### Test Tool Invocation

**Test echo.v1**:
```json
{
  "tool": "hello-go.echo.v1",
  "arguments": {
    "message": "Hello, MCP!"
  }
}
```

Expected response:
```json
{
  "message": "Hello, MCP!"
}
```

**Test sum.v1**:
```json
{
  "tool": "hello-rs.sum.v1",
  "arguments": {
    "numbers": [1, 2, 3, 4, 5]
  }
}
```

Expected response:
```json
{
  "sum": 15
}
```

### Test Resource Access

Request URI: `hello://greeting`

Expected response:
```
Hello, MCP
```

### Test Prompt

Prompt name: `hello-plan`
Arguments:
```json
{
  "name": "Alice"
}
```

Expected response:
```
Hello, Alice! Let me propose a plan for you...
```

---

## Development Workflow

### Running Tests

**Gateway Tests**:
```bash
cd mcp-gateway
uv run pytest tests/
```

**Go Provider Tests**:
```bash
cd providers/hello-go
go test ./...
```

**Rust Provider Tests**:
```bash
cd providers/hello-rs
cargo test
```

### Making Changes

1. **Modify protobuf contract**: Edit `pkg/proto/provider.proto`
2. **Regenerate code**: Run protoc commands from setup section
3. **Update providers**: Implement new RPC methods
4. **Update gateway**: Add routing/proxy logic
5. **Run tests**: Verify all tests pass
6. **Test with Inspector**: Manual integration testing

### Hot Reloading

The gateway does not support hot reloading. Restart the gateway process after making changes:

```bash
# Ctrl+C to stop
uv run python -m mcp_gateway.main  # Restart
```

Providers also require restart after code changes.

---

## Troubleshooting

### Gateway fails to connect to provider

**Error**: `grpc.RpcError: Connection refused`

**Solutions**:
1. Verify provider is running: `ps aux | grep hello-go`
2. Check provider port: `lsof -i :50051`
3. Verify `providers.yaml` address matches provider listen address
4. Check firewall rules

### Tool invocation times out

**Error**: `Tool invocation exceeded 2.5 second timeout`

**Solutions**:
1. Check provider logs for errors
2. Verify provider is responding: `grpcurl -plaintext localhost:50051 provider.v1.Provider/ListCapabilities`
3. Increase timeout in gateway config (if justified)

### Schema validation fails

**Error**: `Invalid payload: 'message' is required`

**Solutions**:
1. Check tool invocation payload matches input schema
2. Verify schema file is correct JSON Schema 2020-12
3. Check for typos in field names (case-sensitive)

### NATS connection fails

**Error**: `NATS server unavailable`

**Solutions**:
1. Verify Docker is running: `docker ps`
2. Check NATS container: `docker logs mcp-trader-nats`
3. Restart NATS: `docker compose -f infra/docker-compose.yml restart`

---

## Next Steps

1. **Implement additional tools**: Add new tools to providers
2. **Enable event streaming**: Implement NATS publishers in providers
3. **Add authentication**: Implement perimeter auth per FR-029
4. **Monitoring**: Set up OpenTelemetry instrumentation
5. **Production deployment**: Containerize services with Kubernetes

See `tasks.md` (generated by `/speckit.tasks`) for detailed implementation tasks.

---

## Quick Reference

### Default Ports

- hello-go: `localhost:50051`
- hello-rs: `localhost:50052`
- NATS: `localhost:4222`
- NATS monitoring: `http://localhost:8222`

### Key Files

- Gateway config: `mcp-gateway/providers.yaml`
- Proto contract: `pkg/proto/provider.proto`
- Tool schemas: `pkg/schemas/*.schema.json`
- Docker Compose: `infra/docker-compose.yml`

### Useful Commands

```bash
# Check provider health
grpcurl -plaintext localhost:50051 provider.v1.Provider/ListCapabilities

# View NATS streams
docker exec -it mcp-trader-nats nats stream list

# Gateway logs (structured JSON)
uv run python -m mcp_gateway.main 2>&1 | jq .

# Run all tests
make test  # (if Makefile target exists)
```

---

## Resources

- MCP Specification: https://modelcontextprotocol.io/specification/2025-06-18
- gRPC Go: https://grpc.io/docs/languages/go/
- Tonic (Rust): https://docs.rs/tonic/
- MCP Python SDK: https://pypi.org/project/mcp/
- NATS JetStream: https://docs.nats.io/nats-concepts/jetstream
