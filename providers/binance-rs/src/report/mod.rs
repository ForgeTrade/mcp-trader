// Report module for unified market data reporting
//
// This module provides a unified interface for generating comprehensive
// market intelligence reports in markdown format.

pub mod formatter;
pub mod generator;
pub mod sections;
pub mod util;

// Re-export main types
pub use generator::ReportGenerator;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Configuration options for report generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportOptions {
    /// List of section names to include in the report.
    /// If None or empty, all sections are included.
    pub include_sections: Option<Vec<String>>,

    /// Time window in hours for volume profile calculation.
    /// Default: 24 hours, Valid range: 1-168 (1 hour to 7 days)
    pub volume_window_hours: Option<u32>,

    /// Number of order book levels to include in depth analysis.
    /// Default: 20 levels, Valid range: 1-100
    pub orderbook_levels: Option<u32>,
}

impl Default for ReportOptions {
    fn default() -> Self {
        Self {
            include_sections: None, // All sections
            volume_window_hours: Some(24),
            orderbook_levels: Some(20),
        }
    }
}

impl ReportOptions {
    /// Validates the report options and returns an error if any option is invalid.
    ///
    /// # Validation Rules
    /// - `volume_window_hours`: Must be between 1 and 168 (1 hour to 7 days)
    /// - `orderbook_levels`: Must be between 1 and 100
    ///
    /// # Returns
    /// - `Ok(())` if all options are valid
    /// - `Err(String)` with descriptive error message if any option is invalid
    ///
    /// # Example
    /// ```
    /// use binance_provider::report::ReportOptions;
    ///
    /// let options = ReportOptions {
    ///     volume_window_hours: Some(48),
    ///     orderbook_levels: Some(50),
    ///     ..Default::default()
    /// };
    /// assert!(options.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<(), String> {
        if let Some(hours) = self.volume_window_hours {
            if hours < 1 || hours > 168 {
                return Err(format!(
                    "volume_window_hours must be between 1 and 168, got {}",
                    hours
                ));
            }
        }

        if let Some(levels) = self.orderbook_levels {
            if levels < 1 || levels > 100 {
                return Err(format!(
                    "orderbook_levels must be between 1 and 100, got {}",
                    levels
                ));
            }
        }

        Ok(())
    }

    /// Generates a deterministic cache key suffix from the report options.
    ///
    /// This method creates a unique string representation of the options that is used
    /// as part of the cache key. Different option combinations produce different suffixes,
    /// ensuring cached reports are isolated by their configuration.
    ///
    /// # Cache Key Format
    /// `"sections:{sections};volume:{hours};levels:{levels}"`
    ///
    /// Where:
    /// - `sections`: Sorted comma-separated list of section names, or "all"
    /// - `hours`: Volume window in hours (default: 24)
    /// - `levels`: Order book depth levels (default: 20)
    ///
    /// # Example
    /// ```
    /// use binance_provider::report::ReportOptions;
    ///
    /// let options = ReportOptions {
    ///     include_sections: Some(vec!["price_overview".to_string(), "liquidity_analysis".to_string()]),
    ///     volume_window_hours: Some(48),
    ///     orderbook_levels: Some(50),
    /// };
    /// let suffix = options.to_cache_key_suffix();
    /// assert!(suffix.contains("sections:liquidity_analysis,price_overview"));
    /// assert!(suffix.contains("volume:48"));
    /// assert!(suffix.contains("levels:50"));
    /// ```
    ///
    /// # Implementation Note
    /// Section names are sorted to ensure deterministic keys regardless of input order.
    pub fn to_cache_key_suffix(&self) -> String {
        // Sort include_sections for deterministic cache key
        let sections_key = match &self.include_sections {
            None => "all".to_string(),
            Some(sections) if sections.is_empty() => "all".to_string(),
            Some(sections) => {
                let mut sorted = sections.clone();
                sorted.sort();
                sorted.join(",")
            }
        };

        // Use default values if None
        let volume_hours = self.volume_window_hours.unwrap_or(24);
        let ob_levels = self.orderbook_levels.unwrap_or(20);

        // Create deterministic cache key suffix
        format!(
            "sections:{};volume:{};levels:{}",
            sections_key, volume_hours, ob_levels
        )
    }

    /// Generates a complete cache key by combining symbol and options.
    ///
    /// This is a convenience method that combines the symbol with the options suffix
    /// to create a fully-qualified cache key.
    ///
    /// # Arguments
    /// * `symbol` - The trading pair symbol (e.g., "BTCUSDT")
    ///
    /// # Returns
    /// A cache key in the format: `"{SYMBOL}:{options_suffix}"`
    ///
    /// # Example
    /// ```
    /// use binance_provider::report::ReportOptions;
    ///
    /// let options = ReportOptions::default();
    /// let key = options.to_cache_key("BTCUSDT");
    /// assert!(key.starts_with("BTCUSDT:"));
    /// ```
    pub fn to_cache_key(&self, symbol: &str) -> String {
        format!("{}:{}", symbol, self.to_cache_key_suffix())
    }
}

/// The complete generated market intelligence report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketReport {
    /// The complete markdown-formatted report
    pub markdown_content: String,

    /// Symbol this report was generated for (e.g., "BTCUSDT")
    pub symbol: String,

    /// Unix timestamp (milliseconds) when report was generated
    pub generated_at: i64,

    /// Age of the oldest data source in milliseconds
    pub data_age_ms: i32,

    /// List of sections that failed to generate (if any)
    pub failed_sections: Vec<String>,

    /// Report generation duration in milliseconds
    pub generation_time_ms: u64,
}

/// Internal representation of a report section
#[derive(Debug, Clone)]
pub(crate) struct ReportSection {
    pub name: String,
    pub title: String,
    pub content: Result<String, SectionError>,
    pub data_age_ms: Option<i32>,
}

impl ReportSection {
    pub fn render(&self) -> String {
        match &self.content {
            Ok(markdown) => markdown.clone(),
            Err(err) => self.render_error(err),
        }
    }

    fn render_error(&self, err: &SectionError) -> String {
        format!(
            "## {}\n\n**[Data Unavailable]**\n\n{}\n\n",
            self.title,
            err.user_message()
        )
    }
}

/// Errors that can occur when building a section
#[derive(Debug, Clone)]
pub(crate) enum SectionError {
    DataSourceUnavailable(String),
    RateLimitExceeded,
    FeatureNotEnabled(String),
    Timeout,
}

impl SectionError {
    pub fn user_message(&self) -> String {
        match self {
            SectionError::DataSourceUnavailable(source) => {
                format!(
                    "The {} section could not be generated due to missing data. \
                    This may be temporary due to rate limiting or service degradation.",
                    source
                )
            }
            SectionError::RateLimitExceeded => {
                "Rate limit exceeded. Please wait a moment and try again.".to_string()
            }
            SectionError::FeatureNotEnabled(feature) => {
                format!(
                    "This build does not include {} features. \
                    Recompile with the appropriate feature flags to enable.",
                    feature
                )
            }
            SectionError::Timeout => "Data fetch timed out. Please try again.".to_string(),
        }
    }
}

/// TTL-based in-memory cache for reports
pub struct ReportCache {
    cache: Mutex<HashMap<String, (MarketReport, Instant)>>,
    ttl: Duration,
}

impl ReportCache {
    /// Creates a new report cache with the specified time-to-live (TTL).
    ///
    /// # Arguments
    /// * `ttl_secs` - Cache entry lifetime in seconds (typically 60s for Feature 018)
    ///
    /// # Example
    /// ```
    /// use binance_provider::report::ReportCache;
    ///
    /// let cache = ReportCache::new(60); // 60-second TTL
    /// ```
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
            ttl: Duration::from_secs(ttl_secs),
        }
    }

    /// Retrieves a cached report if it exists and is not expired.
    ///
    /// If the cached entry has expired, it is automatically removed from the cache.
    ///
    /// # Arguments
    /// * `symbol` - The cache key (typically includes symbol and options)
    ///
    /// # Returns
    /// - `Some(MarketReport)` if a valid cached entry exists
    /// - `None` if no entry exists or the entry has expired
    ///
    /// # Thread Safety
    /// This method is thread-safe and uses internal locking.
    ///
    /// # Example
    /// ```
    /// use binance_provider::report::ReportCache;
    ///
    /// let cache = ReportCache::new(60);
    /// if let Some(report) = cache.get("BTCUSDT:sections:all;volume:24;levels:20") {
    ///     println!("Cache hit!");
    /// }
    /// ```
    pub fn get(&self, symbol: &str) -> Option<MarketReport> {
        let mut cache = self.cache.lock().unwrap();
        if let Some((report, timestamp)) = cache.get(symbol) {
            if timestamp.elapsed() < self.ttl {
                return Some(report.clone());
            }
            cache.remove(symbol);
        }
        None
    }

    /// Stores a report in the cache with the current timestamp.
    ///
    /// If an entry with the same key already exists, it will be replaced.
    ///
    /// # Arguments
    /// * `symbol` - The cache key (typically includes symbol and options)
    /// * `report` - The market report to cache
    ///
    /// # Thread Safety
    /// This method is thread-safe and uses internal locking.
    ///
    /// # Example
    /// ```
    /// use binance_provider::report::{ReportCache, MarketReport};
    ///
    /// let cache = ReportCache::new(60);
    /// let report = MarketReport {
    ///     markdown_content: "# Report".to_string(),
    ///     symbol: "BTCUSDT".to_string(),
    ///     generated_at: 1729780000000,
    ///     data_age_ms: 100,
    ///     failed_sections: vec![],
    ///     generation_time_ms: 245,
    /// };
    /// cache.set("BTCUSDT:sections:all;volume:24;levels:20".to_string(), report);
    /// ```
    pub fn set(&self, symbol: String, report: MarketReport) {
        let mut cache = self.cache.lock().unwrap();
        cache.insert(symbol, (report, Instant::now()));
    }

    /// Invalidates all cached reports for a symbol across all option combinations.
    ///
    /// Since cache keys include both symbol and options (e.g., "BTCUSDT:sections:all;..."),
    /// this method removes all entries that start with the symbol prefix, effectively
    /// clearing all cached variants for the symbol.
    ///
    /// # Arguments
    /// * `symbol` - The trading pair symbol (e.g., "BTCUSDT")
    ///
    /// # Example
    /// ```
    /// use binance_provider::report::ReportCache;
    ///
    /// let cache = ReportCache::new(60);
    /// // ... populate cache with BTCUSDT reports ...
    /// cache.invalidate("BTCUSDT"); // Clears all BTCUSDT-related entries
    /// ```
    pub fn invalidate(&self, symbol: &str) {
        // P0 Fix: Clear all cache entries for this symbol (all option combinations)
        // Since cache keys now include options, we need to remove all keys with this symbol prefix
        let mut cache = self.cache.lock().unwrap();
        let symbol_prefix = format!("{}:", symbol.to_uppercase());
        cache.retain(|key, _| !key.starts_with(&symbol_prefix));
    }
}
