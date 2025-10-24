#!/bin/bash
# Integration test for Feature 018: Market Data Report via SSE
# Tests the complete flow: Binance Provider -> MCP Gateway -> SSE Client

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Directories
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROVIDER_DIR="$SCRIPT_DIR/providers/binance-rs"
GATEWAY_DIR="$SCRIPT_DIR/mcp-gateway"

# Log files
PROVIDER_LOG="/tmp/binance_provider_test.log"
GATEWAY_LOG="/tmp/mcp_gateway_test.log"

# PIDs
PROVIDER_PID=""
GATEWAY_PID=""

# Cleanup function
cleanup() {
    echo -e "\n${YELLOW}🧹 Cleaning up...${NC}"

    if [ ! -z "$GATEWAY_PID" ]; then
        echo "Stopping MCP Gateway (PID: $GATEWAY_PID)..."
        kill $GATEWAY_PID 2>/dev/null || true
        wait $GATEWAY_PID 2>/dev/null || true
    fi

    if [ ! -z "$PROVIDER_PID" ]; then
        echo "Stopping Binance Provider (PID: $PROVIDER_PID)..."
        kill $PROVIDER_PID 2>/dev/null || true
        wait $PROVIDER_PID 2>/dev/null || true
    fi

    echo -e "${GREEN}✅ Cleanup complete${NC}"
}

# Set trap for cleanup
trap cleanup EXIT INT TERM

echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}  Feature 018: Market Data Report - SSE Integration Test${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Step 1: Build Binance Provider (if needed)
echo -e "${YELLOW}📦 Step 1: Building Binance Provider...${NC}"
cd "$PROVIDER_DIR"

if [ ! -f "target/release/binance-provider" ]; then
    echo "Building release binary..."
    cargo build --release --features orderbook_analytics
    echo -e "${GREEN}✅ Build complete${NC}"
else
    echo -e "${GREEN}✅ Binary already exists${NC}"
fi

# Step 2: Start Binance Provider (gRPC mode)
echo -e "\n${YELLOW}🚀 Step 2: Starting Binance Provider (port 50053)...${NC}"
RUST_LOG=info ./target/release/binance-provider --grpc --port 50053 > "$PROVIDER_LOG" 2>&1 &
PROVIDER_PID=$!

sleep 3

if ! kill -0 $PROVIDER_PID 2>/dev/null; then
    echo -e "${RED}❌ Binance Provider failed to start${NC}"
    echo "Log output:"
    cat "$PROVIDER_LOG"
    exit 1
fi

echo -e "${GREEN}✅ Binance Provider running (PID: $PROVIDER_PID)${NC}"
echo "Log file: $PROVIDER_LOG"

# Step 3: Start MCP Gateway (SSE mode)
echo -e "\n${YELLOW}🚀 Step 3: Starting MCP Gateway SSE Server (port 3001)...${NC}"
cd "$GATEWAY_DIR"

# Install dependencies if needed
if [ ! -d ".venv" ]; then
    echo "Setting up Python environment..."
    uv sync
fi

# Start SSE server
uv run python -m mcp_gateway.sse_server > "$GATEWAY_LOG" 2>&1 &
GATEWAY_PID=$!

sleep 5

if ! kill -0 $GATEWAY_PID 2>/dev/null; then
    echo -e "${RED}❌ MCP Gateway failed to start${NC}"
    echo "Log output:"
    cat "$GATEWAY_LOG"
    exit 1
fi

echo -e "${GREEN}✅ MCP Gateway running (PID: $GATEWAY_PID)${NC}"
echo "Log file: $GATEWAY_LOG"

# Step 4: Wait for services to be ready
echo -e "\n${YELLOW}⏳ Step 4: Waiting for services to be ready...${NC}"
sleep 2

# Check health endpoint
if curl -s http://localhost:3001/health | grep -q "healthy"; then
    echo -e "${GREEN}✅ Gateway health check passed${NC}"
else
    echo -e "${RED}❌ Gateway health check failed${NC}"
    exit 1
fi

# Step 5: Run SSE Client Tests
echo -e "\n${YELLOW}🧪 Step 5: Running SSE Client Tests...${NC}"
echo ""

cd "$GATEWAY_DIR"
uv run python test_sse_client.py

EXIT_CODE=$?

if [ $EXIT_CODE -eq 0 ]; then
    echo ""
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${GREEN}  ✅ ALL TESTS PASSED${NC}"
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
    echo -e "📊 Test Summary:"
    echo -e "  - Binance Provider: ${GREEN}✅ Running${NC}"
    echo -e "  - MCP Gateway SSE:  ${GREEN}✅ Running${NC}"
    echo -e "  - SSE Integration:  ${GREEN}✅ Passed${NC}"
    echo -e "  - Market Reports:   ${GREEN}✅ Generated${NC}"
    echo ""
    echo -e "📝 Logs:"
    echo -e "  - Provider: $PROVIDER_LOG"
    echo -e "  - Gateway:  $GATEWAY_LOG"
    echo ""
else
    echo ""
    echo -e "${RED}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${RED}  ❌ TESTS FAILED${NC}"
    echo -e "${RED}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
    echo -e "📝 Check logs for details:"
    echo -e "  - Provider: $PROVIDER_LOG"
    echo -e "  - Gateway:  $GATEWAY_LOG"
    echo ""
fi

exit $EXIT_CODE
