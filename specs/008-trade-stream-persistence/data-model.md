# Data Model: Trade Stream Persistence

**Feature**: 008-trade-stream-persistence
**Date**: 2025-10-19
**Phase**: 1 (Design)

## Overview

This document defines the data entities and their relationships for trade stream persistence. The model captures trade execution data from Binance aggTrade WebSocket streams and organizes it for efficient storage and retrieval by analytics tools.

## Core Entities

### 1. AggTrade (Aggregate Trade)

**Purpose**: Represents a single trade execution on Binance. Aggregates one or more individual trades that occur at the same price and time.

**Attributes**:

| Field | Type | Description | Constraints | Example |
|-------|------|-------------|-------------|---------|
| `price` | Decimal | Execution price in quote currency | > 0, precision up to 8 decimals | 43250.50 (BTCUSDT) |
| `quantity` | Decimal | Trade size in base currency | > 0, precision up to 8 decimals | 0.15 BTC |
| `timestamp` | i64 | Trade execution time (Unix milliseconds) | > 0, chronologically ordered | 1760903627000 |
| `trade_id` | i64 | Unique aggregate trade identifier | > 0, monotonically increasing | 26129 |
| `buyer_is_maker` | bool | Side determination: true = buy side filled limit order | Boolean | true (buy), false (sell) |

**Validation Rules**:
- `price` and `quantity` must be positive
- `timestamp` must be valid Unix milliseconds (Jan 1, 2020 - Dec 31, 2030 range check)
- `trade_id` must be unique per symbol (Binance guarantee)
- `buyer_is_maker = true` means buyer placed limit order (passive), seller was aggressor
- `buyer_is_maker = false` means seller placed limit order (passive), buyer was aggressor

**Relationships**:
- Belongs to one **Symbol** (implicit via storage key)
- Grouped into **TradeBatch** for storage (1-second batches)

**Lifecycle**:
```
Created → Received from WebSocket
         ↓
Buffered → Added to in-memory Vec<AggTrade>
         ↓
Persisted → Written to RocksDB as part of TradeBatch
         ↓
Queried → Retrieved by analytics tools
         ↓
Deleted → Removed after 7-day retention period
```

**Source**: Binance aggTrade WebSocket message (see contracts/aggtrade-websocket.md)

**Existing Implementation**: Already defined in `src/orderbook/analytics/trade_stream.rs`

---

### 2. TradeBatch (Storage Unit)

**Purpose**: Groups trades within a 1-second window for efficient storage and retrieval. Reduces write frequency and enables time-range queries.

**Attributes**:

| Field | Type | Description | Constraints | Example |
|-------|------|-------------|-------------|---------|
| `symbol` | String | Trading pair identifier | Uppercase, 6-12 chars | "BTCUSDT" |
| `batch_timestamp` | i64 | Batch start time (Unix ms, rounded to second) | Multiple of 1000 | 1760903627000 |
| `trades` | Vec<AggTrade> | All trades within this second | 0-1000 trades typical | [trade1, trade2, ...] |
| `count` | usize | Number of trades in batch (derived) | >= 0 | 150 |

**Validation Rules**:
- `symbol` must be uppercase ASCII (enforced at WebSocket subscription time)
- `batch_timestamp` must be aligned to 1000ms boundaries
- `trades` vector ordered chronologically (Binance guarantee)
- `count` = trades.len() (computed field, not stored)

**Relationships**:
- Contains multiple **AggTrade** entities (0 to ~1000, varies by market volatility)
- Associated with one **Symbol**

**Storage Format**:
- **Key**: `trades:{symbol}:{batch_timestamp}` (e.g., `trades:BTCUSDT:1760903627000`)
- **Value**: MessagePack-serialized `Vec<AggTrade>`
- **Size**: ~500 bytes typical (100 trades × 5 bytes average after compression)

**Lifecycle**:
```
Created → Accumulated in-memory for 1 second
         ↓
Serialized → MessagePack encoding
         ↓
Stored → Written to RocksDB
         ↓
Queried → Prefix scan by symbol + time range filter
         ↓
Deserialized → MessagePack decoding
         ↓
Deleted → Batch removed after 7-day retention
```

---

### 3. Symbol

**Purpose**: Represents a trading pair being tracked for trade collection.

**Attributes**:

| Field | Type | Description | Constraints | Example |
|-------|------|-------------|-------------|---------|
| `name` | String | Trading pair identifier | Uppercase, 6-12 chars | "BTCUSDT" |
| `base_currency` | String (implicit) | Base asset | 3-5 chars | "BTC" |
| `quote_currency` | String (implicit) | Quote asset | 3-5 chars | "USDT" |

**Validation Rules**:
- `name` must match Binance symbol format (BASE + QUOTE, no separator)
- Only configured symbols are tracked (hardcoded: BTCUSDT, ETHUSDT initially)

**Relationships**:
- Has many **TradeBatch** entities (1-second batches over 7 days)
- Has many **AggTrade** entities (aggregated across batches)

**Configuration**:
```rust
// Initially hardcoded in main.rs
const TRACKED_SYMBOLS: &[&str] = &["BTCUSDT", "ETHUSDT"];

// Future: Could be environment variable
// TRACKED_SYMBOLS=BTCUSDT,ETHUSDT,SOLUSDT
```

---

### 4. TradeQuery (Query Parameters)

**Purpose**: Encapsulates query criteria for retrieving trades within a time window.

**Attributes**:

| Field | Type | Description | Constraints | Example |
|-------|------|-------------|-------------|---------|
| `symbol` | String | Trading pair to query | Uppercase, must exist | "BTCUSDT" |
| `start_time` | i64 | Query window start (Unix ms) | > 0, < end_time | 1760900000000 |
| `end_time` | i64 | Query window end (Unix ms) | > start_time | 1760903600000 |

**Validation Rules**:
- `end_time` - `start_time` must be <= 168 hours (7 days, max retention)
- `start_time` must be within retention period (now - 7 days)
- `symbol` must be one of the tracked symbols

**Derived Fields** (computed at query time):
- `duration_secs`: (end_time - start_time) / 1000
- `expected_batches`: duration_secs (one batch per second)

**Usage Context**:
```rust
// Analytics tool invocation
let query = TradeQuery {
    symbol: "BTCUSDT".to_string(),
    start_time: now_ms - (24 * 3600 * 1000), // 24 hours ago
    end_time: now_ms,
};

let trades = trade_storage.query_trades(&query).await?;
// Returns: Vec<AggTrade> with all trades in window
```

---

## Entity Relationships Diagram

```
┌─────────────────────────────────────────────────────┐
│                      Symbol                         │
│  ┌──────────────────────────────────────────────┐  │
│  │ name: String ("BTCUSDT", "ETHUSDT")          │  │
│  │ base_currency: "BTC", "ETH"                  │  │
│  │ quote_currency: "USDT"                       │  │
│  └──────────────────────────────────────────────┘  │
│                        │                            │
│                        │ has many                   │
│                        ▼                            │
│  ┌──────────────────────────────────────────────┐  │
│  │               TradeBatch                     │  │
│  │  ┌──────────────────────────────────────┐   │  │
│  │  │ symbol: String                       │   │  │
│  │  │ batch_timestamp: i64 (1-sec aligned) │   │  │
│  │  │ trades: Vec<AggTrade>                │   │  │
│  │  │ count: usize (derived)               │   │  │
│  │  └──────────────────────────────────────┘   │  │
│  │                    │                         │  │
│  │                    │ contains                │  │
│  │                    ▼                         │  │
│  │  ┌──────────────────────────────────────┐   │  │
│  │  │          AggTrade                    │   │  │
│  │  │  ┌──────────────────────────────┐   │   │  │
│  │  │  │ price: Decimal               │   │   │  │
│  │  │  │ quantity: Decimal            │   │   │  │
│  │  │  │ timestamp: i64               │   │   │  │
│  │  │  │ trade_id: i64                │   │   │  │
│  │  │  │ buyer_is_maker: bool         │   │   │  │
│  │  │  └──────────────────────────────┘   │   │  │
│  │  └──────────────────────────────────────┘   │  │
│  └──────────────────────────────────────────────┘  │
│                                                     │
│  Query via TradeQuery:                              │
│  ┌──────────────────────────────────────────────┐  │
│  │ symbol: "BTCUSDT"                            │  │
│  │ start_time: 1760900000000                    │  │
│  │ end_time:   1760903600000                    │  │
│  └──────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

---

## Storage Schema

### RocksDB Key-Value Store

**Key Format**: `trades:{symbol}:{batch_timestamp}`

**Examples**:
```
trades:BTCUSDT:1760903627000  →  MessagePack([trade1, trade2, ...])
trades:BTCUSDT:1760903628000  →  MessagePack([trade1, trade2, ...])
trades:ETHUSDT:1760903627000  →  MessagePack([trade1, trade2, ...])
```

**Key Properties**:
- **Lexicographic ordering**: Keys sorted naturally by symbol then timestamp
- **Prefix scan friendly**: `trades:BTCUSDT:` prefix returns all BTCUSDT batches
- **Timestamp precision**: Milliseconds (vs seconds for orderbook snapshots, prevents collisions)

**Value Format**: MessagePack-serialized `Vec<AggTrade>`

**MessagePack Schema** (binary format):
```
Array [
  Map {
    "p": String (price, e.g., "43250.50"),
    "q": String (quantity, e.g., "0.15"),
    "T": Integer (timestamp, e.g., 1760903627000),
    "a": Integer (trade_id, e.g., 26129),
    "m": Boolean (buyer_is_maker, e.g., true)
  },
  ...
]
```

**Size Estimates**:
- Single AggTrade: ~75 bytes (MessagePack encoded)
- TradeBatch (100 trades): ~500 bytes after MessagePack compression
- 1 hour of data (3600 batches): ~1.8 MB
- 7 days of data (604,800 batches × 2 symbols): ~600 MB

---

## Query Patterns

### Pattern 1: Time-Range Query (Primary Use Case)

**Use Case**: Analytics tools query trades for a specific time window (1-168 hours).

**Query Flow**:
```rust
// 1. Construct prefix key
let prefix = format!("trades:{}:", symbol);

// 2. Iterate over keys in range
let iter = db.prefix_iterator(prefix.as_bytes());
let mut all_trades = Vec::new();

for (key, value) in iter {
    // 3. Parse timestamp from key
    let timestamp = parse_timestamp_from_key(&key);

    // 4. Filter by time range
    if timestamp >= start_time && timestamp <= end_time {
        // 5. Deserialize batch
        let batch: Vec<AggTrade> = rmp_serde::from_slice(&value)?;
        all_trades.extend(batch);
    }

    // 6. Early termination if beyond end_time
    if timestamp > end_time {
        break;
    }
}
```

**Performance**:
- 1-hour query: ~3600 keys scanned, ~360K trades returned, <1s query time
- 24-hour query: ~86,400 keys scanned, ~8.6M trades returned, <3s query time (with optimization)

---

### Pattern 2: Retention Cleanup (Hourly Background Task)

**Use Case**: Delete trades older than 7 days to manage storage growth.

**Query Flow**:
```rust
// 1. Calculate cutoff timestamp
let cutoff = now_ms() - (7 * 24 * 3600 * 1000);

// 2. Iterate all trade keys
let iter = db.iterator(rocksdb::IteratorMode::Start);
let mut batch = WriteBatch::default();

for (key, _) in iter {
    // 3. Parse timestamp from key
    if key.starts_with(b"trades:") {
        let timestamp = parse_timestamp_from_key(&key);

        // 4. Mark for deletion if older than cutoff
        if timestamp < cutoff {
            batch.delete(&key);
        }
    }
}

// 5. Execute batch delete
db.write(batch)?;
```

**Performance**:
- Scans all keys once per hour (~1.2M keys for 7 days × 2 symbols)
- Deletes ~86,400 keys per day (1 day of data)
- Execution time: <10 seconds (acceptable for background task)

---

## Data Validation & Integrity

### Validation Rules Summary

1. **AggTrade Validation**:
   - `price` > 0 (reject negative or zero prices)
   - `quantity` > 0 (reject negative or zero quantities)
   - `timestamp` within reasonable range (2020-01-01 to 2030-12-31)
   - `trade_id` > 0 (Binance guarantees positive IDs)

2. **TradeBatch Validation**:
   - `batch_timestamp` aligned to 1000ms boundary
   - `trades` vector ordered by timestamp (ascending)
   - `symbol` uppercase ASCII

3. **TradeQuery Validation**:
   - `end_time` > `start_time`
   - `end_time` - `start_time` <= 7 days (max retention)
   - `start_time` >= now - 7 days (within retention period)

### Integrity Constraints

1. **Uniqueness**: Trade IDs are unique per symbol (Binance guarantee)
2. **Chronological Ordering**: Trades within a batch are timestamp-ordered (WebSocket guarantee)
3. **Referential Integrity**: All batches reference valid symbols (enforced at subscription time)

### Error Handling

1. **Malformed WebSocket Messages**: Log error, skip trade, continue processing
2. **Serialization Failures**: Log error, skip batch, retry on next interval
3. **Storage Failures**: Log error, continue collection (trades buffered in-memory until next flush)

---

## State Transitions

### AggTrade Lifecycle

```
╔════════════════════════════════════════════════════╗
║                   AggTrade Lifecycle               ║
╠════════════════════════════════════════════════════╣
║                                                    ║
║  [1] Received from WebSocket                       ║
║       ↓ Parse JSON message                         ║
║  [2] Validated (price > 0, quantity > 0, etc.)     ║
║       ↓ Push to in-memory buffer                   ║
║  [3] Buffered (Vec<AggTrade>)                      ║
║       ↓ 1-second interval tick                     ║
║  [4] Batched (TradeBatch created)                  ║
║       ↓ MessagePack serialization                  ║
║  [5] Persisted (RocksDB write)                     ║
║       ↓ Query by analytics tools                   ║
║  [6] Queried (Deserialized from storage)           ║
║       ↓ 7-day retention expires                    ║
║  [7] Deleted (Removed from RocksDB)                ║
║                                                    ║
╚════════════════════════════════════════════════════╝
```

---

## Migration & Backward Compatibility

**Migration**: Not applicable (new feature, no existing data)

**Backward Compatibility**:
- Existing orderbook snapshot storage (Feature 007) unaffected
- New trade storage uses separate key prefix (`trades:` vs `{symbol}:` for snapshots)
- Shared RocksDB instance, no schema conflicts

**Future Extensions**:
- Add more symbols: Extend `TRACKED_SYMBOLS` list, no schema changes
- Change batch interval: Requires new key format (`trades_v2:` prefix)
- Add fields to AggTrade: Backward compatible (MessagePack supports schema evolution)

---

## See Also

- [research.md](./research.md) - Technical decisions and benchmarking
- [contracts/aggtrade-websocket.md](./contracts/aggtrade-websocket.md) - WebSocket message schema
- [contracts/storage-api.md](./contracts/storage-api.md) - TradeStorage API specification
- [quickstart.md](./quickstart.md) - Local testing guide

---

**Data Model Complete** | Ready for contract definitions
