# Data Model: Unified Market Data Report

**Feature**: 018-market-data-report
**Date**: 2025-10-23
**Phase**: 1 (Design & Contracts)

## Core Entities

### 1. ReportOptions

Configuration parameters for customizing report generation.

**Purpose**: Allows users to control report content, time windows, and depth of analysis.

**Rust Struct**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportOptions {
    /// List of section names to include in the report.
    /// If empty/None, all sections are included.
    /// Valid values: "price_overview", "orderbook_metrics", "liquidity_analysis",
    /// "market_microstructure", "market_anomalies", "microstructure_health", "data_health"
    pub include_sections: Option<Vec<String>>,

    /// Time window in hours for volume profile calculation.
    /// Default: 24 hours
    /// Valid range: 1-168 (1 hour to 7 days)
    pub volume_window_hours: Option<u32>,

    /// Number of order book levels to include in depth analysis.
    /// Default: 20 levels
    /// Valid range: 1-100
    pub orderbook_levels: Option<u32>,
}

impl Default for ReportOptions {
    fn default() -> Self {
        Self {
            include_sections: None,  // All sections
            volume_window_hours: Some(24),
            orderbook_levels: Some(20),
        }
    }
}
```

**Validation Rules**:
- `volume_window_hours`: Must be between 1 and 168 (7 days)
- `orderbook_levels`: Must be between 1 and 100
- `include_sections`: Must contain only valid section names (see list above)

**State Transitions**: Immutable configuration object - no state changes.

---

### 2. MarketReport

The complete generated market intelligence report in markdown format.

**Purpose**: Encapsulates the full report with metadata about generation time and data freshness.

**Rust Struct**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketReport {
    /// The complete markdown-formatted report
    pub markdown_content: String,

    /// Symbol this report was generated for (e.g., "BTCUSDT")
    pub symbol: String,

    /// Unix timestamp (milliseconds) when report was generated
    pub generated_at: i64,

    /// Age of the oldest data source in milliseconds
    /// Used to indicate data freshness
    pub data_age_ms: i32,

    /// List of sections that failed to generate (if any)
    pub failed_sections: Vec<String>,

    /// Report generation duration in milliseconds
    pub generation_time_ms: u64,
}
```

**Validation Rules**:
- `markdown_content`: Must not be empty
- `symbol`: Must match `[A-Z0-9]+` pattern (e.g., BTCUSDT, ETHUSDT)
- `generated_at`: Must be valid Unix timestamp
- `data_age_ms`: Non-negative integer
- `failed_sections`: Can be empty (ideal case)

**Relationships**: Contains aggregated data from multiple data sources (Ticker24hr, OrderBookMetrics, Analytics).

---

### 3. ReportSection (Internal)

Individual section builder for modular report construction.

**Purpose**: Internal abstraction for building independent report sections with graceful failure handling.

**Rust Struct** (Internal only, not exposed in API):
```rust
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

#[derive(Debug, Clone)]
pub(crate) enum SectionError {
    DataSourceUnavailable(String),
    RateLimitExceeded,
    FeatureNotEnabled(String),
    Timeout,
}
```

**State Transitions**: Built once during report generation, rendered once, then discarded.

---

### 4. ReportGenerator (Service)

The main service orchestrating report generation.

**Purpose**: Coordinates fetching data from multiple sources, building sections, and assembling the final report.

**Rust Struct**:
```rust
pub struct ReportGenerator {
    /// Binance REST API client for ticker/orderbook data
    client: Arc<BinanceClient>,

    /// Real-time orderbook manager
    orderbook_manager: Arc<OrderBookManager>,

    /// Analytics subsystem (conditionally compiled)
    #[cfg(feature = "orderbook_analytics")]
    analytics: Arc<AnalyticsEngine>,

    /// Report cache (TTL-based in-memory cache)
    cache: Arc<Mutex<ReportCache>>,

    /// Configuration for cache TTL
    cache_ttl_secs: u64,
}

impl ReportGenerator {
    /// Create new report generator with dependencies injected
    pub fn new(
        client: Arc<BinanceClient>,
        orderbook_manager: Arc<OrderBookManager>,
        #[cfg(feature = "orderbook_analytics")]
        analytics: Arc<AnalyticsEngine>,
        cache_ttl_secs: u64,
    ) -> Self {
        Self {
            client,
            orderbook_manager,
            #[cfg(feature = "orderbook_analytics")]
            analytics,
            cache: Arc::new(Mutex::new(ReportCache::new(cache_ttl_secs))),
            cache_ttl_secs,
        }
    }

    /// Generate market report for symbol with options
    pub async fn generate_report(
        &self,
        symbol: &str,
        options: ReportOptions,
    ) -> Result<MarketReport, ReportError>;

    /// Clear cached report for symbol
    pub fn invalidate_cache(&self, symbol: &str);
}
```

**Validation Rules**:
- Constructor: All Arc dependencies must be valid (non-null)
- `generate_report`: Symbol must be valid, options must pass validation
- Report generation must complete within timeout (5 seconds for uncached)

**State Transitions**: Stateless except for internal cache - can handle concurrent requests.

---

## Supporting Data Types (Reused from Existing Codebase)

### Ticker24hr

24-hour ticker statistics from Binance API.

**Source**: `binance/types.rs` (existing)

**Key Fields Used in Report**:
- `symbol`: Trading pair symbol
- `last_price`: Current price
- `price_change`: Absolute 24h price change
- `price_change_percent`: Percentage 24h change
- `high_price`, `low_price`: 24h high/low
- `volume`: 24h base asset volume
- `quote_volume`: 24h quote asset volume

---

### OrderBookMetrics

L1 aggregated orderbook metrics.

**Source**: `orderbook/types.rs` (existing)

**Key Fields Used in Report**:
- `spread_bps`: Bid-ask spread in basis points
- `microprice`: Fair price estimate
- `bid_volume`, `ask_volume`: Total liquidity on each side
- `imbalance_ratio`: Buy/sell pressure indicator (-1 to +1)
- `best_bid`, `best_ask`: Top of book prices
- `walls`: Large orders detected (support/resistance)
- `slippage_estimates`: Expected slippage for various order sizes

---

### OrderBookDepth

L2 orderbook depth with compact encoding.

**Source**: `orderbook/types.rs` (existing)

**Key Fields Used in Report**:
- `bids`, `asks`: Arrays of [price, quantity] pairs
- `price_scale`, `qty_scale`: Decoding factors for compact representation
- `timestamp`: Data timestamp

---

### MarketMicrostructureAnomaly

Detected market anomalies.

**Source**: `orderbook/analytics/types.rs` (existing, feature-gated)

**Key Fields Used in Report**:
- `anomaly_type`: QuoteStuffing, IcebergOrder, FlashCrashRisk
- `severity`: Low, Medium, High, Critical
- `detection_timestamp`: When detected
- `affected_price_level`: Price level affected
- `description`: Human-readable description
- `recommendation`: Actionable guidance

---

### MicrostructureHealth

Composite market health assessment.

**Source**: `orderbook/analytics/types.rs` (existing, feature-gated)

**Key Fields Used in Report**:
- `composite_score`: 0-100 overall health score
- `component_scores`: Breakdown (spread stability, liquidity depth, flow balance, update rate)
- `health_status`: Healthy, Degraded, Poor, Critical
- `warnings`: List of current warnings
- `recommendations`: Actionable recommendations

---

### OrderBookHealth

Service health monitoring for orderbook data feeds.

**Source**: `orderbook/types.rs` (existing)

**Key Fields Used in Report**:
- `status`: Ok, Degraded, Error
- `orderbook_symbols_active`: Number of active symbols
- `last_update_age_ms`: Time since last update
- `websocket_connected`: Connection status
- `timestamp`: Health check timestamp
- `reason`: Optional error message

---

## Data Flow Diagram

```
User Request (symbol="BTCUSDT", options=default)
    ↓
ReportGenerator.generate_report()
    ↓
Check Cache → [Hit] → Return cached MarketReport
    ↓ [Miss]
Parallel Data Fetching (tokio::join!)
    ├─→ BinanceClient.get_24hr_ticker()        → Ticker24hr
    ├─→ OrderBookManager.get_metrics()         → OrderBookMetrics
    ├─→ OrderBookManager.get_depth()           → OrderBookDepth
    ├─→ AnalyticsEngine.detect_anomalies()     → Vec<Anomaly>       [if feature enabled]
    ├─→ AnalyticsEngine.get_health()           → MicrostructureHealth [if feature enabled]
    └─→ OrderBookManager.get_orderbook_health() → OrderBookHealth
    ↓
Build ReportSection for each data source
    ├─→ build_price_overview(Ticker24hr)       → Section 1
    ├─→ build_orderbook_metrics(Metrics)       → Section 2
    ├─→ build_liquidity_analysis(Depth)        → Section 3
    ├─→ build_microstructure(Analytics)        → Section 4
    ├─→ build_anomalies(Vec<Anomaly>)          → Section 5
    ├─→ build_health(MicrostructureHealth)     → Section 6
    └─→ build_data_health(OrderBookHealth)     → Section 7
    ↓
Assemble Sections into Markdown
    ├─→ Add report header (symbol, timestamp, data age)
    ├─→ Concatenate section markdown
    └─→ Add report footer
    ↓
Create MarketReport
    ├─→ markdown_content: String
    ├─→ generated_at: current timestamp
    ├─→ data_age_ms: max(all source ages)
    ├─→ failed_sections: collect errors
    └─→ generation_time_ms: elapsed time
    ↓
Cache Report (TTL = 60s)
    ↓
Return MarketReport to User
```

---

## Validation Summary

| Entity | Key Validations |
|--------|-----------------|
| ReportOptions | `volume_window_hours` ∈ [1, 168], `orderbook_levels` ∈ [1, 100], section names valid |
| MarketReport | Non-empty content, valid symbol pattern, valid timestamp |
| ReportGenerator | Non-null dependencies, symbol validity, timeout enforcement |
| All timestamps | Unix milliseconds, non-negative |
| All currency amounts | Non-negative, precision limited to exchange rules |

---

## Next Steps

With the data model defined:
1. Generate gRPC contracts (protobuf definitions) in `contracts/`
2. Create quickstart guide demonstrating report generation
3. Proceed to task breakdown (`/speckit.tasks`)
