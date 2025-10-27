# MCP Gateway Dockerfile
# Python 3.11+ gateway for provider orchestration

FROM python:3.11-slim

# Install system dependencies
RUN apt-get update && apt-get install -y \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Install uv for faster dependency installation
RUN pip install --no-cache-dir uv

WORKDIR /app

# Copy proto files for code generation
COPY pkg/proto ./pkg/proto

# Copy project files from mcp-gateway subdirectory
COPY mcp-gateway/pyproject.toml mcp-gateway/uv.lock ./
COPY mcp-gateway/mcp_gateway ./mcp_gateway
COPY mcp-gateway/providers.yaml ./

# Install dependencies with uv
RUN uv pip install --system --no-cache -e .

# Generate protobuf files
RUN mkdir -p mcp_gateway/generated && \
    python -m grpc_tools.protoc \
    --proto_path=pkg/proto \
    --python_out=mcp_gateway/generated \
    --grpc_python_out=mcp_gateway/generated \
    pkg/proto/provider.proto && \
    touch mcp_gateway/generated/__init__.py && \
    sed -i 's/^import provider_pb2/from . import provider_pb2/' mcp_gateway/generated/provider_pb2_grpc.py

# Expose SSE server port
EXPOSE 3001

# Set environment defaults
ENV MCP_PROVIDERS=/app/providers.yaml
ENV MCP_SSE_PORT=3001

# Run SSE server
CMD ["python", "-m", "mcp_gateway.sse_server"]
