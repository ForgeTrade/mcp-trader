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

                logger.info(
                    f"Provider {provider_name}: "
                    f"{len(capabilities.get('tools', []))} tools, "
                    f"{len(capabilities.get('resources', []))} resources, "
                    f"{len(capabilities.get('prompts', []))} prompts"
                )
            except Exception as e:
                logger.error(f"Failed to discover capabilities from {provider_name}: {e}")

    def get_all_tools(self) -> list[Tool]:
        """Get all tools from all providers as MCP Tool objects."""
        tools = []
        all_capabilities = self.registry.get_all_capabilities()

        for provider_name, capabilities in all_capabilities.items():
            for tool_def in capabilities.get("tools", []):
                # Create MCP Tool object
                tool = Tool(
                    name=tool_def["name"],
                    description=tool_def.get("description", ""),
                    inputSchema=tool_def.get("input_schema", {}),
                )
                tools.append(tool)

        return tools

    async def invoke_tool(self, tool_name: str, arguments: Dict[str, Any]) -> list[TextContent]:
        """
        Invoke a tool by routing to the appropriate provider.

        Args:
            tool_name: Name of the tool to invoke
            arguments: Tool arguments

        Returns:
            List of TextContent with result
        """
        # Generate correlation ID for tracing
        correlation_id = str(uuid.uuid4())

        # Find provider for this tool
        provider_name = self.tool_provider_map.get(tool_name)
        if not provider_name:
            error_msg = f"Tool not found: {tool_name}"
            logger.error(error_msg)
            return [TextContent(type="text", text=error_msg)]

        # Get the tool's input schema for validation
        capabilities = self.registry.get_cached_capabilities(provider_name)
        tool_def = next((t for t in capabilities["tools"] if t["name"] == tool_name), None)

        if not tool_def:
            error_msg = f"Tool definition not found: {tool_name}"
            logger.error(error_msg)
            return [TextContent(type="text", text=error_msg)]

        # Validate input arguments against schema
        input_schema = tool_def.get("input_schema")
        if input_schema:
            try:
                self.validator.validate(input_schema, arguments)
            except Exception as e:
                error_msg = f"Input validation failed for {tool_name}: {e}"
                logger.error(error_msg)
                return [TextContent(type="text", text=error_msg)]

        # Invoke the tool on the provider
        client = self.clients[provider_name]
        try:
            response = await client.invoke(tool_name, arguments, correlation_id)

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
            return [TextContent(type="text", text=error_msg)]

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
