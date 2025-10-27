# Binance Provider Dockerfile
# Multi-stage build for minimal final image

FROM rust:slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    protobuf-compiler \
    libprotobuf-dev \
    pkg-config \
    libssl-dev \
    clang \
    cmake \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy shared proto files
COPY pkg/proto ./pkg/proto

# Copy binance-provider specific files
COPY providers/binance-rs/Cargo.toml providers/binance-rs/build.rs ./
COPY providers/binance-rs/proto ./proto
COPY providers/binance-rs/src ./src

# Build release binary with all features
# Note: This may take 5-10 minutes on first build
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --release --features "orderbook,orderbook_analytics,http_transport" && \
    cp /build/target/release/binance-provider /tmp/binance-provider

# Runtime stage
FROM debian:sid-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from temp location (cached build)
COPY --from=builder /tmp/binance-provider /app/binance-provider

# Create data directory for analytics
RUN mkdir -p /app/data/analytics

# Expose gRPC port
EXPOSE 50053

# Set environment defaults
ENV RUST_LOG=info
ENV ANALYTICS_DATA_PATH=/app/data/analytics

# Run the provider in gRPC mode
CMD ["/app/binance-provider", "--grpc"]
