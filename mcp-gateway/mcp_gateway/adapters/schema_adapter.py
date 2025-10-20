"""
Schema adapter for normalizing provider-specific responses to unified schemas.
Implements FR-007 (schema normalization), FR-008 (ticker normalization), FR-009 (orderbook normalization).
"""
import logging
from typing import Dict, Any, Callable
import time

logger = logging.getLogger(__name__)


class SchemaAdapter:
    """
    Adapts provider-specific response schemas to unified schemas.
    Each provider has specific normalization functions for different data types.
    """

    def __init__(self):
        """Initialize schema adapter with provider-specific normalizers."""
        self._normalizers: Dict[str, Dict[str, Callable]] = {
            "binance": {
                "ticker": self._normalize_binance_ticker,
                "orderbook_l1": self._normalize_binance_orderbook_l1,
                "orderbook_l2": self._normalize_binance_orderbook_l2,
            },
            # Future providers can be added here
            # "okx": {
            #     "ticker": self._normalize_okx_ticker,
            #     ...
            # },
        }
        logger.info(f"SchemaAdapter initialized with {len(self._normalizers)} provider normalizers")

    def normalize(
        self,
        venue: str,
        data_type: str,
        raw_response: Dict[str, Any],
        additional_fields: Dict[str, Any] | None = None
    ) -> Dict[str, Any]:
        """
        Normalize a provider response to unified schema.

        Args:
            venue: Provider/venue name (e.g., "binance", "okx")
            data_type: Type of data to normalize (e.g., "ticker", "orderbook_l1")
            raw_response: Raw response from provider
            additional_fields: Additional fields to include (e.g., latency_ms, timestamp)

        Returns:
            Normalized response conforming to unified schema

        Raises:
            ValueError: If venue or data_type not supported
        """
        if venue not in self._normalizers:
            raise ValueError(
                f"No normalizer available for venue '{venue}'. "
                f"Supported venues: {list(self._normalizers.keys())}"
            )

        if data_type not in self._normalizers[venue]:
            raise ValueError(
                f"No normalizer available for {venue}.{data_type}. "
                f"Supported types for {venue}: {list(self._normalizers[venue].keys())}"
            )

        normalizer = self._normalizers[venue][data_type]

        try:
            normalized = normalizer(raw_response)

            # Add additional fields if provided
            if additional_fields:
                normalized.update(additional_fields)

            # Ensure venue is set
            if "venue" not in normalized:
                normalized["venue"] = venue

            logger.debug(f"Successfully normalized {venue}.{data_type}")
            return normalized

        except Exception as e:
            logger.error(f"Failed to normalize {venue}.{data_type}: {e}", exc_info=True)
            raise ValueError(f"Normalization failed for {venue}.{data_type}: {e}") from e

    # ==================== Binance Normalizers ====================

    def _normalize_binance_ticker(self, raw: Dict[str, Any]) -> Dict[str, Any]:
        """
        Normalize Binance ticker response to unified schema.
        Implements FR-008 (ticker normalization).

        Input format (Binance 24hr ticker):
        {
            "symbol": "BTCUSDT",
            "bidPrice": "43250.50",
            "askPrice": "43251.00",
            "lastPrice": "43250.75",
            "volume": "12345.67",
            "quoteVolume": "534567890.12",
            "priceChangePercent": "2.45",
            "closeTime": 1697048400000
        }

        Output format (unified):
        {
            "bid": 43250.50,
            "ask": 43251.00,
            "mid": 43250.75,
            "spread_bps": 1.15,
            "volume": 12345.67,
            "timestamp": 1697048400000,
            "venue": "binance",
            ...
        }
        """
        # Parse string prices to floats (FR-008)
        bid = float(raw["bidPrice"])
        ask = float(raw["askPrice"])

        # Calculate mid-price (FR-008)
        mid = (bid + ask) / 2.0

        # Calculate spread in basis points (FR-008)
        spread_bps = ((ask - bid) / mid) * 10000.0 if mid > 0 else 0.0

        # Build normalized response
        normalized = {
            "bid": bid,
            "ask": ask,
            "mid": mid,
            "spread_bps": spread_bps,
            "volume": float(raw["volume"]),
            "timestamp": raw.get("closeTime", int(time.time() * 1000)),
            "venue_symbol": raw["symbol"],
        }

        # Optional fields
        if "lastPrice" in raw:
            normalized["last"] = float(raw["lastPrice"])

        if "quoteVolume" in raw:
            normalized["quote_volume"] = float(raw["quoteVolume"])

        if "priceChangePercent" in raw:
            normalized["price_change_percent"] = float(raw["priceChangePercent"])

        return normalized

    def _normalize_binance_orderbook_l1(self, raw: Dict[str, Any]) -> Dict[str, Any]:
        """
        Normalize Binance orderbook to unified L1 (top-of-book) schema.
        Implements FR-009 (orderbook normalization).

        Input format (Binance orderbook):
        {
            "lastUpdateId": 123456789,
            "bids": [["43250.50", "1.234"], ["43250.00", "2.456"], ...],
            "asks": [["43251.00", "0.987"], ["43251.50", "1.543"], ...]
        }

        Output format (unified L1):
        {
            "bid_price": 43250.50,
            "bid_quantity": 1.234,
            "ask_price": 43251.00,
            "ask_quantity": 0.987,
            "mid": 43250.75,
            "spread_bps": 1.15,
            "timestamp": 1697048400000,
            "venue": "binance",
            ...
        }
        """
        # Extract top of book (FR-009)
        if not raw.get("bids") or not raw.get("asks"):
            raise ValueError("Invalid orderbook: missing bids or asks")

        best_bid = raw["bids"][0]
        best_ask = raw["asks"][0]

        # Parse prices and quantities
        bid_price = float(best_bid[0])
        bid_quantity = float(best_bid[1])
        ask_price = float(best_ask[0])
        ask_quantity = float(best_ask[1])

        # Calculate mid-price (FR-009)
        mid = (bid_price + ask_price) / 2.0

        # Calculate spread in basis points (FR-009)
        spread_bps = ((ask_price - bid_price) / mid) * 10000.0 if mid > 0 else 0.0

        # Calculate absolute spread
        spread_absolute = ask_price - bid_price

        # Calculate order imbalance ratio
        total_quantity = bid_quantity + ask_quantity
        imbalance_ratio = bid_quantity / total_quantity if total_quantity > 0 else 0.5

        # Build normalized response
        normalized = {
            "bid_price": bid_price,
            "bid_quantity": bid_quantity,
            "ask_price": ask_price,
            "ask_quantity": ask_quantity,
            "mid": mid,
            "spread_bps": spread_bps,
            "spread_absolute": spread_absolute,
            "imbalance_ratio": imbalance_ratio,
            "timestamp": int(time.time() * 1000),  # Current time as Binance doesn't provide orderbook timestamp
        }

        # Optional fields
        if "lastUpdateId" in raw:
            normalized["update_id"] = raw["lastUpdateId"]

        return normalized

    def _normalize_binance_orderbook_l2(self, raw: Dict[str, Any]) -> Dict[str, Any]:
        """
        Normalize Binance orderbook to unified L2 (full depth) schema.

        Input format: Same as L1 but includes full bid/ask arrays
        Output format: Full orderbook with all levels normalized
        """
        if not raw.get("bids") or not raw.get("asks"):
            raise ValueError("Invalid orderbook: missing bids or asks")

        # Normalize all bid levels
        bids = [
            {"price": float(level[0]), "quantity": float(level[1])}
            for level in raw["bids"]
        ]

        # Normalize all ask levels
        asks = [
            {"price": float(level[0]), "quantity": float(level[1])}
            for level in raw["asks"]
        ]

        # Calculate top-of-book metrics for convenience
        bid_price = bids[0]["price"]
        ask_price = asks[0]["price"]
        mid = (bid_price + ask_price) / 2.0
        spread_bps = ((ask_price - bid_price) / mid) * 10000.0 if mid > 0 else 0.0

        normalized = {
            "bids": bids,
            "asks": asks,
            "mid": mid,
            "spread_bps": spread_bps,
            "timestamp": int(time.time() * 1000),
        }

        if "lastUpdateId" in raw:
            normalized["update_id"] = raw["lastUpdateId"]

        return normalized

    def is_supported(self, venue: str, data_type: str) -> bool:
        """
        Check if a venue and data type combination is supported.

        Args:
            venue: Provider/venue name
            data_type: Type of data

        Returns:
            True if supported, False otherwise
        """
        return venue in self._normalizers and data_type in self._normalizers[venue]

    def get_supported_venues(self) -> list[str]:
        """Get list of all supported venues."""
        return list(self._normalizers.keys())

    def get_supported_data_types(self, venue: str) -> list[str]:
        """Get list of supported data types for a venue."""
        if venue not in self._normalizers:
            return []
        return list(self._normalizers[venue].keys())
