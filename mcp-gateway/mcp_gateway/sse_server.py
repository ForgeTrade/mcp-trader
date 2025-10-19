"""
SSE (Server-Sent Events) server for ChatGPT MCP integration.
Exposes search and fetch tools over SSE transport.
"""
import asyncio
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
from mcp_gateway.providers_registry import ProviderRegistry

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
        self.grpc_client: ProviderGRPCClient | None = None
        self.provider_tools: list[dict] = []  # Will be populated during initialize()
        self.server = Server("chatgpt-mcp-gateway")

    async def initialize(self):
        """Initialize the server and connect to Binance provider."""
        logger.info("Initializing ChatGPT MCP Server...")

        # Load providers (we only need the Binance provider)
        providers = self.registry.load_providers(self.config_path)
        logger.info(f"Loaded {len(providers)} providers from {self.config_path}")

        # Find Binance provider
        binance_provider = next((p for p in providers if "binance" in p.name.lower()), None)
        if not binance_provider:
            raise ValueError("Binance provider not found in configuration")

        # Create gRPC client for Binance provider
        self.grpc_client = ProviderGRPCClient(binance_provider.name, binance_provider.address)
        logger.info(f"Connected to Binance provider at {binance_provider.address}")

        # Fetch all available tools from the Binance provider
        capabilities = await self.grpc_client.list_capabilities()
        self.provider_tools = capabilities.get("tools", [])
        logger.info(f"Loaded {len(self.provider_tools)} tools from Binance provider")

        # Register tool handlers
        self._register_handlers()

    def _register_handlers(self):
        """Register MCP server handlers."""

        @self.server.list_tools()
        async def list_tools():
            """List all available tools from Binance provider."""
            # Convert provider tools to MCP Tool objects
            tools = []
            for tool_dict in self.provider_tools:
                tools.append(Tool(
                    name=tool_dict["name"],
                    description=tool_dict["description"],
                    inputSchema=tool_dict.get("input_schema", {}),
                ))

            logger.info(f"Returning {len(tools)} tools to client")
            return tools

        @self.server.call_tool()
        async def call_tool(name: str, arguments: dict):
            """Proxy tool calls directly to the Binance provider."""
            logger.info(f"Tool called: {name} with arguments: {arguments}")

            try:
                # Generate a correlation ID for tracking
                import uuid
                correlation_id = str(uuid.uuid4())

                # Call the Binance provider directly via gRPC
                result = await self.grpc_client.invoke(
                    tool_name=name,
                    payload=arguments,
                    correlation_id=correlation_id,
                    timeout=5.0  # 5 second timeout for tool calls
                )

                # Return result as MCP content array
                return [TextContent(
                    type="text",
                    text=json.dumps(result, indent=2)
                )]

            except Exception as e:
                error_msg = f"Tool execution failed: {e}"
                logger.error(error_msg, exc_info=True)
                return [TextContent(type="text", text=json.dumps({"error": error_msg}))]

    async def shutdown(self):
        """Shutdown server and close connections."""
        logger.info("Shutting down ChatGPT MCP Server...")
        if self.grpc_client:
            await self.grpc_client.close()
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

    # Resolve config path
    config_path = Path(__file__).parent.parent / "providers.yaml"

    # Create and initialize server
    server = ChatGPTMCPServer(str(config_path))
    await server.initialize()

    # Get Starlette app
    app = server.get_sse_app()

    # Run with uvicorn
    config = uvicorn.Config(
        app,
        host="0.0.0.0",
        port=3001,
        log_level="info",
        access_log=True,
    )
    uvicorn_server = uvicorn.Server(config)

    try:
        logger.info("Starting SSE server on http://0.0.0.0:3001")
        await uvicorn_server.serve()
    finally:
        await server.shutdown()


if __name__ == "__main__":
    asyncio.run(main())
