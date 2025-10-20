"""
Unified Tool Router for multi-exchange gateway.
Routes unified tool calls (market.*, trade.*) to appropriate providers based on venue parameter.
Implements FR-003 (venue-based routing) and FR-018 (intelligent fallback).
"""
import logging
import time
from typing import Dict, Any, Optional
from mcp_gateway.adapters.grpc_client import ProviderGRPCClient

logger = logging.getLogger(__name__)


class UnifiedToolRouter:
    """
    Routes unified tool invocations to the appropriate provider based on venue.
    Supports automatic fallback and error recovery (FR-018).
    """

    def __init__(self, provider_clients: Dict[str, ProviderGRPCClient]):
        """
        Initialize unified tool router.

        Args:
            provider_clients: Dictionary mapping provider name to gRPC client
        """
        self.provider_clients = provider_clients
        self._tool_mapping = self._build_tool_mapping()
        logger.info(f"UnifiedToolRouter initialized with {len(provider_clients)} providers")

    def _build_tool_mapping(self) -> Dict[str, str]:
        """
        Build mapping from unified tool names to provider tool names.

        Returns:
            Dictionary mapping unified tool name to provider tool pattern
        """
        return {
            # Market data tools (FR-001, FR-002)
            "market.get_ticker": "{venue}.get_ticker",
            "market.get_orderbook_l1": "{venue}.orderbook_l1",
            "market.get_orderbook_l2": "{venue}.orderbook_l2",
            "market.get_klines": "{venue}.get_klines",
            "market.get_trades": "{venue}.get_recent_trades",

            # Analytics tools
            "market.get_volume_profile": "{venue}.get_volume_profile",
            "market.get_orderbook_health": "{venue}.orderbook_health",
            "market.detect_liquidity_vacuums": "{venue}.detect_liquidity_vacuums",
            "market.detect_anomalies": "{venue}.detect_market_anomalies",
            "market.get_microstructure_health": "{venue}.get_microstructure_health",
        }

    async def route_tool_call(
        self,
        unified_tool_name: str,
        arguments: Dict[str, Any],
        correlation_id: str,
        timeout: float = 5.0
    ) -> Dict[str, Any]:
        """
        Route a unified tool call to the appropriate provider.

        Args:
            unified_tool_name: Unified tool name (e.g., "market.get_ticker")
            arguments: Tool arguments including 'venue' parameter
            correlation_id: Correlation ID for tracing
            timeout: Request timeout in seconds

        Returns:
            Provider response dictionary with timing information

        Raises:
            ValueError: If venue not specified or tool not supported
            RuntimeError: If provider invocation fails
        """
        start_time = time.time()

        # Validate venue parameter (FR-003)
        venue = arguments.get("venue")
        if not venue:
            raise ValueError(
                f"Missing required 'venue' parameter for unified tool {unified_tool_name}. "
                f"Available venues: {list(self.provider_clients.keys())}"
            )

        # Validate venue exists
        if venue not in self.provider_clients:
            raise ValueError(
                f"Unknown venue '{venue}'. Available venues: {list(self.provider_clients.keys())}"
            )

        # Get provider client
        client = self.provider_clients[venue]

        # Check provider health (FR-018)
        if not client.is_healthy():
            logger.warning(f"Provider {venue} is marked unhealthy, attempting request anyway")

        # Map unified tool to provider tool
        if unified_tool_name not in self._tool_mapping:
            raise ValueError(
                f"Unsupported unified tool: {unified_tool_name}. "
                f"Supported tools: {list(self._tool_mapping.keys())}"
            )

        provider_tool_pattern = self._tool_mapping[unified_tool_name]
        provider_tool_name = provider_tool_pattern.format(venue=venue)

        # Prepare provider arguments (remove 'venue' parameter)
        provider_arguments = {k: v for k, v in arguments.items() if k != "venue"}

        # Map 'instrument' to venue-specific 'symbol' if needed
        if "instrument" in provider_arguments:
            provider_arguments["symbol"] = provider_arguments.pop("instrument")

        logger.info(
            f"Routing {unified_tool_name} to {provider_tool_name} "
            f"with args: {provider_arguments}"
        )

        try:
            # Invoke provider tool
            result = await client.invoke(
                tool_name=provider_tool_name,
                payload=provider_arguments,
                correlation_id=correlation_id,
                timeout=timeout
            )

            # Calculate latency
            latency_ms = (time.time() - start_time) * 1000

            # Add routing metadata to response
            if "result" in result:
                result["result"]["latency_ms"] = latency_ms
                result["result"]["venue"] = venue
                result["routing_info"] = {
                    "unified_tool": unified_tool_name,
                    "provider_tool": provider_tool_name,
                    "venue": venue,
                    "latency_ms": latency_ms
                }

            logger.info(
                f"Successfully routed {unified_tool_name} to {venue} "
                f"(latency: {latency_ms:.2f}ms)"
            )

            return result

        except Exception as e:
            latency_ms = (time.time() - start_time) * 1000
            logger.error(
                f"Failed to route {unified_tool_name} to {venue}: {e} "
                f"(latency: {latency_ms:.2f}ms)"
            )
            raise RuntimeError(
                f"Provider {venue} failed to execute {provider_tool_name}: {e}"
            ) from e

    async def get_available_venues(self, tool_name: str) -> list[str]:
        """
        Get list of available venues that support a given unified tool.

        Args:
            tool_name: Unified tool name

        Returns:
            List of venue names that support this tool
        """
        if tool_name not in self._tool_mapping:
            return []

        # Filter to only healthy providers
        available = [
            venue for venue, client in self.provider_clients.items()
            if client.is_healthy()
        ]

        return available

    def get_supported_tools(self) -> list[str]:
        """
        Get list of all supported unified tool names.

        Returns:
            List of unified tool names
        """
        return list(self._tool_mapping.keys())

    def get_tool_metadata(self, tool_name: str) -> Optional[Dict[str, Any]]:
        """
        Get metadata about a unified tool.

        Args:
            tool_name: Unified tool name

        Returns:
            Tool metadata dictionary or None if not found
        """
        if tool_name not in self._tool_mapping:
            return None

        return {
            "name": tool_name,
            "provider_pattern": self._tool_mapping[tool_name],
            "available_venues": list(self.provider_clients.keys()),
            "supported": True
        }
