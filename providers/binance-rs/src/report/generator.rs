// Report generator - main orchestrator for creating market intelligence reports

use super::sections;
use super::{MarketReport, ReportCache, ReportOptions};
use crate::binance::BinanceClient;
use crate::orderbook::metrics;
use crate::orderbook::OrderBookManager;
use std::sync::Arc;
use std::time::Instant;

/// Main service for generating market intelligence reports
pub struct ReportGenerator {
    binance_client: Arc<BinanceClient>,
    orderbook_manager: Arc<OrderBookManager>,
    cache: Arc<ReportCache>,
}

impl ReportGenerator {
    /// Creates a new report generator with dependency injection.
    ///
    /// # Arguments
    /// * `binance_client` - Shared Binance API client for market data fetching
    /// * `orderbook_manager` - Shared order book manager for WebSocket-powered data
    /// * `cache_ttl_secs` - Cache time-to-live in seconds (typically 60s)
    ///
    /// # Example
    /// ```no_run
    /// use std::sync::Arc;
    /// use binance_provider::report::ReportGenerator;
    /// use binance_provider::binance::BinanceClient;
    /// use binance_provider::orderbook::OrderBookManager;
    ///
    /// let client = Arc::new(BinanceClient::new(/* ... */));
    /// let orderbook = Arc::new(OrderBookManager::new(/* ... */));
    /// let generator = ReportGenerator::new(client, orderbook, 60);
    /// ```
    pub fn new(
        binance_client: Arc<BinanceClient>,
        orderbook_manager: Arc<OrderBookManager>,
        cache_ttl_secs: u64,
    ) -> Self {
        Self {
            binance_client,
            orderbook_manager,
            cache: Arc::new(ReportCache::new(cache_ttl_secs)),
        }
    }

    /// Generates a comprehensive market intelligence report for the specified symbol.
    ///
    /// This is the primary method for Feature 018. It orchestrates data fetching from
    /// multiple sources in parallel, builds the requested sections, applies caching,
    /// and returns a complete markdown-formatted report.
    ///
    /// # Arguments
    /// * `symbol` - Trading pair (e.g., "BTCUSDT", "ETHUSDT")
    /// * `options` - Report customization options (sections, volume window, depth)
    ///
    /// # Returns
    /// * `Ok(MarketReport)` - Complete report with markdown content and metadata
    /// * `Err(String)` - Validation error if options are invalid
    ///
    /// # Behavior
    /// 1. Validates the provided options
    /// 2. Checks cache for existing report matching symbol + options
    /// 3. If cache miss, fetches data from Binance API and WebSocket streams in parallel
    /// 4. Builds requested report sections (or all sections if unspecified)
    /// 5. Caches the generated report for 60 seconds
    /// 6. Returns the report with generation metadata
    ///
    /// # Performance
    /// - **Cache hit**: <3ms (cached report returned with original metadata)
    /// - **Cache miss**: <500ms (parallel data fetch + report generation)
    /// - **Cache TTL**: 60 seconds
    ///
    /// # Graceful Degradation
    /// If individual data sources fail, the corresponding sections will show
    /// "[Data Unavailable]" messages instead of failing the entire report.
    ///
    /// # Example
    /// ```no_run
    /// # use binance_provider::report::{ReportGenerator, ReportOptions};
    /// # async fn example(generator: &ReportGenerator) -> Result<(), String> {
    /// // Full report with all sections
    /// let report = generator.generate_report("BTCUSDT", ReportOptions::default()).await?;
    /// println!("{}", report.markdown_content);
    ///
    /// // Custom report with specific sections
    /// let options = ReportOptions {
    ///     include_sections: Some(vec![
    ///         "price_overview".to_string(),
    ///         "liquidity_analysis".to_string()
    ///     ]),
    ///     volume_window_hours: Some(48),
    ///     orderbook_levels: Some(50),
    /// };
    /// let custom_report = generator.generate_report("ETHUSDT", options).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn generate_report(
        &self,
        symbol: &str,
        options: ReportOptions,
    ) -> Result<MarketReport, String> {
        let start_time = Instant::now();
        let symbol_upper = symbol.to_uppercase();

        // Validate options
        options.validate()?;

        // P0 Fix: Generate cache key that includes options
        // This prevents returning wrong cached reports when options differ
        let cache_key = options.to_cache_key(&symbol_upper);

        // Check cache (P1 fix: preserve metadata)
        if let Some(cached_report) = self.cache.get(&cache_key) {
            // P1 fix: Return cached report with ALL original metadata preserved
            // This ensures generation_time_ms matches the footer inside markdown_content
            // and allows consumers to reason about actual generation cost vs. cache hits
            return Ok(cached_report);
        }

        // Fetch all data sources in parallel
        let ticker_fut = self.binance_client.get_24hr_ticker(&symbol_upper);
        let orderbook_fut = self.orderbook_manager.get_order_book(&symbol_upper);

        let (ticker_result, orderbook_result) = tokio::join!(ticker_fut, orderbook_fut);

        // Calculate data age
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let data_age_ms = 500; // Placeholder for actual age calculation

        // Build sections
        let ticker_data = ticker_result.ok();
        let orderbook_data = orderbook_result.ok();
        let orderbook_metrics = orderbook_data
            .as_ref()
            .and_then(|ob| metrics::calculate_metrics(ob));

        let mut failed_sections = Vec::new();

        // Build all sections first
        let header = sections::build_report_header(&symbol_upper, now_ms, data_age_ms);
        let price = sections::build_price_overview_section(ticker_data.as_ref());
        let orderbook = sections::build_orderbook_metrics_section(orderbook_metrics.as_ref());
        let volume_hours = options.volume_window_hours.unwrap_or(24);
        let liquidity =
            sections::build_liquidity_analysis_section(orderbook_metrics.as_ref(), volume_hours); // T033-T037: Pass volume window
        let microstructure = sections::build_microstructure_section();
        let anomalies = sections::build_anomalies_section(Some(now_ms)); // T028-T032: Pass timestamp for enhanced display
        let health = sections::build_health_section();
        let data_health = sections::build_data_health_section(data_age_ms);

        // P1 fix: Honor ReportOptions.include_sections
        let should_include_section = |section_name: &str| -> bool {
            match &options.include_sections {
                None => true,                          // Include all
                Some(list) if list.is_empty() => true, // Include all
                Some(list) => list.contains(&section_name.to_string()),
            }
        };

        // Collect failed sections (only for included sections)
        let all_sections = vec![
            ("price_overview", &price),
            ("orderbook_metrics", &orderbook),
            ("liquidity_analysis", &liquidity),
            ("market_anomalies", &anomalies),
            ("microstructure_health", &health),
        ];

        for (name, section) in &all_sections {
            if should_include_section(name) && section.content.is_err() {
                failed_sections.push(section.name.clone());
            }
        }

        // Assemble markdown (P1 fix: filter by include_sections)
        let mut markdown = String::new();
        markdown.push_str(&header.render()); // Header always included

        if should_include_section("price_overview") {
            markdown.push_str(&price.render());
        }
        if should_include_section("orderbook_metrics") {
            markdown.push_str(&orderbook.render());
        }
        if should_include_section("liquidity_analysis") {
            markdown.push_str(&liquidity.render());
        }
        if should_include_section("market_microstructure") {
            markdown.push_str(&microstructure.render());
        }
        if should_include_section("market_anomalies") {
            markdown.push_str(&anomalies.render());
        }
        if should_include_section("microstructure_health") {
            markdown.push_str(&health.render());
        }
        if should_include_section("data_health") {
            markdown.push_str(&data_health.render());
        }

        let generation_time_ms = start_time.elapsed().as_millis() as i32;

        // T043: Add footer to fresh report
        let footer = sections::build_report_footer(generation_time_ms, false);
        markdown.push_str(&footer);

        // Build complete report
        let report = MarketReport {
            markdown_content: markdown,
            symbol: symbol_upper.clone(),
            generated_at: now_ms,
            data_age_ms,
            failed_sections,
            generation_time_ms: generation_time_ms as u64,
        };

        // Cache result (P0 fix: use cache_key that includes options)
        self.cache.set(cache_key, report.clone());

        Ok(report)
    }

    /// Invalidates all cached reports for a symbol across all option combinations.
    ///
    /// This method clears all cached report entries for the specified symbol,
    /// regardless of the options used when generating them. This is useful when
    /// you need to force regeneration of reports due to:
    /// - Significant market events
    /// - Data quality issues
    /// - Manual cache invalidation requests
    ///
    /// # Arguments
    /// * `symbol` - The trading pair symbol to invalidate (e.g., "BTCUSDT")
    ///
    /// # Example
    /// ```no_run
    /// # use binance_provider::report::ReportGenerator;
    /// # fn example(generator: &ReportGenerator) {
    /// // Invalidate all cached BTCUSDT reports
    /// generator.invalidate_cache("BTCUSDT");
    ///
    /// // Next call to generate_report will fetch fresh data
    /// # }
    /// ```
    pub fn invalidate_cache(&self, symbol: &str) {
        self.cache.invalidate(symbol);
    }
}
