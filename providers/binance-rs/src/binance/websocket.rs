//! Binance WebSocket Client
//!
//! Connects to Binance WebSocket streams for real-time market data.
//! Handles automatic reconnection with exponential backoff and message broadcasting.
//!
//! ## Features
//! - Ticker price streams (real-time price updates)
//! - Order book depth streams (bid/ask updates)
//! - User data streams (order/balance notifications)
//! - Automatic reconnection with exponential backoff (100ms → 30s)
//! - Message broadcasting via tokio::sync::broadcast channels

use crate::error::McpError;
use futures_util::StreamExt;
use serde::Deserialize;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

/// Base URL for Binance WebSocket streams
const BINANCE_WS_URL: &str = "wss://stream.binance.com:9443/ws";

/// Maximum reconnection backoff duration
const MAX_BACKOFF: Duration = Duration::from_secs(30);

/// Initial reconnection backoff duration
const INITIAL_BACKOFF: Duration = Duration::from_millis(100);

/// Binance WebSocket client for managing stream connections
///
/// Handles connections to Binance WebSocket API with automatic
/// reconnection and message broadcasting to multiple subscribers.
#[derive(Debug, Clone)]
pub struct BinanceWebSocketClient {
    /// Base WebSocket URL
    pub base_url: String,
}

impl BinanceWebSocketClient {
    /// Create a new Binance WebSocket client with default URL
    pub fn new() -> Self {
        Self {
            base_url: BINANCE_WS_URL.to_string(),
        }
    }

    /// Connect to a WebSocket stream with automatic retry and exponential backoff
    ///
    /// Retries connection failures with exponential backoff starting at 100ms
    /// and capping at 30 seconds between attempts.
    ///
    /// ## Arguments
    /// - `stream_name`: The Binance stream endpoint (e.g., "btcusdt@ticker", "btcusdt@depth")
    ///
    /// ## Returns
    /// WebSocket connection (write, read) split tuple
    ///
    /// ## Example
    /// ```rust,no_run
    /// use mcp_binance_server::binance::websocket::BinanceWebSocketClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = BinanceWebSocketClient::new();
    /// let (_write, _read) = client.connect_with_retry("btcusdt@ticker").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect_with_retry(
        &self,
        stream_name: &str,
    ) -> Result<
        (
            futures_util::stream::SplitSink<
                tokio_tungstenite::WebSocketStream<
                    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
                >,
                Message,
            >,
            futures_util::stream::SplitStream<
                tokio_tungstenite::WebSocketStream<
                    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
                >,
            >,
        ),
        McpError,
    > {
        let url = format!("{}/{}", self.base_url, stream_name);
        let mut backoff = INITIAL_BACKOFF;

        loop {
            tracing::info!("Connecting to Binance WebSocket: {}", url);

            match connect_async(&url).await {
                Ok((ws_stream, _)) => {
                    tracing::info!("Connected to Binance WebSocket: {}", stream_name);
                    let (write, read) = ws_stream.split();
                    return Ok((write, read));
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to connect to {}: {}. Retrying in {:?}",
                        stream_name,
                        e,
                        backoff
                    );

                    sleep(backoff).await;

                    // Exponential backoff with cap
                    backoff = std::cmp::min(backoff * 2, MAX_BACKOFF);
                }
            }
        }
    }

    /// Start a ticker stream task that reads from Binance and broadcasts to subscribers
    ///
    /// Creates a background task that:
    /// 1. Connects to Binance ticker WebSocket stream
    /// 2. Reads ticker update messages
    /// 3. Broadcasts messages to all subscribers via broadcast channel
    /// 4. Automatically reconnects on connection loss
    ///
    /// ## Arguments
    /// - `symbol`: Trading pair symbol in lowercase (e.g., "btcusdt")
    /// - `tx`: Broadcast sender for distributing ticker updates to subscribers
    ///
    /// ## Returns
    /// Task handle that can be awaited or spawned
    ///
    /// ## Example
    /// ```rust,no_run
    /// use mcp_binance_server::binance::websocket::BinanceWebSocketClient;
    /// use tokio::sync::broadcast;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = BinanceWebSocketClient::new();
    /// let (tx, _rx) = broadcast::channel(100);
    ///
    /// // Spawn task to run in background
    /// tokio::spawn(async move {
    ///     if let Err(e) = client.ticker_stream_task("btcusdt", tx).await {
    ///         eprintln!("Ticker stream error: {}", e);
    ///     }
    /// });
    /// # Ok(())
    /// # }
    /// ```
    pub async fn ticker_stream_task(
        &self,
        symbol: &str,
        tx: broadcast::Sender<TickerUpdate>,
    ) -> Result<(), McpError> {
        let stream_name = format!("{}@ticker", symbol.to_lowercase());

        loop {
            tracing::info!("Starting ticker stream for {}", symbol);

            // Connect with retry
            let (_write, mut read) = self.connect_with_retry(&stream_name).await?;

            // Read messages and broadcast to subscribers
            while let Some(msg_result) = read.next().await {
                match msg_result {
                    Ok(Message::Text(text)) => {
                        // Parse ticker update
                        match serde_json::from_str::<TickerUpdate>(&text) {
                            Ok(update) => {
                                // Broadcast to all subscribers
                                // Ignore send errors (no active receivers)
                                let _ = tx.send(update);
                            }
                            Err(e) => {
                                tracing::warn!("Failed to parse ticker update: {}", e);
                            }
                        }
                    }
                    Ok(Message::Ping(data)) => {
                        tracing::debug!("Received ping with {} bytes", data.len());
                    }
                    Ok(Message::Pong(_)) => {
                        tracing::debug!("Received pong");
                    }
                    Ok(Message::Close(frame)) => {
                        tracing::info!("WebSocket closed: {:?}", frame);
                        break;
                    }
                    Err(e) => {
                        tracing::error!("WebSocket read error: {}", e);
                        break;
                    }
                    _ => {
                        tracing::debug!("Received other message type");
                    }
                }
            }

            tracing::warn!("Ticker stream disconnected, reconnecting...");
            sleep(Duration::from_secs(1)).await;
        }
    }

    /// Start a depth stream task that reads from Binance and broadcasts to subscribers
    ///
    /// Creates a background task that:
    /// 1. Connects to Binance depth WebSocket stream
    /// 2. Reads order book depth update messages
    /// 3. Broadcasts messages to all subscribers via broadcast channel
    /// 4. Automatically reconnects on connection loss
    ///
    /// ## Arguments
    /// - `symbol`: Trading pair symbol in lowercase (e.g., "btcusdt")
    /// - `tx`: Broadcast sender for distributing depth updates to subscribers
    ///
    /// ## Returns
    /// Task handle that can be awaited or spawned
    ///
    /// ## Example
    /// ```rust,no_run
    /// use mcp_binance_server::binance::websocket::BinanceWebSocketClient;
    /// use tokio::sync::broadcast;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = BinanceWebSocketClient::new();
    /// let (tx, _rx) = broadcast::channel(100);
    ///
    /// // Spawn task to run in background
    /// tokio::spawn(async move {
    ///     if let Err(e) = client.depth_stream_task("btcusdt", tx).await {
    ///         eprintln!("Depth stream error: {}", e);
    ///     }
    /// });
    /// # Ok(())
    /// # }
    /// ```
    pub async fn depth_stream_task(
        &self,
        symbol: &str,
        tx: broadcast::Sender<DepthUpdate>,
    ) -> Result<(), McpError> {
        let stream_name = format!("{}@depth", symbol.to_lowercase());

        loop {
            tracing::info!("Starting depth stream for {}", symbol);

            // Connect with retry
            let (_write, mut read) = self.connect_with_retry(&stream_name).await?;

            // Read messages and broadcast to subscribers
            while let Some(msg_result) = read.next().await {
                match msg_result {
                    Ok(Message::Text(text)) => {
                        // Parse depth update
                        match serde_json::from_str::<DepthUpdate>(&text) {
                            Ok(update) => {
                                // Broadcast to all subscribers
                                // Ignore send errors (no active receivers)
                                let _ = tx.send(update);
                            }
                            Err(e) => {
                                tracing::warn!("Failed to parse depth update: {}", e);
                            }
                        }
                    }
                    Ok(Message::Ping(data)) => {
                        tracing::debug!("Received ping with {} bytes", data.len());
                    }
                    Ok(Message::Pong(_)) => {
                        tracing::debug!("Received pong");
                    }
                    Ok(Message::Close(frame)) => {
                        tracing::info!("WebSocket closed: {:?}", frame);
                        break;
                    }
                    Err(e) => {
                        tracing::error!("WebSocket read error: {}", e);
                        break;
                    }
                    _ => {
                        tracing::debug!("Received other message type");
                    }
                }
            }

            tracing::warn!("Depth stream disconnected, reconnecting...");
            sleep(Duration::from_secs(1)).await;
        }
    }

    /// Start a user data stream task that reads from Binance and broadcasts to subscribers
    ///
    /// Creates a background task that:
    /// 1. Connects to Binance user data WebSocket stream using listen key
    /// 2. Reads order update and balance change messages
    /// 3. Broadcasts messages to all subscribers via broadcast channel
    /// 4. Automatically reconnects on connection loss
    ///
    /// ## Arguments
    /// - `listen_key`: Listen key obtained from POST /api/v3/userDataStream
    /// - `tx`: Broadcast sender for distributing user data events to subscribers
    ///
    /// ## Returns
    /// Task handle that can be awaited or spawned
    ///
    /// ## Example
    /// ```rust,no_run
    /// use mcp_binance_server::binance::websocket::BinanceWebSocketClient;
    /// use tokio::sync::broadcast;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = BinanceWebSocketClient::new();
    /// let (tx, _rx) = broadcast::channel(100);
    /// let listen_key = "your_listen_key_here";
    ///
    /// // Spawn task to run in background
    /// tokio::spawn(async move {
    ///     if let Err(e) = client.user_data_stream_task(listen_key, tx).await {
    ///         eprintln!("User data stream error: {}", e);
    ///     }
    /// });
    /// # Ok(())
    /// # }
    /// ```
    pub async fn user_data_stream_task(
        &self,
        listen_key: &str,
        tx: broadcast::Sender<UserDataEvent>,
    ) -> Result<(), McpError> {
        let stream_name = listen_key.to_string();

        loop {
            tracing::info!("Starting user data stream with listen key");

            // Connect with retry
            let (_write, mut read) = self.connect_with_retry(&stream_name).await?;

            // Read messages and broadcast to subscribers
            while let Some(msg_result) = read.next().await {
                match msg_result {
                    Ok(Message::Text(text)) => {
                        // Parse user data event
                        match serde_json::from_str::<UserDataEvent>(&text) {
                            Ok(event) => {
                                // Broadcast to all subscribers
                                // Ignore send errors (no active receivers)
                                let _ = tx.send(event);
                            }
                            Err(e) => {
                                tracing::warn!("Failed to parse user data event: {}", e);
                                tracing::debug!("Raw message: {}", text);
                            }
                        }
                    }
                    Ok(Message::Ping(data)) => {
                        tracing::debug!("Received ping with {} bytes", data.len());
                    }
                    Ok(Message::Pong(_)) => {
                        tracing::debug!("Received pong");
                    }
                    Ok(Message::Close(frame)) => {
                        tracing::info!("WebSocket closed: {:?}", frame);
                        break;
                    }
                    Err(e) => {
                        tracing::error!("WebSocket read error: {}", e);
                        break;
                    }
                    _ => {
                        tracing::debug!("Received other message type");
                    }
                }
            }

            tracing::warn!("User data stream disconnected, reconnecting...");
            sleep(Duration::from_secs(1)).await;
        }
    }
}

impl Default for BinanceWebSocketClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Order book depth update message from Binance WebSocket
///
/// Received from the `<symbol>@depth` stream for bid/ask updates
#[derive(Debug, Clone, Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DepthUpdate {
    /// Event type (always "depthUpdate")
    #[serde(rename = "e")]
    pub event_type: String,

    /// Event time (milliseconds since Unix epoch)
    #[serde(rename = "E")]
    pub event_time: i64,

    /// Trading pair symbol
    #[serde(rename = "s")]
    pub symbol: String,

    /// First update ID in event
    #[serde(rename = "U")]
    pub first_update_id: i64,

    /// Final update ID in event
    #[serde(rename = "u")]
    pub final_update_id: i64,

    /// Bids to be updated [[price, quantity], ...]
    #[serde(rename = "b")]
    pub bids: Vec<(String, String)>,

    /// Asks to be updated [[price, quantity], ...]
    #[serde(rename = "a")]
    pub asks: Vec<(String, String)>,
}

/// Ticker price update message from Binance WebSocket
///
/// Received from the `<symbol>@ticker` stream every 1000ms
#[derive(Debug, Clone, Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TickerUpdate {
    /// Event type (always "24hrTicker")
    #[serde(rename = "e")]
    pub event_type: String,

    /// Event time (milliseconds since Unix epoch)
    #[serde(rename = "E")]
    pub event_time: i64,

    /// Trading pair symbol
    #[serde(rename = "s")]
    pub symbol: String,

    /// Price change
    #[serde(rename = "p")]
    pub price_change: String,

    /// Price change percent
    #[serde(rename = "P")]
    pub price_change_percent: String,

    /// Weighted average price
    #[serde(rename = "w")]
    pub weighted_avg_price: String,

    /// Last price
    #[serde(rename = "c")]
    pub last_price: String,

    /// Last quantity
    #[serde(rename = "Q")]
    pub last_quantity: String,

    /// Open price
    #[serde(rename = "o")]
    pub open_price: String,

    /// High price
    #[serde(rename = "h")]
    pub high_price: String,

    /// Low price
    #[serde(rename = "l")]
    pub low_price: String,

    /// Total traded base asset volume
    #[serde(rename = "v")]
    pub volume: String,

    /// Total traded quote asset volume
    #[serde(rename = "q")]
    pub quote_volume: String,
}

/// User data event from Binance WebSocket
///
/// Received from the user data stream (authenticated with listen key)
/// Contains order execution reports and account balance updates
#[derive(Debug, Clone, Deserialize, serde::Serialize)]
#[serde(tag = "e")]
pub enum UserDataEvent {
    /// Order execution report (order update)
    /// Boxed to reduce enum size (ExecutionReport is 480 bytes)
    #[serde(rename = "executionReport")]
    ExecutionReport(Box<ExecutionReport>),

    /// Account position update (balance change)
    #[serde(rename = "outboundAccountPosition")]
    OutboundAccountPosition(OutboundAccountPosition),
}

/// Order execution report from user data stream
///
/// Sent when an order is placed, filled, cancelled, or updated
#[derive(Debug, Clone, Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionReport {
    /// Event time (milliseconds since Unix epoch)
    #[serde(rename = "E")]
    pub event_time: i64,

    /// Trading pair symbol
    #[serde(rename = "s")]
    pub symbol: String,

    /// Client order ID
    #[serde(rename = "c")]
    pub client_order_id: String,

    /// Side (BUY or SELL)
    #[serde(rename = "S")]
    pub side: String,

    /// Order type (LIMIT, MARKET, etc.)
    #[serde(rename = "o")]
    pub order_type: String,

    /// Time in force (GTC, IOC, FOK)
    #[serde(rename = "f")]
    pub time_in_force: String,

    /// Order quantity
    #[serde(rename = "q")]
    pub quantity: String,

    /// Order price
    #[serde(rename = "p")]
    pub price: String,

    /// Current execution type (NEW, CANCELED, REPLACED, REJECTED, TRADE, EXPIRED)
    #[serde(rename = "x")]
    pub execution_type: String,

    /// Current order status (NEW, PARTIALLY_FILLED, FILLED, CANCELED, etc.)
    #[serde(rename = "X")]
    pub order_status: String,

    /// Order reject reason (only if order rejected)
    #[serde(rename = "r")]
    pub order_reject_reason: String,

    /// Order ID
    #[serde(rename = "i")]
    pub order_id: i64,

    /// Last executed quantity
    #[serde(rename = "l")]
    pub last_executed_quantity: String,

    /// Cumulative filled quantity
    #[serde(rename = "z")]
    pub cumulative_filled_quantity: String,

    /// Last executed price
    #[serde(rename = "L")]
    pub last_executed_price: String,

    /// Commission amount
    #[serde(rename = "n")]
    pub commission_amount: String,

    /// Commission asset
    #[serde(rename = "N")]
    pub commission_asset: Option<String>,

    /// Transaction time
    #[serde(rename = "T")]
    pub transaction_time: i64,

    /// Trade ID
    #[serde(rename = "t")]
    pub trade_id: i64,

    /// Is order on the book?
    #[serde(rename = "w")]
    pub is_order_on_book: bool,

    /// Is this trade the maker side?
    #[serde(rename = "m")]
    pub is_maker_side: bool,

    /// Order creation time
    #[serde(rename = "O")]
    pub order_creation_time: i64,

    /// Cumulative quote asset transacted quantity
    #[serde(rename = "Z")]
    pub cumulative_quote_quantity: String,

    /// Last quote asset transacted quantity
    #[serde(rename = "Y")]
    pub last_quote_quantity: String,

    /// Quote order quantity
    #[serde(rename = "Q")]
    pub quote_order_quantity: String,
}

/// Account position update from user data stream
///
/// Sent when account balances change due to trades or transfers
#[derive(Debug, Clone, Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutboundAccountPosition {
    /// Event time (milliseconds since Unix epoch)
    #[serde(rename = "E")]
    pub event_time: i64,

    /// Last account update time
    #[serde(rename = "u")]
    pub last_update_time: i64,

    /// Account balances
    #[serde(rename = "B")]
    pub balances: Vec<BalanceUpdate>,
}

/// Balance update entry in account position
#[derive(Debug, Clone, Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceUpdate {
    /// Asset name
    #[serde(rename = "a")]
    pub asset: String,

    /// Free balance
    #[serde(rename = "f")]
    pub free: String,

    /// Locked balance
    #[serde(rename = "l")]
    pub locked: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binance_ws_client_creation() {
        let client = BinanceWebSocketClient::new();
        assert_eq!(client.base_url, BINANCE_WS_URL);
    }

    #[test]
    fn test_ticker_update_deserialization() {
        let json = r#"{
            "e": "24hrTicker",
            "E": 123456789,
            "s": "BTCUSDT",
            "p": "100.00",
            "P": "0.50",
            "w": "45000.50",
            "c": "45100.00",
            "Q": "0.001",
            "o": "45000.00",
            "h": "45200.00",
            "l": "44900.00",
            "v": "1000.5",
            "q": "45000000.00"
        }"#;

        let update: TickerUpdate = serde_json::from_str(json).unwrap();
        assert_eq!(update.symbol, "BTCUSDT");
        assert_eq!(update.last_price, "45100.00");
        assert_eq!(update.price_change, "100.00");
    }
}
