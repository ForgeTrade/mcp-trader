//! Utility functions for report generation
//!
//! This module provides shared utilities for analytics integration including
//! timeout enforcement and error logging.

use std::future::Future;
use tokio::time::{timeout, Duration};

/// Error types for analytics timeout wrapper
#[derive(Debug)]
pub enum TimeoutError {
    /// Analytics function returned an error
    Analytics(String),
    /// Function exceeded 1-second timeout
    Exceeded,
}

impl std::fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeoutError::Analytics(msg) => write!(f, "Analytics error: {}", msg),
            TimeoutError::Exceeded => write!(f, "Analytics function exceeded 1s timeout"),
        }
    }
}

impl std::error::Error for TimeoutError {}

/// Wraps an analytics function with 1-second timeout and comprehensive error logging
///
/// # Purpose
/// Implements FR-020 (1s timeout enforcement) and FR-021 (parallel execution support).
/// Each analytics function in report generation is wrapped with this utility to ensure:
/// - No single analytics function blocks report generation >1 second
/// - Timeout failures are logged for operators while sections degrade gracefully
/// - Parallel execution via tokio::join! respects per-function timeout limits
///
/// # Arguments
/// - `future`: The async analytics function to execute with timeout
/// - `function_name`: Name of the analytics function for logging (e.g., "generate_volume_profile")
/// - `symbol`: Trading pair symbol for context logging
///
/// # Returns
/// - `Ok(T)`: Analytics function succeeded within timeout
/// - `Err(TimeoutError::Analytics)`: Function returned error (logged)
/// - `Err(TimeoutError::Exceeded)`: Function exceeded 1s timeout (logged)
///
/// # Example
/// ```rust,ignore
/// // Example usage (requires analytics context)
/// let result = timeout_analytics(
///     generate_volume_profile(symbol, trades, 24, None),
///     "generate_volume_profile",
///     "BTCUSDT"
/// ).await;
///
/// match result {
///     Ok(profile) => {
///         // Use volume profile data
///     },
///     Err(TimeoutError::Exceeded) => {
///         // Section degrades gracefully with "[Data Unavailable: timeout]"
///     },
///     Err(TimeoutError::Analytics(e)) => {
///         // Section degrades with "[Data Unavailable: calculation failed]"
///     },
/// }
/// ```
pub async fn timeout_analytics<T, E, F>(
    future: F,
    function_name: &str,
    symbol: &str,
) -> Result<T, TimeoutError>
where
    F: Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    match timeout(Duration::from_secs(1), future).await {
        Ok(Ok(result)) => {
            // FR-018: Log successful execution at debug level
            tracing::debug!(
                symbol = %symbol,
                function = %function_name,
                "Analytics function completed successfully"
            );
            Ok(result)
        }
        Ok(Err(analytics_error)) => {
            // FR-018: Log detailed error context for operators
            tracing::error!(
                symbol = %symbol,
                function = %function_name,
                error = %analytics_error,
                "Analytics function failed during report generation"
            );
            Err(TimeoutError::Analytics(analytics_error.to_string()))
        }
        Err(_elapsed) => {
            // FR-018: Log timeout warning
            tracing::warn!(
                symbol = %symbol,
                function = %function_name,
                timeout_ms = 1000,
                "Analytics function exceeded timeout, section will degrade gracefully"
            );
            Err(TimeoutError::Exceeded)
        }
    }
}

/// Log error for report section rendering
///
/// Implements FR-018 error logging requirements:
/// - Detailed context for operators (function name, error type, symbol)
/// - Generic user-facing messages in report content
///
/// # Arguments
/// - `section_name`: Name of the report section (e.g., "liquidity_analysis")
/// - `symbol`: Trading pair symbol
/// - `error`: The error that occurred
///
/// # Example
/// ```rust,ignore
/// // Example usage (requires error context)
/// log_section_error("liquidity_analysis", "BTCUSDT", &error);
/// ```
pub fn log_section_error(section_name: &str, symbol: &str, error: &dyn std::error::Error) {
    tracing::error!(
        section = %section_name,
        symbol = %symbol,
        error = %error,
        "Report section rendering failed"
    );
}

/// Calculate data age in milliseconds
///
/// Implements FR-015 data age indicator requirement.
///
/// # Arguments
/// - `analytics_timestamp`: When the analytics data was calculated
/// - `now`: Current timestamp
///
/// # Returns
/// Age in milliseconds as i32 (safe for up to ~24 days)
pub fn calculate_data_age_ms(
    analytics_timestamp: chrono::DateTime<chrono::Utc>,
    now: chrono::DateTime<chrono::Utc>,
) -> i32 {
    (now - analytics_timestamp).num_milliseconds() as i32
}

/// Get data freshness indicator based on age
///
/// Maps data age to visual indicator for FR-015 compliance.
///
/// # Arguments
/// - `age_ms`: Data age in milliseconds
///
/// # Returns
/// - 🟢 Fresh (0-1s)
/// - 🟡 Recent (1-5s)
/// - 🟠 Aging (5-30s)
/// - 🔴 Stale (>30s)
pub fn data_age_indicator(age_ms: i32) -> &'static str {
    match age_ms {
        0..=1000 => "🟢 Fresh",
        1001..=5000 => "🟡 Recent",
        5001..=30000 => "🟠 Aging",
        _ => "🔴 Stale",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_data_age_indicator() {
        assert_eq!(data_age_indicator(500), "🟢 Fresh");
        assert_eq!(data_age_indicator(3000), "🟡 Recent");
        assert_eq!(data_age_indicator(15000), "🟠 Aging");
        assert_eq!(data_age_indicator(60000), "🔴 Stale");
    }

    #[test]
    fn test_calculate_data_age_ms() {
        let now = Utc::now();
        let past = now - chrono::Duration::milliseconds(2500);
        let age_ms = calculate_data_age_ms(past, now);
        assert!((age_ms - 2500).abs() < 10); // Allow small timing variance
    }

    #[tokio::test]
    async fn test_timeout_analytics_success() {
        async fn mock_analytics() -> Result<String, String> {
            Ok("success".to_string())
        }

        let result = timeout_analytics(mock_analytics(), "mock_analytics", "BTCUSDT").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
    }

    #[tokio::test]
    async fn test_timeout_analytics_error() {
        async fn mock_analytics() -> Result<String, String> {
            Err("calculation failed".to_string())
        }

        let result = timeout_analytics(mock_analytics(), "mock_analytics", "BTCUSDT").await;
        assert!(matches!(result, Err(TimeoutError::Analytics(_))));
    }

    #[tokio::test]
    async fn test_timeout_analytics_timeout() {
        async fn slow_analytics() -> Result<String, String> {
            tokio::time::sleep(Duration::from_secs(2)).await;
            Ok("too late".to_string())
        }

        let result = timeout_analytics(slow_analytics(), "slow_analytics", "BTCUSDT").await;
        assert!(matches!(result, Err(TimeoutError::Exceeded)));
    }
}
