# Research: Binance Provider Integration

**Feature**: 002-binance-provider-integration
**Date**: 2025-10-18
**Purpose**: Technology decisions and implementation patterns for converting mcp-binance-rs into a gRPC provider

## 1. gRPC Framework Selection (Rust)

**Decision**: Use **Tonic 0.9** with Prost 0.11 for protobuf

**Rationale**:
- Tonic is the de facto standard for gRPC in Rust (7k+ GitHub stars, active development)
- Async/await native design with Tokio integration (already used by mcp-binance-rs)
- Compile-time code generation ensures type safety and zero-cost abstractions
- Battle-tested in production (used by companies like Cloudflare, Discord)
- Excellent documentation and ecosystem compatibility

**Alternatives Considered**:
1. **grpc-rs** (C++ binding) - Rejected: Adds C++ build dependency, less idiomatic Rust, maintenance concerns
2. **tarpc** (Rust RPC framework) - Rejected: Not compatible with existing provider.proto contract, would require gateway rewrite

**References**:
- Tonic documentation: https://github.com/hyperium/tonic
- Provider.proto contract: `/home/limerc/repos/ForgeQuant/mcp-trader/pkg/proto/provider.proto`
- hello-go provider pattern: `/home/limerc/repos/ForgeQuant/mcp-trader/providers/hello-go/`

---

## 2. MCP-to-gRPC Translation Pattern

**Decision**: Implement **adapter layer** that wraps existing rmcp tool handlers

**Rationale**:
- Preserves existing mcp-binance-rs tool implementations without modification (DRY principle)
- Isolates gRPC concerns from business logic (separation of concerns)
- Allows gradual migration - can maintain stdio MCP mode alongside gRPC
- Follows hello-go pattern: separate server layer handles RPC, delegates to tools

**Implementation Pattern**:
```rust
// src/grpc/tools.rs
impl Provider for BinanceProviderServer {
    async fn invoke(&self, request: InvokeRequest) -> Result<InvokeResponse> {
        // 1. Extract tool_name and JSON payload from protobuf
        // 2. Route to existing rmcp tool handler
        // 3. Convert result back to protobuf InvokeResponse

        let tool_name = &request.tool_name;
        let payload_json: Value = serde_json::from_slice(&request.payload.value)?;

        // Reuse existing tool logic from mcp-binance-rs
        let result = self.tool_router.execute(tool_name, payload_json).await?;

        Ok(InvokeResponse {
            result: Some(Json { value: serde_json::to_vec(&result)? }),
            error: String::new(),
        })
    }
}
```

**Alternatives Considered**:
1. **Rewrite all tools for gRPC** - Rejected: Violates DRY, doubles maintenance burden
2. **Fork mcp-binance-rs** - Rejected: Would diverge from upstream, lose MCP compatibility

---

## 3. JSON Schema Handling

**Decision**: **Embed JSON schemas as static strings** in capabilities response

**Rationale**:
- mcp-binance-rs already generates JSON schemas via `schemars` crate at compile time
- Provider.proto uses `Json { bytes value }` for schema flexibility
- Gateway expects JSON Schema Draft 2020-12 format (already satisfied by schemars output)
- No runtime schema generation needed - schemas are deterministic

**Implementation Pattern**:
```rust
// src/grpc/capabilities.rs
impl BinanceProviderServer {
    async fn list_capabilities(&self) -> Result<Capabilities> {
        let tools = vec![
            Tool {
                name: "get_ticker".to_string(),
                description: "Get 24-hour ticker statistics".to_string(),
                input_schema: Some(Json {
                    value: include_bytes!("../schemas/get_ticker.json").to_vec(),
                }),
                output_schema: None,
            },
            // ... repeat for all 16 tools
        ];

        Ok(Capabilities {
            tools,
            resources: vec![/* 4 resources */],
            prompts: vec![/* 2 prompts */],
            provider_version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }
}
```

**Alternatives Considered**:
1. **Generate schemas at runtime** - Rejected: Unnecessary overhead, schemas are static
2. **Load schemas from files** - Rejected: Adds deployment complexity, prefer compile-time embedding

---

## 4. Resource URI Handling

**Decision**: **Reuse existing `binance://` URI parser** from mcp-binance-rs

**Rationale**:
- mcp-binance-rs already implements resource URI parsing in `src/server/resources.rs`
- URIs follow pattern: `binance://category/identifier` (e.g., `binance://market/btcusdt`)
- Existing parser handles validation and routing to appropriate handlers
- Markdown formatting logic already implemented for resource content

**Implementation Pattern**:
```rust
// src/grpc/resources.rs
impl Provider for BinanceProviderServer {
    async fn read_resource(&self, request: ResourceRequest) -> Result<ResourceResponse> {
        // Delegate to existing URI parser
        let resource = ResourceUri::parse(&request.uri)?;

        // Reuse existing resource handlers
        let content = match resource.category {
            ResourceCategory::Market => self.handle_market_resource(&resource).await?,
            ResourceCategory::Account => self.handle_account_resource(&resource).await?,
            ResourceCategory::Orders => self.handle_orders_resource(&resource).await?,
        };

        Ok(ResourceResponse {
            content: content.as_bytes().to_vec(),
            mime_type: "text/markdown".to_string(),
            error: String::new(),
        })
    }
}
```

---

## 5. Prompt Template Handling

**Decision**: **Convert rmcp prompt handlers** to protobuf PromptResponse format

**Rationale**:
- mcp-binance-rs has 2 prompts: `trading_analysis` and `portfolio_risk`
- Existing prompt handlers return structured messages (role + content)
- Direct mapping to PromptMessage protobuf type
- Parameter validation via JSON Schema (already implemented)

**Implementation Pattern**:
```rust
// src/grpc/prompts.rs
impl Provider for BinanceProviderServer {
    async fn get_prompt(&self, request: PromptRequest) -> Result<PromptResponse> {
        let args: Value = serde_json::from_slice(&request.arguments.value)?;

        // Delegate to existing prompt router
        let messages = self.prompt_router.execute(&request.prompt_name, args).await?;

        Ok(PromptResponse {
            messages: messages.into_iter().map(|m| PromptMessage {
                role: m.role,
                content: m.content,
            }).collect(),
            error: String::new(),
        })
    }
}
```

---

## 6. Error Handling Strategy

**Decision**: **Convert Rust errors to gRPC error field** (not status codes)

**Rationale**:
- Provider.proto uses error strings in response messages (InvokeResponse.error, etc.)
- Preserves detailed error context without gRPC status code limitations
- Matches hello-go pattern: return Ok(response) with error field populated
- Allows user-friendly error messages without exposing stack traces

**Implementation Pattern**:
```rust
// src/grpc/mod.rs
impl From<McpError> for InvokeResponse {
    fn from(error: McpError) -> Self {
        InvokeResponse {
            result: None,
            error: error.to_user_message(), // Never expose secrets/traces
        }
    }
}

// Error conversion preserves user-facing messages
async fn invoke(&self, request: InvokeRequest) -> Result<InvokeResponse> {
    match self.execute_tool(&request).await {
        Ok(result) => Ok(InvokeResponse { result: Some(result), error: String::new() }),
        Err(e) => Ok(InvokeResponse::from(e)), // Error in field, not gRPC status
    }
}
```

**Alternatives Considered**:
1. **Use gRPC status codes** - Rejected: Loses error detail, complicates gateway error handling
2. **Panic on errors** - Rejected: Crashes provider, violates graceful degradation

---

## 7. Protobuf Code Generation

**Decision**: Use **build.rs with tonic-build** for compile-time codegen

**Rationale**:
- Standard Rust pattern for protobuf: build script generates code before compilation
- Ensures generated code stays in sync with provider.proto
- Type safety at compile time (no runtime reflection needed)
- Follows hello-go pattern (Go uses protoc with go_out flag, equivalent approach)

**Implementation**:
```rust
// build.rs
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(false)  // Provider only needs server stubs
        .compile(
            &["../../pkg/proto/provider.proto"],
            &["../../pkg/proto/"],
        )?;
    Ok(())
}
```

**Cargo.toml dependencies**:
```toml
[build-dependencies]
tonic-build = "0.9"

[dependencies]
tonic = "0.9"
prost = "0.11"
```

---

## 8. OrderBook Feature Compilation

**Decision**: **Preserve feature flag** from mcp-binance-rs, compile binance-rs provider with `orderbook` feature by default

**Rationale**:
- OrderBook tools (L1/L2 metrics) are marked as P4 (advanced feature) in spec
- WebSocket infrastructure has memory/connection overhead (20 symbol limit)
- Feature flag allows deployment flexibility (basic vs advanced)
- Default to enabled for full functionality

**Cargo.toml configuration**:
```toml
[features]
default = ["orderbook"]  # Enable by default
orderbook = ["tokio-tungstenite", "rust_decimal", "governor"]

[dependencies]
# Core dependencies (always included)
tonic = "0.9"
prost = "0.11"
tokio = { version = "1.48", features = ["rt-multi-thread", "macros"] }
rmcp = "0.8.1"
# ... existing mcp-binance-rs dependencies

# Optional dependencies (only with orderbook feature)
tokio-tungstenite = { version = "0.28", optional = true }
rust_decimal = { version = "1.37", optional = true }
governor = { version = "0.6", optional = true }
```

**Makefile targets**:
```makefile
build-binance:
	cd providers/binance-rs && cargo build --release

build-binance-basic:
	cd providers/binance-rs && cargo build --release --no-default-features
```

---

## 9. Configuration Management

**Decision**: **Reuse existing environment variable configuration** from mcp-binance-rs

**Rationale**:
- mcp-binance-rs already implements 12-Factor config via env vars
- No changes needed: BINANCE_API_KEY, BINANCE_API_SECRET, BINANCE_BASE_URL
- Gateway adds provider-specific config via providers.yaml (address, port)

**providers.yaml entry**:
```yaml
providers:
  - name: binance-rs
    type: grpc
    address: localhost:50052  # Different port from hello-go (50051)
    enabled: true
    metadata:
      description: "Binance cryptocurrency trading provider"
      version: "0.1.0"
      features: ["orderbook"]
```

**Environment variables** (passed by gateway or set in shell):
```bash
BINANCE_API_KEY="your_api_key"
BINANCE_API_SECRET="your_api_secret"
BINANCE_BASE_URL="https://testnet.binance.vision"  # Or production URL
RUST_LOG="info"  # Logging level
```

---

## 10. Testing Strategy

**Decision**: **Three-tier testing approach**

**Tier 1: Unit tests** (Rust - cargo test):
- Test gRPC adapter layer (InvokeRequest → tool routing → InvokeResponse)
- Mock Binance API client responses
- Validate error conversion and edge cases
- JSON schema validation

**Tier 2: Integration tests** (Rust - cargo test):
- Test full RPC flow with tonic test server
- Verify capability discovery returns all 16 tools
- Test resource URI parsing and response formatting
- Test prompt parameter substitution

**Tier 3: E2E tests** (Python - pytest):
- Test gateway → binance-rs provider communication
- Verify tool invocation through gateway (similar to test_gateway.py)
- Test credential handling and error propagation
- Compare results with mcp-binance-rs stdio mode (ensure parity)

**Test structure**:
```
providers/binance-rs/
├── src/
│   └── grpc/
│       └── tests.rs       # Unit tests alongside implementation
└── tests/
    ├── integration/
    │   ├── capabilities_test.rs
    │   ├── tools_test.rs
    │   └── resources_test.rs
    └── common/
        └── mock_binance.rs  # Shared test utilities
```

---

## 11. Performance Considerations

**Decision**: **Connection pooling handled by gateway**, provider remains stateless

**Rationale**:
- Gateway already implements 15-channel pool per provider (grpc_client.py)
- Provider doesn't need to manage client connections
- Binance API client uses reqwest connection pooling (HTTP/1.1 keep-alive)
- WebSocket connections (orderbook feature) managed by tokio tasks

**Concurrency model**:
- Provider uses Tokio runtime with work-stealing scheduler
- Each gRPC request handled by async task
- No shared mutable state (except orderbook cache with Arc<RwLock>)
- Binance API rate limiting handled per-request (existing mcp-binance-rs logic)

---

## 12. Migration Path from stdio MCP

**Decision**: **Dual-mode support** - provider can run as both stdio MCP and gRPC server

**Rationale**:
- Allows gradual migration and A/B testing
- Enables standalone usage via MCP Inspector (testing/debugging)
- Zero risk: MCP mode unchanged, gRPC is additive

**Command-line interface**:
```bash
# stdio MCP mode (existing)
mcp-binance-server

# gRPC provider mode (new)
mcp-binance-server --grpc --port 50052

# With orderbook disabled
mcp-binance-server --grpc --port 50052 --no-default-features
```

**Implementation**:
```rust
// src/main.rs
#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.mode {
        Mode::Stdio => run_mcp_server().await,  // Existing
        Mode::Grpc => run_grpc_server(args.port).await,  // New
    }
}
```

---

## Summary of Key Decisions

| Area | Decision | Primary Driver |
|------|----------|----------------|
| gRPC Framework | Tonic 0.9 + Prost 0.11 | Industry standard, Tokio integration |
| MCP-to-gRPC Translation | Adapter layer wrapping rmcp handlers | DRY principle, code reuse |
| JSON Schema | Static embedding via include_bytes! | Compile-time validation |
| Resource URIs | Reuse existing parser | Avoid duplication |
| Prompts | Convert rmcp handlers | Direct mapping |
| Error Handling | Error field in response | User-friendly messages |
| Protobuf Codegen | build.rs with tonic-build | Type safety |
| OrderBook Feature | Feature flag, enabled by default | Deployment flexibility |
| Configuration | Environment variables | 12-Factor compliance |
| Testing | Three-tier (unit/integration/e2e) | Comprehensive coverage |
| Performance | Stateless provider, gateway pooling | Follows hello-go pattern |
| Migration | Dual-mode support (stdio + gRPC) | Zero-risk adoption |

**Next Phase**: Phase 1 - Data model definition and contract documentation
