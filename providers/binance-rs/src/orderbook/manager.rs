//! Order book manager for tracking multiple symbols
//!
//! Implements lazy initialization, WebSocket streaming, REST API fallback,
//! and symbol limit enforcement (max 20 concurrent symbols).

use crate::binance::client::BinanceClient;
use crate::orderbook::rate_limiter::{RateLimiter, RateLimiterError};
use crate::orderbook::types::{HealthStatus, OrderBook, OrderBookHealth};
use crate::orderbook::websocket::{DepthUpdateEvent, DepthWebSocketClient};
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

/// Maximum number of concurrent symbols that can be tracked
const MAX_CONCURRENT_SYMBOLS: usize = 20;

/// Staleness threshold in milliseconds (5 seconds)
const STALENESS_THRESHOLD_MS: i64 = 5000;

/// Order book manager errors
#[derive(Debug, Error)]
pub enum ManagerError {
    #[error("Symbol limit reached: cannot track more than {MAX_CONCURRENT_SYMBOLS} symbols")]
    SymbolLimitReached,

    #[error("Symbol not found: {0}")]
    SymbolNotFound(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(#[from] RateLimiterError),

    #[error("Initialization failed for {symbol}: {source}")]
    InitializationFailed {
        symbol: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("REST API error: {0}")]
    RestApiError(String),

    #[error("WebSocket error: {0}")]
    WebSocketError(String),
}

/// Internal state for a tracked order book
struct OrderBookState {
    /// Current order book snapshot
    order_book: OrderBook,

    /// WebSocket client task handle (kept alive to prevent task cancellation)
    #[allow(dead_code)]
    websocket_handle: Option<JoinHandle<()>>,

    /// Last successful update timestamp
    last_update_time: i64,

    /// Whether WebSocket is currently connected
    websocket_connected: bool,

    /// CROSSED FIX: Flag indicating orderbook needs re-sync due to gap
    needs_resync: bool,
}

/// Manager for multiple order book subscriptions
///
/// Tracks up to 20 symbols with lazy initialization:
/// 1. First request triggers REST API snapshot + WebSocket subscription
/// 2. Subsequent requests use cached data (updated via WebSocket)
/// 3. REST API fallback when data is stale (>5s old)
pub struct OrderBookManager {
    /// Map of symbol → order book state
    states: Arc<RwLock<HashMap<String, OrderBookState>>>,

    /// Rate limiter for REST API requests
    rate_limiter: Arc<RateLimiter>,

    /// Binance API client (for REST fallback)
    binance_client: Arc<BinanceClient>,
}

impl OrderBookManager {
    /// Create a new order book manager
    pub fn new(binance_client: Arc<BinanceClient>) -> Self {
        Self {
            states: Arc::new(RwLock::new(HashMap::new())),
            rate_limiter: Arc::new(RateLimiter::new()),
            binance_client,
        }
    }

    /// Subscribe to order book updates for a symbol (eager initialization)
    ///
    /// Initiates WebSocket subscription and fetches initial REST API snapshot.
    /// Idempotent - calling subscribe() for an already-subscribed symbol is a no-op.
    ///
    /// Use this for eager subscription (e.g., background persistence tasks).
    /// For one-time queries, use get_order_book() which handles lazy initialization.
    pub async fn subscribe(&self, symbol: &str) -> Result<(), ManagerError> {
        let symbol_upper = symbol.to_uppercase();

        // Check if already subscribed
        {
            let states = self.states.read().await;
            if states.contains_key(&symbol_upper) {
                debug!(symbol = %symbol_upper, "Already subscribed");
                return Ok(());
            }
        }

        // Need to subscribe
        let mut states = self.states.write().await;

        // Double-check after acquiring write lock (avoid race condition)
        if states.contains_key(&symbol_upper) {
            return Ok(());
        }

        // Check symbol limit
        if states.len() >= MAX_CONCURRENT_SYMBOLS {
            return Err(ManagerError::SymbolLimitReached);
        }

        // Initialize order book
        self.initialize_order_book(&mut states, &symbol_upper)
            .await?;

        Ok(())
    }

    /// Get order book for a symbol (lazy initialization)
    ///
    /// On first request:
    /// - Checks symbol limit (20 max)
    /// - Fetches REST API snapshot
    /// - Starts WebSocket subscription
    ///
    /// On subsequent requests:
    /// - Returns cached data if fresh (<5s old)
    /// - Falls back to REST API if stale (>5s old)
    pub async fn get_order_book(&self, symbol: &str) -> Result<OrderBook, ManagerError> {
        let symbol_upper = symbol.to_uppercase();

        // Check if already initialized
        {
            let states = self.states.read().await;
            if let Some(state) = states.get(&symbol_upper) {
                // AUTO-RESYNC FIX: Check if resync is needed due to gap detection
                if state.needs_resync {
                    warn!(
                        symbol = %symbol_upper,
                        "Gap detected in WebSocket updates, forcing resync"
                    );
                    // Drop read lock before acquiring write lock
                    drop(states);
                    // Perform resync with write lock
                    return self.resync_order_book(&symbol_upper).await;
                }

                // Check staleness
                let now = chrono::Utc::now().timestamp_millis();
                let age_ms = now - state.last_update_time;

                if age_ms < STALENESS_THRESHOLD_MS {
                    debug!(
                        symbol = %symbol_upper,
                        age_ms,
                        "Returning cached order book"
                    );
                    return Ok(state.order_book.clone());
                } else {
                    warn!(
                        symbol = %symbol_upper,
                        age_ms,
                        "Cached data is stale, fetching fresh snapshot"
                    );
                }
            }
        }

        // Need to initialize or refresh
        let mut states = self.states.write().await;

        // Check symbol limit (only for new symbols)
        if !states.contains_key(&symbol_upper) && states.len() >= MAX_CONCURRENT_SYMBOLS {
            return Err(ManagerError::SymbolLimitReached);
        }

        // Initialize or refresh order book
        self.initialize_order_book(&mut states, &symbol_upper)
            .await?;

        // Return the freshly initialized order book
        let state = states
            .get(&symbol_upper)
            .expect("order book should exist after initialization");
        Ok(state.order_book.clone())
    }

    /// Initialize order book for a symbol (called with write lock held)
    async fn initialize_order_book(
        &self,
        states: &mut HashMap<String, OrderBookState>,
        symbol: &str,
    ) -> Result<(), ManagerError> {
        info!(symbol = %symbol, "Initializing order book");

        // Wait for rate limit permission
        self.rate_limiter.wait().await?;

        // Fetch initial snapshot from REST API
        let order_book = self.fetch_snapshot(symbol).await?;

        // Start WebSocket subscription
        let (ws_client, mut update_receiver) = DepthWebSocketClient::new(symbol.to_string());
        let websocket_handle = ws_client.start();

        // Store initial state
        let state = OrderBookState {
            order_book: order_book.clone(),
            websocket_handle: Some(websocket_handle),
            last_update_time: chrono::Utc::now().timestamp_millis(),
            websocket_connected: true,
            needs_resync: false, // CROSSED FIX: Initialize resync flag
        };

        states.insert(symbol.to_string(), state);

        // Spawn task to process WebSocket updates
        let states_clone = Arc::clone(&self.states);
        let symbol_owned = symbol.to_string();
        tokio::spawn(async move {
            while let Some(update) = update_receiver.recv().await {
                if let Err(e) =
                    Self::process_depth_update(&states_clone, &symbol_owned, update).await
                {
                    error!(
                        symbol = %symbol_owned,
                        error = %e,
                        "Failed to process depth update"
                    );
                }
            }

            // WebSocket receiver closed - mark as disconnected
            warn!(symbol = %symbol_owned, "WebSocket receiver closed");
            let mut states = states_clone.write().await;
            if let Some(state) = states.get_mut(&symbol_owned) {
                state.websocket_connected = false;
            }
        });

        info!(symbol = %symbol, "Order book initialized successfully");
        Ok(())
    }

    /// Resync order book from REST API without killing WebSocket
    ///
    /// AUTO-RESYNC FIX: This method refreshes the orderbook snapshot while keeping
    /// the WebSocket connection alive. Used when gaps are detected or crossed orderbook appears.
    async fn resync_order_book(&self, symbol: &str) -> Result<OrderBook, ManagerError> {
        info!(symbol = %symbol, "Resyncing order book due to gap or anomaly");

        // Acquire write lock
        let mut states = self.states.write().await;

        // Verify symbol exists
        let state = states
            .get_mut(symbol)
            .ok_or_else(|| ManagerError::SymbolNotFound(symbol.to_string()))?;

        // Wait for rate limit permission
        self.rate_limiter.wait().await?;

        // Fetch fresh snapshot
        let fresh_snapshot = self.fetch_snapshot(symbol).await?;

        // Update orderbook in-place (WebSocket handle remains untouched)
        state.order_book = fresh_snapshot.clone();
        state.last_update_time = chrono::Utc::now().timestamp_millis();
        state.needs_resync = false; // Clear resync flag

        info!(
            symbol = %symbol,
            update_id = fresh_snapshot.last_update_id,
            "Order book resynced successfully"
        );

        Ok(fresh_snapshot)
    }

    /// Fetch order book snapshot from REST API
    async fn fetch_snapshot(&self, symbol: &str) -> Result<OrderBook, ManagerError> {
        debug!(symbol = %symbol, "Fetching order book snapshot from REST API");

        // Use BinanceClient to fetch depth
        let snapshot = self
            .binance_client
            .get_order_book(symbol, Some(100))
            .await
            .map_err(|e| ManagerError::RestApiError(e.to_string()))?;

        // Convert response to OrderBook
        let mut order_book = OrderBook::new(symbol.to_string());
        order_book.last_update_id = snapshot.last_update_id;
        order_book.timestamp = chrono::Utc::now().timestamp_millis();

        // Parse bids - Binance API returns Vec<(String, String)>
        for (price_str, qty_str) in &snapshot.bids {
            let price = Decimal::from_str(price_str)
                .map_err(|e| ManagerError::RestApiError(format!("Invalid bid price: {}", e)))?;
            let qty = Decimal::from_str(qty_str)
                .map_err(|e| ManagerError::RestApiError(format!("Invalid bid qty: {}", e)))?;
            order_book.bids.insert(price, qty);
        }

        // Parse asks - Binance API returns Vec<(String, String)>
        for (price_str, qty_str) in &snapshot.asks {
            let price = Decimal::from_str(price_str)
                .map_err(|e| ManagerError::RestApiError(format!("Invalid ask price: {}", e)))?;
            let qty = Decimal::from_str(qty_str)
                .map_err(|e| ManagerError::RestApiError(format!("Invalid ask qty: {}", e)))?;
            order_book.asks.insert(price, qty);
        }

        debug!(
            symbol = %symbol,
            bid_levels = order_book.bids.len(),
            ask_levels = order_book.asks.len(),
            "Fetched order book snapshot"
        );

        Ok(order_book)
    }

    /// Process a depth update from WebSocket
    ///
    /// CROSSED FIX: Properly handle update sequence according to Binance specification:
    /// - If u <= lastUpdateId: ignore (stale event)
    /// - If U > lastUpdateId + 1: gap detected, skip update to prevent corruption
    /// - If U <= lastUpdateId + 1 <= u: normal case, apply update
    async fn process_depth_update(
        states: &Arc<RwLock<HashMap<String, OrderBookState>>>,
        symbol: &str,
        update: DepthUpdateEvent,
    ) -> Result<(), ManagerError> {
        let mut states = states.write().await;
        let state = states
            .get_mut(symbol)
            .ok_or_else(|| ManagerError::SymbolNotFound(symbol.to_string()))?;

        // CROSSED FIX: Proper sequence validation per Binance spec
        let last_id = state.order_book.last_update_id;

        // Case 1: Stale event (u <= lastUpdateId) - ignore
        if update.final_update_id <= last_id {
            debug!(
                symbol = %symbol,
                update_u = update.final_update_id,
                last_id = last_id,
                "Ignoring stale depth update"
            );
            return Ok(());
        }

        // Case 2: Gap detected (U > lastUpdateId + 1) - skip to prevent corruption
        if update.first_update_id > last_id + 1 {
            error!(
                symbol = %symbol,
                gap = update.first_update_id - last_id - 1,
                expected_U = last_id + 1,
                received_U = update.first_update_id,
                received_u = update.final_update_id,
                "Gap in depth updates detected! Skipping update to prevent orderbook corruption. Re-sync needed."
            );
            // Mark as needing resync
            state.needs_resync = true;
            return Err(ManagerError::WebSocketError(
                format!("Gap detected: expected U={}, got U={}", last_id + 1, update.first_update_id)
            ));
        }

        // Case 3: Normal or overlapping case (U <= lastUpdateId + 1 <= u) - apply
        debug!(
            symbol = %symbol,
            U = update.first_update_id,
            u = update.final_update_id,
            last_id = last_id,
            "Applying depth update"
        );

        // Apply bid updates (log deletions)
        for [price_str, qty_str] in &update.bids {
            let price = Decimal::from_str(price_str)
                .map_err(|e| ManagerError::WebSocketError(format!("Invalid bid price: {}", e)))?;
            let qty = Decimal::from_str(qty_str)
                .map_err(|e| ManagerError::WebSocketError(format!("Invalid bid qty: {}", e)))?;

            if qty.is_zero() {
                debug!(symbol = %symbol, price = %price, "Removing bid level");
            }
            state.order_book.update_bid(price, qty);
        }

        // Apply ask updates (log deletions)
        for [price_str, qty_str] in &update.asks {
            let price = Decimal::from_str(price_str)
                .map_err(|e| ManagerError::WebSocketError(format!("Invalid ask price: {}", e)))?;
            let qty = Decimal::from_str(qty_str)
                .map_err(|e| ManagerError::WebSocketError(format!("Invalid ask qty: {}", e)))?;

            if qty.is_zero() {
                debug!(symbol = %symbol, price = %price, "Removing ask level");
            }
            state.order_book.update_ask(price, qty);
        }

        // Update metadata
        state.order_book.last_update_id = update.final_update_id;
        state.order_book.timestamp = update.event_time;
        state.last_update_time = chrono::Utc::now().timestamp_millis();

        debug!(
            symbol = %symbol,
            update_id = update.final_update_id,
            bid_updates = update.bids.len(),
            ask_updates = update.asks.len(),
            "Processed depth update successfully"
        );

        // AUTO-RESYNC FIX: Detect crossed orderbook (safety check)
        // If best_ask <= best_bid after applying updates, orderbook is corrupted
        if let (Some(best_bid), Some(best_ask)) = (state.order_book.best_bid(), state.order_book.best_ask()) {
            if best_ask <= best_bid {
                error!(
                    symbol = %symbol,
                    best_bid = %best_bid,
                    best_ask = %best_ask,
                    "CRITICAL: Crossed orderbook detected after update! best_ask <= best_bid. Marking for resync."
                );
                state.needs_resync = true;
                return Err(ManagerError::WebSocketError(
                    format!("Crossed orderbook: bid={} >= ask={}", best_bid, best_ask)
                ));
            }
        }

        Ok(())
    }

    /// Get health status of all tracked order books
    pub async fn get_health(&self) -> OrderBookHealth {
        let states = self.states.read().await;
        let now = chrono::Utc::now().timestamp_millis();

        let active_count = states.len();
        let connected_count = states.values().filter(|s| s.websocket_connected).count();

        // Calculate max age across all symbols
        let max_age_ms = states
            .values()
            .map(|s| now - s.last_update_time)
            .max()
            .unwrap_or(0);

        // Determine status
        let (status, reason) = if active_count == 0 {
            (HealthStatus::Ok, None)
        } else if connected_count == 0 {
            (
                HealthStatus::Error,
                Some("All WebSocket connections down".to_string()),
            )
        } else if max_age_ms > STALENESS_THRESHOLD_MS {
            (
                HealthStatus::Degraded,
                Some(format!(
                    "Data is stale ({}ms old), may need refresh",
                    max_age_ms
                )),
            )
        } else if connected_count < active_count {
            (
                HealthStatus::Degraded,
                Some(format!(
                    "{}/{} WebSocket connections active",
                    connected_count, active_count
                )),
            )
        } else {
            (HealthStatus::Ok, None)
        };

        OrderBookHealth {
            status,
            orderbook_symbols_active: active_count,
            last_update_age_ms: max_age_ms,
            websocket_connected: connected_count > 0,
            timestamp: now,
            reason,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_concurrent_symbols() {
        assert_eq!(MAX_CONCURRENT_SYMBOLS, 20);
    }

    #[test]
    fn test_staleness_threshold() {
        assert_eq!(STALENESS_THRESHOLD_MS, 5000);
    }
}
