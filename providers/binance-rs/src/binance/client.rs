//! Binance HTTP Client
//!
//! HTTP client wrapper for making requests to Binance REST API.
//! Provides timeout configuration, user-agent headers, and request signing.

use crate::binance::types::{
    KlineData, OrderBook, ServerTimeResponse, Ticker24hr, TickerPrice, Trade,
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
}

impl Default for BinanceClient {
    fn default() -> Self {
        Self::new()
    }
}
