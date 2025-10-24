# Quickstart: Unified Market Data Report

**Feature**: 018-market-data-report
**Audience**: Developers, traders, and LLM agents using the mcp-trader system
**Last Updated**: 2025-10-23

## Overview

The Unified Market Data Report feature consolidates all market data retrieval into a single method that generates comprehensive markdown-formatted reports. This replaces the need to call 8+ individual methods to gather pricing, liquidity, and health data.

**What's New**:
- ✅ Single method: `generate_market_report(symbol, options)` replaces multiple calls
- ✅ Markdown output: Human-readable and LLM-friendly formatted reports
- ✅ 7-8 report sections: Price, orderbook, liquidity, anomalies, health
- ❌ Order management removed: No more `place_order`, `cancel_order`, etc.
- ❌ Account queries removed: `get_account`, `get_my_trades` no longer available

---

## Quick Examples

### Example 1: Generate Basic Report (Default Options)

**Rust (binance-rs provider)**:
```rust
use binance_rs::report::ReportGenerator;
use binance_rs::report::ReportOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize dependencies (client, orderbook manager, cache)
    let generator = ReportGenerator::new(
        client,
        orderbook_manager,
        #[cfg(feature = "orderbook_analytics")]
        analytics,
        60, // cache TTL in seconds
    );

    // Generate report with default options
    let report = generator.generate_report("BTCUSDT", ReportOptions::default()).await?;

    println!("Generated report at: {}", report.generated_at);
    println!("Data age: {}ms", report.data_age_ms);
    println!("\n{}", report.markdown_content);

    Ok(())
}
```

**Python (MCP Gateway)**:
```python
from mcp_gateway.tools.fetch import fetch

# Fetch unified market report
report = fetch("report:BTCUSDT")

print(f"Report Type: {report['type']}")
print(f"Generated At: {report['generated_at']}")
print(f"Data Age: {report['data_age_ms']}ms")
print("\nMarkdown Report:\n")
print(report['content'])
```

**gRPC Client (Any Language)**:
```python
import grpc
from binance.market_report_pb2 import MarketReportRequest
from binance.market_report_pb2_grpc import MarketReportServiceStub

# Connect to service
channel = grpc.insecure_channel('localhost:50051')
stub = MarketReportServiceStub(channel)

# Generate report
request = MarketReportRequest(symbol="BTCUSDT")
response = stub.GenerateMarketReport(request)

print(response.markdown_report)
```

---

### Example 2: Customized Report (Specific Sections)

Generate a report with only price and liquidity sections:

**Rust**:
```rust
let options = ReportOptions {
    include_sections: Some(vec![
        "price_overview".to_string(),
        "liquidity_analysis".to_string(),
    ]),
    volume_window_hours: Some(24),
    orderbook_levels: Some(20),
};

let report = generator.generate_report("ETHUSDT", options).await?;
```

**gRPC**:
```python
request = MarketReportRequest(
    symbol="ETHUSDT",
    include_sections=["price_overview", "liquidity_analysis"],
    volume_window_hours=24,
    orderbook_levels=20,
)
response = stub.GenerateMarketReport(request)
```

---

### Example 3: Extended Volume Analysis (7-day window)

Analyze volume distribution over a longer timeframe:

**Rust**:
```rust
let options = ReportOptions {
    include_sections: None,  // All sections
    volume_window_hours: Some(168),  // 7 days
    orderbook_levels: Some(50),      // Deeper orderbook
};

let report = generator.generate_report("SOLUSDT", options).await?;
```

**gRPC**:
```python
request = MarketReportRequest(
    symbol="SOLUSDT",
    volume_window_hours=168,  # 7 days
    orderbook_levels=50,
)
response = stub.GenerateMarketReport(request)
```

---

### Example 4: Force Refresh (Bypass Cache)

Get the absolute latest data bypassing cache:

**gRPC**:
```python
request = MarketReportRequest(
    symbol="BTCUSDT",
    force_refresh=True,  # Bypass cache
)
response = stub.GenerateMarketReport(request)
print(f"From cache: {response.from_cache}")  # Should be False
```

---

## Report Structure

Every generated report includes these sections (when data is available):

### 1. Report Header
- Symbol
- Generation timestamp
- Data age indicator
- Cache status

### 2. Price Overview
- Current price
- 24h change (absolute and percentage)
- 24h high/low
- 24h volume (base and quote)

### 3. Order Book Metrics
- Bid-ask spread (basis points)
- Microprice (fair value estimate)
- Bid/ask volume
- Imbalance ratio
- Best bid/ask prices

### 4. Liquidity Analysis
- Major liquidity walls (support/resistance)
- Volume profile with POC/VAH/VAL indicators
- Liquidity vacuum zones

### 5. Market Microstructure
- Order flow direction
- Bid/ask flow rates
- Net flow and cumulative delta

### 6. Market Anomalies
- Detected anomalies (quote stuffing, iceberg orders, flash crash risk)
- Severity levels
- Recommendations

### 7. Microstructure Health
- Composite health score (0-100)
- Component scores breakdown
- Health status (Healthy/Degraded/Poor/Critical)
- Warnings and recommendations

### 8. Data Health Status
- WebSocket connectivity
- Last update age
- Overall service status

---

## Sample Report Output

```markdown
# Market Intelligence Report: BTCUSDT

**Generated**: 2025-10-23 14:32:15 UTC
**Data Age**: 245ms (Fresh)
**Cache Status**: Hit (from cache)

---

## Price Overview

| Metric | Value |
|--------|-------|
| Current Price | $43,256.78 |
| 24h Change | +$1,234.56 (+2.94%) |
| 24h High | $43,500.00 |
| 24h Low | $41,800.00 |
| 24h Volume | 12,345.67 BTC |
| 24h Quote Volume | $534,567,890 USDT |

---

## Order Book Metrics

| Metric | Value |
|--------|-------|
| Spread | 2.3 bps |
| Microprice | $43,256.50 |
| Bid Volume | 156.78 BTC |
| Ask Volume | 142.34 BTC |
| Imbalance Ratio | +0.047 (Slight Buy Pressure) |

**Best Bid**: $43,256.00 | **Best Ask**: $43,257.00

### Liquidity Walls

- **Buy Wall**: $43,100 (125.5 BTC) - Strong support
- **Sell Wall**: $43,500 (89.2 BTC) - Moderate resistance

---

## Liquidity Analysis

### Volume Profile (24h)

- **Point of Control (POC)**: $43,150 (Highest volume zone)
- **Value Area High (VAH)**: $43,400
- **Value Area Low (VAL)**: $42,900

### Liquidity Vacuums

1. **$43,350 - $43,450** (Medium Impact)
   - Volume deficit: 45%
   - Potential rapid price movement zone

---

## Market Microstructure

**Order Flow Direction**: MODERATE_BUY
**Bid Flow Rate**: 12.5 BTC/min
**Ask Flow Rate**: 10.3 BTC/min
**Net Flow**: +2.2 BTC/min (Buy pressure)

---

## Market Anomalies

✅ **No anomalies detected**

Last analysis: 2025-10-23 14:32:00 UTC

---

## Microstructure Health

**Composite Score**: 87/100 (Healthy)

| Component | Score |
|-----------|-------|
| Spread Stability | 92/100 |
| Liquidity Depth | 85/100 |
| Flow Balance | 84/100 |
| Update Rate | 88/100 |

**Status**: ✅ Healthy

---

## Data Health Status

| Indicator | Status |
|-----------|--------|
| Service Status | ✅ Healthy |
| WebSocket | ✅ Connected |
| Active Symbols | 47 |
| Last Update Age | 245ms |

---

*Report generated in 1,234ms | Feature build: [orderbook_analytics]*
```

---

## Performance Guidelines

| Scenario | Expected Time | Cache Strategy |
|----------|---------------|----------------|
| First request (cold) | <5 seconds | Cache miss → fetch all data |
| Cached request | <3 seconds | Cache hit → return immediately |
| Force refresh | <5 seconds | Bypass cache → fetch fresh data |
| Concurrent requests (different symbols) | <5 seconds each | Independent cache entries |

---

## Error Handling

### Common Errors and Solutions

**Error: Invalid Symbol**
```
Error: INVALID_SYMBOL - Symbol 'INVALID' not found or unsupported
Solution: Use valid uppercase symbols like "BTCUSDT", "ETHUSDT"
```

**Error: Data Unavailable**
```
Error: DATA_UNAVAILABLE - Required data sources unavailable
Solution: Wait a few seconds and retry. Check service health.
```

**Error: Rate Limit Exceeded**
```
Error: RATE_LIMIT_EXCEEDED - Exchange rate limit hit
Solution: Use cached data or reduce request frequency
```

**Error: Timeout**
```
Error: TIMEOUT - Report generation exceeded 5 second timeout
Solution: Reduce orderbook_levels or retry with default options
```

### Graceful Degradation

Reports continue to generate even when some sections fail:

```rust
let report = generator.generate_report("BTCUSDT", options).await?;

if !report.failed_sections.is_empty() {
    println!("Warning: Some sections failed to generate:");
    for section in &report.failed_sections {
        println!("  - {}", section);
    }
}

// Report still contains available sections
println!("{}", report.markdown_content);
```

---

## Service Health Check

Before generating reports, check service health:

**gRPC**:
```python
from binance.market_report_pb2 import ServiceHealthRequest

request = ServiceHealthRequest(symbols=["BTCUSDT", "ETHUSDT"])
health = stub.GetServiceHealth(request)

if health.status == HealthStatus.HEALTHY:
    print("✅ Service is healthy")
    print(f"Active symbols: {health.active_symbols}")
    print(f"WebSocket: {'Connected' if health.websocket_connected else 'Disconnected'}")
else:
    print(f"⚠️  Service status: {health.status}")
    print(f"Message: {health.message}")
```

---

## Migration Guide

### Before (Old API):

```rust
// Old: Multiple method calls required
let ticker = client.get_24hr_ticker("BTCUSDT").await?;
let orderbook = client.get_order_book("BTCUSDT", Some(20)).await?;
let metrics = orderbook_manager.get_metrics("BTCUSDT").await?;
let depth = orderbook_manager.get_depth("BTCUSDT", 20).await?;
let anomalies = analytics.detect_anomalies("BTCUSDT").await?;
let health = analytics.get_health("BTCUSDT").await?;

// Manually format and combine data...
```

### After (New API):

```rust
// New: Single method call
let report = generator.generate_report("BTCUSDT", ReportOptions::default()).await?;
println!("{}", report.markdown_content);
// Done! Formatted markdown with all sections
```

---

## Advanced Usage

### Custom Section Ordering

Sections appear in this fixed order (cannot be reordered):
1. Price Overview
2. Order Book Metrics
3. Liquidity Analysis
4. Market Microstructure
5. Market Anomalies
6. Microstructure Health
7. Data Health Status

To omit sections, use `include_sections`:
```rust
let options = ReportOptions {
    include_sections: Some(vec![
        "price_overview".to_string(),
        "orderbook_metrics".to_string(),
        "data_health".to_string(),
    ]),
    ..Default::default()
};
```

### Programmatic Report Parsing

The report is structured markdown - parse sections with regex or markdown parsers:

```python
import re

# Extract specific section
def extract_section(markdown, section_title):
    pattern = rf"## {section_title}\n\n(.*?)(?=\n## |\Z)"
    match = re.search(pattern, markdown, re.DOTALL)
    return match.group(1).strip() if match else None

price_section = extract_section(report['content'], "Price Overview")
print(price_section)
```

---

## Next Steps

1. **Read the full specification**: [spec.md](./spec.md)
2. **Explore the data model**: [data-model.md](./data-model.md)
3. **Review implementation tasks**: [tasks.md](./tasks.md) (generated via `/speckit.tasks`)
4. **Check gRPC contracts**: [contracts/market-report.proto](./contracts/market-report.proto)

---

## FAQ

**Q: Can I still place orders?**
A: No. All order management methods have been removed. This system is now read-only for market data analysis.

**Q: What happened to authentication?**
A: Authentication infrastructure is preserved for future authenticated read-only endpoints (e.g., balance queries), but order management is permanently removed.

**Q: Can I get reports for multiple symbols at once?**
A: Not in a single call. Generate reports concurrently for multiple symbols using async/parallel requests.

**Q: How fresh is the data?**
A: Check the `data_age_ms` field in the response. Typically <500ms when WebSocket connections are active.

**Q: What if analytics features are disabled?**
A: Reports gracefully show "Feature Not Available" messages for sections requiring `orderbook_analytics` feature flag.

---

**Questions or Issues?** File an issue in the repository or contact the team.
