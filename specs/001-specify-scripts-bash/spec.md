# Feature Specification: MCP Gateway System with Provider Orchestration

**Feature Branch**: `001-specify-scripts-bash`
**Created**: 2025-10-18
**Status**: Draft
**Input**: User description: "@plan.md"

## Clarifications

### Session 2025-10-18

- Q: How should authentication/authorization be enforced between components (AI client ↔ Gateway, Gateway ↔ Providers)? → A: Trust boundaries enforce auth at perimeter only (client-gateway authenticated, gateway-provider uses network isolation/mutual TLS)
- Q: What retry/timeout strategy should be used when a provider becomes unavailable during tool invocation? → A: Fail fast with timeout, no retries (timeout: 2-3 seconds)
- Q: How should the gateway handle provider responses that don't match declared output schemas (validation failures)? → A: Reject with error, log warning (strict validation, fail the tool invocation)
- Q: How should the system handle concurrent tool invocations to the same provider? → A: Connection pool with limit (10-20 concurrent connections per provider, queue additional requests)
- Q: How should the system handle very large tool payloads (>1MB JSON)? → A: Enforce size limit with error (10MB max payload, reject oversized requests with clear error message)

## User Scenarios & Testing *(mandatory)*

### User Story 1 - AI Client Tool Discovery (Priority: P1)

As an AI client (Claude, VS Code, Cursor), I need to discover and invoke tools provided by microservices through a central gateway, so that I can access distributed capabilities without managing multiple connections.

**Why this priority**: This is the core value proposition - enabling AI clients to seamlessly access tools from multiple providers through a single MCP connection. Without this, the entire system has no purpose.

**Independent Test**: Can be fully tested by connecting MCP Inspector to the gateway and verifying that tools from at least one provider (hello-go) are visible and invocable, delivering immediate value even before adding more providers.

**Acceptance Scenarios**:

1. **Given** hello-go provider is running and registered, **When** AI client connects to gateway via stdio, **Then** client sees tools prefixed with "hello-go.echo.v1" and "hello-go.sum.v1" in the available tools list
2. **Given** AI client has discovered "hello-go.echo.v1" tool, **When** client invokes it with {"message": "Hi"}, **Then** client receives {"message": "Hi"} response within 2 seconds
3. **Given** AI client has discovered "hello-go.sum.v1" tool, **When** client invokes it with {"numbers": [1,2,3]}, **Then** client receives {"sum": 6} response

---

### User Story 2 - Resource Access Through Gateway (Priority: P2)

As an AI client, I need to read resources from providers using URI schemes, so that I can access provider-specific content and data without implementation knowledge.

**Why this priority**: Resources are a key MCP primitive that enables content retrieval. This builds on the basic tool invocation foundation and adds content access capabilities.

**Independent Test**: Can be tested by requesting the "hello://greeting" resource through MCP Inspector and receiving the expected greeting message, demonstrating resource proxy functionality independently of tool invocation.

**Acceptance Scenarios**:

1. **Given** hello-go provider exposes "hello" URI scheme, **When** AI client requests "hello://greeting", **Then** client receives "Hello, MCP" content
2. **Given** resource does not exist, **When** AI client requests "hello://nonexistent", **Then** client receives clear error message indicating resource not found

---

### User Story 3 - Prompt Template Discovery (Priority: P3)

As an AI client, I need to discover and utilize prompt templates from providers, so that I can access pre-configured prompt patterns with parameter substitution.

**Why this priority**: Prompts enhance user experience but are not critical for core functionality. The system delivers value through tools and resources first.

**Independent Test**: Can be tested by listing available prompts through MCP Inspector and invoking "hello-plan" with a name parameter, demonstrating prompt discovery and parameter substitution independently.

**Acceptance Scenarios**:

1. **Given** hello-go provider registers "hello-plan" prompt, **When** AI client lists prompts, **Then** "hello-plan" appears with description "Greet a user and propose a plan"
2. **Given** "hello-plan" prompt is available, **When** AI client invokes it with {"name": "Alice"}, **Then** client receives personalized greeting with Alice's name substituted

---

### User Story 4 - Multi-Provider Aggregation (Priority: P2)

As a system administrator, I need the gateway to aggregate capabilities from multiple providers (Go and Rust), so that AI clients have unified access to heterogeneous microservices.

**Why this priority**: Multi-provider support is essential for the architecture's scalability and demonstrates the gateway's orchestration value. However, it can be tested after single-provider functionality works.

**Independent Test**: Can be tested by running both hello-go and hello-rs providers, then verifying through MCP Inspector that tools from both providers appear in the same tools list, each with correct provider prefixes.

**Acceptance Scenarios**:

1. **Given** hello-go and hello-rs providers are both running, **When** AI client lists tools, **Then** client sees both "hello-go.echo.v1" and "hello-rs.echo.v1" in the same list
2. **Given** both providers are registered, **When** AI client invokes "hello-rs.sum.v1", **Then** request is correctly routed to Rust provider and returns correct sum
3. **Given** one provider is unavailable, **When** AI client lists tools, **Then** client sees tools only from available providers without error

---

### User Story 5 - Event Stream Consumption (Priority: P4)

As an AI client, I need to receive events published by providers, so that I can react to real-time updates and state changes in the microservices.

**Why this priority**: Event streaming is marked as optional in the MVP. It's valuable for real-time scenarios but not required for basic tool/resource functionality.

**Independent Test**: Can be tested by having a provider publish CloudEvents to NATS, then verifying the gateway forwards them to subscribed clients, demonstrating event bus integration independently of other features.

**Acceptance Scenarios**:

1. **Given** hello-go publishes CloudEvent to "hello.events" topic, **When** gateway consumer is active, **Then** gateway receives event within 1 second
2. **Given** gateway restarts after downtime, **When** gateway reconnects to NATS, **Then** gateway replays missed events from durable consumer
3. **Given** event schema is invalid, **When** provider publishes malformed event, **Then** gateway logs error but continues processing other events

---

### Edge Cases

- **Provider unavailable during tool invocation**: Gateway fails fast with 2-3 second timeout and returns error to AI client with no automatic retries. Client receives clear error message indicating provider unavailability.
- **Provider response validation failures**: Gateway validates provider responses against declared output schemas. Responses that don't match are rejected, tool invocation fails with error returned to AI client, and gateway logs warning for debugging provider issues.
- **Concurrent tool invocations to same provider**: Gateway uses connection pooling with 10-20 concurrent connections per provider. Additional requests are queued until a connection becomes available or the 2-3 second timeout occurs, at which point queued requests fail with timeout error.
- **Large tool payloads (>1MB JSON)**: Gateway enforces 10MB maximum payload size limit for tool invocations. Requests exceeding this limit are rejected with clear error message indicating size limit violation. This prevents memory exhaustion and ensures system stability.
- What occurs when multiple providers register tools with identical names (naming conflicts)?
- How does the gateway behave when configuration contains invalid addresses or unreachable hosts?
- What happens when a provider's ListCapabilities response is empty or malformed?
- What occurs when message broker is unavailable but providers are running (event stream failure)?
- How does the gateway handle circular dependencies if providers attempt to call each other?
- What happens when JSON Schema validation passes but tool execution fails (business logic errors)?

## Requirements *(mandatory)*

### Functional Requirements

#### Gateway Core Capabilities

- **FR-001**: Gateway MUST load provider configurations from a configuration file and establish connections to each registered provider on startup
- **FR-002**: Gateway MUST discover capabilities from each connected provider and aggregate all tools, resources, and prompts into a unified interface
- **FR-003**: Gateway MUST namespace all provider capabilities with provider name (e.g., "hello-go.echo.v1") to prevent naming collisions across multiple providers
- **FR-004**: Gateway MUST validate provider capability schemas during discovery to ensure they conform to expected interface definitions
- **FR-005**: Gateway MUST route tool invocation requests to the correct provider based on tool name prefix
- **FR-006**: Gateway MUST route resource read requests to the correct provider based on URI scheme (e.g., "hello://")
- **FR-007**: Gateway MUST expose a standard MCP-compliant interface for AI client connections
- **FR-008**: Gateway MUST validate tool invocation payloads against declared input schemas before forwarding to provider
- **FR-009**: Gateway MUST return validation errors to AI client when payload does not match schema requirements
- **FR-010**: Gateway MUST enforce maximum payload size limit of 10MB for tool invocations and reject requests exceeding this limit with clear error message
- **FR-011**: Gateway MUST validate provider responses against declared output schemas and reject responses that don't match, returning error to AI client and logging warning
- **FR-012**: Gateway MUST handle provider connection failures gracefully and exclude unavailable providers from capability list
- **FR-013**: Gateway MUST emit structured logs with correlation IDs for request tracing across provider boundaries
- **FR-014**: Gateway MUST support observability instrumentation for metrics and distributed tracing
- **FR-015**: Gateway MUST fail fast when provider becomes unavailable during tool invocation, using timeout of 2-3 seconds with no automatic retries
- **FR-016**: Gateway MUST use connection pooling with 10-20 concurrent connections per provider, queuing additional requests until a connection becomes available or timeout occurs

#### Provider Contract Requirements

- **FR-017**: Providers MUST implement standard service interface with capability discovery, invocation, and event streaming endpoints
- **FR-018**: Providers MUST return tools with valid, machine-readable input schemas for validation
- **FR-019**: Providers MUST support versioned capability names (e.g., "echo.v1", "sum.v1") for backward compatibility
- **FR-020**: Providers MUST be implementable in multiple programming languages to demonstrate language-agnostic contract

#### Demo Provider Requirements (hello-go, hello-rs)

- **FR-021**: First demo provider MUST implement echo tool that returns input message unchanged
- **FR-022**: First demo provider MUST implement sum tool that calculates sum of array of numbers
- **FR-023**: First demo provider MUST expose URI scheme resource that returns greeting message
- **FR-024**: First demo provider MUST register prompt template with name parameter placeholder
- **FR-025**: Second demo provider MUST implement identical tools/resources/prompts as first provider using different implementation language

#### Event Streaming Requirements (Optional)

- **FR-026**: Gateway MAY subscribe to message broker topics for at-least-once event delivery from providers
- **FR-027**: Providers MAY publish structured events to message broker topics for asynchronous updates
- **FR-028**: Gateway MUST replay missed events after restart to maintain event continuity when event streaming is enabled

#### Security and Compliance

- **FR-029**: System MUST NOT store secrets (API keys, passwords) in version control or source code - only in environment variables or secure secret stores
- **FR-030**: Gateway MUST authenticate AI client connections at the perimeter to enforce access control
- **FR-031**: Gateway-to-provider communication MUST use network isolation or mutual TLS for secure internal communication without application-layer authentication

### Key Entities

- **Provider**: A backend microservice that implements the standard provider interface and exposes MCP capabilities (tools/resources/prompts). Each provider is registered in gateway configuration with name and connection details.

- **Capability**: A unit of functionality exposed by a provider - can be a Tool (executable function), Resource (readable content via URI), or Prompt (template with parameters). Each capability has a name, description, and machine-readable schema.

- **Tool**: An executable operation with typed inputs and outputs defined by schema validation rules. Examples include echo (message passthrough) and sum (numeric aggregation). Tools are invoked synchronously with request/response pattern.

- **Resource**: Content accessible via URI scheme (e.g., "hello://greeting"). Providers declare supported URI schemes and gateway routes requests based on scheme matching.

- **Prompt**: A template with parameter placeholders (e.g., {name}) that AI clients can use for structured interactions. Prompts are defined with schemas for parameter validation.

- **Event**: Structured message envelope with metadata (identifier, source, type, timestamp) and payload. Used for asynchronous provider-to-gateway communication through message broker.

- **Correlation ID**: Unique identifier propagated across gateway and provider boundaries for distributed request tracing and log correlation across service boundaries.

## Assumptions

The following assumptions were made during specification creation based on industry standards and the source architecture plan:

1. **Provider Communication**: Providers communicate with gateway using synchronous request-response for tool invocations and asynchronous pub/sub for events
2. **Schema Validation**: Input validation uses standard JSON Schema specification for cross-language compatibility
3. **Service Discovery**: Static provider configuration is sufficient for MVP; dynamic service discovery can be added later
4. **Error Handling**: Standard error codes and messages provide sufficient detail for AI clients to handle failures gracefully
5. **Performance Requirements**: Sub-second response times (500ms) are acceptable for simple synchronous tool operations
6. **Scalability**: Single gateway instance with connection pooling to providers is sufficient for MVP workload
7. **Event Delivery**: At-least-once event delivery semantics are acceptable (idempotent event handling at consumer)
8. **Security Model**: Environment-based secrets and secure transport are sufficient for initial deployment
9. **Observability**: Structured logging and correlation IDs provide adequate traceability without full distributed tracing initially
10. **Demo Scope**: Two providers (different implementation languages) are sufficient to validate multi-provider architecture

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: AI client can discover all tools from running providers within 3 seconds of connecting to gateway
- **SC-002**: Tool invocation round-trip (client request to provider response) completes in under 500ms for simple operations (echo, sum)
- **SC-003**: Gateway successfully aggregates capabilities from 2 or more providers implemented in different languages without naming conflicts
- **SC-004**: 100% of tool invocations with schema-compliant payloads return successful responses from providers
- **SC-005**: Standard MCP testing tools can connect to gateway, list all capabilities, and invoke tools without errors
- **SC-006**: Gateway maintains operational state when one provider becomes unavailable, continuing to serve requests to remaining providers
- **SC-007**: All tool invocations have correlation IDs present in both gateway and provider logs for end-to-end tracing
- **SC-008**: Developer can implement new provider from scratch and integrate with gateway in under 1 day using provider contract documentation
- **SC-009**: System components start cleanly via standard development workflow commands with all services becoming operational within 30 seconds
- **SC-010**: Event published by provider is received by gateway within 2 seconds when event streaming is enabled
- **SC-011**: After gateway restart, all events published during downtime are replayed from message broker without loss
