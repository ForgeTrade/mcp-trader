//! Binance API Type Definitions
//!
//! Type definitions for Binance API responses and requests.
//! All types include validation and proper deserialization.

use serde::{Deserialize, Serialize};

/// Response from Binance /api/v3/time endpoint
///
/// Returns the current server time in milliseconds since Unix epoch.
/// Used for time synchronization and validating request signatures.
///
/// # Example Response
/// ```json
/// {
///   "serverTime": 1699564800000
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerTimeResponse {
    /// Server time in milliseconds since Unix epoch
    ///
    /// Must be a positive i64 value. Typical range: 1600000000000 to 2000000000000
    pub server_time: i64,
}

impl ServerTimeResponse {
    /// Validates the server time is within reasonable bounds
    ///
    /// Returns true if server_time is positive (after Unix epoch).
    /// This prevents issues with negative timestamps or zero values.
    pub fn is_valid(&self) -> bool {
        self.server_time > 0
    }

    /// Returns the server time as milliseconds since Unix epoch
    pub fn time_ms(&self) -> i64 {
        self.server_time
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_time_deserialization() {
        let json = r#"{"serverTime": 1699564800000}"#;
        let response: ServerTimeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.server_time, 1699564800000);
        assert!(response.is_valid());
    }

    #[test]
    fn test_invalid_server_time() {
        let response = ServerTimeResponse { server_time: -1 };
        assert!(!response.is_valid());
    }

    #[test]
    fn test_zero_server_time() {
        let response = ServerTimeResponse { server_time: 0 };
        assert!(!response.is_valid());
    }
}

/// Response from /api/v3/ticker/price endpoint
///
/// Returns the latest price for a symbol or all symbols.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickerPrice {
    /// Trading pair symbol (e.g., "BTCUSDT")
    pub symbol: String,
    /// Current price as string to preserve precision
    pub price: String,
}

/// Response from /api/v3/ticker/24hr endpoint
///
/// Returns 24-hour rolling window price statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ticker24hr {
    /// Trading pair symbol
    pub symbol: String,
    /// Price change
    pub price_change: String,
    /// Price change percent
    pub price_change_percent: String,
    /// Weighted average price
    pub weighted_avg_price: String,
    /// Previous close price
    pub prev_close_price: String,
    /// Last price
    pub last_price: String,
    /// Last quantity
    pub last_qty: String,
    /// Best bid price
    pub bid_price: String,
    /// Best ask price
    pub ask_price: String,
    /// Open price
    pub open_price: String,
    /// High price
    pub high_price: String,
    /// Low price
    pub low_price: String,
    /// Total traded base asset volume
    pub volume: String,
    /// Total traded quote asset volume
    pub quote_volume: String,
    /// Open time
    pub open_time: i64,
    /// Close time
    pub close_time: i64,
    /// First trade ID
    pub first_id: i64,
    /// Last trade ID
    pub last_id: i64,
    /// Total number of trades
    pub count: i64,
}

/// Response from /api/v3/klines endpoint
///
/// Returns candlestick/kline data.
/// Array format: [open_time, open, high, low, close, volume, close_time, quote_volume, trades, taker_buy_base, taker_buy_quote, ignore]
pub type KlineData = Vec<serde_json::Value>;

/// Response from /api/v3/depth endpoint
///
/// Returns order book depth.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderBook {
    /// Last update ID
    pub last_update_id: i64,
    /// Bid levels [price, quantity]
    pub bids: Vec<(String, String)>,
    /// Ask levels [price, quantity]
    pub asks: Vec<(String, String)>,
}

/// Response from /api/v3/trades endpoint
///
/// Returns recent trades list.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Trade {
    /// Trade ID
    pub id: i64,
    /// Price
    pub price: String,
    /// Quantity
    pub qty: String,
    /// Quote quantity
    pub quote_qty: String,
    /// Trade time
    pub time: i64,
    /// Was the buyer the maker?
    pub is_buyer_maker: bool,
    /// Was the trade the best price match?
    pub is_best_match: bool,
}

// Phase 7: Order management types removed per FR-001
// Removed: Balance, AccountInfo, Fill, Order, MyTrade structs
// This system is now read-only market data analysis only
