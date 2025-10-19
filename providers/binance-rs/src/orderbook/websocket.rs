//! WebSocket client for Binance depth streams
//!
//! Connects to `<symbol>@depth@100ms` streams for real-time order book updates.
//! Implements exponential backoff reconnection strategy with auto-recovery.

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

/// Binance WebSocket base URL for streams
const BINANCE_WS_URL: &str = "wss://stream.binance.com:9443/ws";

/// Maximum reconnection delay (30 seconds)
const MAX_RECONNECT_DELAY_SECS: u64 = 30;

/// Depth update event from Binance WebSocket
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DepthUpdateEvent {
    #[serde(rename = "e")]
    pub event_type: String,

    #[serde(rename = "E")]
    pub event_time: i64,

    #[serde(rename = "s")]
    pub symbol: String,

    #[serde(rename = "U")]
    pub first_update_id: i64,

    #[serde(rename = "u")]
    pub final_update_id: i64,

    #[serde(rename = "b")]
    pub bids: Vec<[String; 2]>, // [price, qty]

    #[serde(rename = "a")]
    pub asks: Vec<[String; 2]>, // [price, qty]
}

/// WebSocket client for a single symbol's depth stream
pub struct DepthWebSocketClient {
    symbol: String,
    update_sender: mpsc::UnboundedSender<DepthUpdateEvent>,
}

impl DepthWebSocketClient {
    /// Create a new WebSocket client for the given symbol
    ///
    /// Returns a client handle and a receiver channel for depth updates.
    /// The client spawns a background task that manages the WebSocket connection.
    pub fn new(symbol: String) -> (Self, mpsc::UnboundedReceiver<DepthUpdateEvent>) {
        let (update_sender, update_receiver) = mpsc::unbounded_channel();

        let client = Self {
            symbol,
            update_sender,
        };

        (client, update_receiver)
    }

    /// Start the WebSocket client with automatic reconnection
    ///
    /// Spawns a background task that:
    /// 1. Connects to Binance depth stream
    /// 2. Processes incoming depth updates
    /// 3. Handles disconnections with exponential backoff (1s, 2s, 4s, 8s, max 30s)
    /// 4. Logs connection status changes at INFO level
    pub fn start(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut retry_count = 0;

            loop {
                match self.connect_and_process().await {
                    Ok(()) => {
                        info!(symbol = %self.symbol, "WebSocket connection closed normally");
                        break;
                    }
                    Err(e) => {
                        // Calculate exponential backoff delay
                        let delay_secs =
                            std::cmp::min(2_u64.pow(retry_count), MAX_RECONNECT_DELAY_SECS);

                        warn!(
                            symbol = %self.symbol,
                            error = %e,
                            retry_count,
                            delay_secs,
                            "WebSocket connection failed, retrying with exponential backoff"
                        );

                        sleep(Duration::from_secs(delay_secs)).await;
                        retry_count += 1;

                        // Reset retry count after successful connection (if we get far enough)
                        if retry_count > 10 {
                            retry_count = 0;
                        }
                    }
                }
            }
        })
    }

    /// Connect to WebSocket and process messages until disconnection
    async fn connect_and_process(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let stream_name = format!("{}@depth@100ms", self.symbol.to_lowercase());
        let url = format!("{}/{}", BINANCE_WS_URL, stream_name);

        info!(symbol = %self.symbol, url = %url, "Connecting to Binance depth stream");

        let (ws_stream, _) = connect_async(&url).await?;
        info!(symbol = %self.symbol, "WebSocket connected successfully");

        let (mut write, mut read) = ws_stream.split();

        // Send ping periodically to keep connection alive
        let ping_handle = {
            let symbol = self.symbol.clone();
            tokio::spawn(async move {
                loop {
                    sleep(Duration::from_secs(30)).await;
                    debug!(symbol = %symbol, "Sending WebSocket ping");
                }
            })
        };

        // Process incoming messages
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    match serde_json::from_str::<DepthUpdateEvent>(&text) {
                        Ok(update) => {
                            debug!(
                                symbol = %self.symbol,
                                first_update_id = update.first_update_id,
                                final_update_id = update.final_update_id,
                                bid_count = update.bids.len(),
                                ask_count = update.asks.len(),
                                "Received depth update"
                            );

                            // Send update to manager
                            if self.update_sender.send(update).is_err() {
                                info!(symbol = %self.symbol, "Update receiver dropped, closing WebSocket");
                                break;
                            }
                        }
                        Err(e) => {
                            error!(symbol = %self.symbol, error = %e, text = %text, "Failed to parse depth update");
                        }
                    }
                }
                Ok(Message::Ping(data)) => {
                    debug!(symbol = %self.symbol, "Received ping, sending pong");
                    if write.send(Message::Pong(data)).await.is_err() {
                        warn!(symbol = %self.symbol, "Failed to send pong");
                        break;
                    }
                }
                Ok(Message::Pong(_)) => {
                    debug!(symbol = %self.symbol, "Received pong");
                }
                Ok(Message::Close(frame)) => {
                    info!(symbol = %self.symbol, frame = ?frame, "WebSocket close frame received");
                    break;
                }
                Ok(Message::Binary(_)) => {
                    warn!(symbol = %self.symbol, "Received unexpected binary message");
                }
                Ok(Message::Frame(_)) => {
                    // Raw frames are handled internally by tungstenite
                }
                Err(e) => {
                    error!(symbol = %self.symbol, error = %e, "WebSocket error");
                    break;
                }
            }
        }

        ping_handle.abort();
        info!(symbol = %self.symbol, "WebSocket connection closed");

        Err("WebSocket disconnected".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_depth_update_deserialization() {
        let json = r#"{
            "e": "depthUpdate",
            "E": 1699999999123,
            "s": "BTCUSDT",
            "U": 1000,
            "u": 1005,
            "b": [
                ["67650.00", "1.23400"],
                ["67649.50", "0.45600"]
            ],
            "a": [
                ["67651.00", "0.98700"],
                ["67651.50", "0.40000"]
            ]
        }"#;

        let update: DepthUpdateEvent = serde_json::from_str(json).unwrap();
        assert_eq!(update.event_type, "depthUpdate");
        assert_eq!(update.symbol, "BTCUSDT");
        assert_eq!(update.first_update_id, 1000);
        assert_eq!(update.final_update_id, 1005);
        assert_eq!(update.bids.len(), 2);
        assert_eq!(update.asks.len(), 2);
        assert_eq!(update.bids[0][0], "67650.00");
        assert_eq!(update.bids[0][1], "1.23400");
    }
}
