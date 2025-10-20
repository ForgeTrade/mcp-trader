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
            """List ONLY unified tools (FR-001, FR-002). Provider-specific tools are hidden."""
            # FR-001, FR-002: Expose ONLY unified tools, hide provider-specific tools
            # Feature 014: Use PUBLIC_VENUES instead of internal provider IDs (FR-005, FR-010)
            venues_list = PUBLIC_VENUES

            unified_tools = [
                Tool(
                    name="market.get_ticker",
                    description=f"Get normalized ticker data (bid, ask, mid, spread_bps) for any venue. Available venues: {venues_list}",
                    inputSchema={
                        "type": "object",
                        "required": ["instrument"],  # Feature 014: venue is optional (FR-011)
                        "properties": {
                            "venue": {
                                "type": "string",
                                "description": "Exchange venue (optional, default: binance)",
                                "enum": venues_list,
                                "default": "binance"  # Feature 014: FR-002
                            },
                            "instrument": {
                                "type": "string",
                                "description": "Trading pair symbol (e.g., BTCUSDT)",
                                "examples": ["BTCUSDT", "ETHUSDT"]
                            }
                        }
                    }
                ),
                Tool(
                    name="market.get_orderbook_l1",
                    description=f"Get normalized top-of-book orderbook (L1) for any venue. Available venues: {venues_list}",
                    inputSchema={
                        "type": "object",
                        "required": ["instrument"],  # Feature 014: venue is optional (FR-011)
                        "properties": {
                            "venue": {
                                "type": "string",
                                "description": "Exchange venue (optional, default: binance)",
                                "enum": venues_list,
                                "default": "binance"  # Feature 014: FR-002
                            },
                            "instrument": {
                                "type": "string",
                                "description": "Trading pair symbol (e.g., BTCUSDT)",
                                "examples": ["BTCUSDT", "ETHUSDT"]
                            }
                        }
                    }
                ),
                Tool(
                    name="market.get_orderbook_l2",
                    description=f"Get normalized full depth orderbook (L2) for any venue. Available venues: {venues_list}",
                    inputSchema={
                        "type": "object",
                        "required": ["instrument"],  # Feature 014: venue is optional (FR-011)
                        "properties": {
                            "venue": {
                                "type": "string",
                                "description": "Exchange venue (optional, default: binance)",
                                "enum": venues_list,
                                "default": "binance"  # Feature 014: FR-002
                            },
                            "instrument": {
                                "type": "string",
                                "description": "Trading pair symbol (e.g., BTCUSDT)",
                                "examples": ["BTCUSDT", "ETHUSDT"]
                            },
                            "limit": {
                                "type": "integer",
                                "description": "Number of price levels to return (default: 100)",
                                "default": 100
                            }
                        }
                    }
                ),
                Tool(
                    name="market.get_klines",
                    description=f"Get normalized historical klines/candlesticks for any venue. Available venues: {venues_list}",
                    inputSchema={
                        "type": "object",
                        "required": ["instrument", "interval"],  # Feature 014: venue is optional (FR-011)
                        "properties": {
                            "venue": {
                                "type": "string",
                                "description": "Exchange venue (optional, default: binance)",
                                "enum": venues_list,
                                "default": "binance"  # Feature 014: FR-002
                            },
                            "instrument": {
                                "type": "string",
                                "description": "Trading pair symbol (e.g., BTCUSDT)",
                                "examples": ["BTCUSDT", "ETHUSDT"]
                            },
                            "interval": {
                                "type": "string",
                                "description": "Kline interval (e.g., 1m, 5m, 1h, 1d)",
                                "examples": ["1m", "5m", "15m", "1h", "4h", "1d"]
                            },
                            "limit": {
                                "type": "integer",
                                "description": "Number of klines to return (default: 500)",
                                "default": 500
                            }
                        }
                    }
                ),

                # NEW: Additional market data tools (Feature 013 - FR-014, FR-015, FR-016)
                Tool(
                    name="market.get_recent_trades",
                    description=f"Get recent trades for an instrument on any venue. Available venues: {venues_list}",
                    inputSchema={
                        "type": "object",
                        "required": ["instrument"],  # Feature 014: venue is optional (FR-011)
                        "properties": {
                            "venue": {
                                "type": "string",
                                "description": "Exchange venue (optional, default: binance)",
                                "enum": venues_list,
                                "default": "binance"  # Feature 014: FR-002
                            },
                            "instrument": {
                                "type": "string",
                                "description": "Trading pair symbol (e.g., BTCUSDT)",
                                "examples": ["BTCUSDT", "ETHUSDT"]
                            },
                            "limit": {
                                "type": "integer",
                                "description": "Number of recent trades to return (default: 100, max: 1000)",
                                "default": 100
                            }
                        }
                    }
                ),
                Tool(
                    name="market.get_exchange_info",
                    description=f"Get trading rules and symbol information for any venue. Available venues: {venues_list}",
                    inputSchema={
                        "type": "object",
                        "required": ["instrument"],  # Feature 014: venue is optional (FR-011)
                        "properties": {
                            "venue": {
                                "type": "string",
                                "description": "Exchange venue (optional, default: binance)",
                                "enum": venues_list,
                                "default": "binance"  # Feature 014: FR-002
                            },
                            "instrument": {
                                "type": "string",
                                "description": "Trading pair symbol (e.g., BTCUSDT)",
                                "examples": ["BTCUSDT", "ETHUSDT"]
                            }
                        }
                    }
                ),
                Tool(
                    name="market.get_avg_price",
                    description=f"Get current average price for an instrument on any venue. Available venues: {venues_list}",
                    inputSchema={
                        "type": "object",
                        "required": ["instrument"],  # Feature 014: venue is optional (FR-011)
                        "properties": {
                            "venue": {
                                "type": "string",
                                "description": "Exchange venue (optional, default: binance)",
                                "enum": venues_list,
                                "default": "binance"  # Feature 014: FR-002
                            },
                            "instrument": {
                                "type": "string",
                                "description": "Trading pair symbol (e.g., BTCUSDT)",
                                "examples": ["BTCUSDT", "ETHUSDT"]
                            }
                        }
                    }
                ),

                # NEW: Trading tools (Feature 013 - FR-001 to FR-007)
                Tool(
                    name="trade.place_order",
                    description=f"Place a new order on any venue. Available venues: {venues_list}",
                    inputSchema={
                        "type": "object",
                        "required": ["instrument", "side", "type", "quantity"],  # Feature 014: venue is optional (FR-011)
                        "properties": {
                            "venue": {
                                "type": "string",
                                "description": f"Exchange venue to place order. Available: {', '.join(venues_list)}",
                                "enum": venues_list,
                                "default": "binance"  # Feature 014: FR-002
                            },
                            "instrument": {
                                "type": "string",
                                "description": "Trading pair (e.g., BTCUSDT)"
                            },
                            "side": {
                                "type": "string",
                                "description": "Order side",
                                "enum": ["BUY", "SELL"]
                            },
                            "type": {
                                "type": "string",
                                "description": "Order type",
                                "enum": ["LIMIT", "MARKET", "STOP_LOSS", "STOP_LOSS_LIMIT"]
                            },
                            "quantity": {
                                "type": "number",
                                "description": "Order quantity"
                            },
                            "price": {
                                "type": "number",
                                "description": "Order price (required for LIMIT orders)"
                            }
                        }
                    }
                ),
                Tool(
                    name="trade.cancel_order",
                    description=f"Cancel an existing order on any venue. Available venues: {venues_list}",
                    inputSchema={
                        "type": "object",
                        "required": ["instrument", "order_id"],  # Feature 014: venue is optional (FR-011)
                        "properties": {
                            "venue": {
                                "type": "string",
                                "description": f"Exchange venue. Available: {', '.join(venues_list)}",
                                "enum": venues_list,
                                "default": "binance"  # Feature 014: FR-002
                            },
                            "instrument": {
                                "type": "string",
                                "description": "Trading pair (e.g., BTCUSDT)"
                            },
                            "order_id": {
                                "type": "string",
                                "description": "Order ID to cancel"
                            }
                        }
                    }
                ),
                Tool(
                    name="trade.get_order",
                    description=f"Get order status on any venue. Available venues: {venues_list}",
                    inputSchema={
                        "type": "object",
                        "required": ["instrument", "order_id"],  # Feature 014: venue is optional (FR-011)
                        "properties": {
                            "venue": {
                                "type": "string",
                                "description": f"Exchange venue. Available: {', '.join(venues_list)}",
                                "enum": venues_list,
                                "default": "binance"  # Feature 014: FR-002
                            },
                            "instrument": {
                                "type": "string",
                                "description": "Trading pair (e.g., BTCUSDT)"
                            },
                            "order_id": {
                                "type": "string",
                                "description": "Order ID to query"
                            }
                        }
                    }
                ),
                Tool(
                    name="trade.get_open_orders",
                    description=f"Get all open orders on any venue. Available venues: {venues_list}",
                    inputSchema={
                        "type": "object",
                        "required": [],  # Feature 014: venue is optional (FR-011)
                        "properties": {
                            "venue": {
                                "type": "string",
                                "description": f"Exchange venue. Available: {', '.join(venues_list)}",
                                "enum": venues_list,
                                "default": "binance"  # Feature 014: FR-002
                            },
                            "instrument": {
                                "type": "string",
                                "description": "Trading pair (optional filter, e.g., BTCUSDT)"
                            }
                        }
                    }
                ),
                Tool(
                    name="trade.get_all_orders",
                    description=f"Get all orders (open and closed) on any venue. Available venues: {venues_list}",
                    inputSchema={
                        "type": "object",
                        "required": [],  # Feature 014: venue is optional (FR-011)
                        "properties": {
                            "venue": {
                                "type": "string",
                                "description": f"Exchange venue. Available: {', '.join(venues_list)}",
                                "enum": venues_list,
                                "default": "binance"  # Feature 014: FR-002
                            },
                            "instrument": {
                                "type": "string",
                                "description": "Trading pair (optional filter, e.g., BTCUSDT)"
                            },
                            "start_time": {
                                "type": "integer",
                                "description": "Start time filter (milliseconds)"
                            },
                            "end_time": {
                                "type": "integer",
                                "description": "End time filter (milliseconds)"
                            }
                        }
                    }
                ),
                Tool(
                    name="trade.get_account",
                    description=f"Get account balances and trading permissions on any venue. Available venues: {venues_list}",
                    inputSchema={
                        "type": "object",
                        "required": [],  # Feature 014: venue is optional (FR-011)
                        "properties": {
                            "venue": {
                                "type": "string",
                                "description": f"Exchange venue. Available: {', '.join(venues_list)}",
                                "enum": venues_list,
                                "default": "binance"  # Feature 014: FR-002
                            }
                        }
                    }
                ),
                Tool(
                    name="trade.get_my_trades",
                    description=f"Get account trade history on any venue. Available venues: {venues_list}",
                    inputSchema={
                        "type": "object",
                        "required": [],  # Feature 014: venue is optional (FR-011)
                        "properties": {
                            "venue": {
                                "type": "string",
                                "description": f"Exchange venue. Available: {', '.join(venues_list)}",
                                "enum": venues_list,
                                "default": "binance"  # Feature 014: FR-002
                            },
                            "instrument": {
                                "type": "string",
                                "description": "Trading pair (optional filter, e.g., BTCUSDT)"
                            },
                            "start_time": {
                                "type": "integer",
                                "description": "Start time filter (milliseconds)"
                            },
                            "end_time": {
                                "type": "integer",
                                "description": "End time filter (milliseconds)"
                            }
                        }
                    }
                ),

                # NEW: Analytics tools (Feature 013 - FR-008 to FR-013)
                Tool(
                    name="analytics.get_orderbook_health",
                    description=f"Get orderbook health metrics on any venue. Available venues: {venues_list}",
                    inputSchema={
                        "type": "object",
                        "required": ["instrument"],  # Feature 014: venue is optional (FR-011)
                        "properties": {
                            "venue": {
                                "type": "string",
                                "description": "Exchange venue (optional, default: binance)",
                                "enum": venues_list,
                                "default": "binance"  # Feature 014: FR-002
                            },
                            "instrument": {
                                "type": "string",
                                "description": "Trading pair (e.g., BTCUSDT)",
                                "examples": ["BTCUSDT", "ETHUSDT"]
                            }
                        }
                    }
                ),
                Tool(
                    name="analytics.get_order_flow",
                    description=f"Analyze bid/ask order flow dynamics on any venue. Available venues: {venues_list}",
                    inputSchema={
                        "type": "object",
                        "required": ["instrument"],  # Feature 014: venue is optional (FR-011)
                        "properties": {
                            "venue": {
                                "type": "string",
                                "description": "Exchange venue (optional, default: binance)",
                                "enum": venues_list,
                                "default": "binance"  # Feature 014: FR-002
                            },
                            "instrument": {
                                "type": "string",
                                "description": "Trading pair (e.g., BTCUSDT)"
                            },
                            "window_seconds": {
                                "type": "integer",
                                "description": "Analysis window in seconds (default: 300)",
                                "default": 300
                            }
                        }
                    }
                ),
                Tool(
                    name="analytics.get_volume_profile",
                    description=f"Get volume distribution histogram on any venue. Available venues: {venues_list}",
                    inputSchema={
                        "type": "object",
                        "required": ["instrument"],  # Feature 014: venue is optional (FR-011)
                        "properties": {
                            "venue": {
                                "type": "string",
                                "description": "Exchange venue (optional, default: binance)",
                                "enum": venues_list,
                                "default": "binance"  # Feature 014: FR-002
                            },
                            "instrument": {
                                "type": "string",
                                "description": "Trading pair (e.g., BTCUSDT)"
                            },
                            "interval": {
                                "type": "string",
                                "description": "Time interval (e.g., 1h, 4h, 1d)",
                                "default": "1h"
                            },
                            "periods": {
                                "type": "integer",
                                "description": "Number of periods to analyze (default: 24)",
                                "default": 24
                            }
                        }
                    }
                ),
                Tool(
                    name="analytics.detect_market_anomalies",
                    description=f"Detect market anomalies (quote stuffing, flash crash risks) on any venue. Available venues: {venues_list}",
                    inputSchema={
                        "type": "object",
                        "required": ["instrument"],  # Feature 014: venue is optional (FR-011)
                        "properties": {
                            "venue": {
                                "type": "string",
                                "description": "Exchange venue (optional, default: binance)",
                                "enum": venues_list,
                                "default": "binance"  # Feature 014: FR-002
                            },
                            "instrument": {
                                "type": "string",
                                "description": "Trading pair (e.g., BTCUSDT)"
                            },
                            "window_seconds": {
                                "type": "integer",
                                "description": "Detection window in seconds (default: 60)",
                                "default": 60
                            }
                        }
                    }
                ),
                Tool(
                    name="analytics.get_microstructure_health",
                    description=f"Get composite market microstructure health score on any venue. Available venues: {venues_list}",
                    inputSchema={
                        "type": "object",
                        "required": ["instrument"],  # Feature 014: venue is optional (FR-011)
                        "properties": {
                            "venue": {
                                "type": "string",
                                "description": "Exchange venue (optional, default: binance)",
                                "enum": venues_list,
                                "default": "binance"  # Feature 014: FR-002
                            },
                            "instrument": {
                                "type": "string",
                                "description": "Trading pair (e.g., BTCUSDT)",
                                "examples": ["BTCUSDT", "ETHUSDT"]
                            }
                        }
                    }
                ),
                Tool(
                    name="analytics.detect_liquidity_vacuums",
                    description=f"Identify price levels with abnormally low liquidity on any venue. Available venues: {venues_list}",
                    inputSchema={
                        "type": "object",
                        "required": ["instrument"],  # Feature 014: venue is optional (FR-011)
                        "properties": {
                            "venue": {
                                "type": "string",
                                "description": "Exchange venue (optional, default: binance)",
                                "enum": venues_list,
                                "default": "binance"  # Feature 014: FR-002
                            },
                            "instrument": {
                                "type": "string",
                                "description": "Trading pair (e.g., BTCUSDT)",
                                "examples": ["BTCUSDT", "ETHUSDT"]
                            }
                        }
                    }
                ),
            ]

            logger.info(f"Returning {len(unified_tools)} unified tools (provider-specific tools hidden)")
            return unified_tools

        @self.server.call_tool()
        async def call_tool(name: str, arguments: dict):
            """Handle tool calls - ONLY unified tools are accepted (FR-001, FR-002, FR-007)."""
            logger.info(f"Tool called: {name} with arguments: {arguments}")

            try:
                # Generate a correlation ID for tracking
                import uuid
                correlation_id = str(uuid.uuid4())

                # FR-001, FR-002: Check if this is a unified tool (Feature 013: added analytics.*)
                is_unified_tool = name.startswith("market.") or name.startswith("trade.") or name.startswith("analytics.")

                # FR-007: Reject provider-specific tool calls with helpful error
                if not is_unified_tool:
                    # This is a provider-specific tool - reject it
                    error_msg = f"Tool '{name}' is not available. This gateway only exposes unified tools."

                    # Suggest the unified alternative
                    suggestion = None
                    if "get_ticker" in name:
                        suggestion = "market.get_ticker"
                    elif "orderbook_l1" in name:
                        suggestion = "market.get_orderbook_l1"
                    elif "orderbook_l2" in name:
                        suggestion = "market.get_orderbook_l2"
                    elif "get_klines" in name:
                        suggestion = "market.get_klines"

                    if suggestion:
                        error_msg += f" Use '{suggestion}' with a 'venue' parameter instead."

                    logger.warning(f"Rejected provider-specific tool call: {name}")
                    return [TextContent(
                        type="text",
                        text=json.dumps({
                            "error": error_msg,
                            "error_code": "TOOL_NOT_AVAILABLE",
                            "unified_alternative": suggestion,
                            "available_venues": list(self.provider_clients.keys())
                        }, indent=2)
                    )]

                # Handle unified tool calls
                if is_unified_tool and self.unified_router:
                    # Route through UnifiedToolRouter (T021, FR-028)
                    logger.info(f"Routing unified tool {name} through UnifiedToolRouter")

                    try:
                        result = await self.unified_router.route_tool_call(
                            unified_tool_name=name,
                            arguments=arguments,
                            correlation_id=correlation_id,
                            timeout=5.0
                        )

                        # Extract venue and raw result
                        venue = arguments.get("venue")
                        if "result" in result:
                            raw_response = result["result"]

                            # T021: Apply schema normalization based on tool type (FR-007)
                            if name == "market.get_ticker":
                                normalized = self.schema_adapter.normalize(
                                    venue=venue,
                                    data_type="ticker",
                                    raw_response=raw_response,
                                    additional_fields={"latency_ms": result.get("routing_info", {}).get("latency_ms")}
                                )
                                return [TextContent(
                                    type="text",
                                    text=json.dumps({"result": normalized}, indent=2)
                                )]

                            elif name == "market.get_orderbook_l1":
                                normalized = self.schema_adapter.normalize(
                                    venue=venue,
                                    data_type="orderbook_l1",
                                    raw_response=raw_response,
                                    additional_fields={"latency_ms": result.get("routing_info", {}).get("latency_ms")}
                                )
                                return [TextContent(
                                    type="text",
                                    text=json.dumps({"result": normalized}, indent=2)
                                )]

                            elif name == "market.get_orderbook_l2":
                                normalized = self.schema_adapter.normalize(
                                    venue=venue,
                                    data_type="orderbook_l2",
                                    raw_response=raw_response,
                                    additional_fields={"latency_ms": result.get("routing_info", {}).get("latency_ms")}
                                )
                                return [TextContent(
                                    type="text",
                                    text=json.dumps({"result": normalized}, indent=2)
                                )]

                            # NEW: Additional market data tools (Feature 013)
                            elif name == "market.get_recent_trades":
                                normalized = self.schema_adapter.normalize(
                                    venue=venue,
                                    data_type="recent_trades",
                                    raw_response=raw_response,
                                    additional_fields={"latency_ms": result.get("routing_info", {}).get("latency_ms")}
                                )
                                return [TextContent(
                                    type="text",
                                    text=json.dumps({"result": normalized}, indent=2)
                                )]

                            elif name == "market.get_exchange_info":
                                normalized = self.schema_adapter.normalize(
                                    venue=venue,
                                    data_type="exchange_info",
                                    raw_response=raw_response,
                                    additional_fields={"latency_ms": result.get("routing_info", {}).get("latency_ms")}
                                )
                                return [TextContent(
                                    type="text",
                                    text=json.dumps({"result": normalized}, indent=2)
                                )]

                            # NEW: Trading tools (Feature 013)
                            elif name == "trade.place_order":
                                normalized = self.schema_adapter.normalize(
                                    venue=venue,
                                    data_type="order",
                                    raw_response=raw_response,
                                    additional_fields={"latency_ms": result.get("routing_info", {}).get("latency_ms")}
                                )
                                return [TextContent(
                                    type="text",
                                    text=json.dumps({"result": normalized}, indent=2)
                                )]

                            elif name == "trade.cancel_order":
                                normalized = self.schema_adapter.normalize(
                                    venue=venue,
                                    data_type="order",
                                    raw_response=raw_response,
                                    additional_fields={"latency_ms": result.get("routing_info", {}).get("latency_ms")}
                                )
                                return [TextContent(
                                    type="text",
                                    text=json.dumps({"result": normalized}, indent=2)
                                )]

                            elif name == "trade.get_order":
                                normalized = self.schema_adapter.normalize(
                                    venue=venue,
                                    data_type="order",
                                    raw_response=raw_response,
                                    additional_fields={"latency_ms": result.get("routing_info", {}).get("latency_ms")}
                                )
                                return [TextContent(
                                    type="text",
                                    text=json.dumps({"result": normalized}, indent=2)
                                )]

                            elif name == "trade.get_open_orders":
                                # Normalize list of orders
                                orders_list = raw_response if isinstance(raw_response, list) else [raw_response]
                                normalized_orders = [
                                    self.schema_adapter.normalize(
                                        venue=venue,
                                        data_type="order",
                                        raw_response=order,
                                        additional_fields={}
                                    )
                                    for order in orders_list
                                ]
                                return [TextContent(
                                    type="text",
                                    text=json.dumps({
                                        "result": {
                                            "orders": normalized_orders,
                                            "latency_ms": result.get("routing_info", {}).get("latency_ms"),
                                            "venue": venue
                                        }
                                    }, indent=2)
                                )]

                            elif name == "trade.get_all_orders":
                                # Normalize list of orders
                                orders_list = raw_response if isinstance(raw_response, list) else [raw_response]
                                normalized_orders = [
                                    self.schema_adapter.normalize(
                                        venue=venue,
                                        data_type="order",
                                        raw_response=order,
                                        additional_fields={}
                                    )
                                    for order in orders_list
                                ]
                                return [TextContent(
                                    type="text",
                                    text=json.dumps({
                                        "result": {
                                            "orders": normalized_orders,
                                            "latency_ms": result.get("routing_info", {}).get("latency_ms"),
                                            "venue": venue
                                        }
                                    }, indent=2)
                                )]

                            elif name == "trade.get_account":
                                normalized = self.schema_adapter.normalize(
                                    venue=venue,
                                    data_type="account",
                                    raw_response=raw_response,
                                    additional_fields={"latency_ms": result.get("routing_info", {}).get("latency_ms")}
                                )
                                return [TextContent(
                                    type="text",
                                    text=json.dumps({"result": normalized}, indent=2)
                                )]

                            elif name == "trade.get_my_trades":
                                # Normalize list of trades
                                trades_list = raw_response if isinstance(raw_response, list) else [raw_response]
                                normalized_trades = [
                                    self.schema_adapter.normalize(
                                        venue=venue,
                                        data_type="trade",
                                        raw_response=trade,
                                        additional_fields={}
                                    )
                                    for trade in trades_list
                                ]
                                return [TextContent(
                                    type="text",
                                    text=json.dumps({
                                        "result": {
                                            "trades": normalized_trades,
                                            "latency_ms": result.get("routing_info", {}).get("latency_ms"),
                                            "venue": venue
                                        }
                                    }, indent=2)
                                )]

                            # NEW: Analytics tools (Feature 013)
                            elif name == "analytics.get_orderbook_health":
                                normalized = self.schema_adapter.normalize(
                                    venue=venue,
                                    data_type="orderbook_health",
                                    raw_response=raw_response,
                                    additional_fields={"latency_ms": result.get("routing_info", {}).get("latency_ms")}
                                )
                                return [TextContent(
                                    type="text",
                                    text=json.dumps({"result": normalized}, indent=2)
                                )]

                            elif name == "analytics.get_volume_profile":
                                normalized = self.schema_adapter.normalize(
                                    venue=venue,
                                    data_type="volume_profile",
                                    raw_response=raw_response,
                                    additional_fields={"latency_ms": result.get("routing_info", {}).get("latency_ms")}
                                )
                                return [TextContent(
                                    type="text",
                                    text=json.dumps({"result": normalized}, indent=2)
                                )]

                            elif name == "analytics.detect_market_anomalies":
                                normalized = self.schema_adapter.normalize(
                                    venue=venue,
                                    data_type="market_anomalies",
                                    raw_response=raw_response,
                                    additional_fields={"latency_ms": result.get("routing_info", {}).get("latency_ms")}
                                )
                                return [TextContent(
                                    type="text",
                                    text=json.dumps({"result": normalized}, indent=2)
                                )]

                            elif name == "analytics.get_microstructure_health":
                                normalized = self.schema_adapter.normalize(
                                    venue=venue,
                                    data_type="microstructure_health",
                                    raw_response=raw_response,
                                    additional_fields={"latency_ms": result.get("routing_info", {}).get("latency_ms")}
                                )
                                return [TextContent(
                                    type="text",
                                    text=json.dumps({"result": normalized}, indent=2)
                                )]

                            elif name == "market.get_klines":
                                # Klines don't need normalization yet - just add venue and latency
                                pass

                            # For tools without specific normalization (market.get_avg_price, analytics.get_order_flow, analytics.detect_liquidity_vacuums)
                            # Return as-is with routing metadata

                        # Return result as-is if no normalization needed
                        return [TextContent(
                            type="text",
                            text=json.dumps(result, indent=2)
                        )]

                    except ValueError as ve:
                        # T022: Enhanced error handling for non-existent instruments (US1 Scenario 4)
                        error_msg = str(ve)
                        logger.warning(f"Unified tool error: {error_msg}")

                        # If error mentions missing symbol/instrument, provide alternatives
                        if "symbol" in error_msg.lower() or "instrument" in error_msg.lower():
                            alternatives = ["BTCUSDT", "ETHUSDT", "BNBUSDT", "SOLUSDT"]
                            return [TextContent(
                                type="text",
                                text=json.dumps({
                                    "error": error_msg,
                                    "error_code": "SYMBOL_NOT_FOUND",
                                    "alternatives": alternatives,
                                    "venue": arguments.get("venue")
                                }, indent=2)
                            )]
                        else:
                            return [TextContent(
                                type="text",
                                text=json.dumps({"error": error_msg}, indent=2)
                            )]

                # FR-001, FR-002: This point should never be reached since we reject non-unified tools earlier
                # If we somehow get here with a non-unified tool, return an error
                error_msg = f"Internal error: unified tool check failed for '{name}'"
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
