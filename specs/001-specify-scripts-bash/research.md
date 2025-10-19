# Research: MCP Gateway System

**Date**: 2025-10-18
**Feature**: MCP Gateway System with Provider Orchestration

This document consolidates research findings for technology choices and implementation patterns.

## 1. MCP Python SDK

**Decision**: Use MCP Python SDK with FastMCP high-level API

**Rationale**:
- Official SDK provides stdio transport out-of-the-box
- FastMCP decorator-based approach simplifies tool/resource/prompt registration
- Built-in Pydantic integration for automatic JSON Schema generation and validation
- Full asyncio support compatible with async gRPC clients
- No need for external schema validation library

**Key Implementation Details**:
```python
from mcp.server.fastmcp import FastMCP

mcp = FastMCP("gateway-server")

@mcp.tool()
async def proxy_tool(tool_name: str, args: dict) -> dict:
    """Automatic schema generation from type hints"""
    # Route to provider via gRPC
    return await invoke_provider(tool_name, args)
```

**Handlers Required**:
- `@mcp.tool()` for tool invocations
- `@mcp.resource("uri://{param}")` for resource access
- `@mcp.prompt()` for prompt templates

**Lifecycle**: Use `asynccontextmanager` for startup/shutdown hooks to initialize gRPC channels and cleanup on exit.

**Alternatives Considered**:
- Low-level Server API: More verbose, requires manual handler registration
- Custom stdio implementation: Unnecessary reinvention of protocol handling

**Reference**: MCP Python SDK v1.7.1 (PyPI)

---

## 2. gRPC Python Client (Gateway → Providers)

**Decision**: Use `grpc.aio` from `grpcio` package for asyncio-compatible gRPC clients

**Rationale**:
- Native asyncio support using battle-tested C-Core implementation
- Full compatibility with MCP SDK's async architecture
- Channels are thread-safe and designed for reuse
- Production-proven stability (stable since v1.32)

**Connection Pooling Pattern** (10-20 concurrent connections):
```python
class ProviderGateway:
    def __init__(self, num_channels=15):
        self.channels = [
            grpc.aio.insecure_channel(
                "provider:50051",
                options=[
                    ("grpc.channel_pool_id", i),
                    ("grpc.keepalive_time_ms", 55000),
                    ("grpc.keepalive_timeout_ms", 10000),
                ]
            ) for i in range(num_channels)
        ]
        self.current_idx = 0

    def get_channel(self):
        """Round-robin channel selection"""
        channel = self.channels[self.current_idx]
        self.current_idx = (self.current_idx + 1) % len(self.current_idx)
        return channel
```

**Timeout Configuration**:
```python
response = await stub.Invoke(request, timeout=2.5)  # 2.5 seconds fail-fast
```

**Error Handling**:
```python
try:
    response = await stub.Method(request, timeout=2.5)
except grpc.RpcError as e:
    status_code = e.code()  # grpc.StatusCode enum
    if status_code in [grpc.StatusCode.UNAVAILABLE, grpc.StatusCode.DEADLINE_EXCEEDED]:
        # Log and return error to MCP client (no retries per FR-014)
        raise
```

**Alternatives Considered**:
- grpcio synchronous: Incompatible with asyncio event loop
- grpcio-tools: Build-time codegen only, doesn't affect client choice

**Reference**: gRPC Python documentation, grpc.aio module

---

## 3. Go gRPC Server (hello-go Provider)

**Decision**: Standard `google.golang.org/grpc` with repository/service pattern separation

**Rationale**:
- Official Go gRPC library with excellent performance
- Clean separation between transport (gRPC handlers) and business logic
- Built-in support for reflection and testing via `bufconn`
- Aligns with Constitution Principle V (Service pattern)

**Server Setup Pattern**:
```go
lis, err := net.Listen("tcp", ":50051")
if err != nil {
    log.Fatalf("failed to listen: %v", err)
}
server := grpc.NewServer(
    grpc.MaxRecvMsgSize(10 * 1024 * 1024), // 10MB payload limit
)
pb.RegisterProviderServer(server, &providerServiceImpl{})
server.Serve(lis)
```

**Service Implementation Structure**:
```go
type ProviderServiceServer struct {
    pb.UnimplementedProviderServer
    tools   *ToolsService     // Business logic layer
    resources *ResourcesService
}

func (s *ProviderServiceServer) Invoke(ctx context.Context, req *pb.InvokeRequest) (*pb.InvokeResponse, error) {
    // Validation only in gRPC layer
    result, err := s.tools.Execute(ctx, req.ToolName, req.Payload)
    if err != nil {
        return nil, status.Error(codes.Internal, err.Error())
    }
    return &pb.InvokeResponse{Result: result}, nil
}
```

**Error Handling Status Codes**:
- `codes.InvalidArgument`: Malformed requests
- `codes.NotFound`: Tool/resource not found
- `codes.Internal`: Unexpected errors
- `codes.Unavailable`: Transient failures

**Testing**: Use `bufconn.Listen()` for in-memory integration tests without real network sockets.

**Alternatives Considered**:
- Custom HTTP/JSON API: More implementation work, lacks type safety of protobuf
- Other RPC frameworks: gRPC is industry standard with best cross-language support

**Reference**: gRPC Go documentation, google.golang.org/grpc package

---

## 4. Rust Tonic Framework (hello-rs Provider)

**Decision**: Tonic v0.9 with prost for protobuf, tokio for async runtime

**Rationale**:
- Production-ready gRPC framework with first-class async/await support
- Excellent type safety and performance characteristics of Rust
- Seamless protobuf integration via prost codegen
- Validates language-agnostic provider contract (FR-016)

**Dependencies** (Cargo.toml):
```toml
[dependencies]
tonic = "0.9"
prost = "0.11"
tokio = { version = "1.28", features = ["macros", "rt-multi-thread"] }

[build-dependencies]
tonic-build = "0.9"
```

**Server Setup**:
```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    Server::builder()
        .add_service(ProviderServer::new(ProviderImpl::default()))
        .serve(addr)
        .await?;
    Ok(())
}
```

**Service Implementation** (trait-based):
```rust
#[tonic::async_trait]
impl Provider for ProviderImpl {
    async fn invoke(&self, request: Request<InvokeRequest>)
        -> Result<Response<InvokeResponse>, Status> {
        let req = request.into_inner();
        // Business logic here
        Ok(Response::new(InvokeResponse { result }))
    }
}
```

**Protobuf Codegen** (build.rs):
```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_client(false)  // Server-only
        .build_server(true)
        .compile(&["../../../pkg/proto/provider.proto"], &["../../../pkg/proto/"])
        .unwrap();
    Ok(())
}
```

**Error Handling**: Use `Status::new(Code::InvalidArgument, msg)` for errors. Consider `tonic-types` crate for rich error details.

**Alternatives Considered**:
- tarpc: Less mature gRPC support
- grpc-rs (C++ bindings): More complex, less idiomatic Rust

**Reference**: Tonic documentation (docs.rs/tonic), Rust gRPC guides

---

## 5. JSON Schema Validation

**Decision**: Use `jsonschema` library with Draft 2020-12 support, reuse validator instances

**Rationale**:
- Full support for JSON Schema Draft 2020-12 (required by spec)
- Most mature and widely-adopted Python library
- Excellent error messages with `best_match()` helper for debugging
- 10x faster when reusing compiled validators
- MCP SDK already uses Pydantic for input validation, but we need explicit output validation per FR-011

**Usage Pattern**:
```python
from jsonschema import Draft202012Validator

class SchemaValidator:
    def __init__(self):
        self.validators = {}  # Cache compiled validators

    def validate(self, schema: dict, payload: dict):
        schema_id = hash(json.dumps(schema, sort_keys=True))
        if schema_id not in self.validators:
            self.validators[schema_id] = Draft202012Validator(schema)
        self.validators[schema_id].validate(payload)  # Raises ValidationError
```

**Integration with Async Code**: Validation is fast enough (<5ms for typical payloads) to call synchronously without blocking event loop.

**Alternatives Considered**:
- `fastjsonschema`: Doesn't support Draft 2020-12
- `jsonschema-rs`: 60-390x faster but adds Rust dependency; overkill for <10MB payloads with <100 concurrent requests
- Pydantic validation: Already used by MCP SDK for inputs, but need separate output validation

**Reference**: jsonschema documentation (python-jsonschema.readthedocs.io)

---

## 6. Protobuf Contract Design

**Decision**: Single shared `provider.proto` in `/pkg/proto/` with `bytes` field for JSON payloads

**Rationale**:
- Shared contract ensures DRY principle (Constitution IV)
- `bytes` type allows flexible JSON payloads without protobuf schema coupling
- Both Go and Rust can compile same proto file (language-agnostic)
- Simpler than defining every tool schema in protobuf

**Proto Structure**:
```protobuf
syntax = "proto3";
package provider.v1;

message Json { bytes value = 1; }

message Tool {
  string name = 1;
  string description = 2;
  Json input_schema = 3;
  Json output_schema = 4;
}

service Provider {
  rpc ListCapabilities(google.protobuf.Empty) returns (Capabilities);
  rpc Invoke(InvokeRequest) returns (InvokeResponse);
  rpc Stream(StreamRequest) returns (stream CloudEvent);
}
```

**Alternatives Considered**:
- Separate proto per provider: Violates DRY, harder to maintain
- Full proto schemas for tools: Too rigid, doesn't match MCP's JSON Schema approach
- REST/JSON API: Loses type safety and cross-language codegen benefits

---

## 7. NATS JetStream (Optional Event Streaming)

**Decision**: Use `nats-py` with asyncio support for Python gateway, NATS Go SDK for providers

**Rationale**:
- Official NATS client libraries for both Python and Go
- JetStream provides durable consumers for at-least-once delivery (FR-027)
- Replay capability for missed events after gateway restart (FR-028)
- Priority P4 feature - can defer if time-constrained

**Not researched in depth** as it's optional for MVP. Implementation deferred to Phase 1 if needed.

**Reference**: NATS documentation (docs.nats.io)

---

## Summary of Technology Stack

| Component | Technology | Version |
|-----------|-----------|---------|
| Gateway Language | Python | 3.11+ |
| Gateway MCP SDK | mcp (FastMCP) | 1.7.1+ |
| Gateway gRPC Client | grpc.aio (grpcio) | Latest stable |
| Gateway Validation | jsonschema | Latest with Draft 2020-12 |
| Provider 1 Language | Go | 1.21+ |
| Provider 1 gRPC | google.golang.org/grpc | Latest stable |
| Provider 2 Language | Rust | 1.75+ |
| Provider 2 gRPC | Tonic | 0.9+ |
| Provider 2 Runtime | Tokio | 1.28+ |
| Protobuf Compiler | protoc | 3.x |
| Message Broker (Optional) | NATS JetStream | Latest |
| Testing | pytest, Go testing, cargo test | - |

All choices align with **Constitution Principle II (Library-First Development)** by leveraging official SDKs and battle-tested libraries rather than custom implementations.
