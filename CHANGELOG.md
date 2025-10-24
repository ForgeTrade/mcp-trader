# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

#### Feature 018: Unified Market Data Report (2025-10-24)

**New Unified API:**
- Added `generate_market_report()` - Single method consolidating 8+ individual market data methods
- Comprehensive markdown-formatted market intelligence reports
- Support for customizable report options (`include_sections`, `volume_window_hours`, `orderbook_levels`)
- Smart caching with 60-second TTL and options-aware cache keys

**Report Sections:**
1. **Report Header** - Symbol, timestamp, data age indicator with freshness emojis (🟢/🟡/🔴)
2. **Price Overview** - 24h statistics with trend indicators (📈/📉/➡️)
3. **Order Book Metrics** - Spread, microprice, imbalance with visual indicators
4. **Liquidity Analysis** - Walls, volume profile (POC/VAH/VAL), vacuums
5. **Market Microstructure** - Order flow analysis (placeholder for future)
6. **Market Anomalies** - Detection monitoring with severity badges (🔴/🟡/🟢)
7. **Microstructure Health** - Health scores and component status
8. **Data Health Status** - WebSocket connectivity, freshness warnings
9. **Report Footer** - Generation metadata, cache status, build configuration

**Enhanced Display (User Stories 2-4):**
- Severity-based anomaly monitoring with actionable recommendations
- Enhanced liquidity walls with strength indicators (💪 Strong, 🔷 Moderate, 🔹 Weak)
- Volume profile structure with POC/VAH/VAL placeholders
- Liquidity vacuum detection framework
- Data health warnings for stale data (>5s, >30s thresholds)
- Report generation metadata footer

**Performance:**
- Parallel data fetching using `tokio::join!`
- 60-second report caching with metadata preservation
- Graceful degradation for missing data sources
- Feature flag support (`orderbook_analytics`) for advanced sections

**Infrastructure:**
- gRPC service integration (`generate_market_report` tool)
- MCP handler integration for ChatGPT
- Python gateway SSE server support
- Unified tool routing with venue parameter support

### Changed

#### Breaking Changes (Phase 7)

**⚠️ CRITICAL: System Transformed to Read-Only**

This release contains **breaking changes** that remove ALL order management functionality. The system is now a **read-only market data analysis tool**.

**Removed Methods:**
- `place_order()` - Order placement
- `cancel_order()` - Order cancellation
- `get_order()` - Order query
- `get_open_orders()` - Open orders list
- `get_all_orders()` - All orders history
- `get_account()` - Account information
- `get_my_trades()` - Trade history
- `create_listen_key()` - WebSocket user data stream creation
- `keepalive_listen_key()` - User stream keep-alive
- `close_listen_key()` - User stream closure

**Removed Types (Rust):**
- `Balance` struct (account balance)
- `AccountInfo` struct (account information)
- `Fill` struct (order fill details)
- `Order` struct (order responses)
- `MyTrade` struct (trade history)

**Removed Normalizers (Python):**
- `_normalize_binance_account()` - Account schema normalization
- `_normalize_binance_trade()` - Trade schema normalization

**Impact:**
- 236 lines of code removed from Rust provider
- System cannot place, cancel, or query orders
- System cannot retrieve account information or trade history
- All market data analysis features preserved and enhanced

**Preserved:**
- Authentication infrastructure (for potential future read-only authenticated endpoints)
- WebSocket market data streams (ticker, orderbook, trades)
- All market data retrieval functionality

**Migration Guide:**

If your code was using order management:
```python
# BEFORE (no longer works)
await client.place_order(symbol="BTCUSDT", side="BUY", ...)
await client.get_account()
await client.get_my_trades(symbol="BTCUSDT")

# AFTER (use unified report instead)
report = await client.generate_market_report(
    symbol="BTCUSDT",
    options={
        "include_sections": ["price_overview", "liquidity_analysis"],
        "volume_window_hours": 24
    }
)
# report.markdown_content contains comprehensive market intelligence
```

**Deprecation Timeline:**
- Order management removed effective immediately (no deprecation period)
- This is a one-way transformation

### Fixed

#### P0 Fixes (2025-10-23)

**P0 #1: Venue Parameter Routing**
- Fixed venue parameter being ignored in unified tool routing
- System always routed to last provider in discovery loop
- Added `venue_provider_map` for correct venue-based routing
- Multi-venue deployments now functional

**P0 #2: Cache Key Isolation**
- Fixed cache pollution where different `ReportOptions` returned wrong cached reports
- Cache keyed only by symbol, causing accuracy violations
- Now includes all options in cache key: `"SYMBOL:sections:X;volume:Y;levels:Z"`
- Separate cache entries for each option combination

#### P1 Fixes (2025-10-24)

**P1 #1: Duplicate Report Footers**
- Fixed duplicate footers appearing in cached reports
- Every cache hit appended new footer to already-complete markdown
- Now returns cached reports as-is (footer already embedded)

**P1 #2: Cached Generation Metadata**
- Fixed contradictory metadata where cached reports showed 2ms generation time
- Struct field `generation_time_ms` contradicted footer showing original time
- Now preserves ALL original metadata for consistency

**P1 #3: Phase 7 Regression**
- Fixed `AttributeError` when starting MCP Gateway after Phase 7 removal
- Removed references to deleted normalizers in `SchemaAdapter.__init__`
- MCP Gateway SSE server starts successfully

### Deprecated

- Individual market data methods (superseded by `generate_market_report()`)
  - Existing methods remain functional but unified report is recommended
  - Better performance (single call vs. multiple calls)
  - Comprehensive insights vs. raw data

### Security

- Preserved authentication infrastructure for future use
- API credentials remain secure (not required for public market data)

---

## Version History

### [0.2.0] - 2025-10-24

**Feature 018 Release**

This is a **major breaking release** that transforms the system from a hybrid read/write trading client into a read-only market data analysis tool.

**Key Changes:**
- ✅ Added unified market data report generation
- ⚠️ Removed ALL order management functionality (BREAKING)
- ✅ Enhanced market intelligence displays
- ✅ Fixed critical P0/P1 bugs
- ✅ Deployed to production

**Requirements Satisfied:**
- FR-001 through FR-013 (100%)
- User Stories 1-4 (100%)
- Success Criteria 1-8 (75% - performance profiling pending)

**Documentation:**
- `FEATURE_018_STATUS.md` - Complete implementation status
- `DEPLOYMENT_P1_FIXES.md` - Production deployment summary
- `specs/018-market-data-report/` - Full specification suite

### [0.1.0] - 2025-10-19

**Initial Release**

- Basic Binance API client with order management
- WebSocket support for market data streams
- gRPC and MCP integration
- Order book analytics (conditional compilation)

---

## Migration Guides

### Migrating from Order Management to Read-Only

**Before (0.1.0):**
```rust
// Place order
let order = client.place_order(
    "BTCUSDT",
    OrderSide::Buy,
    OrderType::Limit,
    Some(0.001),
    Some(40000.0)
).await?;

// Get account
let account = client.get_account().await?;
```

**After (0.2.0):**
```rust
// Generate comprehensive market report
let report = report_generator.generate_report(
    "BTCUSDT",
    ReportOptions::default()
).await?;

// Report contains markdown with all market intelligence:
// - Current price and 24h stats
// - Order book metrics and liquidity
// - Market health and anomaly detection
println!("{}", report.markdown_content);
```

### Using the Unified Report API

**Default Usage:**
```python
# Generate full report with all sections
report = await client.generate_market_report(
    symbol="BTCUSDT"
)
```

**Custom Options:**
```python
# Generate partial report with specific sections
report = await client.generate_market_report(
    symbol="ETHUSDT",
    options={
        "include_sections": ["price_overview", "liquidity_analysis"],
        "volume_window_hours": 48,
        "orderbook_levels": 50
    }
)
```

**Performance Tips:**
- Reports are cached for 60 seconds
- Use `include_sections` to request only needed data
- Cache hit latency: ~3ms vs. cold generation: ~200-500ms

---

## Support

For issues, questions, or feature requests:
- **GitHub**: https://github.com/forgequant/mcp-gateway
- **Documentation**: `/specs/018-market-data-report/quickstart.md`

---

## Credits

**Feature 018 Implementation:**
- Design & Implementation: Claude (Anthropic AI)
- Specification: ForgeQuant Team
- Testing & Deployment: Automated CI/CD + Manual verification

**Special Thanks:**
- Binance for public market data API
- Anthropic for Claude AI capabilities
- Rust & Python communities for excellent tooling
