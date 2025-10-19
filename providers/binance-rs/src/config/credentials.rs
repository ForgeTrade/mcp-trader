//! API Credential Management
//!
//! Secure handling of Binance API credentials loaded from environment variables.
//! Credentials are never logged at INFO/WARN levels and are masked when displayed.

use std::fmt;

/// Secure string wrapper that masks sensitive data in logs
///
/// This type wraps sensitive strings (API keys, secrets) and ensures they are
/// never accidentally exposed in logs or error messages. Debug output shows only
/// `SecretString(***)` and Display shows truncated form `first4...last4`.
#[derive(Clone)]
pub struct SecretString(String);

impl SecretString {
    /// Creates a new SecretString from a String
    pub fn new(value: String) -> Self {
        SecretString(value)
    }

    /// Returns a reference to the inner string
    ///
    /// **Security Warning**: Only use this when actually needed for API calls.
    /// Never log or display the returned value.
    pub fn expose_secret(&self) -> &str {
        &self.0
    }

    /// Returns a masked version of the secret for safe logging
    ///
    /// Format: `first4...last4` (e.g., "abcd...wxyz")
    pub fn masked(&self) -> String {
        let s = &self.0;
        if s.len() <= 8 {
            return "***".to_string();
        }
        format!("{}...{}", &s[..4], &s[s.len() - 4..])
    }
}

// Debug implementation masks the value completely
impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SecretString(***)")
    }
}

// Display implementation shows truncated form
impl fmt::Display for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.masked())
    }
}

impl From<String> for SecretString {
    fn from(s: String) -> Self {
        SecretString::new(s)
    }
}

/// Binance API credentials loaded from environment variables
///
/// Credentials are stored as SecretString to prevent accidental logging.
/// Use `is_some()` on `Credentials::from_env()` result to check if credentials
/// are configured.
#[derive(Clone, Debug)]
pub struct Credentials {
    /// Binance API key (public identifier)
    pub api_key: SecretString,
    /// Binance secret key (private signing key)
    pub secret_key: SecretString,
}

impl Credentials {
    /// Loads credentials from environment variables
    ///
    /// Reads `BINANCE_API_KEY` and `BINANCE_SECRET_KEY` from environment.
    /// Trims whitespace and validates non-empty.
    ///
    /// Returns `Ok(Credentials)` if both variables are set and valid.
    /// Returns `Err` with descriptive message if variables are missing or invalid.
    pub fn from_env() -> Result<Self, String> {
        let api_key = std::env::var("BINANCE_API_KEY").map_err(|_| {
            "BINANCE_API_KEY not set. Configure in Claude Desktop MCP settings:\n\
             \"env\": { \"BINANCE_API_KEY\": \"your_key\" }"
                .to_string()
        })?;

        let secret_key = std::env::var("BINANCE_SECRET_KEY").map_err(|_| {
            "BINANCE_SECRET_KEY not set. Configure in Claude Desktop MCP settings:\n\
             \"env\": { \"BINANCE_SECRET_KEY\": \"your_secret\" }"
                .to_string()
        })?;

        // Trim whitespace
        let api_key = api_key.trim().to_string();
        let secret_key = secret_key.trim().to_string();

        // Validate non-empty
        if api_key.is_empty() {
            return Err("BINANCE_API_KEY is empty after trimming whitespace".to_string());
        }
        if secret_key.is_empty() {
            return Err("BINANCE_SECRET_KEY is empty after trimming whitespace".to_string());
        }

        Ok(Self {
            api_key: SecretString::new(api_key),
            secret_key: SecretString::new(secret_key),
        })
    }
}
