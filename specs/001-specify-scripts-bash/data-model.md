# Data Model: MCP Gateway System

**Date**: 2025-10-18
**Feature**: MCP Gateway System with Provider Orchestration

This document defines the domain entities and their relationships for the MCP Gateway system.

## Core Entities

### 1. Provider

A backend microservice that implements the gRPC Provider service contract and exposes MCP capabilities.

**Attributes**:
- `name` (string, required): Unique identifier for the provider (e.g., "hello-go", "hello-rs")
- `type` (string, required): Provider connection type (e.g., "grpc")
- `address` (string, required): Network address for gRPC connection (e.g., "localhost:50051")
- `status` (enum): Connection status (CONNECTED, DISCONNECTED, ERROR)
- `capabilities` (Capabilities): Cached capabilities from ListCapabilities() call

**Relationships**:
- Has many Capabilities (tools, resources, prompts)

**Validation Rules**:
- Name must be unique across all registered providers
- Address must be valid host:port format
- Status transitions: DISCONNECTED → CONNECTED → ERROR → DISCONNECTED

**State Transitions**:
```
DISCONNECTED --[connect success]--> CONNECTED
CONNECTED --[heartbeat fail]--> ERROR
ERROR --[reconnect success]--> CONNECTED
ERROR --[max retries]--> DISCONNECTED
```

---

### 2. Capability

A unit of functionality exposed by a provider (abstract base for Tool, Resource, Prompt).

**Common Attributes**:
- `name` (string, required): Capability identifier
- `description` (string, required): Human-readable description
- `provider_name` (string, required): Provider that owns this capability
- `qualified_name` (string, computed): Namespaced name (e.g., "hello-go.echo.v1")

**Relationships**:
- Belongs to one Provider

**Validation Rules**:
- Name must follow semantic versioning pattern for tools (e.g., "echo.v1")
- Qualified name computed as `{provider_name}.{name}`
- Description must be non-empty

---

### 3. Tool (extends Capability)

An executable operation with typed inputs and outputs.

**Attributes** (in addition to Capability):
- `input_schema` (JSON Schema, required): Validation schema for tool arguments
- `output_schema` (JSON Schema, optional): Validation schema for tool results

**Relationships**:
- Is-a Capability
- Has one InputSchema
- Has zero-or-one OutputSchema

**Validation Rules**:
- Input schema must conform to JSON Schema Draft 2020-12
- Output schema (if present) must conform to JSON Schema Draft 2020-12
- Schema must validate successfully with jsonschema library

**Examples**:
```json
{
  "name": "echo.v1",
  "description": "Returns input message unchanged",
  "provider_name": "hello-go",
  "qualified_name": "hello-go.echo.v1",
  "input_schema": {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "type": "object",
    "properties": {
      "message": { "type": "string" }
    },
    "required": ["message"]
  }
}
```

---

### 4. Resource (extends Capability)

Content accessible via URI scheme.

**Attributes** (in addition to Capability):
- `uri_scheme` (string, required): URI scheme handler (e.g., "hello" for "hello://greeting")
- `mime_type` (string, optional): Content type (e.g., "text/plain", "application/json")

**Relationships**:
- Is-a Capability

**Validation Rules**:
- URI scheme must be valid scheme format (alphanumeric + hyphen)
- Multiple providers can expose same URI scheme (gateway routes by provider name prefix)

**Examples**:
```json
{
  "name": "greeting",
  "description": "Returns Hello, MCP greeting message",
  "provider_name": "hello-go",
  "uri_scheme": "hello",
  "mime_type": "text/plain"
}
```

**URI Format**: `{uri_scheme}://{path}` (e.g., "hello://greeting")

---

### 5. Prompt (extends Capability)

A template with parameter placeholders for structured AI interactions.

**Attributes** (in addition to Capability):
- `args_schema` (JSON Schema, required): Schema for prompt parameters
- `template` (string, optional): Template string with placeholders (e.g., "Hello, {name}!")

**Relationships**:
- Is-a Capability
- Has one ArgsSchema

**Validation Rules**:
- Args schema must conform to JSON Schema Draft 2020-12
- Template placeholders must match schema properties

**Examples**:
```json
{
  "name": "hello-plan",
  "description": "Greet a user and propose a plan",
  "provider_name": "hello-go",
  "args_schema": {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "type": "object",
    "properties": {
      "name": { "type": "string" }
    },
    "required": ["name"]
  }
}
```

---

### 6. ToolInvocation

Represents a single tool execution request and response.

**Attributes**:
- `correlation_id` (UUID, required): Unique identifier for request tracing
- `tool_qualified_name` (string, required): Fully qualified tool name
- `provider_name` (string, required): Target provider
- `payload` (JSON, required): Tool arguments
- `result` (JSON, optional): Tool execution result
- `error` (Error, optional): Error if invocation failed
- `timestamp` (ISO 8601, required): Request timestamp
- `duration_ms` (integer, optional): Execution duration in milliseconds

**Relationships**:
- References one Tool (via qualified_name)
- References one Provider (via provider_name)

**Validation Rules**:
- Payload must validate against tool's input_schema
- Result (if present) must validate against tool's output_schema (FR-011)
- Either result or error must be present (not both)

**State Flow**:
```
CREATED → VALIDATING → ROUTING → EXECUTING → COMPLETED
                    ↓              ↓         ↓
                  FAILED        FAILED   FAILED
```

---

### 7. CloudEvent (Optional - Event Streaming)

Structured event envelope for asynchronous provider-to-gateway communication.

**Attributes**:
- `id` (string, required): Unique event identifier
- `source` (URI, required): Event source (provider identifier)
- `type` (string, required): Event type (e.g., "provider.tool.executed")
- `time` (ISO 8601, required): Event timestamp
- `specversion` (string, required): CloudEvents version ("1.0")
- `data` (JSON, optional): Event payload

**Relationships**:
- Originates from one Provider

**Validation Rules**:
- Must conform to CloudEvents 1.0 specification
- Source must be valid URI format
- Time must be RFC3339 format

**Examples**:
```json
{
  "id": "A234-1234-1234",
  "source": "urn:provider:hello-go",
  "type": "provider.metric.reported",
  "time": "2025-10-18T12:00:00Z",
  "specversion": "1.0",
  "data": {
    "metric": "tool_invocations",
    "value": 42
  }
}
```

---

### 8. CorrelationContext

Distributed tracing context propagated across gateway and provider boundaries.

**Attributes**:
- `correlation_id` (UUID, required): Unique request identifier
- `timestamp` (ISO 8601, required): Request initiation time
- `client_info` (string, optional): AI client identification
- `span_ids` (list[string], optional): Distributed trace span identifiers

**Relationships**:
- Associated with ToolInvocations, ResourceReads, PromptGets

**Validation Rules**:
- Correlation ID must be globally unique
- Propagated in gRPC metadata/headers across service boundaries

**Purpose**: Enables end-to-end request tracing per FR-013 (correlation IDs in logs)

---

## Entity Relationship Diagram

```
┌─────────────┐
│  Provider   │
│             │
│ - name      │
│ - type      │
│ - address   │
│ - status    │
└──────┬──────┘
       │ 1:N
       │
       ▼
┌─────────────────┐
│  Capability     │◄──────────┐
│  (abstract)     │           │
│                 │           │
│ - name          │           │
│ - description   │           │
│ - provider_name │           │
└─────────────────┘           │
       △                      │
       │                      │
   ┌───┴───────┬──────────┐   │
   │           │          │   │
┌──┴───┐  ┌───┴────┐  ┌──┴──┴───┐
│ Tool │  │Resource│  │  Prompt  │
│      │  │        │  │          │
└──────┘  └────────┘  └──────────┘

┌──────────────────┐
│ ToolInvocation   │
│                  │
│ - correlation_id │───────┐
│ - tool_name      │       │
│ - payload        │       │
│ - result/error   │       │
└──────────────────┘       │
                           │
                           ▼
                  ┌─────────────────────┐
                  │ CorrelationContext  │
                  │                     │
                  │ - correlation_id    │
                  │ - timestamp         │
                  └─────────────────────┘
```

---

## Configuration Schema

### providers.yaml

Configuration file for static provider registration.

**Schema**:
```yaml
providers:
  - name: string          # Provider unique name
    type: "grpc"          # Connection type
    address: string       # host:port
    enabled: boolean      # Optional, default true
    metadata:             # Optional provider metadata
      version: string
      environment: string
```

**Example**:
```yaml
providers:
  - name: hello-go
    type: grpc
    address: localhost:50051
    enabled: true
    metadata:
      version: "1.0.0"
      environment: development

  - name: hello-rs
    type: grpc
    address: localhost:50052
    enabled: true
    metadata:
      version: "1.0.0"
      environment: development
```

**Validation**:
- Names must be unique
- Addresses must be valid host:port format
- Type must be supported (currently only "grpc")

---

## Data Constraints Summary

| Entity | Key Constraint | Uniqueness | Required Fields |
|--------|---------------|------------|-----------------|
| Provider | Name unique | Global | name, type, address |
| Tool | Qualified name unique | Global | name, description, input_schema |
| Resource | URI scheme + provider | Per-provider | name, uri_scheme |
| Prompt | Name unique per provider | Per-provider | name, args_schema |
| ToolInvocation | Correlation ID unique | Global | correlation_id, tool_name, payload |

---

## Persistence

**None** - The MCP Gateway is stateless per Constitution Principle VI (12-Factor: stateless processes).

- Provider configuration loaded from `providers.yaml` on startup
- Capabilities cached in memory after discovery
- No database or persistent storage required
- All state ephemeral and reconstructed on restart

This aligns with FR-001 (load config on startup) and Assumption #3 (static provider configuration sufficient for MVP).
