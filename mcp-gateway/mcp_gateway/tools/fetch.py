"""
Fetch tool for ChatGPT MCP integration.
Retrieves complete market data for a specific document ID.
"""
import json
import logging
import uuid
from typing import Dict, Any, Optional

from mcp_gateway.document_registry import DocumentRegistry, DocumentID
from mcp_gateway.adapters.grpc_client import ProviderGRPCClient
from mcp_gateway.cache import market_data_cache

logger = logging.getLogger(__name__)


class FetchTool:
    """
    Implements the `fetch` tool for ChatGPT.
    Retrieves complete data for a specific document ID.
    """

    def __init__(self, grpc_client: ProviderGRPCClient, base_url: str = "https://mcp-gateway.thevibe.trading"):
        """
        Initialize fetch tool.

        Args:
            grpc_client: gRPC client for Binance provider
            base_url: Base URL for citation links
        """
        self.client = grpc_client
        self.base_url = base_url
        self.registry = DocumentRegistry()

    async def fetch(self, document_id: str) -> Dict[str, Any]:
        """
        Fetch complete data for a document ID.

        Args:
            document_id: Document ID (e.g., "ticker:BTCUSDT", "orderbook:ETHUSDT")

        Returns:
            Document in ChatGPT-compatible format:
            {
                "id": "ticker:BTCUSDT",
                "title": "Bitcoin (BTC) Price - BTCUSDT",
                "text": "{...full JSON data...}",
                "url": "https://mcp-gateway.thevibe.trading/data/ticker:BTCUSDT",
                "metadata": {"symbol": "BTCUSDT", "type": "ticker"}
            }
        """
        logger.info(f"Fetch document: {document_id}")

        # Parse document ID
        doc = self.registry.parse_document_id(document_id)
        if not doc:
            error_msg = f"Invalid document ID format: {document_id}"
            logger.error(error_msg)
            return {
                "id": document_id,
                "title": "Error",
                "text": error_msg,
                "url": f"{self.base_url}/data/{document_id}",
                "metadata": {"error": "invalid_document_id"}
            }

        # Get tool name for this document type
        tool_name = self.registry.get_tool_for_document(doc)
        if not tool_name:
            error_msg = f"No tool found for document type: {doc.doc_type}"
            logger.error(error_msg)
            return {
                "id": document_id,
                "title": "Error",
                "text": error_msg,
                "url": f"{self.base_url}/data/{document_id}",
                "metadata": {"error": "unsupported_document_type"}
            }

        try:
            # Try to get cached data
            cache_key = f"{tool_name}:{doc.symbol}"
            cached_data = market_data_cache.get(cache_key)

            if cached_data:
                data = cached_data
                logger.debug(f"Using cached data for {cache_key}")
            else:
                # Fetch data from Binance provider
                tool_args = self.registry.create_tool_arguments(doc)
                correlation_id = str(uuid.uuid4())

                response = await self.client.invoke(
                    tool_name=tool_name,
                    payload=tool_args,
                    correlation_id=correlation_id,
                    timeout=3.0  # Slightly longer timeout for fetch
                )

                if "error" in response:
                    error_msg = response["error"]
                    logger.error(f"Error fetching {tool_name} for {doc.symbol}: {error_msg}")
                    return {
                        "id": document_id,
                        "title": "Fetch Error",
                        "text": error_msg,
                        "url": f"{self.base_url}/data/{document_id}",
                        "metadata": {"error": "tool_invocation_failed"}
                    }

                data = response.get("result", {})
                # Cache the result
                market_data_cache.set(cache_key, data)

            # Create document response
            return self._create_document(document_id, doc, data)

        except Exception as e:
            error_msg = f"Failed to fetch document: {e}"
            logger.error(error_msg, exc_info=True)
            return {
                "id": document_id,
                "title": "Fetch Error",
                "text": error_msg,
                "url": f"{self.base_url}/data/{document_id}",
                "metadata": {"error": "internal_error"}
            }

    def _create_document(self, doc_id: str, doc: DocumentID, data: Dict[str, Any]) -> Dict[str, Any]:
        """
        Create a document response.

        Args:
            doc_id: Document ID string
            doc: Parsed DocumentID object
            data: Data from Binance

        Returns:
            Document dictionary
        """
        # Generate title
        coin_name = self._get_coin_name(doc.symbol)
        title = self._generate_title(coin_name, doc.symbol, doc.doc_type)

        # Format full data as JSON text
        text = self._format_data(data, doc.doc_type)

        # Create citation URL
        url = f"{self.base_url}/data/{doc_id}"

        # Create metadata
        metadata = {
            "symbol": doc.symbol,
            "type": doc.doc_type,
        }
        if doc.interval:
            metadata["interval"] = doc.interval
        if doc.analytics_type:
            metadata["analytics_type"] = doc.analytics_type

        return {
            "id": doc_id,
            "title": title,
            "text": text,
            "url": url,
            "metadata": metadata,
        }

    def _get_coin_name(self, symbol: str) -> str:
        """Get human-readable coin name from symbol."""
        # Reverse lookup in COIN_NAME_TO_SYMBOL
        for name, sym in self.registry.COIN_NAME_TO_SYMBOL.items():
            if sym == symbol:
                return name.title()

        # Extract base currency from symbol
        if symbol.endswith("USDT"):
            return symbol[:-4]
        elif symbol.endswith("BUSD"):
            return symbol[:-4]
        elif symbol.endswith("BTC"):
            return symbol[:-3]
        return symbol

    def _generate_title(self, coin_name: str, symbol: str, data_type: str) -> str:
        """Generate title for document."""
        type_labels = {
            "ticker": "Price Data",
            "orderbook": "Full Order Book",
            "orderbook_l1": "Order Book (Level 1)",
            "orderbook_l2": "Order Book (Level 2)",
            "klines": "Candlestick Data",
            "trades": "Recent Trades",
            "volume_profile": "Volume Profile Analysis",
            "orderbook_health": "Order Book Health Metrics",
            "liquidity_vacuums": "Liquidity Vacuum Analysis",
            "market_anomalies": "Market Anomaly Detection",
            "microstructure_health": "Market Microstructure Health",
        }

        type_label = type_labels.get(data_type, data_type.replace("_", " ").title())
        return f"{coin_name} - {type_label} ({symbol})"

    def _format_data(self, data: Dict[str, Any], data_type: str) -> str:
        """
        Format data as text for ChatGPT consumption.

        Args:
            data: Raw data from Binance
            data_type: Type of data

        Returns:
            Formatted text (JSON or human-readable)
        """
        try:
            if data_type == "ticker":
                return self._format_ticker(data)
            elif data_type in ("orderbook", "orderbook_l1", "orderbook_l2"):
                return self._format_orderbook(data)
            elif data_type == "volume_profile":
                return self._format_volume_profile(data)
            elif data_type == "orderbook_health":
                return self._format_orderbook_health(data)
            else:
                # Default: Pretty-print JSON
                return json.dumps(data, indent=2)

        except Exception as e:
            logger.error(f"Error formatting data: {e}")
            return json.dumps(data, indent=2)

    def _format_ticker(self, data: Dict[str, Any]) -> str:
        """Format ticker data as human-readable text."""
        lines = [
            "=== TICKER DATA ===",
            f"Symbol: {data.get('symbol', 'N/A')}",
            f"Last Price: ${data.get('lastPrice', 'N/A')}",
            f"24h High: ${data.get('highPrice', 'N/A')}",
            f"24h Low: ${data.get('lowPrice', 'N/A')}",
            f"24h Volume: {data.get('volume', 'N/A')}",
            f"24h Quote Volume: ${data.get('quoteVolume', 'N/A')}",
            f"Price Change: ${data.get('priceChange', 'N/A')} ({data.get('priceChangePercent', 'N/A')}%)",
            f"Weighted Avg Price: ${data.get('weightedAvgPrice', 'N/A')}",
            "",
            "Raw JSON:",
            json.dumps(data, indent=2)
        ]
        return "\n".join(lines)

    def _format_orderbook(self, data: Dict[str, Any]) -> str:
        """Format orderbook data as human-readable text."""
        bids = data.get("bids", [])
        asks = data.get("asks", [])

        lines = [
            "=== ORDER BOOK ===",
            f"Bids: {len(bids)} levels",
            f"Asks: {len(asks)} levels",
            ""
        ]

        if bids:
            lines.append("Top 5 Bids:")
            for i, (price, qty) in enumerate(bids[:5]):
                lines.append(f"  {i+1}. ${price} @ {qty}")
            lines.append("")

        if asks:
            lines.append("Top 5 Asks:")
            for i, (price, qty) in enumerate(asks[:5]):
                lines.append(f"  {i+1}. ${price} @ {qty}")
            lines.append("")

        if bids and asks:
            spread = float(asks[0][0]) - float(bids[0][0])
            lines.append(f"Spread: ${spread:.2f}")
            lines.append("")

        lines.extend([
            "Raw JSON:",
            json.dumps(data, indent=2)
        ])
        return "\n".join(lines)

    def _format_volume_profile(self, data: Dict[str, Any]) -> str:
        """Format volume profile data as human-readable text."""
        lines = [
            "=== VOLUME PROFILE ===",
            f"POC (Point of Control): ${data.get('poc_price', 'N/A')}",
            f"POC Volume: {data.get('poc_volume', 'N/A')}",
            f"Value Area High: ${data.get('value_area_high', 'N/A')}",
            f"Value Area Low: ${data.get('value_area_low', 'N/A')}",
            f"Total Volume: {data.get('total_volume', 'N/A')}",
            "",
            "Raw JSON:",
            json.dumps(data, indent=2)
        ]
        return "\n".join(lines)

    def _format_orderbook_health(self, data: Dict[str, Any]) -> str:
        """Format orderbook health data as human-readable text."""
        lines = [
            "=== ORDERBOOK HEALTH ===",
            f"Spread (bps): {data.get('spread_bps', 'N/A')}",
            f"Imbalance Ratio: {data.get('imbalance_ratio', 'N/A')}",
            f"Bid Depth: {data.get('bid_depth', 'N/A')}",
            f"Ask Depth: {data.get('ask_depth', 'N/A')}",
            f"Total Depth: {data.get('total_depth', 'N/A')}",
            "",
            "Raw JSON:",
            json.dumps(data, indent=2)
        ]
        return "\n".join(lines)


# Tool schema for ChatGPT
FETCH_TOOL_SCHEMA = {
    "name": "fetch",
    "description": "Fetch complete market data for a specific document ID. Use document IDs from search results.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "document_id": {
                "type": "string",
                "description": "Document ID from search results (e.g., 'ticker:BTCUSDT', 'orderbook:ETHUSDT')"
            }
        },
        "required": ["document_id"]
    }
}
