.PHONY: help proto-gen run-gateway run-hello-go run-hello-rs build-binance run-binance test clean

help:
	@echo "MCP Gateway System - Available targets:"
	@echo "  proto-gen       - Generate protobuf code for all projects"
	@echo "  run-gateway     - Start MCP gateway (Python)"
	@echo "  run-hello-go    - Start hello-go provider"
	@echo "  run-hello-rs    - Start hello-rs provider"
	@echo "  build-binance   - Build binance-rs provider"
	@echo "  run-binance     - Start binance-rs provider"
	@echo "  test            - Run all tests"
	@echo "  clean           - Clean generated files"

proto-gen:
	@echo "Generating protobuf code..."
	@# Python
	@cd mcp-gateway && uv run python -m grpc_tools.protoc \
		-I../pkg/proto \
		--python_out=. \
		--grpc_python_out=. \
		--pyi_out=. \
		../pkg/proto/provider.proto
	@# Move generated files to the right location
	@mv mcp-gateway/provider_pb2.py mcp-gateway/mcp_gateway/generated/
	@mv mcp-gateway/provider_pb2_grpc.py mcp-gateway/mcp_gateway/generated/
	@mv mcp-gateway/provider_pb2.pyi mcp-gateway/mcp_gateway/generated/ 2>/dev/null || true
	@# Go
	@cd providers/hello-go && protoc -I../../pkg/proto \
		--go_out=internal/pb --go_opt=paths=source_relative \
		--go-grpc_out=internal/pb --go-grpc_opt=paths=source_relative \
		../../pkg/proto/provider.proto
	@# Rust (handled by build.rs during cargo build)
	@echo "Protobuf code generated"

run-gateway:
	@echo "Starting MCP Gateway..."
	@cd mcp-gateway && uv run python -m mcp_gateway.main

run-hello-go:
	@echo "Starting hello-go provider..."
	@cd providers/hello-go && go run cmd/server/main.go

run-hello-rs:
	@echo "Starting hello-rs provider..."
	@cd providers/hello-rs && cargo run

build-binance:
	@echo "Building binance-rs provider..."
	@cd providers/binance-rs && cargo build --release
	@echo "Binary: providers/binance-rs/target/release/binance-provider"

run-binance:
	@echo "Starting binance-rs provider on port 50053..."
	@cd providers/binance-rs && ./target/release/binance-provider --grpc --port 50053

test:
	@echo "Running tests..."
	@cd mcp-gateway && uv run pytest tests/ || true
	@cd providers/hello-go && go test ./... || true
	@cd providers/hello-rs && cargo test || true
	@cd providers/binance-rs && cargo test || true

clean:
	@echo "Cleaning generated files..."
	@rm -rf mcp-gateway/mcp_gateway/generated/*
	@rm -rf providers/hello-go/internal/pb/*
	@rm -rf providers/hello-rs/target/
	@rm -rf providers/binance-rs/target/
	@echo "Clean complete"
