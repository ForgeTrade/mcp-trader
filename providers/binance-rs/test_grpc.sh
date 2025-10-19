#!/bin/bash
set -e

echo "=== T085: Testing gRPC Mode ==="
echo ""

# Start gRPC server
echo "Starting gRPC server on port 50053..."
RUST_LOG=info ./target/release/binance-provider --grpc --port 50053 > /tmp/grpc_server.log 2>&1 &
GRPC_PID=$!
sleep 3

if ! kill -0 $GRPC_PID 2>/dev/null; then
  echo "❌ gRPC server failed to start"
  cat /tmp/grpc_server.log
  exit 1
fi

echo "✅ gRPC server running (PID: $GRPC_PID)"
echo ""

# Test with grpcurl if available
if command -v grpcurl >/dev/null 2>&1; then
  echo "Testing with grpcurl..."
  grpcurl -plaintext -import-path ../../pkg/proto -proto provider.proto \
    localhost:50053 provider.v1.Provider/ListCapabilities 2>&1 | head -20
else
  echo "⚠️  grpcurl not installed - skipping gRPC tool verification"
  echo "✅ gRPC server started successfully (manual testing required)"
fi

echo ""
echo "Server log (first 20 lines):"
head -20 /tmp/grpc_server.log

kill $GRPC_PID 2>/dev/null
wait $GRPC_PID 2>/dev/null

echo ""
echo "=== T085 Complete ==="
