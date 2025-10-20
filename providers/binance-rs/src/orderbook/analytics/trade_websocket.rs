// WebSocket client for Binance aggTrade stream
// Handles connection lifecycle, reconnection, and message parsing

use anyhow::Result;
use std::time::Duration;

pub struct TradeStreamClient {
    symbol: String,
    websocket_url: String,
    reconnect_delay: Duration,
}

impl TradeStreamClient {
    pub fn new(symbol: &str) -> Self {
        let symbol_lower = symbol.to_lowercase();
        let websocket_url = format!("wss://stream.binance.com/ws/{}@aggTrade", symbol_lower);

        Self {
            symbol: symbol.to_string(),
            websocket_url,
            reconnect_delay: Duration::from_secs(1),
        }
    }
}
