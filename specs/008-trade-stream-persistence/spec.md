# Feature Specification: Trade Stream Persistence

**Feature Branch**: `008-trade-stream-persistence`
**Created**: 2025-10-19
**Status**: Draft
**Input**: User description: "Implement trade stream persistence to collect historical trades for volume profile analytics"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Analytics Tools Access Historical Trades (Priority: P1) 🎯 MVP

When ChatGPT users invoke volume profile analytics tools (`get_volume_profile`, `get_liquidity_vacuums`), the tools successfully retrieve and analyze historical trade data instead of returning "insufficient trades" errors.

**Why this priority**: This is the core value proposition. Currently these tools fail with "need ≥1000 trades for 24h, got 0" because no trade data is being collected. Without this, two out of five analytics tools are non-functional.

**Independent Test**: After the service runs for 10 minutes, calling `get_volume_profile` with `symbol=BTCUSDT, duration_hours=1` returns a valid volume profile with POC/VAH/VAL metrics (not an error about insufficient data).

**Acceptance Scenarios**:

1. **Given** service has been running for 10+ minutes with trade collection active, **When** user invokes `get_volume_profile(symbol="BTCUSDT", duration_hours=1)`, **Then** tool returns volume distribution histogram with ≥1000 trades analyzed
2. **Given** service has been running for 30+ minutes, **When** user invokes `get_liquidity_vacuums(symbol="ETHUSDT", duration_hours=1)`, **Then** tool identifies low-volume price zones using accumulated trade history
3. **Given** trade collection just started (< 10 minutes), **When** user invokes volume profile tool, **Then** system returns clear error message indicating data collection in progress with estimated wait time

---

### User Story 2 - Operators Monitor Trade Collection Activity (Priority: P2)

System operators can observe trade collection activity through structured logs to verify data is being collected correctly and troubleshoot issues when analytics tools report insufficient data.

**Why this priority**: Operational visibility is essential for production support. Without logs, operators cannot diagnose why analytics tools might be failing or verify that trade streams are working correctly.

**Independent Test**: Review service logs and verify trade collection messages appear continuously with symbol, trade count, and timestamp information. Confirm operators can determine collection health without accessing the database.

**Acceptance Scenarios**:

1. **Given** service is running normally, **When** operator reviews logs, **Then** they see periodic "Stored N trades for BTCUSDT" messages with timestamps and trade counts
2. **Given** Binance WebSocket connection drops, **When** operator checks logs, **Then** they see ERROR-level messages indicating connection failure and automatic reconnection attempts
3. **Given** trade collection has been running for 1 hour, **When** operator queries log aggregation, **Then** they can determine average trades/minute per symbol and identify any gaps in collection

---

### User Story 3 - Service Maintains Stability During Trade Persistence Failures (Priority: P3)

The service continues operating normally and serving live market data even when trade persistence encounters errors (network issues, storage failures, serialization problems), ensuring that live orderbook functionality remains available.

**Why this priority**: Resilience is critical for production reliability, but less urgent than actually collecting trade data. Service stability should not be compromised by analytics features.

**Independent Test**: Simulate storage write failures (e.g., full disk) and verify the service continues running, live orderbook tools remain functional, and errors are logged without crashing the background task.

**Acceptance Scenarios**:

1. **Given** RocksDB storage write fails due to disk full, **When** trade persistence attempts to store trades, **Then** error is logged but background task continues running and live orderbook tools still work
2. **Given** WebSocket receives malformed trade data, **When** deserialization fails, **Then** error is logged for that specific trade but stream processing continues for subsequent trades
3. **Given** trade persistence is experiencing intermittent failures, **When** storage recovers, **Then** collection automatically resumes without requiring service restart

---

### Edge Cases

- What happens when trade stream connection drops mid-operation? (Should reconnect automatically and log the gap in data collection)
- How does system handle extremely high trade velocity during volatile markets? (Batch writes to RocksDB to prevent I/O bottlenecks)
- What happens when querying analytics tools during the first 10 minutes of service startup? (Return clear error indicating data collection in progress)
- How does system behave when RocksDB retention cleanup runs during active trade queries? (Queries should complete successfully; cleanup should not block reads)
- What happens when storage reaches 7-day retention limit? (Automatic cleanup of oldest trades, logged at INFO level)

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST subscribe to Binance aggTrade WebSocket streams for configured symbols (BTCUSDT, ETHUSDT) immediately on service startup
- **FR-002**: System MUST collect and persist every trade from the aggTrade stream including: price, quantity, timestamp, buyer_is_maker flag
- **FR-003**: System MUST batch trades every 1 second and write them to persistent storage in a single transaction
- **FR-004**: System MUST store trades with a key format that enables efficient time-range queries: `trades:{symbol}:{unix_timestamp_ms}`
- **FR-005**: System MUST provide a query interface for analytics tools to retrieve trades within a specified time window (1-168 hours)
- **FR-006**: System MUST automatically delete trades older than 7 days to manage storage growth
- **FR-007**: System MUST gracefully shut down trade collection when receiving termination signal, ensuring no data loss for in-flight trades
- **FR-008**: System MUST log trade collection activity at INFO level (periodic summaries) and errors at ERROR level
- **FR-009**: Analytics tools (`get_volume_profile`, `get_liquidity_vacuums`) MUST retrieve trades from persistent storage instead of returning empty datasets
- **FR-010**: System MUST continue operating and collecting trades even when individual write operations fail (error isolation)

### Key Entities

- **AggTrade (Aggregate Trade)**: Represents a single trade execution on Binance, containing price (execution price), quantity (trade size), timestamp (Unix milliseconds), buyer_is_maker flag (buy vs sell determination), trade_id (unique identifier)
- **TradeBuffer**: Time-series collection of trades for a specific symbol within a time window, used by analytics tools to calculate volume profiles and liquidity metrics
- **TradeStream**: WebSocket subscription providing real-time trade execution data from Binance, delivering 1-10 trades per second per symbol during normal market conditions

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Analytics tools (`get_volume_profile`, `get_liquidity_vacuums`) successfully return valid results after service runs for 10+ minutes, with no "insufficient trades" errors
- **SC-002**: Trade collection rate matches Binance trade stream velocity: 60-600 trades per minute per symbol during normal market hours
- **SC-003**: Storage growth remains predictable: approximately 10-15 MB per day for 2 symbols (BTCUSDT, ETHUSDT) over 7-day retention period
- **SC-004**: Background trade collection task consumes less than 2% CPU and less than 50MB memory on production server
- **SC-005**: Analytics tool queries for 24-hour windows complete in under 3 seconds (target: <1 second for 1-hour windows)
- **SC-006**: Service maintains 99.9% uptime for trade collection (tolerates up to 1.4 minutes of downtime per day due to reconnections)

### Feature Readiness Indicators

- Volume profile tool can generate histogram with POC/VAH/VAL metrics after 10 minutes of collection
- Liquidity vacuum detection identifies low-volume zones using real trade distribution data
- Operators can verify trade collection health through log queries without database access
- Service continues serving live orderbook data even during trade persistence failures
- 7-day retention cleanup runs hourly without impacting query performance

## Assumptions *(optional - document reasonable defaults)*

- **Trade Stream Reliability**: Binance aggTrade WebSocket provides reliable trade data with occasional reconnection needs (industry-standard for WebSocket streams)
- **Trade Velocity**: BTCUSDT and ETHUSDT generate 1-10 trades per second during normal market hours, with spikes to 50+ trades/sec during high volatility
- **Storage Format**: RocksDB key-value storage (same as feature 007 orderbook snapshots) is appropriate for time-series trade data
- **Retention Period**: 7 days matches feature 007 retention policy and provides sufficient historical depth for analytics (168-hour max window)
- **Serialization**: MessagePack binary format (same as feature 007) provides efficient storage (~100-150 bytes per trade)
- **Query Pattern**: Analytics tools query recent history (1-24 hours most common), with occasional requests for longer windows (up to 168 hours)
- **Error Handling**: Transient failures (network issues, brief storage unavailability) resolve within seconds to minutes; no manual intervention needed

## Dependencies *(if applicable)*

- **Feature 007 (Snapshot Persistence)**: Reuses RocksDB storage infrastructure, shutdown signal handling, and persistence task patterns established in feature 007
- **Binance API**: Requires reliable access to wss://stream.binance.com/ws/aggTrade WebSocket endpoint
- **RocksDB**: Storage backend must support concurrent writes (trades) and reads (orderbook snapshots) without blocking
- **tokio-tungstenite**: WebSocket library with native-tls feature (already configured in feature 007)

## Constraints *(if applicable)*

- **Symbol Limit**: Initially limited to 2 symbols (BTCUSDT, ETHUSDT) to match feature 007 orderbook subscriptions
- **Storage Growth**: 7-day retention for 2 symbols results in approximately 100-150 MB total storage (manageable for production server)
- **WebSocket Connection**: Single WebSocket connection per symbol (Binance API limit); cannot aggregate multiple symbols into one connection
- **Query Performance**: Time-range queries must complete within service timeout (10 seconds) to avoid blocking analytics tool responses

## Out of Scope *(explicitly excluded)*

- **Trade Aggregation Logic**: No VWAP, order flow imbalance, or trade clustering calculations in this feature (handled by existing analytics modules)
- **Multi-Exchange Support**: Only Binance aggTrade stream; other exchanges (Kraken, Coinbase) not included
- **Real-Time Trade Streaming**: Analytics tools query historical persisted trades; no direct WebSocket subscriptions for tools
- **Trade Filtering**: All trades are persisted; no filtering by size, price range, or buyer/seller side
- **Backfilling Historical Data**: Only collects trades from service startup onward; no retroactive data fetching from Binance REST API
- **Trade Deduplication**: Assumes Binance WebSocket provides unique trades; no duplicate detection logic
