# Implementation Plan: Fix SSE Schema Normalization Bugs

**Branch**: `016-fix-sse-schema-bugs` | **Date**: 2025-10-20 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/016-fix-sse-schema-bugs/spec.md`

## Summary

Fix two schema normalization bugs in the MCP gateway's SSE transport that prevent `market.get_orderbook_l1` and `market.get_klines` tools from working correctly. The primary issues are: (1) orderbook normalizer not correctly parsing bids/asks arrays from Binance provider response, (2) klines normalizer attempting string-keyed access on array responses, and (3) venue parameter defaulting to `None` instead of `"binance"`.

**Technical Approach**: Debug and fix schema normalization logic in `mcp-gateway/mcp_gateway/adapters/schema_adapter.py` for Binance provider responses, update venue parameter defaulting logic in unified router.

## Technical Context

**Language/Version**: Python 3.11
**Primary Dependencies**: mcp>=1.7.1, grpcio>=1.60.0, jsonschema>=4.20.0, pyyaml>=6.0
**Storage**: N/A (in-memory processing only)
**Testing**: pytest>=7.4.0, existing SSE test client (`test_sse_client.py`)
**Target Platform**: Linux server (SSE HTTP server on port 3001)
**Project Type**: Single project (Python package with client-server architecture)
**Performance Goals**: No regression - maintain existing latency (orderbook_l1: <5ms, ticker: <600ms)
**Constraints**: Must not break existing working tools (ticker, volume_profile, orderbook_health)
**Scale/Scope**: 2-3 Python files to modify, 20 existing tools to verify

## Constitution Check

*No constitution file present - skipping gate checks for bugfix*

**Rationale**: This is a targeted bugfix to existing functionality. No new complexity being added, only fixing broken schema normalization logic.

## Project Structure

### Documentation (this feature)

```
specs/016-fix-sse-schema-bugs/
├── spec.md              # Feature specification
├── plan.md              # This file
├── research.md          # Bug analysis and root cause
├── data-model.md        # N/A (no data model changes)
├── quickstart.md        # Testing and verification guide
└── tasks.md             # Implementation tasks (created by /speckit.tasks)
```

### Source Code (repository root)

```
mcp-trader/
├── mcp-gateway/
│   ├── mcp_gateway/
│   │   ├── adapters/
│   │   │   ├── schema_adapter.py      # PRIMARY FIX: Binance normalizer logic
│   │   │   ├── unified_router.py      # SECONDARY FIX: venue defaulting
│   │   │   └── grpc_client.py         # (unchanged)
│   │   ├── sse_server.py              # (unchanged - venue default in tool defs)
│   │   └── main.py                    # (unchanged)
│   ├── providers.yaml                 # (unchanged)
│   └── pyproject.toml                 # (unchanged)
├── test_sse_client.py                 # VALIDATION: Existing test script
└── specs/016-fix-sse-schema-bugs/     # This planning directory
```

**Structure Decision**: Single Python project. Changes isolated to 2 files in `mcp_gateway/adapters/` directory. No structural changes needed.

## Complexity Tracking

*Not applicable - this is a bugfix with no new complexity.*

## Phase 0: Research - Bug Analysis

**Objective**: Identify root causes of the two failing tools through code inspection and test result analysis.

### Research Tasks

1. **Orderbook L1 Normalization Failure Analysis**
   - Task: Examine `schema_adapter.py` Binance normalizer for orderbook_l1
   - Question: How does normalizer expect bids/asks structure vs actual provider response?
   - Method: Compare provider response format (from test logs) with normalizer expectations
   - Expected Output: Root cause of "missing bids or asks" error

2. **Klines Array Access Error Analysis**
   - Task: Examine klines normalization code for array vs string access mismatch
   - Question: Where is code attempting `response["key"]` instead of `response[0]`?
   - Method: Code inspection + stack trace analysis from test error
   - Expected Output: Exact line causing "list indices must be integers" error

3. **Venue Parameter Defaulting Issue**
   - Task: Trace venue parameter flow from SSE tool call to schema normalizer
   - Question: Where should venue default to "binance" but currently defaults to None?
   - Method: Follow parameter flow through unified_router.py and sse_server.py
   - Expected Output: Location where default value should be applied

4. **Working Tools Comparison**
   - Task: Compare working tools (ticker, volume_profile) with broken tools
   - Question: What do working normalizers do differently?
   - Method: Side-by-side code comparison
   - Expected Output: Pattern or structure that works vs doesn't work

### Research Output

**File**: `research.md`

**Structure**:
```markdown
# Bug Analysis: SSE Schema Normalization

## Issue 1: Orderbook L1 - Missing Bids/Asks

**Root Cause**: [Specific code location and logic error]
**Provider Response Format**: [Actual JSON structure from Binance]
**Normalizer Expectation**: [What code expects]
**Fix Approach**: [How to align expectation with reality]

## Issue 2: Klines - Invalid Array Access

**Root Cause**: [Specific code location]
**Error Pattern**: [String key used where integer index needed]
**Fix Approach**: [Change access pattern]

## Issue 3: Venue Parameter None

**Root Cause**: [Where default should be set but isn't]
**Current Flow**: [Parameter path through code]
**Fix Approach**: [Where to add default value]

## Verification Strategy

- Test all 20 tools after fixes
- Ensure no regression in working tools
- Validate with existing test_sse_client.py
```

## Phase 1: Design & Implementation

**Prerequisites**: research.md complete with root causes identified

### Data Model

**File**: `data-model.md`

**Content**: N/A - This is a bugfix with no data model changes. The existing response schemas (OrderbookL1, Klines) remain unchanged; only normalization logic is fixed.

### API Contracts

**Directory**: `contracts/`

**Content**: N/A - This is a bugfix. The tool contracts (input/output schemas) remain unchanged. Only internal normalization behavior is being fixed.

### Quickstart Guide

**File**: `quickstart.md`

**Purpose**: Guide for testing and verifying the fixes

**Structure**:
```markdown
# Testing SSE Schema Normalization Fixes

## Prerequisites

- Binance provider running on port 50053
- SSE gateway running on port 3001
- Python 3.11+ with uv

## Running Tests

### 1. Start Services

```bash
# Terminal 1: Binance provider
cd providers/binance-rs
./target/release/binance-provider --grpc --port 50053

# Terminal 2: SSE gateway
cd mcp-gateway
uv run python -m mcp_gateway.sse_server
```

### 2. Run Test Suite

```bash
uv run --directory mcp-gateway python ../test_sse_client.py
```

### 3. Expected Results

**Before fixes** (baseline - 60% pass rate):
- ✅ market.get_ticker: PASS
- ❌ market.get_orderbook_l1: FAIL (missing bids/asks)
- ❌ market.get_klines: FAIL (list indices error)
- ✅ analytics.get_volume_profile: PASS
- ✅ analytics.get_orderbook_health: PASS

**After fixes** (target - 100% pass rate):
- ✅ market.get_ticker: PASS
- ✅ market.get_orderbook_l1: PASS (valid bid/ask data)
- ✅ market.get_klines: PASS (5 candlesticks returned)
- ✅ analytics.get_volume_profile: PASS
- ✅ analytics.get_orderbook_health: PASS

### 4. Verification Checklist

- [ ] All 5 tested tools return valid data
- [ ] No venue=None or venue="N/A" in responses
- [ ] Orderbook shows best_bid, best_ask, spread_bps
- [ ] Klines returns correct number of candles
- [ ] No regression in previously working tools

## Manual Testing

Test individual tools via SSE:

```python
# See test_sse_client.py for full examples
result = await session.call_tool(
    "market.get_orderbook_l1",
    arguments={"instrument": "ETHUSDT"}  # venue defaults to binance
)
```

## Debugging

If tests fail, check:
1. SSE server logs: `/tmp/sse-server.log`
2. Provider logs: `/tmp/binance-provider.log`
3. Tool responses for error messages
```

## Phase 2: Task Breakdown

**Note**: Tasks will be generated by `/speckit.tasks` command, not included here.

**Expected task categories**:
1. Fix orderbook_l1 normalization (P1)
2. Fix klines array access (P1)
3. Fix venue parameter defaulting (P2)
4. Verify all tools with test suite (P1)
5. Update test expectations if needed (P3)

## Implementation Notes

### Files to Modify

1. **`mcp-gateway/mcp_gateway/adapters/schema_adapter.py`**
   - Fix Binance normalizer for orderbook_l1 tool
   - Fix klines normalization array access
   - Estimated: 10-20 lines changed

2. **`mcp-gateway/mcp_gateway/adapters/unified_router.py`**
   - Add venue="binance" default when venue parameter is None
   - Estimated: 5-10 lines changed

### Testing Strategy

- Use existing `test_sse_client.py` for validation
- No new tests needed - fixing broken functionality
- Success = 100% pass rate (currently 60%)
- Regression testing: Ensure ticker, volume_profile, orderbook_health still work

### Rollback Plan

- Changes are isolated to normalization logic
- Git revert if any regression detected
- No database migrations or state changes to worry about

## Dependencies

**External**: None - using existing dependencies
**Internal**:
- Binance provider must be running (already working)
- gRPC communication layer (already working)
- MCP protocol layer (already working)

## Risks

**Low risk bugfix**:
- Changes isolated to 2 files
- Well-defined problem from test results
- Easy to verify (automated test)
- Easy to rollback (no state changes)

**Mitigation**:
- Test all 20 tools after changes
- Compare before/after test results
- Keep changes minimal and focused
