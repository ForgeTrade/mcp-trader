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
    /// Create new report generator with dependencies injected
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

    /// Generate market report for symbol with options
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
            let cache_retrieval_time = start_time.elapsed().as_millis() as i32;

            // P1 fix: Don't append footer to cached report - it already has one from initial generation
            // Cached reports are stored WITH footer, so returning as-is avoids duplication
            return Ok(MarketReport {
                markdown_content: cached_report.markdown_content,
                symbol: cached_report.symbol,
                generated_at: cached_report.generated_at,
                data_age_ms: cached_report.data_age_ms, // Preserved from original
                failed_sections: cached_report.failed_sections, // Preserved from original
                generation_time_ms: cache_retrieval_time as u64, // Cache retrieval time
            });
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

    /// Clear cached report for symbol
    pub fn invalidate_cache(&self, symbol: &str) {
        self.cache.invalidate(symbol);
    }
}
