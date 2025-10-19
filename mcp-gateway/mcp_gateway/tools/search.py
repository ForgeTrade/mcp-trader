"""
Search tool for ChatGPT MCP integration.
Enables natural language search for cryptocurrency market data.
"""
import json
import logging
import uuid
from typing import Dict, Any, List

from mcp_gateway.document_registry import DocumentRegistry
from mcp_gateway.adapters.grpc_client import ProviderGRPCClient
from mcp_gateway.cache import market_data_cache

logger = logging.getLogger(__name__)


class SearchTool:
    """
    Implements the `search` tool for ChatGPT.
    Maps natural language queries to Binance market data.
    """

    def __init__(self, grpc_client: ProviderGRPCClient, base_url: str = "https://mcp-gateway.thevibe.trading"):
        """
        Initialize search tool.

        Args:
            grpc_client: gRPC client for Binance provider
            base_url: Base URL for citation links
        """
        self.client = grpc_client
        self.base_url = base_url
        self.registry = DocumentRegistry()

    async def search(self, query: str, limit: int = 10) -> Dict[str, Any]:
        """
        Search for cryptocurrency market data.

        Args:
            query: Natural language search query
            limit: Maximum number of results to return

        Returns:
            Search results in ChatGPT-compatible format:
            {
                "results": [
                    {
                        "id": "ticker:BTCUSDT",
                        "title": "Bitcoin (BTC) Price - BTCUSDT",
                        "text": "Current price: $43,250.50...",
                        "url": "https://mcp-gateway.thevibe.trading/data/ticker:BTCUSDT"
                    },
                    ...
                ]
            }
        """
        logger.info(f"Search query: {query}")

        # Extract symbols from query
        symbols = self.registry.extract_symbols_from_query(query)
        if not symbols:
            logger.warning(f"No symbols found in query: {query}")
            return {"results": []}

        # Detect data type requested
        data_type = self.registry.detect_data_type(query)
        logger.info(f"Detected data type: {data_type}, symbols: {symbols}")

        # Generate search results
        results = []
        for symbol in symbols[:limit]:  # Limit number of symbols
            try:
                # Create document ID
                doc_id = self.registry.create_document_id(data_type, symbol)

                # Get tool name for this data type
                doc_obj = self.registry.parse_document_id(doc_id)
                if not doc_obj:
                    continue

                tool_name = self.registry.get_tool_for_document(doc_obj)
                if not tool_name:
                    logger.warning(f"No tool found for document type: {data_type}")
                    continue

                # Try to get cached data, otherwise fetch fresh
                cache_key = f"{tool_name}:{symbol}"
                cached_data = market_data_cache.get(cache_key)

                if cached_data:
                    data = cached_data
                    logger.debug(f"Using cached data for {cache_key}")
                else:
                    # Fetch data from Binance provider
                    tool_args = self.registry.create_tool_arguments(doc_obj)
                    correlation_id = str(uuid.uuid4())

                    response = await self.client.invoke(
                        tool_name=tool_name,
                        payload=tool_args,
                        correlation_id=correlation_id,
                        timeout=2.5
                    )

                    if "error" in response:
                        logger.error(f"Error fetching {tool_name} for {symbol}: {response['error']}")
                        continue

                    data = response.get("result", {})
                    # Cache the result
                    market_data_cache.set(cache_key, data)

                # Create search result
                result = self._create_search_result(doc_id, symbol, data_type, data)
                results.append(result)

            except Exception as e:
                logger.error(f"Error processing symbol {symbol}: {e}", exc_info=True)
                continue

        logger.info(f"Search returned {len(results)} results")
        return {"results": results}

    def _create_search_result(self, doc_id: str, symbol: str, data_type: str, data: Dict[str, Any]) -> Dict[str, Any]:
        """
        Create a search result entry.

        Args:
            doc_id: Document ID
            symbol: Trading symbol
            data_type: Type of data
            data: Data from Binance

        Returns:
            Search result dictionary
        """
        # Generate title
        coin_name = self._get_coin_name(symbol)
        title = self._generate_title(coin_name, symbol, data_type)

        # Generate snippet (first 200 chars of data)
        snippet = self._generate_snippet(data, data_type)

        # Create citation URL
        url = f"{self.base_url}/data/{doc_id}"

        return {
            "id": doc_id,
            "title": title,
            "text": snippet,
            "url": url,
        }

    def _get_coin_name(self, symbol: str) -> str:
        """Get human-readable coin name from symbol."""
        # Reverse lookup in COIN_NAME_TO_SYMBOL
        for name, sym in self.registry.COIN_NAME_TO_SYMBOL.items():
            if sym == symbol:
                return name.title()

        # Extract base currency from symbol (e.g., BTC from BTCUSDT)
        if symbol.endswith("USDT"):
            return symbol[:-4]
        elif symbol.endswith("BUSD"):
            return symbol[:-4]
        elif symbol.endswith("BTC"):
            return symbol[:-3]
        return symbol

    def _generate_title(self, coin_name: str, symbol: str, data_type: str) -> str:
        """Generate title for search result."""
        type_labels = {
            "ticker": "Price",
            "orderbook": "Order Book",
            "orderbook_l1": "Order Book",
            "orderbook_l2": "Order Book (L2)",
            "klines": "Candlestick Data",
            "trades": "Recent Trades",
            "volume_profile": "Volume Profile",
            "orderbook_health": "Order Book Health",
            "liquidity_vacuums": "Liquidity Analysis",
            "market_anomalies": "Market Anomalies",
            "microstructure_health": "Microstructure Health",
        }

        type_label = type_labels.get(data_type, data_type.replace("_", " ").title())
        return f"{coin_name} {type_label} - {symbol}"

    def _generate_snippet(self, data: Dict[str, Any], data_type: str) -> str:
        """Generate snippet text from data (max 200 chars)."""
        try:
            if data_type == "ticker":
                # Ticker data snippet
                price = data.get("lastPrice", "N/A")
                volume = data.get("volume", "N/A")
                change = data.get("priceChangePercent", "N/A")
                return f"Price: ${price} | 24h Volume: {volume} | Change: {change}%"

            elif data_type in ("orderbook", "orderbook_l1", "orderbook_l2"):
                # Orderbook snippet
                bids = len(data.get("bids", []))
                asks = len(data.get("asks", []))
                if bids > 0 and asks > 0:
                    best_bid = data["bids"][0][0] if data.get("bids") else "N/A"
                    best_ask = data["asks"][0][0] if data.get("asks") else "N/A"
                    return f"Best Bid: ${best_bid} | Best Ask: ${best_ask} | Levels: {bids} bids, {asks} asks"
                return f"Order book with {bids} bids and {asks} asks"

            elif data_type == "volume_profile":
                # Volume profile snippet
                poc = data.get("poc_price", "N/A")
                total_vol = data.get("total_volume", "N/A")
                return f"POC: ${poc} | Total Volume: {total_vol}"

            elif data_type == "orderbook_health":
                # Orderbook health snippet
                spread = data.get("spread_bps", "N/A")
                imbalance = data.get("imbalance_ratio", "N/A")
                return f"Spread: {spread} bps | Imbalance: {imbalance}"

            else:
                # Generic snippet - JSON dump truncated to 200 chars
                json_str = json.dumps(data, indent=None)
                return json_str[:200] + ("..." if len(json_str) > 200 else "")

        except Exception as e:
            logger.error(f"Error generating snippet: {e}")
            return "Data available - use fetch for details"


# Tool schema for ChatGPT
SEARCH_TOOL_SCHEMA = {
    "name": "search",
    "description": "Search for cryptocurrency market data from Binance. Supports queries for price, orderbook, volume, and analytics.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "query": {
                "type": "string",
                "description": "Natural language search query (e.g., 'Bitcoin price', 'ETHUSDT orderbook', 'Solana volume')"
            },
            "limit": {
                "type": "integer",
                "description": "Maximum number of results to return (default: 10)",
                "default": 10,
                "minimum": 1,
                "maximum": 50
            }
        },
        "required": ["query"]
    }
}
