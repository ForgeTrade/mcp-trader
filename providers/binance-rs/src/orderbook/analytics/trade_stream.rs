//! Binance @aggTrade WebSocket stream for volume profile data
//!
//! Connects to wss://stream.binance.com:9443/ws/<symbol>@aggTrade for real-time
//! aggregated trade data. Supports exponential backoff reconnection (1s, 2s, 4s, 8s, max 60s).

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

/// Binance aggregated trade event from @aggTrade stream
///
/// Example JSON:
/// ```json
/// {
///   "e": "aggTrade",
///   "E": 1672531200000,
///   "s": "BTCUSDT",
///   "a": 12345,
///   "p": "16800.50",
///   "q": "1.25",
///   "f": 100,
///   "l": 105,
///   "T": 1672531199999,
///   "m": true,
///   "M": true
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggTrade {
    /// Event type (always "aggTrade")
    #[serde(rename = "e")]
    pub event_type: String,

    /// Event timestamp (Unix milliseconds)
    #[serde(rename = "E")]
    pub event_time: i64,

    /// Symbol (e.g., "BTCUSDT")
    #[serde(rename = "s")]
    pub symbol: String,

    /// Aggregate trade ID
    #[serde(rename = "a")]
    pub agg_trade_id: u64,

    /// Price (as string to preserve precision)
    #[serde(rename = "p")]
    pub price: String,

    /// Quantity (as string to preserve precision)
    #[serde(rename = "q")]
    pub quantity: String,

    /// First trade ID
    #[serde(rename = "f")]
    pub first_trade_id: u64,

    /// Last trade ID
    #[serde(rename = "l")]
    pub last_trade_id: u64,

    /// Trade timestamp (Unix milliseconds)
    #[serde(rename = "T")]
    pub trade_time: i64,

    /// Is buyer the market maker? (true = sell, false = buy)
    #[serde(rename = "m")]
    pub is_buyer_maker: bool,

    /// Was trade the best price match?
    #[serde(rename = "M")]
    pub is_best_match: bool,
}

/// Trade stream handler for volume profile collection
pub struct TradeStreamHandler {
    symbol: String,
    url: String,
    trade_buffer: Vec<AggTrade>,
}

impl TradeStreamHandler {
    /// Create new trade stream handler for symbol
    ///
    /// # Arguments
    /// * `symbol` - Trading pair (e.g., "BTCUSDT")
    ///
    /// # Example
    /// ```no_run
    /// let handler = TradeStreamHandler::new("BTCUSDT");
    /// handler.connect_with_backoff().await?;
    /// ```
    pub fn new(symbol: &str) -> Self {
        let symbol_lower = symbol.to_lowercase();
        let url = format!("wss://stream.binance.com:9443/ws/{}@aggTrade", symbol_lower);

        Self {
            symbol: symbol.to_uppercase(),
            url,
            trade_buffer: Vec::new(),
        }
    }

    /// Connect to WebSocket stream with exponential backoff
    ///
    /// Retry delays: 1s, 2s, 4s, 8s, 16s, 32s, capped at 60s max
    ///
    /// # Returns
    /// Never returns (runs until fatal error or task cancellation)
    pub async fn connect_with_backoff(
        &mut self,
        trade_tx: tokio::sync::mpsc::UnboundedSender<AggTrade>,
    ) -> Result<()> {
        let mut retry_delay = Duration::from_secs(1);
        let max_delay = Duration::from_secs(60);

        loop {
            match self.connect_once(&trade_tx).await {
                Ok(_) => {
                    // Connection closed gracefully, reset backoff
                    retry_delay = Duration::from_secs(1);
                    info!(symbol = %self.symbol, "WebSocket stream closed, reconnecting...");
                }
                Err(e) => {
                    error!(
                        symbol = %self.symbol,
                        error = %e,
                        retry_delay_secs = retry_delay.as_secs(),
                        "WebSocket connection failed, retrying..."
                    );
                }
            }

            sleep(retry_delay).await;

            // Exponential backoff: double delay up to max
            retry_delay = std::cmp::min(retry_delay * 2, max_delay);
        }
    }

    /// Single WebSocket connection attempt
    ///
    /// # Errors
    /// - Connection failure
    /// - JSON parse error
    /// - Channel send error
    async fn connect_once(
        &mut self,
        trade_tx: &tokio::sync::mpsc::UnboundedSender<AggTrade>,
    ) -> Result<()> {
        info!(symbol = %self.symbol, url = %self.url, "Connecting to @aggTrade stream...");

        let (ws_stream, _) = connect_async(&self.url)
            .await
            .context("Failed to connect to Binance WebSocket")?;

        info!(symbol = %self.symbol, "WebSocket connected successfully");

        let (mut _write, mut read) = ws_stream.split();

        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    match serde_json::from_str::<AggTrade>(&text) {
                        Ok(trade) => {
                            debug!(
                                symbol = %trade.symbol,
                                price = %trade.price,
                                quantity = %trade.quantity,
                                "Received aggTrade event"
                            );

                            // Send to processing channel
                            if let Err(e) = trade_tx.send(trade) {
                                error!(error = %e, "Failed to send trade to channel");
                                return Err(e.into());
                            }
                        }
                        Err(e) => {
                            warn!(error = %e, text = %text, "Failed to parse aggTrade event");
                        }
                    }
                }
                Ok(Message::Ping(data)) => {
                    debug!("Received ping, responding with pong");
                    // Auto-handled by tokio-tungstenite
                    let _ = data;
                }
                Ok(Message::Close(frame)) => {
                    info!(
                        symbol = %self.symbol,
                        reason = ?frame,
                        "WebSocket closed by server"
                    );
                    break;
                }
                Err(e) => {
                    error!(symbol = %self.symbol, error = %e, "WebSocket error");
                    return Err(e.into());
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Get buffered trades for aggregation
    pub fn drain_buffer(&mut self) -> Vec<AggTrade> {
        std::mem::take(&mut self.trade_buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trade_stream_handler_new() {
        let handler = TradeStreamHandler::new("BTCUSDT");
        assert_eq!(handler.symbol, "BTCUSDT");
        assert!(handler.url.contains("btcusdt@aggTrade"));
    }

    #[test]
    fn test_aggtrade_deserialization() {
        let json = r#"{
            "e": "aggTrade",
            "E": 1672531200000,
            "s": "BTCUSDT",
            "a": 12345,
            "p": "16800.50",
            "q": "1.25",
            "f": 100,
            "l": 105,
            "T": 1672531199999,
            "m": true,
            "M": true
        }"#;

        let trade: AggTrade = serde_json::from_str(json).unwrap();
        assert_eq!(trade.symbol, "BTCUSDT");
        assert_eq!(trade.price, "16800.50");
        assert_eq!(trade.quantity, "1.25");
        assert_eq!(trade.is_buyer_maker, true);
    }
}
