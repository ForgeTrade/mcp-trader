# MCP Gateway Dockerfile
# Python 3.11+ gateway for provider orchestration

FROM python:3.11-slim

# Install uv for faster dependency installation
RUN pip install --no-cache-dir uv

WORKDIR /app

# Copy project files
COPY pyproject.toml uv.lock ./
COPY mcp_gateway ./mcp_gateway
COPY providers.yaml ./

# Install dependencies with uv
RUN uv pip install --system --no-cache -e .

# Expose SSE server port
EXPOSE 3001

# Set environment defaults
ENV MCP_PROVIDERS=/app/providers.yaml
ENV MCP_SSE_PORT=3001

# Run SSE server
CMD ["python", "-m", "mcp_gateway.sse_server"]
