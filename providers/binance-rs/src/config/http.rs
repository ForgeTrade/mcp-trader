//! HTTP Server Configuration
//!
//! Configuration for HTTP REST API server and WebSocket connections.

use std::net::SocketAddr;

/// HTTP server configuration
///
/// ## Environment Variables
///
/// - `HTTP_HOST`: Server bind address (default: 127.0.0.1)
/// - `HTTP_PORT`: Server port (default: 8080)
/// - `HTTP_BEARER_TOKEN`: API authentication token (required)
/// - `HTTP_RATE_LIMIT`: Requests per minute per client (default: 100)
/// - `HTTP_MAX_WEBSOCKET_CONNECTIONS`: Max concurrent WebSocket connections (default: 50)
#[derive(Debug, Clone)]
pub struct HttpConfig {
    /// Server bind address
    pub addr: SocketAddr,

    /// Bearer token for API authentication
    /// Clients must include `Authorization: Bearer <token>` header
    pub bearer_token: String,

    /// Rate limit: requests per minute per client
    pub rate_limit: u32,

    /// Maximum concurrent WebSocket connections
    pub max_websocket_connections: usize,
}

impl HttpConfig {
    /// Load HTTP configuration from environment variables
    ///
    /// # Errors
    ///
    /// Returns error if HTTP_BEARER_TOKEN is not set or invalid values provided
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let host = std::env::var("HTTP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let port: u16 = std::env::var("HTTP_PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse()?;

        let bearer_token = std::env::var("HTTP_BEARER_TOKEN")
            .map_err(|_| "HTTP_BEARER_TOKEN environment variable is required")?;

        let rate_limit: u32 = std::env::var("HTTP_RATE_LIMIT")
            .unwrap_or_else(|_| "100".to_string())
            .parse()?;

        let max_websocket_connections: usize = std::env::var("HTTP_MAX_WEBSOCKET_CONNECTIONS")
            .unwrap_or_else(|_| "50".to_string())
            .parse()?;

        Ok(Self {
            addr: format!("{}:{}", host, port).parse()?,
            bearer_token,
            rate_limit,
            max_websocket_connections,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_values() {
        // This test requires HTTP_BEARER_TOKEN to be set
        // SAFETY: Test-only code, single-threaded test environment
        unsafe {
            std::env::set_var("HTTP_BEARER_TOKEN", "test_token_12345");
            std::env::remove_var("HTTP_HOST");
            std::env::remove_var("HTTP_PORT");
            std::env::remove_var("HTTP_RATE_LIMIT");
            std::env::remove_var("HTTP_MAX_WEBSOCKET_CONNECTIONS");
        }

        let config = HttpConfig::from_env().expect("Failed to load config");

        assert_eq!(config.addr.to_string(), "127.0.0.1:8080");
        assert_eq!(config.bearer_token, "test_token_12345");
        assert_eq!(config.rate_limit, 100);
        assert_eq!(config.max_websocket_connections, 50);
    }
}
