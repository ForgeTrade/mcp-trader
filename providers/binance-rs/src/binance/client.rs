//! Binance HTTP Client
//!
//! HTTP client wrapper for making requests to Binance REST API.
//! Provides timeout configuration, user-agent headers, and request signing.

use crate::binance::types::{
    AccountInfo, KlineData, MyTrade, Order, OrderBook, ServerTimeResponse, Ticker24hr, TickerPrice,
    Trade,
};
use crate::error::McpError;
use hmac::{Hmac, Mac};
use reqwest::Client;
use sha2::Sha256;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

/// Binance REST API HTTP client
///
/// Wraps reqwest::Client with Binance-specific configuration including
/// timeouts, base URL, user-agent headers, and API credentials for signing.
#[derive(Clone)]
pub struct BinanceClient {
    /// HTTP client for making requests
    pub(crate) client: Client,
    /// Base URL for Binance API (default: https://api.binance.com)
    pub(crate) base_url: String,
    /// Optional API key for authenticated requests
    pub(crate) api_key: Option<String>,
    /// Optional API secret for request signing
    pub(crate) api_secret: Option<String>,
}

impl std::fmt::Debug for BinanceClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BinanceClient")
            .field("base_url", &self.base_url)
            .field("api_key", &self.api_key.as_ref().map(|_| "***"))
            .field("api_secret", &self.api_secret.as_ref().map(|_| "***"))
            .finish()
    }
}

impl BinanceClient {
    /// Creates a new Binance client with default settings (no credentials)
    ///
    /// Default configuration:
    /// - Base URL: https://api.binance.com
    /// - Timeout: 10 seconds
    /// - User-Agent: mcp-binance-server/0.1.0
    /// - No API credentials (public endpoints only)
    pub fn new() -> Self {
        Self::with_timeout(Duration::from_secs(10))
    }

    /// Creates a new Binance client with API credentials from environment
    ///
    /// Reads credentials from:
    /// - `BINANCE_API_KEY` - API key for authenticated requests
    /// - `BINANCE_API_SECRET` - API secret for signing requests
    ///
    /// # Returns
    /// Client with credentials if both env vars are set, otherwise no credentials
    pub fn with_credentials() -> Self {
        let api_key = std::env::var("BINANCE_API_KEY").ok();
        let api_secret = std::env::var("BINANCE_API_SECRET").ok();

        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .user_agent("mcp-binance-server/0.1.0")
                .build()
                .expect("Failed to create HTTP client"),
            base_url: "https://api.binance.com".to_string(),
            api_key,
            api_secret,
        }
    }

    /// Creates a new Binance client with custom timeout
    ///
    /// # Arguments
    /// * `timeout` - Request timeout duration
    ///
    /// # Example
    /// ```
    /// use std::time::Duration;
    /// use mcp_binance_server::binance::client::BinanceClient;
    ///
    /// let client = BinanceClient::with_timeout(Duration::from_secs(5));
    /// ```
    pub fn with_timeout(timeout: Duration) -> Self {
        let client = Client::builder()
            .timeout(timeout)
            .user_agent("mcp-binance-server/0.1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: "https://api.binance.com".to_string(),
            api_key: None,
            api_secret: None,
        }
    }

    /// Returns the configured base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Generates HMAC-SHA256 signature for request parameters
    ///
    /// # Arguments
    /// * `query_string` - URL-encoded query string to sign
    ///
    /// # Returns
    /// Hexadecimal signature string
    ///
    /// # Errors
    /// Returns error if API secret is not configured
    fn sign_request(&self, query_string: &str) -> Result<String, McpError> {
        let secret = self
            .api_secret
            .as_ref()
            .ok_or_else(|| McpError::InvalidRequest("API secret not configured".to_string()))?;

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .map_err(|e| McpError::ParseError(format!("Invalid secret key: {}", e)))?;

        mac.update(query_string.as_bytes());
        let result = mac.finalize();
        let signature = hex::encode(result.into_bytes());

        Ok(signature)
    }

    /// Gets current timestamp in milliseconds
    ///
    /// Uses system time as milliseconds since Unix epoch
    fn get_timestamp() -> Result<u64, McpError> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .map_err(|e| McpError::ParseError(format!("System time error: {}", e)))
    }

    /// Fetches current Binance server time
    ///
    /// Calls GET /api/v3/time endpoint and returns the server timestamp in milliseconds.
    /// Implements exponential backoff for rate limit (429) responses with up to 3 retries.
    ///
    /// # Returns
    /// * `Ok(i64)` - Server time in milliseconds since Unix epoch
    /// * `Err(McpError)` - Network error, rate limit exceeded, or parse error
    ///
    /// # Errors
    /// * `ConnectionError` - Network failures, timeouts, 5xx server errors
    /// * `RateLimitError` - HTTP 429 after max retries (3 attempts)
    /// * `ParseError` - Invalid JSON response or unexpected format
    ///
    /// # Example
    /// ```no_run
    /// use mcp_binance_server::binance::client::BinanceClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = BinanceClient::new();
    /// let server_time = client.get_server_time().await?;
    /// println!("Binance server time: {}", server_time);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_server_time(&self) -> Result<i64, McpError> {
        let url = format!("{}/api/v3/time", self.base_url);
        let max_retries = 3;
        let mut retry_count = 0;

        loop {
            let response = self.client.get(&url).send().await;

            match response {
                Ok(resp) => {
                    let status = resp.status();

                    // Handle 429 rate limit with exponential backoff
                    if status.as_u16() == 429 {
                        if retry_count >= max_retries {
                            return Err(McpError::RateLimitError(format!(
                                "Rate limit exceeded after {} retries. Wait 60 seconds before retrying.",
                                max_retries
                            )));
                        }

                        // Parse Retry-After header if present, otherwise use exponential backoff
                        let retry_after = resp
                            .headers()
                            .get("retry-after")
                            .and_then(|h| h.to_str().ok())
                            .and_then(|s| s.parse::<u64>().ok())
                            .unwrap_or_else(|| 2_u64.pow(retry_count)); // 1s, 2s, 4s

                        tracing::warn!(
                            "Rate limit hit (429). Retry {} of {}. Waiting {}s before retry.",
                            retry_count + 1,
                            max_retries,
                            retry_after
                        );

                        tokio::time::sleep(Duration::from_secs(retry_after)).await;
                        retry_count += 1;
                        continue;
                    }

                    // Check for other HTTP errors
                    if !status.is_success() {
                        return Err(McpError::from(resp.error_for_status().unwrap_err()));
                    }

                    // Parse successful response
                    let server_time_response: ServerTimeResponse = resp.json().await?;

                    // Validate response
                    if !server_time_response.is_valid() {
                        return Err(McpError::ParseError(format!(
                            "Invalid server time received: {}",
                            server_time_response.server_time
                        )));
                    }

                    return Ok(server_time_response.time_ms());
                }
                Err(err) => {
                    // Network errors are not retryable in this simple implementation
                    return Err(McpError::from(err));
                }
            }
        }
    }

    /// Get latest price for a symbol
    ///
    /// Calls GET /api/v3/ticker/price
    ///
    /// # Arguments
    /// * `symbol` - Trading pair symbol (e.g., "BTCUSDT")
    ///
    /// # Returns
    /// * `Ok(TickerPrice)` - Current price data
    /// * `Err(McpError)` - Network error or API error
    pub async fn get_ticker_price(&self, symbol: &str) -> Result<TickerPrice, McpError> {
        let url = format!("{}/api/v3/ticker/price?symbol={}", self.base_url, symbol);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(McpError::from(response.error_for_status().unwrap_err()));
        }

        let ticker: TickerPrice = response.json().await?;
        Ok(ticker)
    }

    /// Get 24-hour ticker price statistics
    ///
    /// Calls GET /api/v3/ticker/24hr
    ///
    /// # Arguments
    /// * `symbol` - Trading pair symbol (e.g., "BTCUSDT")
    ///
    /// # Returns
    /// * `Ok(Ticker24hr)` - 24-hour statistics
    /// * `Err(McpError)` - Network error or API error
    pub async fn get_24hr_ticker(&self, symbol: &str) -> Result<Ticker24hr, McpError> {
        let url = format!("{}/api/v3/ticker/24hr?symbol={}", self.base_url, symbol);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(McpError::from(response.error_for_status().unwrap_err()));
        }

        let ticker: Ticker24hr = response.json().await?;
        Ok(ticker)
    }

    /// Get candlestick/kline data
    ///
    /// Calls GET /api/v3/klines
    ///
    /// # Arguments
    /// * `symbol` - Trading pair symbol (e.g., "BTCUSDT")
    /// * `interval` - Kline interval (e.g., "1m", "5m", "1h", "1d")
    /// * `limit` - Number of klines to return (default 500, max 1000)
    ///
    /// # Returns
    /// * `Ok(KlineData)` - Array of kline data
    /// * `Err(McpError)` - Network error or API error
    pub async fn get_klines(
        &self,
        symbol: &str,
        interval: &str,
        limit: Option<u32>,
    ) -> Result<KlineData, McpError> {
        let mut url = format!(
            "{}/api/v3/klines?symbol={}&interval={}",
            self.base_url, symbol, interval
        );

        if let Some(lim) = limit {
            url.push_str(&format!("&limit={}", lim));
        }

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(McpError::from(response.error_for_status().unwrap_err()));
        }

        let klines: KlineData = response.json().await?;
        Ok(klines)
    }

    /// Get order book depth
    ///
    /// Calls GET /api/v3/depth
    ///
    /// # Arguments
    /// * `symbol` - Trading pair symbol (e.g., "BTCUSDT")
    /// * `limit` - Number of levels to return (default 100, valid: 5, 10, 20, 50, 100, 500, 1000, 5000)
    ///
    /// # Returns
    /// * `Ok(OrderBook)` - Order book with bids and asks
    /// * `Err(McpError)` - Network error or API error
    pub async fn get_order_book(
        &self,
        symbol: &str,
        limit: Option<u32>,
    ) -> Result<OrderBook, McpError> {
        let mut url = format!("{}/api/v3/depth?symbol={}", self.base_url, symbol);

        if let Some(lim) = limit {
            url.push_str(&format!("&limit={}", lim));
        }

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(McpError::from(response.error_for_status().unwrap_err()));
        }

        let order_book: OrderBook = response.json().await?;
        Ok(order_book)
    }

    /// Get recent trades
    ///
    /// Calls GET /api/v3/trades
    ///
    /// # Arguments
    /// * `symbol` - Trading pair symbol (e.g., "BTCUSDT")
    /// * `limit` - Number of trades to return (default 500, max 1000)
    ///
    /// # Returns
    /// * `Ok(Vec<Trade>)` - List of recent trades
    /// * `Err(McpError)` - Network error or API error
    pub async fn get_recent_trades(
        &self,
        symbol: &str,
        limit: Option<u32>,
    ) -> Result<Vec<Trade>, McpError> {
        let mut url = format!("{}/api/v3/trades?symbol={}", self.base_url, symbol);

        if let Some(lim) = limit {
            url.push_str(&format!("&limit={}", lim));
        }

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(McpError::from(response.error_for_status().unwrap_err()));
        }

        let trades: Vec<Trade> = response.json().await?;
        Ok(trades)
    }

    /// Get account information
    ///
    /// Calls GET /api/v3/account (requires API key and secret)
    ///
    /// Returns account balances, commission rates, and permissions.
    /// Requires HMAC-SHA256 signature.
    ///
    /// # Returns
    /// * `Ok(AccountInfo)` - Account information including balances
    /// * `Err(McpError)` - Network error, authentication error, or API error
    ///
    /// # Errors
    /// * `InvalidRequest` - API credentials not configured
    /// * `ConnectionError` - Network failures or timeouts
    ///
    /// # Example
    /// ```no_run
    /// use mcp_binance_server::binance::client::BinanceClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = BinanceClient::with_credentials();
    /// let account = client.get_account().await?;
    /// println!("Account balances: {:?}", account.balances);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_account(&self) -> Result<AccountInfo, McpError> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| McpError::InvalidRequest("API key not configured".to_string()))?;

        // Build query string with timestamp
        let timestamp = Self::get_timestamp()?;
        let query_string = format!("timestamp={}", timestamp);

        // Sign the request
        let signature = self.sign_request(&query_string)?;

        // Build final URL with signature
        let url = format!(
            "{}/api/v3/account?{}&signature={}",
            self.base_url, query_string, signature
        );

        // Make signed request with API key header
        let response = self
            .client
            .get(&url)
            .header("X-MBX-APIKEY", api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(McpError::from(response.error_for_status().unwrap_err()));
        }

        let account: AccountInfo = response.json().await?;
        Ok(account)
    }

    /// Create a new order
    ///
    /// Calls POST /api/v3/order (requires API key and secret)
    ///
    /// # Arguments
    /// * `symbol` - Trading pair (e.g., "BTCUSDT")
    /// * `side` - Order side: "BUY" or "SELL"
    /// * `order_type` - Order type: "LIMIT", "MARKET", etc.
    /// * `quantity` - Order quantity as string
    /// * `price` - Order price as string (required for LIMIT orders)
    ///
    /// # Returns
    /// * `Ok(Order)` - Created order details
    /// * `Err(McpError)` - Error if order creation fails
    pub async fn create_order(
        &self,
        symbol: &str,
        side: &str,
        order_type: &str,
        quantity: &str,
        price: Option<&str>,
    ) -> Result<Order, McpError> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| McpError::InvalidRequest("API key not configured".to_string()))?;

        let timestamp = Self::get_timestamp()?;
        let mut params = vec![
            format!("symbol={}", symbol),
            format!("side={}", side),
            format!("type={}", order_type),
            format!("quantity={}", quantity),
            format!("timestamp={}", timestamp),
        ];

        // Add price for LIMIT orders
        if let Some(p) = price {
            params.push(format!("price={}", p));
            params.push("timeInForce=GTC".to_string());
        }

        let query_string = params.join("&");
        let signature = self.sign_request(&query_string)?;
        let url = format!(
            "{}/api/v3/order?{}&signature={}",
            self.base_url, query_string, signature
        );

        let response = self
            .client
            .post(&url)
            .header("X-MBX-APIKEY", api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(McpError::from(response.error_for_status().unwrap_err()));
        }

        let order: Order = response.json().await?;
        Ok(order)
    }

    /// Cancel an existing order
    ///
    /// Calls DELETE /api/v3/order (requires API key and secret)
    ///
    /// # Arguments
    /// * `symbol` - Trading pair (e.g., "BTCUSDT")
    /// * `order_id` - Order ID to cancel
    ///
    /// # Returns
    /// * `Ok(Order)` - Canceled order details
    /// * `Err(McpError)` - Error if cancellation fails
    pub async fn cancel_order(&self, symbol: &str, order_id: i64) -> Result<Order, McpError> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| McpError::InvalidRequest("API key not configured".to_string()))?;

        let timestamp = Self::get_timestamp()?;
        let query_string = format!(
            "symbol={}&orderId={}&timestamp={}",
            symbol, order_id, timestamp
        );
        let signature = self.sign_request(&query_string)?;
        let url = format!(
            "{}/api/v3/order?{}&signature={}",
            self.base_url, query_string, signature
        );

        let response = self
            .client
            .delete(&url)
            .header("X-MBX-APIKEY", api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(McpError::from(response.error_for_status().unwrap_err()));
        }

        let order: Order = response.json().await?;
        Ok(order)
    }

    /// Query order status
    ///
    /// Calls GET /api/v3/order (requires API key and secret)
    ///
    /// # Arguments
    /// * `symbol` - Trading pair (e.g., "BTCUSDT")
    /// * `order_id` - Order ID to query
    ///
    /// # Returns
    /// * `Ok(Order)` - Order details
    /// * `Err(McpError)` - Error if query fails
    pub async fn query_order(&self, symbol: &str, order_id: i64) -> Result<Order, McpError> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| McpError::InvalidRequest("API key not configured".to_string()))?;

        let timestamp = Self::get_timestamp()?;
        let query_string = format!(
            "symbol={}&orderId={}&timestamp={}",
            symbol, order_id, timestamp
        );
        let signature = self.sign_request(&query_string)?;
        let url = format!(
            "{}/api/v3/order?{}&signature={}",
            self.base_url, query_string, signature
        );

        let response = self
            .client
            .get(&url)
            .header("X-MBX-APIKEY", api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(McpError::from(response.error_for_status().unwrap_err()));
        }

        let order: Order = response.json().await?;
        Ok(order)
    }

    /// Get all open orders for a symbol
    ///
    /// Calls GET /api/v3/openOrders (requires API key and secret)
    ///
    /// # Arguments
    /// * `symbol` - Optional trading pair. If None, returns all open orders.
    ///
    /// # Returns
    /// * `Ok(Vec<Order>)` - List of open orders
    /// * `Err(McpError)` - Error if query fails
    pub async fn get_open_orders(&self, symbol: Option<&str>) -> Result<Vec<Order>, McpError> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| McpError::InvalidRequest("API key not configured".to_string()))?;

        let timestamp = Self::get_timestamp()?;
        let query_string = if let Some(sym) = symbol {
            format!("symbol={}&timestamp={}", sym, timestamp)
        } else {
            format!("timestamp={}", timestamp)
        };

        let signature = self.sign_request(&query_string)?;
        let url = format!(
            "{}/api/v3/openOrders?{}&signature={}",
            self.base_url, query_string, signature
        );

        let response = self
            .client
            .get(&url)
            .header("X-MBX-APIKEY", api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(McpError::from(response.error_for_status().unwrap_err()));
        }

        let orders: Vec<Order> = response.json().await?;
        Ok(orders)
    }

    /// Get all orders (active, canceled, or filled) for a symbol
    ///
    /// Calls GET /api/v3/allOrders (requires API key and secret)
    ///
    /// # Arguments
    /// * `symbol` - Trading pair (e.g., "BTCUSDT")
    /// * `limit` - Number of orders to return (default 500, max 1000)
    ///
    /// # Returns
    /// * `Ok(Vec<Order>)` - List of all orders
    /// * `Err(McpError)` - Error if query fails
    pub async fn get_all_orders(
        &self,
        symbol: &str,
        limit: Option<u32>,
    ) -> Result<Vec<Order>, McpError> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| McpError::InvalidRequest("API key not configured".to_string()))?;

        let timestamp = Self::get_timestamp()?;
        let mut query_string = format!("symbol={}&timestamp={}", symbol, timestamp);

        if let Some(lim) = limit {
            query_string.push_str(&format!("&limit={}", lim));
        }

        let signature = self.sign_request(&query_string)?;
        let url = format!(
            "{}/api/v3/allOrders?{}&signature={}",
            self.base_url, query_string, signature
        );

        let response = self
            .client
            .get(&url)
            .header("X-MBX-APIKEY", api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(McpError::from(response.error_for_status().unwrap_err()));
        }

        let orders: Vec<Order> = response.json().await?;
        Ok(orders)
    }

    /// Get trade history for the account
    ///
    /// Calls GET /api/v3/myTrades (requires API key and secret)
    ///
    /// # Arguments
    /// * `symbol` - Trading pair (e.g., "BTCUSDT")
    /// * `limit` - Number of trades to return (default 500, max 1000)
    ///
    /// # Returns
    /// * `Ok(Vec<MyTrade>)` - List of trades
    /// * `Err(McpError)` - Error if query fails
    pub async fn get_my_trades(
        &self,
        symbol: &str,
        limit: Option<u32>,
    ) -> Result<Vec<MyTrade>, McpError> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| McpError::InvalidRequest("API key not configured".to_string()))?;

        let timestamp = Self::get_timestamp()?;
        let mut query_string = format!("symbol={}&timestamp={}", symbol, timestamp);

        if let Some(lim) = limit {
            query_string.push_str(&format!("&limit={}", lim));
        }

        let signature = self.sign_request(&query_string)?;
        let url = format!(
            "{}/api/v3/myTrades?{}&signature={}",
            self.base_url, query_string, signature
        );

        let response = self
            .client
            .get(&url)
            .header("X-MBX-APIKEY", api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(McpError::from(response.error_for_status().unwrap_err()));
        }

        let trades: Vec<MyTrade> = response.json().await?;
        Ok(trades)
    }

    /// Create a listen key for user data stream
    ///
    /// Calls POST /api/v3/userDataStream (requires API key)
    ///
    /// Creates a listen key valid for 60 minutes. The key must be kept alive
    /// by calling `keepalive_listen_key` every 30 minutes.
    ///
    /// # Returns
    /// * `Ok(String)` - Listen key for WebSocket user data stream
    /// * `Err(McpError)` - Error if creation fails
    ///
    /// # Example
    /// ```no_run
    /// use mcp_binance_server::binance::client::BinanceClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = BinanceClient::with_credentials();
    /// let listen_key = client.create_listen_key().await?;
    /// println!("Listen key: {}", listen_key);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_listen_key(&self) -> Result<String, McpError> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| McpError::InvalidRequest("API key not configured".to_string()))?;

        let url = format!("{}/api/v3/userDataStream", self.base_url);

        let response = self
            .client
            .post(&url)
            .header("X-MBX-APIKEY", api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(McpError::from(response.error_for_status().unwrap_err()));
        }

        #[derive(serde::Deserialize)]
        struct ListenKeyResponse {
            #[serde(rename = "listenKey")]
            listen_key: String,
        }

        let response_data: ListenKeyResponse = response.json().await?;
        Ok(response_data.listen_key)
    }

    /// Keep alive a listen key
    ///
    /// Calls PUT /api/v3/userDataStream (requires API key)
    ///
    /// Extends the validity of the listen key by 60 minutes from the current time.
    /// Must be called at least once every 30 minutes to prevent expiration.
    ///
    /// # Arguments
    /// * `listen_key` - The listen key to keep alive
    ///
    /// # Returns
    /// * `Ok(())` - Listen key successfully renewed
    /// * `Err(McpError)` - Error if renewal fails
    pub async fn keepalive_listen_key(&self, listen_key: &str) -> Result<(), McpError> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| McpError::InvalidRequest("API key not configured".to_string()))?;

        let url = format!(
            "{}/api/v3/userDataStream?listenKey={}",
            self.base_url, listen_key
        );

        let response = self
            .client
            .put(&url)
            .header("X-MBX-APIKEY", api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(McpError::from(response.error_for_status().unwrap_err()));
        }

        Ok(())
    }

    /// Close a listen key
    ///
    /// Calls DELETE /api/v3/userDataStream (requires API key)
    ///
    /// Closes the user data stream and invalidates the listen key immediately.
    ///
    /// # Arguments
    /// * `listen_key` - The listen key to close
    ///
    /// # Returns
    /// * `Ok(())` - Listen key successfully closed
    /// * `Err(McpError)` - Error if closure fails
    pub async fn close_listen_key(&self, listen_key: &str) -> Result<(), McpError> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| McpError::InvalidRequest("API key not configured".to_string()))?;

        let url = format!(
            "{}/api/v3/userDataStream?listenKey={}",
            self.base_url, listen_key
        );

        let response = self
            .client
            .delete(&url)
            .header("X-MBX-APIKEY", api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(McpError::from(response.error_for_status().unwrap_err()));
        }

        Ok(())
    }
}

impl Default for BinanceClient {
    fn default() -> Self {
        Self::new()
    }
}
