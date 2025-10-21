# Quickstart: Binance Provider Integration

**Feature**: 002-binance-provider-integration
**Date**: 2025-10-18
**Purpose**: Developer setup and testing guide for Binance provider

## Prerequisites

### Required Tools

- **Rust** 1.75+ with cargo
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  rustup update
  rustc --version  # Should be 1.75.0 or higher
  ```

- **Protocol Buffers Compiler** (protoc)
  ```bash
  # Ubuntu/Debian
  sudo apt-get install -y protobuf-compiler

  # macOS
  brew install protobuf

  # Verify installation
  protoc --version  # Should be 3.x or higher
  ```

- **Go** 1.21+ (for protoc-gen-go plugins)
  ```bash
  go install google.golang.org/protobuf/cmd/protoc-gen-go@latest
  go install google.golang.org/grpc/cmd/protoc-gen-go-grpc@latest

  # Verify Go tools are in PATH
  export PATH=$PATH:$(go env GOPATH)/bin
  protoc-gen-go --version
  ```

- **Python** 3.11+ with uv
  ```bash
  # Install uv (recommended)
  curl -LsSf https://astral.sh/uv/install.sh | sh

  # Or use pip
  pip install uv

  # Verify
  uv --version
  python3 --version  # Should be 3.11.0 or higher
  ```

### Optional Tools

- **Binance Testnet Account** (for testing with real API)
  - Sign up at https://testnet.binance.vision/
  - Generate API key/secret from account settings

- **MCP Inspector** (for debugging MCP protocol)
  ```bash
  npm install -g @modelcontextprotocol/inspector
  ```

---

## Project Setup

### 1. Clone and Navigate

```bash
cd /home/limerc/repos/ForgeQuant/mcp-trader
git checkout 002-binance-provider-integration
```

### 2. Install Dependencies

**Python Gateway Dependencies**:
```bash
cd mcp-gateway
uv sync
cd ..
```

**Binance Provider Dependencies** (will be installed during build):
```bash
cd providers/binance-rs
cargo fetch  # Pre-fetch dependencies
cd ../..
```

---

## Building the Binance Provider

### Generate Protobuf Code

```bash
# From repository root
make proto-gen
```

This generates:
- `providers/binance-rs/src/pb/provider.rs` (Rust protobuf types)
- `providers/binance-rs/src/pb/provider.tonic.rs` (Rust gRPC server stubs)
- `mcp-gateway/mcp_gateway/generated/provider_pb2.py` (Python protobuf types)

### Build the Provider Binary

**With orderbook feature** (default):
```bash
make build-binance
# Or manually:
cd providers/binance-rs
cargo build --release
```

**Without orderbook feature** (basic mode):
```bash
cd providers/binance-rs
cargo build --release --no-default-features
```

**Verify build**:
```bash
ls -lh providers/binance-rs/target/release/binance-provider
# Should show ~15-20MB binary
```

---

## Configuration

### Environment Variables

Create a `.env` file in the repository root or export variables:

```bash
# API Credentials (required for account/trading tools)
export BINANCE_API_KEY="your_testnet_api_key"
export BINANCE_API_SECRET="your_testnet_api_secret"

# API Configuration
export BINANCE_BASE_URL="https://testnet.binance.vision"

# Logging
export RUST_LOG="info"
```

**Security Note**: Never commit API credentials to version control. Use `.env` file (already in `.gitignore`) or environment variables.

### Gateway Configuration

Update `mcp-gateway/providers.yaml`:

```yaml
providers:
  - name: hello-go
    type: grpc
    address: localhost:50051
    enabled: true

  - name: binance-rs  # NEW
    type: grpc
    address: localhost:50052  # Different port
    enabled: true
    metadata:
      description: "Binance cryptocurrency trading provider"
      version: "0.1.0"
      features: ["orderbook"]
```

---

## Running the System

### Terminal 1: Start Binance Provider

```bash
cd providers/binance-rs
./target/release/binance-provider --grpc --port 50052
```

**Expected output**:
```
INFO binance_provider: Starting Binance gRPC provider on 0.0.0.0:50052
INFO binance_provider: Loaded 16 tools
INFO binance_provider: Loaded 4 resources
INFO binance_provider: Loaded 2 prompts
INFO binance_provider: Provider ready, waiting for connections...
```

### Terminal 2: Start Gateway

```bash
cd mcp-gateway
uv run python -m mcp_gateway.main
```

**Expected output**:
```
INFO:mcp_gateway.main:Initializing MCP Gateway...
INFO:mcp_gateway.main:Loaded 2 providers from providers.yaml
INFO:mcp_gateway.main:Created gRPC client for provider: hello-go
INFO:mcp_gateway.main:Created gRPC client for provider: binance-rs
INFO:mcp_gateway.main:Provider binance-rs: 16 tools, 4 resources, 2 prompts
INFO:mcp_gateway.main:MCP Gateway server started on stdio
```

### Terminal 3: Test with MCP Inspector

```bash
npx @modelcontextprotocol/inspector uv run python -m mcp_gateway.main
```

Opens browser at `http://localhost:5173` with MCP Inspector UI.

---

## Testing

### Quick Smoke Test

**Test 1: List Capabilities**

```python
# In MCP Inspector or via Python client
tools = await client.list_tools()
print(f"Binance tools: {len([t for t in tools if 'binance' in t.name])}")
# Expected: 16 tools with binance prefix
```

**Test 2: Get Ticker (Public Endpoint)**

```python
result = await client.call_tool("get_ticker", {"symbol": "BTCUSDT"})
print(result)
# Expected: JSON with price, volume, price change statistics
```

**Test 3: Get Order Book**

```python
result = await client.call_tool("get_order_book", {"symbol": "ETHUSDT", "limit": 20})
print(result)
# Expected: JSON with bids/asks arrays
```

### Integration Test Suite

```bash
# From repository root
cd mcp-gateway
uv run pytest tests/integration/test_binance_provider.py -v
```

**Test coverage**:
- Tool invocation (all 16 tools)
- Resource queries (4 resources)
- Prompt generation (2 prompts)
- Error handling (invalid symbols, missing credentials)
- Schema validation

### Unit Tests (Provider)

```bash
cd providers/binance-rs
cargo test --all-features
```

**Test coverage**:
- gRPC adapter layer
- Capability discovery
- JSON schema validation
- Error conversion
- OrderBook metrics calculation (if orderbook feature enabled)

---

## Development Workflow

### Making Changes

1. **Edit provider code**: `providers/binance-rs/src/`
2. **Rebuild**: `cargo build --release`
3. **Restart provider**: Ctrl+C in Terminal 1, then re-run
4. **Test changes**: Use MCP Inspector or pytest

### Adding a New Tool

1. **Define tool in mcp-binance-rs**:
   ```rust
   // src/tools/my_new_tool.rs
   #[tool(description = "My new tool description")]
   pub async fn my_new_tool(&self, params: MyToolParams) -> Result<MyToolResult> {
       // Implementation
   }
   ```

2. **Add to tool router**:
   ```rust
   // src/grpc/tools.rs
   match tool_name {
       "my_new_tool" => self.my_new_tool(params).await,
       // ... existing tools
   }
   ```

3. **Add JSON schema**:
   ```rust
   // Schema auto-generated from MyToolParams via schemars
   ```

4. **Update capabilities**:
   ```rust
   // src/grpc/capabilities.rs
   tools.push(Tool {
       name: "my_new_tool".to_string(),
       description: "My new tool description".to_string(),
       input_schema: Some(Json { value: schema_bytes }),
       output_schema: None,
   });
   ```

5. **Rebuild and test**:
   ```bash
   cargo build --release
   ./target/release/binance-provider --grpc --port 50052
   ```

### Debugging

**Enable verbose logging**:
```bash
export RUST_LOG="debug,binance_provider=trace"
./target/release/binance-provider --grpc --port 50052
```

**Test individual RPC with grpcurl**:
```bash
# Install grpcurl
go install github.com/fullstorydev/grpcurl/cmd/grpcurl@latest

# List services
grpcurl -plaintext localhost:50052 list

# Call ListCapabilities
grpcurl -plaintext -d '{}' localhost:50052 provider.v1.Provider/ListCapabilities
```

**Inspect protobuf traffic**:
```bash
# Enable gRPC tracing
export GRPC_TRACE=all
export GRPC_VERBOSITY=DEBUG
```

---

## Common Issues

### Issue 1: Provider won't start - "Address already in use"

**Symptom**: `Error: Address already in use (os error 98)`

**Solution**: Kill existing process on port 50052
```bash
lsof -ti:50052 | xargs kill -9
```

### Issue 2: Authentication errors

**Symptom**: `{"error": "API key not found"}`

**Solution**: Verify environment variables are set
```bash
echo $BINANCE_API_KEY
echo $BINANCE_API_SECRET
# Should print your keys, not empty
```

### Issue 3: Gateway can't connect to provider

**Symptom**: `Failed to list capabilities from binance-rs: UNAVAILABLE`

**Solution**: Check provider is running and listening
```bash
# In provider terminal, should see:
# INFO binance_provider: Provider ready, waiting for connections...

# Test connectivity
grpcurl -plaintext localhost:50052 list
```

### Issue 4: Schema validation failures

**Symptom**: `Input validation failed: missing field 'symbol'`

**Solution**: Check tool parameters match JSON schema
```python
# Correct
await client.call_tool("get_ticker", {"symbol": "BTCUSDT"})

# Incorrect (missing required field)
await client.call_tool("get_ticker", {})
```

### Issue 5: Protobuf generation fails

**Symptom**: `protoc: command not found` or `protoc-gen-go: program not found`

**Solution**: Install protoc and Go plugins (see Prerequisites)

---

## Performance Benchmarks

**Expected latencies** (with Binance Testnet, ~100ms network latency):

| Operation | Target | Typical |
|-----------|--------|---------|
| List Capabilities | N/A | <10ms (cached) |
| Get Ticker (market data) | <2s | 150-300ms |
| Get Order Book | <2s | 200-400ms |
| Place Order | <3s | 400-800ms |
| Cancel Order | <3s | 300-600ms |
| OrderBook L1 Metrics | <200ms | 5-20ms (WebSocket) |

**Resource usage**:
- Provider memory: ~50MB (basic), ~150MB (with orderbook + 20 symbols)
- Provider CPU: <5% idle, <25% under load
- Network: ~10KB/request (market data), ~5KB/request (orders)

---

## Next Steps

After completing this quickstart:

1. **Read the specification**: [spec.md](spec.md)
2. **Review the data model**: [data-model.md](data-model.md)
3. **Explore the research**: [research.md](research.md)
4. **Check the task breakdown**: [tasks.md](tasks.md) (generated by `/speckit.tasks`)

---

## Support Resources

- **Binance API Documentation**: https://binance-docs.github.io/apidocs/spot/en/
- **Tonic Documentation**: https://github.com/hyperium/tonic
- **MCP Specification**: https://modelcontextprotocol.io/
- **Project Issues**: https://github.com/ForgeQuant/mcp-gateway/issues

---

**Last Updated**: 2025-10-18
