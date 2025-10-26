"""
SSE (Server-Sent Events) server for ChatGPT MCP integration.
Exposes search and fetch tools over SSE transport.
"""
import asyncio
import os
import json
import logging
from pathlib import Path

from mcp.server import Server
from mcp.types import Tool, TextContent
from mcp.server.sse import SseServerTransport
from starlette.applications import Starlette
from starlette.routing import Route, Mount
from starlette.responses import Response

from mcp_gateway.adapters.grpc_client import ProviderGRPCClient
from mcp_gateway.adapters.unified_router import UnifiedToolRouter
from mcp_gateway.adapters.schema_adapter import SchemaAdapter
from mcp_gateway.providers_registry import ProviderRegistry
from mcp_gateway.config import PUBLIC_VENUES  # Feature 014

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)


class ChatGPTMCPServer:
    """
    MCP Server for ChatGPT integration with SSE transport.
    Exposes search and fetch tools for cryptocurrency market data.
    """

    def __init__(self, config_path: str = "providers.yaml", base_url: str = "https://mcp-gateway.thevibe.trading"):
        """
        Initialize ChatGPT MCP server.

        Args:
            config_path: Path to providers configuration
            base_url: Base URL for citations
        """
        self.config_path = config_path
        self.base_url = base_url
        self.registry = ProviderRegistry()
        # FR-047: Support multiple providers instead of hardcoding Binance
        self.provider_clients: dict[str, ProviderGRPCClient] = {}  # provider_name -> client
        self.provider_tools: list[dict] = []  # Will be populated during initialize()
        # T020, T021: Unified tool routing and schema normalization
        self.unified_router: UnifiedToolRouter | None = None
        self.schema_adapter = SchemaAdapter()
        self.server = Server("chatgpt-mcp-gateway")

    async def initialize(self):
        """Initialize the server and connect to all providers."""
        logger.info("Initializing ChatGPT MCP Server...")

        # FR-047: Load ALL providers dynamically instead of hardcoding Binance
        providers = self.registry.load_providers(self.config_path)
        logger.info(f"Loaded {len(providers)} providers from {self.config_path}")

        if not providers:
            raise ValueError("No providers found in configuration")

        # Create gRPC clients for all providers and fetch their capabilities
        all_tools = []
        for provider in providers:
            try:
                # Create gRPC client for this provider
                client = ProviderGRPCClient(provider.name, provider.address)
                self.provider_clients[provider.name] = client
                logger.info(f"Connected to {provider.name} provider at {provider.address}")

                # Fetch available tools from this provider
                capabilities = await client.list_capabilities()
                tools = capabilities.get("tools", [])
                logger.info(f"Loaded {len(tools)} tools from {provider.name} provider")
                all_tools.extend(tools)
            except Exception as e:
                logger.error(f"Failed to connect to {provider.name} provider: {e}")
                # Continue with other providers instead of failing completely

        self.provider_tools = all_tools
        logger.info(f"Total tools loaded from all providers: {len(self.provider_tools)}")

        # T020, T021: Initialize unified tool router
        self.unified_router = UnifiedToolRouter(self.provider_clients)
        logger.info("UnifiedToolRouter initialized")

        # Register tool handlers
        self._register_handlers()

    def _register_handlers(self):
        """Register MCP server handlers."""

        @self.server.list_tools()
        async def list_tools():
            """List ONLY the unified market report tool (Feature 018 - FR-002)."""
            # Feature 018: Single unified method consolidates ALL market data (FR-002)
            # All individual market/trade/analytics tools removed per spec
            venues_list = PUBLIC_VENUES

            unified_tools = [
                Tool(
                    name="market.generate_report",
                    description=f"Generate comprehensive market intelligence report combining price, orderbook, liquidity, volume profile, order flow, anomalies, and market health into single markdown document. Available venues: {venues_list}",
                    inputSchema={
                        "type": "object",
                        "required": ["instrument"],
                        "properties": {
                            "venue": {
                                "type": "string",
                                "description": "Exchange venue (optional, default: binance)",
                                "enum": venues_list,
                                "default": "binance"
                            },
                            "instrument": {
                                "type": "string",
                                "description": "Trading pair symbol (e.g., BTCUSDT)",
                                "examples": ["BTCUSDT", "ETHUSDT"]
                            },
                            "options": {
                                "type": "object",
                                "description": "Report generation options (optional)",
                                "properties": {
                                    "include_sections": {
                                        "type": "array",
                                        "description": "Section names to include (omit for all sections). Valid values: price_overview, orderbook_metrics, liquidity_analysis, market_microstructure, market_anomalies, microstructure_health, data_health",
                                        "items": {
                                            "type": "string",
                                            "enum": ["price_overview", "orderbook_metrics", "liquidity_analysis", "market_microstructure", "market_anomalies", "microstructure_health", "data_health"]
                                        },
                                        "examples": [["price_overview", "orderbook_metrics", "liquidity_analysis"]]
                                    },
                                    "volume_window_hours": {
                                        "type": "integer",
                                        "description": "Time window for volume profile (hours, default: 24)",
                                        "minimum": 1,
                                        "maximum": 168,
                                        "default": 24
                                    },
                                    "orderbook_levels": {
                                        "type": "integer",
                                        "description": "Number of orderbook levels for depth analysis (default: 20)",
                                        "minimum": 1,
                                        "maximum": 100,
                                        "default": 20
                                    }
                                }
                            }
                        }
                    }
                ),
            ]

            logger.info(f"Returning {len(unified_tools)} unified tool: generate_report (Feature 018 - FR-002)")
            return unified_tools

        @self.server.call_tool()
        async def call_tool(name: str, arguments: dict):
            """Handle tool calls - ONLY market.generate_report is accepted (Feature 018 - FR-002)."""
            logger.info(f"Tool called: {name} with arguments: {arguments}")

            try:
                # Generate a correlation ID for tracking
                import uuid
                correlation_id = str(uuid.uuid4())

                # Feature 018 FR-002: ONLY accept market.generate_report
                if name != "market.generate_report":
                    error_msg = f"Tool '{name}' is not available. Only 'market.generate_report' is exposed (Feature 018 - FR-002)."
                    logger.warning(f"Rejected unavailable tool call: {name}")
                    return [TextContent(
                        type="text",
                        text=json.dumps({
                            "error": error_msg,
                            "error_code": "TOOL_NOT_AVAILABLE",
                            "available_tool": "market.generate_report",
                            "available_venues": list(self.provider_clients.keys())
                        }, indent=2)
                    )]

                # Handle the unified market report tool
                if self.unified_router:
                    logger.info(f"Routing market.generate_report through UnifiedToolRouter")

                    try:
                        # Use 15s timeout for analytics-heavy market reports (ChatGPT integration)
                        result = await self.unified_router.route_tool_call(
                            unified_tool_name=name,
                            arguments=arguments,
                            correlation_id=correlation_id,
                            timeout=15.0
                        )

                        # Feature 018: Report is returned as markdown text
                        # No normalization needed - return result as-is
                        return [TextContent(
                            type="text",
                            text=json.dumps(result, indent=2)
                        )]

                    except ValueError as ve:
                        # Enhanced error handling for invalid instruments
                        error_msg = str(ve)
                        logger.warning(f"Market report generation error: {error_msg}")

                        if "symbol" in error_msg.lower() or "instrument" in error_msg.lower():
                            alternatives = ["BTCUSDT", "ETHUSDT", "BNBUSDT", "SOLUSDT"]
                            return [TextContent(
                                type="text",
                                text=json.dumps({
                                    "error": error_msg,
                                    "error_code": "SYMBOL_NOT_FOUND",
                                    "alternatives": alternatives,
                                    "venue": arguments.get("venue", "binance")
                                }, indent=2)
                            )]
                        else:
                            return [TextContent(
                                type="text",
                                text=json.dumps({"error": error_msg}, indent=2)
                            )]

                # Should never reach here
                error_msg = "Internal error: UnifiedToolRouter not available"
                logger.error(error_msg)
                return [TextContent(
                    type="text",
                    text=json.dumps({"error": error_msg}, indent=2)
                )]

            except Exception as e:
                error_msg = f"Tool execution failed: {e}"
                logger.error(error_msg, exc_info=True)
                return [TextContent(type="text", text=json.dumps({"error": error_msg}))]

    async def shutdown(self):
        """Shutdown server and close connections."""
        logger.info("Shutting down ChatGPT MCP Server...")
        # Close all provider clients
        for provider_name, client in self.provider_clients.items():
            try:
                await client.close()
                logger.info(f"Closed connection to {provider_name}")
            except Exception as e:
                logger.error(f"Error closing {provider_name}: {e}")
        logger.info("Server shutdown complete")

    def get_sse_app(self) -> Starlette:
        """
        Create Starlette app for SSE transport.

        Returns:
            Starlette application
        """
        # Create SSE transport with full path
        sse = SseServerTransport("/sse/messages")

        # Create main ASGI app that routes requests
        async def main_app(scope, receive, send):
            if scope["type"] != "http":
                return

            path = scope.get("path", "")
            method = scope.get("method", "GET")

            logger.info(f"Request: {method} {path}")

            # Health check endpoint
            if path == "/health":
                response = Response(
                    json.dumps({"status": "healthy", "service": "chatgpt-mcp-gateway"}),
                    media_type="application/json"
                )
                await response(scope, receive, send)

            # SSE connection endpoint (GET only)
            elif (path == "/sse" or path == "/sse/") and method == "GET":
                logger.info(f"New SSE connection from {scope.get('client', ['unknown'])[0]}")
                async with sse.connect_sse(scope, receive, send) as (read_stream, write_stream):
                    init_options = self.server.create_initialization_options()
                    await self.server.run(read_stream, write_stream, init_options)

            # SSE messages endpoint (POST only)
            elif path.startswith("/sse/messages") and method == "POST":
                logger.info(f"Handling SSE message POST from {scope.get('client', ['unknown'])[0]}")
                await sse.handle_post_message(scope, receive, send)

            # Handle incorrect POST to /sse or /sse/
            elif (path == "/sse" or path == "/sse/") and method == "POST":
                logger.warning(f"POST request to {path} - should POST to /sse/messages instead")
                response = Response(
                    json.dumps({"error": "POST should be sent to /sse/messages with session_id parameter"}),
                    status_code=400,
                    media_type="application/json"
                )
                await response(scope, receive, send)

            # 404 for other paths
            else:
                logger.warning(f"No route matched for {method} {path}")
                response = Response(f"Not Found: {method} {path}", status_code=404)
                await response(scope, receive, send)

        return main_app


async def main():
    """Main entry point for SSE server."""
    import uvicorn

    # Resolve config path (allow override via env for testing)
    config_env = os.getenv("MCP_PROVIDERS")
    if config_env:
        config_path = Path(config_env)
    else:
        config_path = Path(__file__).parent.parent / "providers.yaml"

    # Create and initialize server
    server = ChatGPTMCPServer(str(config_path))
    await server.initialize()

    # Get Starlette app
    app = server.get_sse_app()

    # Run with uvicorn
    port = int(os.getenv("MCP_SSE_PORT", "3001"))
    config = uvicorn.Config(
        app,
        host="0.0.0.0",
        port=port,
        log_level="info",
        access_log=True,
    )
    uvicorn_server = uvicorn.Server(config)

    try:
        logger.info(f"Starting SSE server on http://0.0.0.0:{port}")
        await uvicorn_server.serve()
    finally:
        await server.shutdown()


if __name__ == "__main__":
    asyncio.run(main())
