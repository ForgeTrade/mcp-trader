# mcp-trader Development Guidelines

Auto-generated from all feature plans. Last updated: 2025-10-27

## Active Technologies
- Rust 1.75+ (providers/binance-rs), Python 3.11+ (mcp-gateway) (018-market-data-report, 019-expand-market-report)
- tokio async runtime (019-expand-market-report)
- GitHub Actions (CI/CD orchestration), Docker 20.10+, Docker Compose V2 (019-github-cicd-pipeline)

## Project Structure
```
.github/workflows/
  build-and-push.yml       # CI workflow (019-github-cicd-pipeline)
scripts/deploy/
  pull-and-restart.sh      # Deployment automation (019-github-cicd-pipeline)
  rollback.sh              # Rollback script (019-github-cicd-pipeline)
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

# CI/CD (019-github-cicd-pipeline)
# Trigger CI build: Push to main branch
# Deploy: ssh user@server '/opt/mcp-trader/scripts/deploy/pull-and-restart.sh'
# Rollback: ssh user@server '/opt/mcp-trader/scripts/deploy/rollback.sh <commit-sha>'
```

## Code Style
- Rust 1.75+ (providers/binance-rs): Follow standard conventions, use tokio for async
- Python 3.11+ (mcp-gateway): Follow standard conventions
- GitHub Actions workflows: Use official Docker actions, enable BuildKit caching
- Shell scripts: Follow bash best practices, use set -euo pipefail

## Recent Changes
- 019-github-cicd-pipeline: Added GitHub Actions CI/CD, GHCR image registry, deployment automation
- 019-expand-market-report: Added tokio async runtime, analytics integration in report sections
- 018-market-data-report: Added Rust 1.75+ (providers/binance-rs), Python 3.11+ (mcp-gateway)

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
