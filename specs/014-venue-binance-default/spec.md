# Feature Specification: Standardize Venue Parameter with Binance Default

**Feature Branch**: `014-venue-binance-default`
**Created**: 2025-10-20
**Status**: Draft
**Input**: "Нужно только переименовать venue, показывать, что доступна сейчас только одно значение binance и пока сделать его дефолтным. Нужно убрать оттуда hello-go, hello-rs. Должно быть видно только binance. И НЕ binance-rs, а binance."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - API User Calls Unified Tools Without Specifying Venue (Priority: P1)

An API user or application wants to call unified market data, trading, or analytics tools without explicitly specifying a venue parameter, and the system should automatically use Binance as the default exchange.

**Why this priority**: This is the MVP - it immediately simplifies the API for single-exchange users and removes confusion about which venue to use when only one is available. It reduces cognitive load and API call complexity.

**Independent Test**: User can call any unified tool (e.g., `market.get_ticker(instrument="BTCUSDT")`) without providing `venue` parameter, and the request succeeds using Binance automatically.

**Acceptance Scenarios**:

1. **Given** a user calls `market.get_ticker(instrument="BTCUSDT")` without the `venue` parameter, **When** the gateway processes the request, **Then** it automatically routes to Binance and returns ticker data

2. **Given** a user calls `trade.get_account()` without the `venue` parameter, **When** the gateway processes the request, **Then** it retrieves account data from Binance by default

3. **Given** a user calls `analytics.get_orderbook_health(instrument="BTCUSDT")` without `venue`, **When** the request is processed, **Then** orderbook health metrics are fetched from Binance

4. **Given** a user explicitly provides `venue="binance"` in their tool call, **When** the gateway processes it, **Then** the behavior is identical to omitting the parameter (backward compatibility maintained)

---

### User Story 2 - API User Sees Clean Provider List (Priority: P2)

An API user or developer examining the available venues should only see "binance" as the valid option, with no test providers (hello-go, hello-rs) or internal naming (binance-rs) visible.

**Why this priority**: Clean API documentation and clear available options improve user experience. Hiding internal/test providers prevents confusion and accidental misconfiguration.

**Independent Test**: User inspects tool schemas or available venues list and sees only "binance" as an option. Attempts to use "binance-rs", "hello-go", or "hello-rs" result in clear error messages.

**Acceptance Scenarios**:

1. **Given** a user queries the tool schema for `market.get_ticker`, **When** examining the `venue` parameter enum, **Then** only "binance" appears in the list of valid values

2. **Given** a user attempts to call a tool with `venue="binance-rs"`, **When** the gateway validates the request, **Then** it returns an error: "Unknown venue 'binance-rs'. Available venues: binance"

3. **Given** a user attempts to call a tool with `venue="hello-go"` or `venue="hello-rs"`, **When** the request is processed, **Then** the system returns an error indicating the venue is not available

4. **Given** a developer examines available venues via the MCP tool listing, **When** viewing venue options, **Then** only "binance" is shown in descriptions and schema enums

---

### Edge Cases

- What happens if a user explicitly specifies `venue="binance"` in their request?
  - The system treats it identically to omitting the parameter - Binance is used. No error, full backward compatibility.

- What if the Binance provider is down or unreachable when using the default?
  - Standard error handling applies - the system returns a provider unavailability error with context: "Provider 'binance' is unavailable"

- How does the system handle future addition of new venues (e.g., OKX, Kraken)?
  - When new providers are added, the venue parameter will remain optional with binance as default. Tool schemas will be updated to include new venue enums. This feature only affects the default behavior, not the extensibility.

- What if configuration still references "binance-rs" internally (e.g., provider client keys)?
  - Internal configuration can keep provider IDs like "binance-rs" in code/config, but the public-facing API must normalize these to "binance". The adapter layer handles the mapping transparently.

## Requirements *(mandatory)*

### Functional Requirements

**Venue Parameter Default Behavior:**

- **FR-001**: All unified tools (market.*, trade.*, analytics.*) MUST accept an optional `venue` parameter with "binance" as the default value

- **FR-002**: When `venue` parameter is omitted in a tool call, the system MUST automatically route the request to the Binance provider

- **FR-003**: When `venue="binance"` is explicitly provided, the system MUST behave identically to when the parameter is omitted (backward compatibility)

**Provider Visibility and Naming:**

- **FR-004**: Tool schemas MUST expose "binance" (not "binance-rs") as the display name for the Binance exchange in all public-facing documentation, enums, and error messages

- **FR-005**: Test providers ("hello-go", "hello-rs") MUST NOT appear in the available venues list exposed to API users

- **FR-006**: Internal provider registration MAY use technical identifiers (e.g., "binance-rs"), but all external representations MUST normalize to "binance"

**Validation and Error Handling:**

- **FR-007**: When a user specifies an invalid venue (e.g., "binance-rs", "hello-go", "hello-rs", or any unrecognized value), the system MUST return an error: "Unknown venue '{invalid_venue}'. Available venues: binance"

- **FR-008**: The UnifiedToolRouter MUST validate venue parameters against the list of publicly available venues (currently only "binance")

- **FR-009**: Error messages referencing venues MUST use the normalized public name ("binance") not internal provider IDs

**Schema and Documentation:**

- **FR-010**: The SSE server tool definitions MUST include "binance" as the only enum value for the `venue` parameter

- **FR-011**: The `venue` parameter in all tool input schemas MUST be marked as optional (not required)

- **FR-012**: Tool descriptions MUST indicate that "binance" is the current default and only available venue

### Key Entities

- **Venue**: The public-facing identifier for an exchange provider. Currently only "binance" is exposed. Maps to internal provider identifiers but abstracts implementation details.

- **Provider Client Registry**: Internal mapping between public venue names (e.g., "binance") and provider implementation identifiers (e.g., "binance-rs"). Users never see internal IDs.

- **Unified Tool Definition**: Tool schema exposed via SSE/MCP that specifies venue as an optional parameter with "binance" default and "binance" as the only enum value.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: API users can successfully call all 20 unified tools without specifying the `venue` parameter, and requests automatically route to Binance

- **SC-002**: Tool schemas for all 20 unified tools show `venue` parameter with `enum: ["binance"]` and no "binance-rs", "hello-go", or "hello-rs" values

- **SC-003**: Existing API clients that explicitly specify `venue="binance"` continue to work without modification (100% backward compatibility)

- **SC-004**: Attempts to use "binance-rs", "hello-go", or "hello-rs" as venue values return clear error messages indicating only "binance" is available

- **SC-005**: All error messages and logs reference "binance" as the venue name, with no user-visible mentions of "binance-rs" or test provider names

- **SC-006**: Tool documentation and schema descriptions clearly indicate that "binance" is the current default venue

- **SC-007**: The change reduces API call parameter requirements by making venue optional, improving API ergonomics for single-exchange scenarios
