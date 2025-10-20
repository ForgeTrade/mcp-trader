# Contract: Binance aggTrade WebSocket

**Feature**: 008-trade-stream-persistence
**Type**: External API Integration
**Provider**: Binance

## Overview

This contract defines the WebSocket interface for receiving aggregate trade execution data from Binance. The aggTrade stream provides real-time trade events, aggregating multiple trades at the same price/time into single events.

## WebSocket Endpoint

**Base URL**: `wss://stream.binance.com/ws`
**Path Format**: `/{symbol}@aggTrade`
**Example**: `wss://stream.binance.com/ws/btcusdt@aggTrade`

**Notes**:
- Symbol must be lowercase (e.g., `btcusdt`, not `BTCUSDT`)
- No authentication required (public stream)
- Connection automatically closes after 24 hours (client must reconnect)
- Ping/pong keepalive recommended every 3 minutes

## Message Format

### Aggregate Trade Event

**Event Type**: `aggTrade`

**JSON Schema**:
```json
{
  "e": "aggTrade",         // Event type (string, always "aggTrade")
  "E": 1499405254326,      // Event time (integer, Unix milliseconds)
  "s": "BTCUSDT",          // Symbol (string, uppercase)
  "a": 26129,              // Aggregate trade ID (integer, unique per symbol)
  "p": "0.01633102",       // Price (string, decimal representation)
  "q": "4.70443515",       // Quantity (string, decimal representation)
  "f": 27781,              // First trade ID (integer)
  "l": 27781,              // Last trade ID (integer)
  "T": 1499405254324,      // Trade time (integer, Unix milliseconds)
  "m": true,               // Is buyer maker? (boolean)
  "M": true                // Ignore (deprecated, always true)
}
```

### Field Descriptions

| Field | Type | Description | Validation | Example |
|-------|------|-------------|------------|---------|
| `e` | string | Event type identifier | Must equal "aggTrade" | "aggTrade" |
| `E` | integer | Event timestamp (when Binance generated event) | > 0, Unix milliseconds | 1499405254326 |
| `s` | string | Trading pair symbol | Uppercase, 6-12 chars | "BTCUSDT" |
| `a` | integer | Aggregate trade ID (unique identifier) | > 0, monotonically increasing | 26129 |
| `p` | string | Execution price in quote currency | Numeric string, > 0 | "0.01633102" |
| `q` | string | Trade quantity in base currency | Numeric string, > 0 | "4.70443515" |
| `f` | integer | First individual trade ID in aggregation | > 0 | 27781 |
| `l` | integer | Last individual trade ID in aggregation | >= f | 27781 |
| `T` | integer | Trade execution timestamp | > 0, Unix milliseconds | 1499405254324 |
| `m` | boolean | Buyer is maker flag | true or false | true |
| `M` | boolean | Deprecated field (ignore) | Always true | true |

### Buyer Is Maker Flag (`m`)

**Interpretation**:
- `m = true`: Buyer placed limit order (passive), seller was market taker (aggressive)
  - **Direction**: Buy-side limit order filled (someone sold into bid)
  - **Market sentiment**: Selling pressure
- `m = false`: Seller placed limit order (passive), buyer was market taker (aggressive)
  - **Direction**: Sell-side limit order filled (someone bought from ask)
  - **Market sentiment**: Buying pressure

**Usage in Volume Profile**:
- Count `m = false` trades as "buy volume" (aggressive buying)
- Count `m = true` trades as "sell volume" (aggressive selling)

## Connection Lifecycle

### Establishment

```
Client                                 Binance
  |                                       |
  |--- WebSocket Handshake ------------->|
  |<-- 101 Switching Protocols ----------|
  |                                       |
  |<-- aggTrade messages (streaming) ----|
  |<-- aggTrade messages (streaming) ----|
  |                                       |
```

### Keepalive (Recommended)

```
Client                                 Binance
  |                                       |
  |--- Ping frame ----------------------->|
  |<-- Pong frame ------------------------|
  |                                       |
  (repeat every 3 minutes)
```

**Note**: WebSocket library (tokio-tungstenite) handles ping/pong automatically.

### Disconnection Scenarios

| Scenario | Client Action | Binance Behavior |
|----------|---------------|------------------|
| Normal close | Send close frame | Acknowledge and close |
| 24-hour limit | Reconnect automatically | Close after 24 hours |
| Network failure | Detect timeout, reconnect | Close connection |
| Invalid symbol | Handle error, log | Close with error code |

## Error Handling

### WebSocket Error Codes

| Code | Description | Client Action |
|------|-------------|---------------|
| 1000 | Normal closure | Reconnect after delay |
| 1001 | Going away (server shutdown) | Reconnect with backoff |
| 1006 | Abnormal closure (network issue) | Reconnect with backoff |
| 1008 | Policy violation (invalid symbol) | Log error, do not retry |
| 1011 | Server error | Reconnect with backoff |

### Malformed Message Handling

**Strategy**: Skip malformed message, log error, continue processing stream

**Example Malformed Messages**:
1. Missing required field (e.g., no `p` price field)
2. Invalid JSON structure
3. Type mismatch (e.g., `p` is number instead of string)

**Client Behavior**:
```rust
match serde_json::from_str::<AggTradeMessage>(&msg) {
    Ok(trade) => process_trade(trade),
    Err(e) => {
        tracing::error!(
            symbol = %symbol,
            error = %e,
            raw_message = %msg,
            "Failed to parse aggTrade message, skipping"
        );
        // Continue to next message
    }
}
```

## Performance Characteristics

### Message Rate

| Symbol | Normal Market | Volatile Market | Overnight |
|--------|---------------|-----------------|-----------|
| BTCUSDT | 1-5 trades/sec | 10-50 trades/sec | 0.1-1 trades/sec |
| ETHUSDT | 1-5 trades/sec | 10-50 trades/sec | 0.1-1 trades/sec |

**Peak Rate**: Up to 100 trades/sec during extreme volatility (flash crashes, major news)

### Message Size

- **Typical**: ~250 bytes per message (JSON)
- **Bandwidth**: ~1-5 KB/sec per symbol (normal market)
- **Peak**: ~25 KB/sec per symbol (volatile market)

### Latency

- **Event to delivery**: <100ms typical (Binance to client)
- **Network variance**: ±50ms depending on geographic location

## Example Messages

### Normal Trade (Buyer is Maker)

```json
{
  "e": "aggTrade",
  "E": 1760903627000,
  "s": "BTCUSDT",
  "a": 123456789,
  "p": "43250.50",
  "q": "0.15",
  "f": 987654321,
  "l": 987654321,
  "T": 1760903627000,
  "m": true,
  "M": true
}
```

**Interpretation**: 0.15 BTC sold at $43,250.50 (aggressive sell into buy-side limit order)

### Normal Trade (Seller is Maker)

```json
{
  "e": "aggTrade",
  "E": 1760903628000,
  "s": "BTCUSDT",
  "a": 123456790,
  "p": "43251.00",
  "q": "0.25",
  "f": 987654322,
  "l": 987654322,
  "T": 1760903628000,
  "m": false,
  "M": true
}
```

**Interpretation**: 0.25 BTC bought at $43,251.00 (aggressive buy lifting sell-side limit order)

### Aggregated Trade (Multiple Individual Trades)

```json
{
  "e": "aggTrade",
  "E": 1760903629000,
  "s": "BTCUSDT",
  "a": 123456791,
  "p": "43252.00",
  "q": "1.50",
  "f": 987654323,
  "l": 987654327,
  "T": 1760903629000,
  "m": false,
  "M": true
}
```

**Interpretation**: 5 individual trades (IDs 987654323-987654327) at same price/time, aggregated into 1.50 BTC total

## Mapping to Internal AggTrade Struct

```rust
// src/orderbook/analytics/trade_stream.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggTrade {
    #[serde(rename = "p")]
    pub price: Decimal,          // Parse from string: Decimal::from_str(msg.p)?

    #[serde(rename = "q")]
    pub quantity: Decimal,       // Parse from string: Decimal::from_str(msg.q)?

    #[serde(rename = "T")]
    pub timestamp: i64,          // Direct mapping: msg.T

    #[serde(rename = "a")]
    pub trade_id: i64,           // Direct mapping: msg.a

    #[serde(rename = "m")]
    pub buyer_is_maker: bool,    // Direct mapping: msg.m
}

// Fields NOT mapped (unused):
// - E (event time): Use T (trade time) instead
// - s (symbol): Known from WebSocket subscription
// - f, l (first/last trade IDs): Not needed for analytics
// - M (deprecated): Ignore
```

## Testing Considerations

### Mock WebSocket Server (for unit tests)

```rust
// Test helper for simulating Binance aggTrade stream
async fn mock_aggtrade_server() -> MockServer {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/ws/btcusdt@aggTrade"))
        .respond_with(ResponseTemplate::new(101)
            .insert_header("Upgrade", "websocket")
            .set_body_json(json!({
                "e": "aggTrade",
                "E": 1760903627000,
                "s": "BTCUSDT",
                "a": 123456,
                "p": "43250.50",
                "q": "0.15",
                "f": 123,
                "l": 123,
                "T": 1760903627000,
                "m": true,
                "M": true
            })))
        .mount(&server)
        .await;

    server
}
```

### Integration Test Checklist

- [ ] Connect to real Binance endpoint (testnet or production)
- [ ] Receive at least 10 consecutive messages without errors
- [ ] Verify message format matches schema
- [ ] Verify prices and quantities parse to Decimal correctly
- [ ] Verify timestamp ordering (chronologically increasing)
- [ ] Verify trade_id monotonically increases
- [ ] Test reconnection after 24-hour disconnect
- [ ] Test reconnection after network failure simulation

## References

- **Official Documentation**: https://binance-docs.github.io/apidocs/spot/en/#aggregate-trade-streams
- **WebSocket API Overview**: https://binance-docs.github.io/apidocs/spot/en/#websocket-market-streams
- **Rate Limits**: https://binance-docs.github.io/apidocs/spot/en/#limits

---

**Contract Version**: 1.0
**Last Updated**: 2025-10-19
**Review Date**: 2026-10-19 (annual)
