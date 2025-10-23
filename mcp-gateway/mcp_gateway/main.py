"""
MCP Gateway main server.
Orchestrates provider discovery and exposes aggregated MCP tools.
"""
import asyncio
import logging
import uuid
from pathlib import Path
from typing import Dict, Any

from mcp.server import Server
from mcp.server.stdio import stdio_server
from mcp.types import Tool, TextContent

from mcp_gateway.providers_registry import ProviderRegistry
from mcp_gateway.adapters.grpc_client import ProviderGRPCClient
from mcp_gateway.validation import SchemaValidator

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)


class MCPGateway:
    """MCP Gateway orchestrator."""

    def __init__(self, config_path: str = "providers.yaml"):
        self.registry = ProviderRegistry()
        self.validator = SchemaValidator()
        self.clients: Dict[str, ProviderGRPCClient] = {}
        self.config_path = config_path
        self.tool_provider_map: Dict[str, str] = {}  # tool_name -> provider_name
        self.venue_provider_map: Dict[str, str] = {}  # venue -> provider_name (for unified tool routing)

    async def initialize(self):
        """Initialize gateway by discovering providers and their capabilities."""
        logger.info("Initializing MCP Gateway...")

        # Load provider configurations
        providers = self.registry.load_providers(self.config_path)
        logger.info(f"Loaded {len(providers)} providers from {self.config_path}")

        # Create gRPC clients for each provider
        for provider in providers:
            client = ProviderGRPCClient(provider.name, provider.address)
            self.clients[provider.name] = client
            logger.info(f"Created gRPC client for provider: {provider.name}")

        # Discover capabilities from all providers
        await self.discover_all_capabilities()

    async def discover_all_capabilities(self):
        """Discover capabilities from all providers."""
        for provider_name, client in self.clients.items():
            try:
                capabilities = await client.list_capabilities()
                self.registry.cache_capabilities(provider_name, capabilities)

                # Build tool -> provider mapping
                for tool in capabilities.get("tools", []):
                    tool_name = tool["name"]
                    self.tool_provider_map[tool_name] = provider_name

                # Build venue -> provider mapping for unified tool routing
                # Extract venue from provider name (e.g., "binance-provider" -> "binance")
                # This supports multi-venue deployments
                venue = provider_name.split("-")[0] if "-" in provider_name else provider_name
                self.venue_provider_map[venue.lower()] = provider_name
                logger.info(f"Registered venue '{venue}' -> provider '{provider_name}'")

                logger.info(
                    f"Provider {provider_name}: "
                    f"{len(capabilities.get('tools', []))} tools, "
                    f"{len(capabilities.get('resources', []))} resources, "
                    f"{len(capabilities.get('prompts', []))} prompts"
                )
            except Exception as e:
                logger.error(f"Failed to discover capabilities from {provider_name}: {e}")

    def get_all_tools(self) -> list[Tool]:
        """
        Get all tools from all providers as MCP Tool objects.
        Feature 018 - FR-002: ONLY expose the unified market report tool.
        """
        tools = []
        all_capabilities = self.registry.get_all_capabilities()

        for provider_name, capabilities in all_capabilities.items():
            for tool_def in capabilities.get("tools", []):
                # Feature 018 - FR-002: Only allow generate_market_report tool
                tool_name = tool_def["name"]

                # Skip all tools except generate_market_report (per FR-002)
                if not tool_name.endswith("generate_market_report"):
                    logger.debug(f"Skipping tool {tool_name} per FR-002 (not generate_market_report)")
                    continue

                # Create MCP Tool object for the unified report tool
                # Expose as market.generate_report (unified name)
                tool = Tool(
                    name="market.generate_report",
                    description="Generate comprehensive market intelligence report combining price, orderbook, liquidity, volume profile, order flow, anomalies, and market health into single markdown document.",
                    inputSchema={
                        "type": "object",
                        "required": ["instrument"],
                        "properties": {
                            "venue": {
                                "type": "string",
                                "description": "Exchange venue (optional, default: binance)",
                                "default": "binance"
                            },
                            "instrument": {
                                "type": "string",
                                "description": "Trading pair symbol (e.g., BTCUSDT)"
                            },
                            "options": {
                                "type": "object",
                                "description": "Report generation options (optional)",
                                "properties": {
                                    "include_sections": {
                                        "type": "array",
                                        "items": {"type": "string"}
                                    },
                                    "volume_window_hours": {
                                        "type": "integer",
                                        "minimum": 1,
                                        "maximum": 168,
                                        "default": 24
                                    },
                                    "orderbook_levels": {
                                        "type": "integer",
                                        "minimum": 1,
                                        "maximum": 100,
                                        "default": 20
                                    }
                                }
                            }
                        }
                    }
                )
                tools.append(tool)
                # Map the unified tool name to the provider tool
                self.tool_provider_map["market.generate_report"] = provider_name

        logger.info(f"Returning {len(tools)} tool(s) per FR-002: {[t.name for t in tools]}")
        return tools

    async def invoke_tool(self, tool_name: str, arguments: Dict[str, Any]) -> list[TextContent]:
        """
        Invoke a tool by routing to the appropriate provider.
        Feature 018 - FR-002: Only market.generate_report is accepted.

        Args:
            tool_name: Name of the tool to invoke
            arguments: Tool arguments

        Returns:
            List of TextContent with result
        """
        # Generate correlation ID for tracing
        correlation_id = str(uuid.uuid4())

        # Feature 018 - FR-002: Only accept market.generate_report
        if tool_name != "market.generate_report":
            error_msg = f"Tool '{tool_name}' is not available. Only 'market.generate_report' is exposed (Feature 018 - FR-002)."
            logger.warning(error_msg)
            import json
            return [TextContent(type="text", text=json.dumps({
                "error": error_msg,
                "error_code": "TOOL_NOT_AVAILABLE",
                "available_tool": "market.generate_report"
            }, indent=2))]

        # P0 Fix: Honor venue parameter to route to correct provider
        # Extract venue from arguments (default to "binance")
        venue = arguments.get("venue", "binance").lower()

        # Find provider for this venue
        provider_name = self.venue_provider_map.get(venue)
        if not provider_name:
            available_venues = list(self.venue_provider_map.keys())
            error_msg = f"Venue '{venue}' not found. Available venues: {available_venues}"
            logger.error(error_msg)
            import json
            return [TextContent(type="text", text=json.dumps({
                "error": error_msg,
                "error_code": "VENUE_NOT_FOUND",
                "available_venues": available_venues
            }, indent=2))]

        # Get the provider's actual tool name (binance.generate_market_report)
        capabilities = self.registry.get_cached_capabilities(provider_name)
        provider_tool_name = None
        for tool_def in capabilities.get("tools", []):
            if tool_def["name"].endswith("generate_market_report"):
                provider_tool_name = tool_def["name"]
                break

        if not provider_tool_name:
            error_msg = f"Provider tool generate_market_report not found in {provider_name}"
            logger.error(error_msg)
            return [TextContent(type="text", text=error_msg)]

        # Map 'instrument' to 'symbol' if needed
        provider_arguments = arguments.copy()
        if "instrument" in provider_arguments:
            provider_arguments["symbol"] = provider_arguments.pop("instrument")

        # Remove 'venue' parameter as it's already used for routing
        provider_arguments.pop("venue", None)

        logger.info(f"Routing {tool_name} for venue '{venue}' to provider '{provider_name}' (tool: {provider_tool_name}) with args: {provider_arguments}")

        # Invoke the tool on the provider
        client = self.clients[provider_name]
        try:
            response = await client.invoke(provider_tool_name, provider_arguments, correlation_id)

            if "error" in response:
                error_msg = f"Tool invocation failed: {response['error']}"
                logger.error(error_msg)
                return [TextContent(type="text", text=error_msg)]

            # Return result as text content
            result = response.get("result", {})
            import json
            return [TextContent(type="text", text=json.dumps(result, indent=2))]

        except Exception as e:
            error_msg = f"Failed to invoke tool {tool_name} on provider {provider_name}: {e}"
            logger.error(error_msg)
            import json
            return [TextContent(type="text", text=json.dumps({"error": error_msg}, indent=2))]

    async def shutdown(self):
        """Shutdown gateway and close all provider connections."""
        logger.info("Shutting down MCP Gateway...")
        for provider_name, client in self.clients.items():
            await client.close()
        logger.info("Gateway shutdown complete")


async def main():
    """Main entry point for MCP Gateway server."""
    # Resolve config path
    config_path = Path(__file__).parent.parent / "providers.yaml"

    # Create gateway instance
    gateway = MCPGateway(str(config_path))

    # Create MCP server
    server = Server("mcp-gateway")

    # Initialize gateway
    await gateway.initialize()

    # Register list_tools handler
    @server.list_tools()
    async def list_tools():
        """List all available tools from all providers."""
        return gateway.get_all_tools()

    # Register call_tool handler
    @server.call_tool()
    async def call_tool(name: str, arguments: dict):
        """Call a tool by routing to the appropriate provider."""
        return await gateway.invoke_tool(name, arguments)

    # Run server with stdio transport
    async with stdio_server() as (read_stream, write_stream):
        logger.info("MCP Gateway server started on stdio")
        try:
            await server.run(
                read_stream,
                write_stream,
                server.create_initialization_options()
            )
        finally:
            await gateway.shutdown()


if __name__ == "__main__":
    asyncio.run(main())
