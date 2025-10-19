#!/bin/bash
# Build script for binance-rs provider with feature flags

set -e

echo "=== Binance Provider Build Script ==="
echo ""

# Parse command line arguments
BUILD_MODE="${1:-all}"

case "$BUILD_MODE" in
  "base")
    echo "Building BASE version (13 tools - no orderbook features)..."
    cargo build --release
    echo "✅ Built: target/release/binance-provider (base)"
    ;;
  
  "orderbook")
    echo "Building ORDERBOOK version (16 tools - with L1/L2 depth)..."
    cargo build --release --features "orderbook"
    echo "✅ Built: target/release/binance-provider (orderbook)"
    ;;
  
  "analytics")
    echo "Building ANALYTICS version (21 tools - full analytics suite)..."
    cargo build --release --features "orderbook,orderbook_analytics"
    echo "✅ Built: target/release/binance-provider (analytics)"
    ;;
  
  "all")
    echo "Building ALL versions..."
    echo ""
    
    echo "[1/3] Base version..."
    cargo build --release
    
    echo ""
    echo "[2/3] OrderBook version..."
    cargo build --release --features "orderbook"
    
    echo ""
    echo "[3/3] Analytics version..."
    cargo build --release --features "orderbook,orderbook_analytics"
    
    echo ""
    echo "✅ All versions built successfully!"
    ;;
  
  *)
    echo "Usage: ./build.sh [base|orderbook|analytics|all]"
    echo ""
    echo "Modes:"
    echo "  base       - 13 tools (market data, account, orders)"
    echo "  orderbook  - 16 tools (+ orderbook L1/L2)"
    echo "  analytics  - 21 tools (+ order flow, volume profile, anomalies)"
    echo "  all        - Build all versions (default)"
    exit 1
    ;;
esac

echo ""
echo "Binary location: target/release/binance-provider"
echo ""
echo "Run with: ./target/release/binance-provider --help"
