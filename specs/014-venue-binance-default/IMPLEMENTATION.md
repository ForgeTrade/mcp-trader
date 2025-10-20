# Implementation Plan: Feature 014 - Standardize Venue Parameter with Binance Default

**Branch**: `014-venue-binance-default`
**Dependencies**: None (modifies existing Feature 013 infrastructure)
**Estimated Complexity**: Low (configuration and parameter handling changes)

## Constitution Check

This section documents compliance with the project constitution (`.specify/memory/constitution.md` v1.0.0).

### Principle I: Simplicity and Readability
✅ **Compliant**: Changes involve simple parameter defaulting and string mapping. No complex logic introduced.

### Principle II: Library-First Development
✅ **Compliant**: Uses existing Python standard library features (function default parameters). No new dependencies required.

### Principle III: Justified Abstractions
✅ **Compliant**: No new abstractions added. Modifies existing parameter handling and provider name mapping - both concrete, present needs.

### Principle IV: DRY Principle
✅ **Compliant**: Venue normalization logic centralized in one location (provider registry or router initialization). Not duplicated across 20 tools.

### Principle V: Service and Repository Patterns
✅ **Compliant**: Maintains existing architecture - UnifiedToolRouter (service layer) and ProviderGRPCClient (repository layer) unchanged in structure.

### Principle VI: 12-Factor Methodology
✅ **Compliant**: No impact on 12-factor compliance. Configuration remains externalized, processes remain stateless.

### Principle VII: Minimal Object-Oriented Programming
✅ **Compliant**: Changes to existing class methods only. No new classes or inheritance introduced.

**Overall Assessment**: Feature 014 fully complies with all seven core principles of the project constitution.

## Technical Context

### Existing Architecture (Current State)

**UnifiedToolRouter** (`mcp-gateway/mcp_gateway/adapters/unified_router.py`):
- Currently requires `venue` parameter for all unified tool calls
- Maps `{venue}.tool_name` to provider-specific implementations
- Validates venue against `self.provider_clients` keys

**SSE Server** (`mcp-gateway/mcp_gateway/sse_server.py`):
- Defines 20 unified tools in `list_tools()` handler
- Each tool has `venue` parameter marked as **required** in input schema
- Venue parameter has enum listing all available venues

**Provider Registry** (`mcp-gateway/mcp_gateway/main.py` or gateway initialization):
- Registers provider clients with internal IDs (e.g., "binance-rs", "hello-go", "hello-rs")
- These IDs are exposed directly to users in tool schemas

### What Needs to Change

**1. Provider Name Normalization**
- Create public venue name → internal provider ID mapping
- Map "binance" → "binance-rs" (or whatever internal ID is used)
- Filter out test providers ("hello-go", "hello-rs") from public exposure

**2. Default Parameter Handling**
- Make `venue` parameter optional in all 20 tool definitions
- Add `default="binance"` to venue parameter in SSE tool schemas
- Update UnifiedToolRouter to handle missing venue parameter

**3. Schema Updates**
- Change venue enum from `["binance-rs", "hello-go", "hello-rs"]` to `["binance"]`
- Mark venue parameter as not required (remove from `required` list)
- Update tool descriptions to indicate binance is default

**4. Validation Updates**
- Update venue validation to check against public names ("binance"), not internal IDs
- Return error messages using public names: "Unknown venue '{venue}'. Available venues: binance"

## Implementation Tasks

### Task 1: Create Venue Name Mapping (T014-01)

**Location**: Determine where provider clients are registered (likely `mcp-gateway/mcp_gateway/main.py` or init code)

**Changes**:
```python
# Define public venue names and their internal provider IDs
VENUE_MAPPING = {
    "binance": "binance-rs",  # Public name → Internal provider ID
}

# Filter out test providers
PUBLIC_VENUES = list(VENUE_MAPPING.keys())  # ["binance"]
```

**Rationale**: Centralizes naming logic, makes it easy to add new venues later

**Requirements Covered**: FR-004, FR-005, FR-006

---

### Task 2: Update UnifiedToolRouter for Optional Venue (T014-02)

**File**: `mcp-gateway/mcp_gateway/adapters/unified_router.py`

**Changes**:
```python
async def route_tool_call(
    self,
    unified_tool_name: str,
    arguments: Dict[str, Any],
    correlation_id: str,
    timeout: float = 5.0
) -> Dict[str, Any]:
    """Route a unified tool call to the appropriate provider."""

    # NEW: Default venue to "binance" if not provided
    venue = arguments.get("venue", "binance")  # <-- Add default here

    # NEW: Map public venue name to internal provider ID
    provider_id = VENUE_MAPPING.get(venue)
    if not provider_id:
        raise ValueError(
            f"Unknown venue '{venue}'. Available venues: {', '.join(PUBLIC_VENUES)}"
        )

    # Use provider_id instead of venue for client lookup
    if provider_id not in self.provider_clients:
        raise ValueError(f"Provider for venue '{venue}' is not configured")

    client = self.provider_clients[provider_id]
    # ... rest of routing logic unchanged
```

**Testing**: Update `test_unified_routing.py` to test missing venue parameter

**Requirements Covered**: FR-001, FR-002, FR-003, FR-007, FR-008

---

### Task 3: Update SSE Server Tool Definitions (T014-03)

**File**: `mcp-gateway/mcp_gateway/sse_server.py`

**Changes**: For all 20 Tool definitions, update venue parameter:

**BEFORE**:
```python
Tool(
    name="market.get_ticker",
    description=f"Get ticker data. Available venues: {venues_list}",
    inputSchema={
        "type": "object",
        "required": ["venue", "instrument"],  # <-- venue is required
        "properties": {
            "venue": {
                "type": "string",
                "description": "Exchange venue",
                "enum": ["binance-rs", "hello-go", "hello-rs"]  # <-- Internal IDs
            },
            # ...
        }
    }
)
```

**AFTER**:
```python
Tool(
    name="market.get_ticker",
    description="Get ticker data. Default venue: binance",
    inputSchema={
        "type": "object",
        "required": ["instrument"],  # <-- venue removed from required
        "properties": {
            "venue": {
                "type": "string",
                "description": "Exchange venue (optional, default: binance)",
                "enum": ["binance"],  # <-- Only public name
                "default": "binance"  # <-- Explicit default
            },
            # ...
        }
    }
)
```

**Apply to all 20 tools**: market.* (7), trade.* (7), analytics.* (6)

**Requirements Covered**: FR-010, FR-011, FR-012

---

### Task 4: Update Error Messages (T014-04)

**File**: `mcp-gateway/mcp_gateway/adapters/unified_router.py`

**Changes**: Ensure all error messages use public venue names

```python
# Example error messages
f"Unknown venue '{venue}'. Available venues: {', '.join(PUBLIC_VENUES)}"
f"Provider for venue '{venue}' is not configured"
f"Provider '{venue}' is unavailable"
```

**Review**: Check all logging and error handling to ensure no leakage of internal provider IDs

**Requirements Covered**: FR-009

---

### Task 5: Update Provider Registry Initialization (T014-05)

**File**: `mcp-gateway/mcp_gateway/main.py` (or wherever providers are registered)

**Changes**: Ensure provider clients are registered with internal IDs, but only mapped to public names

```python
# Initialize provider clients (internal IDs)
provider_clients = {
    "binance-rs": ProviderGRPCClient(...),
    # Remove or don't register: "hello-go", "hello-rs"
}

# Only expose public venues to router
router = UnifiedToolRouter(provider_clients, public_venues=PUBLIC_VENUES)
```

**Requirements Covered**: FR-005, FR-006

---

## Testing Strategy

### Manual Testing

1. **Test default venue behavior**:
   - Call `market.get_ticker(instrument="BTCUSDT")` without venue → should succeed
   - Call `trade.get_account()` without venue → should succeed

2. **Test explicit venue**:
   - Call `market.get_ticker(venue="binance", instrument="BTCUSDT")` → should succeed (same as default)

3. **Test invalid venues**:
   - Call `market.get_ticker(venue="binance-rs", ...)` → should error: "Unknown venue 'binance-rs'. Available venues: binance"
   - Call `market.get_ticker(venue="hello-go", ...)` → should error with same message format

4. **Test schema inspection**:
   - Query tool listing via MCP/SSE → venue enum should show only ["binance"]
   - Check that venue is not in required fields list

### Unit Tests (Optional)

- `test_venue_default.py` - Test UnifiedToolRouter with missing venue parameter
- `test_venue_validation.py` - Test error messages for invalid venues
- `test_venue_mapping.py` - Test public name → internal ID mapping

---

## Deployment Checklist

- [ ] All 20 tool definitions updated in SSE server
- [ ] UnifiedToolRouter handles missing venue parameter
- [ ] Venue mapping centralized and documented
- [ ] Error messages use public venue names only
- [ ] Manual testing completed (default, explicit, invalid venues)
- [ ] Deploy to staging environment
- [ ] Verify tool schemas show venue as optional with "binance" enum
- [ ] Deploy to production (198.13.46.14:3001)
- [ ] Verify backward compatibility (explicit venue="binance" still works)

---

## Risk Mitigation

**Risk**: Breaking existing clients that explicitly specify venue
- **Mitigation**: Maintain backward compatibility - `venue="binance"` continues to work (FR-003, SC-003)

**Risk**: Confusion if future exchanges are added
- **Mitigation**: Design supports multi-venue - just add to VENUE_MAPPING and PUBLIC_VENUES list

**Risk**: Internal provider ID exposure in logs or errors
- **Mitigation**: Audit all error messages and logging to ensure venue name normalization (T014-04)

---

## Dependencies

- Feature 013 (Complete Unified Multi-Exchange API) deployed and working
- Existing UnifiedToolRouter and SSE server infrastructure in place

## Estimated Timeline

- Task 1 (Venue Mapping): 30 minutes
- Task 2 (UnifiedToolRouter): 1 hour
- Task 3 (SSE Tool Definitions): 2 hours (20 tools)
- Task 4 (Error Messages): 30 minutes
- Task 5 (Provider Registry): 30 minutes
- Testing + Verification: 1 hour
- Deployment: 30 minutes

**Total**: ~6 hours

---

## Implementation Notes

This is a **configuration and parameter handling feature** - no new functionality, just improving API ergonomics and cleaning up provider naming. The core routing and normalization logic from Feature 013 remains unchanged.

**Key insight**: The venue parameter becomes a "smart default" - users can omit it for single-exchange scenarios, but the system remains extensible for multi-exchange support in the future.
