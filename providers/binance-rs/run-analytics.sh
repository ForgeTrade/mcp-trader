#!/bin/bash
# Run binance-provider with analytics features enabled

set -e

echo "=== Binance Provider - Analytics Mode ==="
echo ""

# Check for API credentials
if [ -z "$BINANCE_API_KEY" ] || [ -z "$BINANCE_API_SECRET" ]; then
    echo "⚠️  Warning: API credentials not set"
    echo "Set BINANCE_API_KEY and BINANCE_API_SECRET for authenticated operations"
    echo ""
fi

# Set default analytics data path
export ANALYTICS_DATA_PATH="${ANALYTICS_DATA_PATH:-./data/analytics}"

echo "Configuration:"
echo "  - Analytics data: $ANALYTICS_DATA_PATH"
echo "  - Log level: ${RUST_LOG:-info}"
echo "  - Port: ${PORT:-50053}"
echo ""

# Create data directory if it doesn't exist
mkdir -p "$ANALYTICS_DATA_PATH"

# Build if binary doesn't exist
if [ ! -f "target/release/binance-provider" ]; then
    echo "Binary not found. Building with analytics features..."
    cargo build --release --features "orderbook,orderbook_analytics"
    echo ""
fi

echo "Starting server..."
echo "Press Ctrl+C to stop"
echo ""

# Run the provider
exec ./target/release/binance-provider --grpc --port ${PORT:-50053}
