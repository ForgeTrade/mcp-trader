# Feature Specification: OrderBook Snapshot Persistence

**Feature Branch**: `007-snapshot-persistence`
**Created**: 2025-10-19
**Status**: Draft
**Input**: User description: "Implement background snapshot persistence task - Analytics tools require historical orderbook data but WebSocket connections are lazy and snapshots aren't being persisted to RocksDB storage"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Analytics Tool Users Get Historical Data (Priority: P1)

Users of analytics tools (get_order_flow, get_volume_profile, get_microstructure_health, get_liquidity_vacuums, detect_market_anomalies) can successfully retrieve historical market data for analysis instead of receiving "Insufficient historical data" errors.

**Why this priority**: This is the core value proposition - without historical data, all 5 analytics tools are non-functional. This directly impacts the primary use case of the system.

**Independent Test**: Can be fully tested by calling any analytics tool (e.g., get_order_flow for BTCUSDT with 60-second window) after system has been running for 60 seconds, and receiving valid analysis results instead of an error.

**Acceptance Scenarios**:

1. **Given** the service has been running for 60 seconds, **When** a user calls get_order_flow(symbol="BTCUSDT", window_duration_secs=60), **Then** the system returns order flow analysis with bid/ask pressure metrics
2. **Given** the service has been running for 24 hours, **When** a user calls get_volume_profile(symbol="ETHUSDT", duration_hours=24), **Then** the system returns volume distribution analysis with POC/VAH/VAL metrics
3. **Given** the service has been running for 1 minute, **When** a user calls get_microstructure_health(symbol="BTCUSDT"), **Then** the system returns composite health score (0-100) with component breakdowns
4. **Given** no WebSocket connections have been manually triggered, **When** a user queries analytics tools within 2 minutes of service startup, **Then** historical data is already available from automatic snapshot collection

---

### User Story 2 - System Operators Monitor Data Collection (Priority: P2)

System operators can observe snapshot persistence activity through logs to verify that data collection is working correctly and troubleshoot issues.

**Why this priority**: Operational visibility is critical for diagnosing issues and confirming the feature is working, but is secondary to the actual functionality.

**Independent Test**: Can be tested by starting the service and checking logs for snapshot persistence messages without needing to call analytics tools.

**Acceptance Scenarios**:

1. **Given** the service has just started, **When** operators check the logs, **Then** they see "Stored snapshot for BTCUSDT at [timestamp]" messages appearing every second
2. **Given** the service is running with multiple symbols subscribed, **When** operators check the logs, **Then** they see distinct log entries for each symbol being persisted
3. **Given** a network error occurs during snapshot collection, **When** the error is logged, **Then** the service continues running and retries on the next cycle

---

### User Story 3 - Service Remains Stable Under Errors (Priority: P3)

The service continues operating normally even when snapshot persistence encounters errors, ensuring that other functionality (live orderbook, market data tools) remains available.

**Why this priority**: Reliability is important, but this is a background task that shouldn't impact primary services. It's a quality attribute rather than core functionality.

**Independent Test**: Can be tested by simulating RocksDB write failures and verifying the service doesn't crash and continues serving live data.

**Acceptance Scenarios**:

1. **Given** RocksDB disk is full, **When** snapshot persistence fails, **Then** the error is logged and the service continues running without crashing
2. **Given** a snapshot serialization error occurs, **When** the background task encounters the error, **Then** it logs the error and continues with the next snapshot in the next cycle
3. **Given** WebSocket connection is temporarily disrupted, **When** the persistence task attempts to snapshot, **Then** it handles the missing data gracefully and resumes when connection is restored

---

### Edge Cases

- What happens when the service starts and RocksDB already contains historical data? (Answer: Continues appending new snapshots, existing data is preserved)
- How does the system handle rapid service restarts? (Answer: Each startup begins fresh snapshot collection; gaps in data are acceptable)
- What if a symbol subscription fails? (Answer: Log the error, continue with other symbols, retry subscription on next cycle)
- What happens when disk space runs low? (Answer: RocksDB write fails, error is logged, service continues, cleanup task will run per retention policy)
- How are different snapshot frequencies for different symbols handled? (Answer: Initially all symbols use 1-second interval; future enhancement could support per-symbol intervals)

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST subscribe to WebSocket orderbook streams for BTCUSDT and ETHUSDT immediately upon service startup, before any client requests
- **FR-002**: System MUST capture orderbook snapshots every 1 second for each subscribed symbol
- **FR-003**: System MUST serialize each snapshot using MessagePack format before storage
- **FR-004**: System MUST store serialized snapshots in RocksDB using the key format "{symbol}:{unix_timestamp_sec}"
- **FR-005**: System MUST continue operating normally if snapshot persistence fails (errors must not crash the service)
- **FR-006**: System MUST log successful snapshot storage with message format "Stored snapshot for {symbol} at {timestamp}" at INFO level
- **FR-007**: System MUST log snapshot persistence errors at ERROR level with details about the failure cause
- **FR-008**: Analytics tools MUST be able to query historical snapshots using existing query_snapshots_in_window() function
- **FR-009**: System MUST preserve existing orderbook functionality (live WebSocket updates, L1/L2 queries) while adding persistence
- **FR-010**: Background persistence task MUST run independently from client request handling

### Key Entities

- **OrderBook Snapshot**: Represents the state of the orderbook at a specific point in time
  - Contains symbol identifier, timestamp, bid levels (price/quantity), ask levels (price/quantity)
  - Serialized using MessagePack for efficient storage
  - Stored with unique key combining symbol and timestamp

- **Symbol Subscription**: Represents an active WebSocket connection collecting data for a trading pair
  - Initially includes BTCUSDT and ETHUSDT
  - Can be extended to additional symbols
  - Maintains connection state and handles reconnection

- **Persistence Task**: Background process that periodically saves snapshots
  - Runs every 1 second interval
  - Operates independently for each subscribed symbol
  - Handles errors without interrupting service

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Analytics tools successfully return historical data instead of "Insufficient historical data" errors within 60 seconds of service startup
- **SC-002**: Snapshot persistence logs appear in system logs at 1-second intervals for each subscribed symbol
- **SC-003**: RocksDB storage accumulates at minimum 60 snapshots per symbol per minute (allowing for occasional missed snapshots due to errors)
- **SC-004**: Service uptime is unaffected by snapshot persistence errors (service continues running even when persistence fails)
- **SC-005**: Live orderbook functionality (orderbook_l1, orderbook_l2 tools) continues to work with sub-200ms latency during snapshot persistence

### Assumptions

- RocksDB cleanup/retention policy (7-day retention) is already implemented or will be handled separately
- WebSocket connection reliability and reconnection logic already exists in OrderBookManager
- MessagePack serialization library (rmp-serde) is already included in dependencies
- Disk space is sufficient for ~60 snapshots/min/symbol × 2 symbols × ~500 bytes/snapshot = ~3.6 MB/hour = ~600 MB/week (before cleanup)
- Current orderbook data structure (OrderBookSnapshot) is suitable for persistence without modifications

### Out of Scope

- Dynamic symbol subscription management (ability to add/remove symbols at runtime)
- Variable persistence intervals (different frequencies for different symbols)
- Snapshot compression beyond MessagePack's built-in efficiency
- Historical data backfill from external sources
- Snapshot deduplication or delta encoding
- Real-time snapshot streaming to external consumers

