# Research: Unified Market Data Report

**Feature**: 018-market-data-report
**Date**: 2025-10-23
**Phase**: 0 (Outline & Research)

## Research Tasks Completed

### 1. Markdown Generation in Rust

**Decision**: Use simple string formatting with `format!()` macro and `String::push_str()`

**Rationale**:
- No external markdown library needed for our use case
- Simple table/list formatting can be done with string templates
- Rust's `format!()` macro provides clean, readable templating
- Reduces dependencies and compilation time

**Alternatives Considered**:
- **comrak** (CommonMark parser): Over-engineered for our needs - we're generating markdown, not parsing it
- **markdown-rs**: Primarily a parser, not a generator
- **Custom builder pattern**: Would add unnecessary abstraction for straightforward template logic

**Implementation Approach**:
```rust
// Example section builder
fn build_price_overview_section(ticker: &Ticker24hr) -> String {
    format!(
        "## Price Overview\n\n\
        | Metric | Value |\n\
        |--------|-------|\n\
        | Current Price | ${} |\n\
        | 24h Change | {}% |\n\
        | 24h High | ${} |\n\
        | 24h Low | ${} |\n\
        | 24h Volume | {} |\n\n",
        ticker.last_price,
        ticker.price_change_percent,
        ticker.high_price,
        ticker.low_price,
        ticker.volume
    )
}
```

---

### 2. Parallel Data Fetching Strategy

**Decision**: Use `tokio::join!` macro for concurrent data fetching

**Rationale**:
- Tokio already used throughout the codebase
- `join!` macro provides ergonomic concurrent execution
- Faster report generation by parallelizing independent API calls
- Built-in error propagation with `Result` handling

**Alternatives Considered**:
- **futures::join_all**: More verbose, requires collecting into Vec
- **tokio::spawn + manual join handles**: Over-complicates simple concurrent fetching
- **Sequential fetching**: Too slow - 8 data sources would serialize to 8+ seconds

**Implementation Approach**:
```rust
pub async fn generate_report(
    &self,
    symbol: &str,
    options: ReportOptions,
) -> Result<String, ReportError> {
    // Fetch all data sources in parallel
    let (ticker_result, orderbook_result, analytics_result, health_result) = tokio::join!(
        self.client.get_24hr_ticker(symbol),
        self.orderbook_manager.get_metrics(symbol),
        self.fetch_analytics(symbol, &options),
        self.orderbook_manager.get_health()
    );

    // Handle errors gracefully per section
    let report = self.build_report(
        ticker_result.ok(),
        orderbook_result.ok(),
        analytics_result.ok(),
        health_result.ok(),
    );

    Ok(report)
}
```

---

### 3. Error Handling and Graceful Degradation

**Decision**: Use `Option<T>` for section data, render placeholder if `None`

**Rationale**:
- Sections should be independent - one failure shouldn't block the entire report
- Users need visibility into which data is unavailable
- Consistent with spec requirement FR-005 (graceful degradation)

**Alternatives Considered**:
- **Fail-fast approach**: Violates FR-005 requirement for partial reports
- **Default values**: Misleading - better to show "[Data Unavailable]" than fake zeros
- **Retry logic**: Adds complexity and latency - better to use cached data or show unavailable

**Implementation Approach**:
```rust
fn build_section<T>(
    section_name: &str,
    data: Option<T>,
    builder: fn(&T) -> String,
) -> String {
    match data {
        Some(d) => builder(&d),
        None => format!(
            "## {}\n\n**[Data Unavailable]**\n\n\
            The {} section could not be generated due to missing data. \
            This may be temporary due to rate limiting or service degradation.\n\n",
            section_name, section_name.to_lowercase()
        ),
    }
}
```

---

### 4. Report Caching Strategy

**Decision**: Leverage existing `SnapshotStorage` for orderbook data, add TTL-based cache for complete reports

**Rationale**:
- Orderbook data already cached via `SnapshotStorage` with real-time updates
- Complete report caching reduces repeated markdown generation overhead
- 30-60 second TTL balances freshness with performance (meets <3s cached request goal)

**Alternatives Considered**:
- **No caching**: Violates SC-001 performance requirement (<3s cached requests)
- **Redis-only caching**: Over-engineered for local report caching
- **Perpetual cache**: Stale data risk - market data changes rapidly

**Implementation Approach**:
```rust
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct ReportCache {
    cache: HashMap<String, (String, Instant)>,
    ttl: Duration,
}

impl ReportCache {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            cache: HashMap::new(),
            ttl: Duration::from_secs(ttl_secs),
        }
    }

    pub fn get(&mut self, symbol: &str) -> Option<String> {
        if let Some((report, timestamp)) = self.cache.get(symbol) {
            if timestamp.elapsed() < self.ttl {
                return Some(report.clone());
            }
            self.cache.remove(symbol);
        }
        None
    }

    pub fn set(&mut self, symbol: String, report: String) {
        self.cache.insert(symbol, (report, Instant::now()));
    }
}
```

---

### 5. gRPC Service Definition Updates

**Decision**: Add `GenerateMarketReport` RPC method, deprecate order management methods

**Rationale**:
- gRPC allows graceful deprecation with `deprecated` option
- Breaking change documented but backward-incompatible removal prevents silent failures
- New method follows existing naming conventions (`GetTicker`, `GetOrderBook` → `GenerateMarketReport`)

**Alternatives Considered**:
- **Immediate removal without deprecation**: Too abrupt for existing clients
- **Versioned API (v2)**: Over-engineered for a read-only simplification
- **Keep order management as stubs**: Misleading and maintains attack surface

**Implementation Approach** (contracts/market-report.proto):
```protobuf
syntax = "proto3";

package binance;

service BinanceProvider {
  // New unified method
  rpc GenerateMarketReport(MarketReportRequest) returns (MarketReportResponse);

  // Deprecated - will be removed in next major version
  rpc PlaceOrder(PlaceOrderRequest) returns (Order) [deprecated = true];
  rpc CancelOrder(CancelOrderRequest) returns (Order) [deprecated = true];
  rpc GetOrder(GetOrderRequest) returns (Order) [deprecated = true];
  // ... other deprecated methods
}

message MarketReportRequest {
  string symbol = 1;
  repeated string include_sections = 2;  // Optional filter
  int32 volume_window_hours = 3;         // Default 24
  int32 orderbook_levels = 4;            // Default 20
}

message MarketReportResponse {
  string markdown_report = 1;
  int64 generated_at = 2;  // Unix timestamp
  int32 data_age_ms = 3;   // Oldest data source age
}
```

---

### 6. Feature Flag Handling for Analytics

**Decision**: Conditional compilation with `#[cfg(feature = "orderbook_analytics")]` for advanced sections

**Rationale**:
- Existing codebase already uses `orderbook_analytics` feature flag
- Compile-time feature detection prevents runtime crashes
- Report gracefully shows "Feature not available" for disabled analytics

**Alternatives Considered**:
- **Runtime feature detection**: Adds overhead and complexity
- **Separate report variants**: Code duplication
- **Mandatory analytics**: Forces all deployments to include heavyweight analytics

**Implementation Approach**:
```rust
#[cfg(feature = "orderbook_analytics")]
fn build_analytics_sections(&self, symbol: &str) -> Result<String, ReportError> {
    // Advanced analytics sections
    let anomalies = self.analytics.detect_anomalies(symbol)?;
    let microstructure = self.analytics.get_health(symbol)?;
    // ... build sections
}

#[cfg(not(feature = "orderbook_analytics"))]
fn build_analytics_sections(&self, _symbol: &str) -> Result<String, ReportError> {
    Ok(String::from(
        "## Advanced Analytics\n\n\
        **[Feature Not Available]**\n\n\
        This build does not include advanced analytics features. \
        Recompile with `--features orderbook_analytics` to enable.\n\n"
    ))
}
```

---

### 7. Python MCP Gateway Integration

**Decision**: Add new tool registration `binance_generate_market_report`, update fetch.py document handler

**Rationale**:
- Follows existing MCP tool naming convention (`binance_get_ticker`, `binance_get_orderbook`)
- Minimal changes to gateway - just tool registration and gRPC proxy
- Document-based fetching via `fetch("report:BTCUSDT")` maintains consistent UX

**Alternatives Considered**:
- **Replace all existing tools**: Breaking change for current MCP users
- **Separate service**: Over-engineered for a simple proxy layer
- **Direct Rust MCP server**: Would require rewriting entire gateway

**Implementation Approach** (mcp_gateway/tools/fetch.py):
```python
def fetch(document_id: str) -> Dict[str, Any]:
    """Fetch document by ID (ticker:SYMBOL, orderbook:SYMBOL, report:SYMBOL)"""
    doc_type, symbol = document_id.split(":", 1)

    if doc_type == "report":
        # New unified report fetching
        report = grpc_client.generate_market_report(
            symbol=symbol,
            include_sections=None,  # All sections by default
            volume_window_hours=24,
            orderbook_levels=20,
        )
        return {
            "id": document_id,
            "type": "market_report",
            "content": report.markdown_report,
            "generated_at": report.generated_at,
            "data_age_ms": report.data_age_ms,
        }
    # ... existing handlers for ticker, orderbook
```

---

## Technology Stack Summary

| Component | Technology | Justification |
|-----------|------------|---------------|
| Markdown Generation | Native Rust `format!()` | Simple, no dependencies, performant |
| Async Execution | tokio::join! | Already used, ergonomic, parallel execution |
| Error Handling | Option<T> + Result<T,E> | Idiomatic Rust, graceful degradation |
| Caching | In-memory HashMap + TTL | Lightweight, meets <3s cached requirement |
| gRPC Interface | Protobuf with deprecated flag | Backward compatibility signal |
| Feature Flags | Cargo features (`cfg`) | Compile-time conditional compilation |
| Python Integration | Minimal MCP tool addition | Follows existing patterns |

---

## Open Questions Resolved

1. **Q: Should we version the API?**
   - **A**: No. This is a breaking simplification, not a parallel v2. Deprecation flags provide transition period.

2. **Q: How to handle rate limiting from Binance API?**
   - **A**: Use existing cached data from `SnapshotStorage` and `OrderBookManager`. Report shows data age to indicate staleness.

3. **Q: Should reports be persisted for historical analysis?**
   - **A**: No (per spec "Out of Scope"). Reports are ephemeral snapshots. Future feature can add persistence.

4. **Q: What if all data sources fail simultaneously?**
   - **A**: Return error per FR-008. Minimum requirement: ticker + basic orderbook must succeed or report generation fails fast.

---

## Next Phase: Design & Contracts

With all technology decisions made, proceed to **Phase 1**:
- Define data model for `ReportOptions`, `MarketReport`, section structures
- Generate complete gRPC contract (market-report.proto)
- Create quickstart guide for using the unified report method
