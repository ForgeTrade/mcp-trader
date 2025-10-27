# Binance Provider Dockerfile
# Multi-stage build for minimal final image

FROM rust:slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    protobuf-compiler \
    libprotobuf-dev \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy manifests
COPY Cargo.toml Cargo.lock build.rs ./
COPY proto ./proto

# Copy source
COPY src ./src

# Build release binary with all features
# Note: This may take 5-10 minutes on first build
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --release --features "orderbook,orderbook_analytics,http_transport" && \
    cp /build/target/release/binance-provider /tmp/binance-provider

# Runtime stage
FROM debian:bookworm-slim

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
