# mcp-trader Development Guidelines

Auto-generated from all feature plans. Last updated: 2025-10-24

## Active Technologies
- Rust 1.75+ (providers/binance-rs), Python 3.11+ (mcp-gateway) (018-market-data-report, 019-expand-market-report)
- tokio async runtime (019-expand-market-report)

## Project Structure
```
providers/binance-rs/
  src/
    report/sections.rs     # Report section builders (modified in 019)
    orderbook/analytics/   # Analytics functions (used by 019)
  tests/
    unit/analytics_integration.rs    # New in 019
    integration/report_generation.rs # New in 019
mcp-gateway/
  mcp_gateway/binance_mcp.py
```

## Commands
```bash
# Build with analytics
cd providers/binance-rs && cargo build --features orderbook_analytics --release

# Run tests
cargo test --features orderbook_analytics

# Python gateway
cd mcp-gateway && pytest && ruff check .
```

## Code Style
- Rust 1.75+ (providers/binance-rs): Follow standard conventions, use tokio for async
- Python 3.11+ (mcp-gateway): Follow standard conventions

## Recent Changes
- 019-expand-market-report: Added tokio async runtime, analytics integration in report sections
- 018-market-data-report: Added Rust 1.75+ (providers/binance-rs), Python 3.11+ (mcp-gateway)

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
