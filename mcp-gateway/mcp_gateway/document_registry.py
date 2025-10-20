"""
Document ID registry for ChatGPT MCP integration.
Maps document IDs to Binance tool calls.
"""
import re
from dataclasses import dataclass
from typing import Dict, Any, Optional, List
import logging

logger = logging.getLogger(__name__)


@dataclass
class DocumentID:
    """Represents a parsed document ID."""
    doc_type: str  # ticker, orderbook, orderbook_l1, orderbook_l2, klines, analytics, etc.
    symbol: str  # Trading pair (e.g., BTCUSDT)
    interval: Optional[str] = None  # For klines (e.g., 1h, 4h, 1d)
    analytics_type: Optional[str] = None  # For analytics (e.g., order_flow, volume_profile)

    def to_id(self) -> str:
        """Convert to document ID string."""
        if self.doc_type == "klines" and self.interval:
            return f"klines:{self.symbol}:{self.interval}"
        elif self.doc_type.startswith("analytics") and self.analytics_type:
            return f"analytics:{self.analytics_type}:{self.symbol}"
        else:
            return f"{self.doc_type}:{self.symbol}"

    @classmethod
    def from_id(cls, doc_id: str) -> Optional["DocumentID"]:
        """Parse document ID string."""
        parts = doc_id.split(":")
        if len(parts) < 2:
            return None

        doc_type = parts[0]
        symbol = parts[1]

        if doc_type == "klines" and len(parts) == 3:
            return cls(doc_type=doc_type, symbol=symbol, interval=parts[2])
        elif doc_type == "analytics" and len(parts) == 3:
            return cls(doc_type=doc_type, symbol=symbol, analytics_type=parts[1], interval=None)
        else:
            return cls(doc_type=doc_type, symbol=symbol)


class DocumentRegistry:
    """
    Registry mapping document IDs to Binance gRPC tool calls.
    Supports all 21 Binance tools.
    """

    # Mapping of document types to Binance tool names
    # FR-046: Use dot notation (binance.get_*) to match capability tool names
    DOC_TYPE_TO_TOOL = {
        "ticker": "binance.get_ticker",
        "orderbook": "binance.orderbook_l2",  # Full orderbook
        "orderbook_l1": "binance.orderbook_l1",
        "orderbook_l2": "binance.orderbook_l2",
        "klines": "binance.get_klines",
        "trades": "binance.get_recent_trades",
        "volume_profile": "binance.get_volume_profile",
        "orderbook_health": "binance.orderbook_health",
        "liquidity_vacuums": "binance.detect_liquidity_vacuums",
        "market_anomalies": "binance.detect_market_anomalies",
        "microstructure_health": "binance.get_microstructure_health",
    }

    # Analytics type mapping
    ANALYTICS_TO_TOOL = {
        "order_flow": "binance.get_volume_profile",  # Uses volume profile
        "volume_profile": "binance.get_volume_profile",
        "orderbook_health": "binance.orderbook_health",
        "liquidity_vacuums": "binance.detect_liquidity_vacuums",
        "market_anomalies": "binance.detect_market_anomalies",
        "microstructure": "binance.get_microstructure_health",
    }

    # Common trading symbols (for search suggestions)
    POPULAR_SYMBOLS = [
        "BTCUSDT", "ETHUSDT", "BNBUSDT", "SOLUSDT", "XRPUSDT",
        "ADAUSDT", "DOGEUSDT", "MATICUSDT", "DOTUSDT", "AVAXUSDT",
        "LINKUSDT", "UNIUSDT", "ATOMUSDT", "LTCUSDT", "ETCUSDT",
    ]

    # Coin name to symbol mapping
    COIN_NAME_TO_SYMBOL = {
        "bitcoin": "BTCUSDT",
        "btc": "BTCUSDT",
        "ethereum": "ETHUSDT",
        "eth": "ETHUSDT",
        "binance coin": "BNBUSDT",
        "bnb": "BNBUSDT",
        "solana": "SOLUSDT",
        "sol": "SOLUSDT",
        "ripple": "XRPUSDT",
        "xrp": "XRPUSDT",
        "cardano": "ADAUSDT",
        "ada": "ADAUSDT",
        "dogecoin": "DOGEUSDT",
        "doge": "DOGEUSDT",
        "polygon": "MATICUSDT",
        "matic": "MATICUSDT",
        "polkadot": "DOTUSDT",
        "dot": "DOTUSDT",
        "avalanche": "AVAXUSDT",
        "avax": "AVAXUSDT",
        "chainlink": "LINKUSDT",
        "link": "LINKUSDT",
        "uniswap": "UNIUSDT",
        "uni": "UNIUSDT",
        "cosmos": "ATOMUSDT",
        "atom": "ATOMUSDT",
        "litecoin": "LTCUSDT",
        "ltc": "LTCUSDT",
        "ethereum classic": "ETCUSDT",
        "etc": "ETCUSDT",
    }

    @classmethod
    def parse_document_id(cls, doc_id: str) -> Optional[DocumentID]:
        """Parse a document ID string."""
        return DocumentID.from_id(doc_id)

    @classmethod
    def create_document_id(cls, doc_type: str, symbol: str, **kwargs) -> str:
        """Create a document ID string."""
        doc = DocumentID(doc_type=doc_type, symbol=symbol, **kwargs)
        return doc.to_id()

    @classmethod
    def get_tool_for_document(cls, doc_id: DocumentID) -> Optional[str]:
        """Get the Binance tool name for a document ID."""
        if doc_id.doc_type.startswith("analytics") and doc_id.analytics_type:
            return cls.ANALYTICS_TO_TOOL.get(doc_id.analytics_type)
        return cls.DOC_TYPE_TO_TOOL.get(doc_id.doc_type)

    @classmethod
    def create_tool_arguments(cls, doc_id: DocumentID) -> Dict[str, Any]:
        """Create tool arguments from a document ID."""
        args: Dict[str, Any] = {"symbol": doc_id.symbol}

        if doc_id.doc_type == "klines" and doc_id.interval:
            args["interval"] = doc_id.interval
            args["limit"] = 100  # Default limit

        elif doc_id.doc_type in ("orderbook", "orderbook_l2"):
            args["limit"] = 20  # Top 20 bids/asks

        return args

    @classmethod
    def extract_symbols_from_query(cls, query: str) -> List[str]:
        """
        Extract trading symbols from a natural language query.

        Args:
            query: Natural language query (e.g., "Bitcoin price", "ETHUSDT orderbook")

        Returns:
            List of matched symbols
        """
        query_lower = query.lower()
        symbols = []

        # Check for exact symbol matches (case-insensitive)
        for symbol in cls.POPULAR_SYMBOLS:
            if symbol.lower() in query_lower:
                symbols.append(symbol)

        # Check for coin name matches
        for coin_name, symbol in cls.COIN_NAME_TO_SYMBOL.items():
            if coin_name in query_lower and symbol not in symbols:
                symbols.append(symbol)

        # If no symbols found, default to BTC
        if not symbols:
            # Check if query mentions "price", "orderbook", or market data terms
            if any(term in query_lower for term in ["price", "market", "trading", "orderbook", "volume"]):
                symbols.append("BTCUSDT")

        return symbols

    @classmethod
    def detect_data_type(cls, query: str) -> str:
        """
        Detect the type of data being requested from the query.

        Returns:
            Document type (ticker, orderbook, klines, analytics, etc.)
        """
        query_lower = query.lower()

        # Data type keywords
        if any(word in query_lower for word in ["orderbook", "order book", "bids", "asks", "spread"]):
            return "orderbook_l1"
        elif any(word in query_lower for word in ["kline", "candlestick", "candle", "ohlc", "chart"]):
            return "klines"
        elif any(word in query_lower for word in ["trade", "recent trade", "last trade"]):
            return "trades"
        elif any(word in query_lower for word in ["volume profile", "poc", "value area"]):
            return "volume_profile"
        elif any(word in query_lower for word in ["liquidity", "vacuum"]):
            return "liquidity_vacuums"
        elif any(word in query_lower for word in ["anomaly", "anomalies", "unusual"]):
            return "market_anomalies"
        elif any(word in query_lower for word in ["microstructure", "market health"]):
            return "microstructure_health"
        elif any(word in query_lower for word in ["health", "metrics"]):
            return "orderbook_health"
        else:
            # Default to ticker (price data)
            return "ticker"

    @classmethod
    def validate_document_type(cls, doc_type: str) -> bool:
        """Validate if a document type is supported."""
        return doc_type in cls.DOC_TYPE_TO_TOOL or doc_type.startswith("analytics")
