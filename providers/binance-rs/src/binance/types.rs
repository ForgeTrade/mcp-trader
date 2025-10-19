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

/// Balance information for an asset
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Balance {
    /// Asset symbol (e.g., "BTC", "USDT")
    pub asset: String,
    /// Available balance
    pub free: String,
    /// Locked balance (in orders)
    pub locked: String,
}

/// Response from /api/v3/account endpoint
///
/// Returns account information including balances and permissions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountInfo {
    /// Maker commission rate
    pub maker_commission: i64,
    /// Taker commission rate
    pub taker_commission: i64,
    /// Buyer commission rate
    pub buyer_commission: i64,
    /// Seller commission rate
    pub seller_commission: i64,
    /// Can trade
    pub can_trade: bool,
    /// Can withdraw
    pub can_withdraw: bool,
    /// Can deposit
    pub can_deposit: bool,
    /// Account update time
    pub update_time: i64,
    /// Account type (e.g., "SPOT")
    pub account_type: String,
    /// List of asset balances
    pub balances: Vec<Balance>,
    /// Account permissions
    pub permissions: Vec<String>,
}

/// Order fill information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Fill {
    /// Price of the fill
    pub price: String,
    /// Quantity of the fill
    pub qty: String,
    /// Commission paid for this fill
    pub commission: String,
    /// Commission asset
    pub commission_asset: String,
}

/// Response from order creation/query endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    /// Order symbol
    pub symbol: String,
    /// Order ID
    pub order_id: i64,
    /// Order list ID (for OCO orders)
    #[serde(default)]
    pub order_list_id: i64,
    /// Client order ID
    pub client_order_id: String,
    /// Transaction time
    #[serde(default)]
    pub transact_time: i64,
    /// Order price
    #[serde(default)]
    pub price: String,
    /// Original quantity
    #[serde(default)]
    pub orig_qty: String,
    /// Executed quantity
    #[serde(default)]
    pub executed_qty: String,
    /// Cumulative quote quantity
    #[serde(default)]
    pub cummulative_quote_qty: String,
    /// Order status (NEW, PARTIALLY_FILLED, FILLED, CANCELED, etc.)
    pub status: String,
    /// Time in force (GTC, IOC, FOK)
    #[serde(default)]
    pub time_in_force: String,
    /// Order type (LIMIT, MARKET, STOP_LOSS, etc.)
    #[serde(rename = "type")]
    pub order_type: String,
    /// Order side (BUY, SELL)
    pub side: String,
    /// List of fills (for market orders)
    #[serde(default)]
    pub fills: Vec<Fill>,
}

/// Response from /api/v3/myTrades endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MyTrade {
    /// Trade symbol
    pub symbol: String,
    /// Trade ID
    pub id: i64,
    /// Order ID
    pub order_id: i64,
    /// Order list ID
    #[serde(default)]
    pub order_list_id: i64,
    /// Price
    pub price: String,
    /// Quantity
    pub qty: String,
    /// Quote quantity
    pub quote_qty: String,
    /// Commission
    pub commission: String,
    /// Commission asset
    pub commission_asset: String,
    /// Trade time
    pub time: i64,
    /// Is buyer
    pub is_buyer: bool,
    /// Is maker
    pub is_maker: bool,
    /// Is best match
    pub is_best_match: bool,
}
