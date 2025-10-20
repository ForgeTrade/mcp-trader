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
                # Existing normalizers (Feature 012)
                "ticker": self._normalize_binance_ticker,
                "orderbook_l1": self._normalize_binance_orderbook_l1,
                "orderbook_l2": self._normalize_binance_orderbook_l2,

                # NEW: Trading normalizers (Feature 013 - FR-001 to FR-007)
                "order": self._normalize_binance_order,
                "account": self._normalize_binance_account,
                "trade": self._normalize_binance_trade,

                # NEW: Market info normalizers (Feature 013 - FR-014 to FR-016)
                "exchange_info": self._normalize_binance_exchange_info,
                "recent_trades": self._normalize_binance_recent_trades,

                # NEW: Analytics normalizers (Feature 013 - FR-008 to FR-013)
                "orderbook_health": self._normalize_binance_orderbook_health,
                "volume_profile": self._normalize_binance_volume_profile,
                "market_anomalies": self._normalize_binance_market_anomalies,
                "microstructure_health": self._normalize_binance_microstructure_health,
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

    # ==================== NEW: Trading Normalizers (Feature 013) ====================

    def _normalize_binance_order(self, raw: Dict[str, Any]) -> Dict[str, Any]:
        """
        Normalize Binance order response to unified schema.
        Implements FR-001 to FR-003 (trading tools).

        Input format (Binance order response):
        {
            "symbol": "BTCUSDT",
            "orderId": 12345,
            "clientOrderId": "abc123",
            "side": "BUY",
            "type": "LIMIT",
            "status": "NEW",
            "origQty": "0.01",
            "executedQty": "0.0",
            "price": "43000.00",
            "avgPrice": "0.0",
            "timeInForce": "GTC",
            "transactTime": 1697048400000
        }

        Output format (unified):
        {
            "order_id": "12345",
            "client_order_id": "abc123",
            "instrument": "BTCUSDT",
            "side": "BUY",
            "type": "LIMIT",
            "status": "NEW",
            "quantity": 0.01,
            "filled_quantity": 0.0,
            "remaining_quantity": 0.01,
            "price": 43000.00,
            "average_price": 0.0,
            "timestamp": 1697048400000,
            ...
        }
        """
        orig_qty = float(raw.get("origQty", 0))
        executed_qty = float(raw.get("executedQty", 0))

        normalized = {
            "order_id": str(raw["orderId"]),
            "instrument": raw["symbol"],
            "side": raw["side"],
            "type": raw["type"],
            "status": raw["status"],
            "quantity": orig_qty,
            "filled_quantity": executed_qty,
            "remaining_quantity": orig_qty - executed_qty,
            "timestamp": raw.get("transactTime", int(time.time() * 1000)),
        }

        # Optional fields
        if "clientOrderId" in raw:
            normalized["client_order_id"] = raw["clientOrderId"]
        if "price" in raw:
            normalized["price"] = float(raw["price"])
        if "avgPrice" in raw and raw["avgPrice"] != "0.0":
            normalized["average_price"] = float(raw["avgPrice"])
        if "timeInForce" in raw:
            normalized["time_in_force"] = raw["timeInForce"]

        return normalized

    def _normalize_binance_account(self, raw: Dict[str, Any]) -> Dict[str, Any]:
        """
        Normalize Binance account response to unified schema.
        Implements FR-006 (get_account).

        Input format (Binance account):
        {
            "makerCommission": 10,
            "takerCommission": 10,
            "canTrade": true,
            "canWithdraw": true,
            "canDeposit": true,
            "updateTime": 1697048400000,
            "balances": [
                {"asset": "BTC", "free": "1.234", "locked": "0.01"},
                {"asset": "USDT", "free": "10000.00", "locked": "500.00"}
            ]
        }

        Output format (unified):
        {
            "can_trade": true,
            "can_withdraw": true,
            "balances": [
                {"asset": "BTC", "free": 1.234, "locked": 0.01, "total": 1.244},
                {"asset": "USDT", "free": 10000.00, "locked": 500.00, "total": 10500.00}
            ],
            "timestamp": 1697048400000,
            ...
        }
        """
        # Normalize balances
        balances = []
        for balance in raw.get("balances", []):
            free = float(balance["free"])
            locked = float(balance["locked"])
            total = free + locked

            # Only include non-zero balances
            if total > 0:
                balances.append({
                    "asset": balance["asset"],
                    "free": free,
                    "locked": locked,
                    "total": total
                })

        normalized = {
            "can_trade": raw.get("canTrade", False),
            "can_withdraw": raw.get("canWithdraw", False),
            "can_deposit": raw.get("canDeposit", False),
            "balances": balances,
            "timestamp": raw.get("updateTime", int(time.time() * 1000)),
        }

        return normalized

    def _normalize_binance_trade(self, raw: Dict[str, Any]) -> Dict[str, Any]:
        """
        Normalize Binance trade response to unified schema.
        Implements FR-007 (get_my_trades).

        Input format (Binance trade):
        {
            "id": 123456,
            "symbol": "BTCUSDT",
            "orderId": 12345,
            "price": "43250.50",
            "qty": "0.01",
            "quoteQty": "432.505",
            "commission": "0.432505",
            "commissionAsset": "USDT",
            "time": 1697048400000,
            "isBuyer": true
        }

        Output format (unified):
        {
            "trade_id": "123456",
            "order_id": "12345",
            "instrument": "BTCUSDT",
            "side": "BUY",
            "price": 43250.50,
            "quantity": 0.01,
            "quote_quantity": 432.505,
            "fee": 0.432505,
            "fee_asset": "USDT",
            "timestamp": 1697048400000,
            ...
        }
        """
        normalized = {
            "trade_id": str(raw["id"]),
            "order_id": str(raw["orderId"]),
            "instrument": raw["symbol"],
            "side": "BUY" if raw.get("isBuyer", False) else "SELL",
            "price": float(raw["price"]),
            "quantity": float(raw["qty"]),
            "quote_quantity": float(raw["quoteQty"]),
            "fee": float(raw.get("commission", 0)),
            "fee_asset": raw.get("commissionAsset", ""),
            "timestamp": raw["time"],
        }

        return normalized

    # ==================== NEW: Market Info Normalizers (Feature 013) ====================

    def _normalize_binance_exchange_info(self, raw: Dict[str, Any]) -> Dict[str, Any]:
        """
        Normalize Binance exchange info response to unified schema.
        Implements FR-015 (get_exchange_info).

        Input format (Binance exchange info for a symbol):
        {
            "symbol": "BTCUSDT",
            "status": "TRADING",
            "baseAsset": "BTC",
            "quoteAsset": "USDT",
            "filters": [
                {"filterType": "PRICE_FILTER", "minPrice": "0.01", "maxPrice": "1000000.00", "tickSize": "0.01"},
                {"filterType": "LOT_SIZE", "minQty": "0.00001", "maxQty": "9000.00", "stepSize": "0.00001"}
            ]
        }

        Output format (unified):
        {
            "instrument": "BTCUSDT",
            "status": "TRADING",
            "base_asset": "BTC",
            "quote_asset": "USDT",
            "min_price": 0.01,
            "max_price": 1000000.00,
            "price_tick_size": 0.01,
            "min_quantity": 0.00001,
            "max_quantity": 9000.00,
            "quantity_step_size": 0.00001,
            ...
        }
        """
        normalized = {
            "instrument": raw["symbol"],
            "status": raw["status"],
            "base_asset": raw["baseAsset"],
            "quote_asset": raw["quoteAsset"],
        }

        # Extract filters
        for filter_item in raw.get("filters", []):
            filter_type = filter_item.get("filterType")
            if filter_type == "PRICE_FILTER":
                normalized["min_price"] = float(filter_item.get("minPrice", 0))
                normalized["max_price"] = float(filter_item.get("maxPrice", 0))
                normalized["price_tick_size"] = float(filter_item.get("tickSize", 0))
            elif filter_type == "LOT_SIZE":
                normalized["min_quantity"] = float(filter_item.get("minQty", 0))
                normalized["max_quantity"] = float(filter_item.get("maxQty", 0))
                normalized["quantity_step_size"] = float(filter_item.get("stepSize", 0))

        return normalized

    def _normalize_binance_recent_trades(self, raw: Dict[str, Any]) -> Dict[str, Any]:
        """
        Normalize Binance recent trades response to unified schema.
        Implements FR-014 (get_recent_trades).

        Input format (list of trades):
        [
            {
                "id": 123456,
                "price": "43250.50",
                "qty": "0.01",
                "quoteQty": "432.505",
                "time": 1697048400000,
                "isBuyerMaker": false
            },
            ...
        ]

        Output format (unified):
        {
            "trades": [
                {
                    "trade_id": "123456",
                    "price": 43250.50,
                    "quantity": 0.01,
                    "quote_quantity": 432.505,
                    "side": "BUY",
                    "timestamp": 1697048400000
                },
                ...
            ]
        }
        """
        # Handle both single trade and list of trades
        trades_list = raw if isinstance(raw, list) else [raw]

        trades = []
        for trade in trades_list:
            trades.append({
                "trade_id": str(trade["id"]),
                "price": float(trade["price"]),
                "quantity": float(trade["qty"]),
                "quote_quantity": float(trade.get("quoteQty", 0)),
                "side": "SELL" if trade.get("isBuyerMaker", False) else "BUY",
                "timestamp": trade["time"],
            })

        return {"trades": trades}

    # ==================== NEW: Analytics Normalizers (Feature 013) ====================

    def _normalize_binance_orderbook_health(self, raw: Dict[str, Any]) -> Dict[str, Any]:
        """
        Normalize Binance orderbook health response to unified schema.
        Implements FR-008 (get_orderbook_health).

        Input format (Binance orderbook health):
        {
            "spread_quality": 85.5,
            "depth_imbalance": 0.45,
            "health_score": 78.2,
            "bid_depth": 125.5,
            "ask_depth": 98.3
        }

        Output format (unified - already in good shape):
        Same as input but ensure all fields are present
        """
        normalized = {
            "spread_quality": float(raw.get("spread_quality", 0)),
            "depth_imbalance": float(raw.get("depth_imbalance", 0.5)),
            "health_score": float(raw.get("health_score", 0)),
            "bid_depth": float(raw.get("bid_depth", 0)),
            "ask_depth": float(raw.get("ask_depth", 0)),
        }

        return normalized

    def _normalize_binance_volume_profile(self, raw: Dict[str, Any]) -> Dict[str, Any]:
        """
        Normalize Binance volume profile response to unified schema.
        Implements FR-010 (get_volume_profile).

        Input format (Binance volume profile):
        {
            "poc": 43250.00,
            "value_area_high": 43500.00,
            "value_area_low": 43000.00,
            "distribution": [
                {"price": 43000.00, "volume": 125.5},
                {"price": 43100.00, "volume": 150.3},
                ...
            ]
        }

        Output format (unified - already in good shape):
        Same as input
        """
        return raw

    def _normalize_binance_market_anomalies(self, raw: Dict[str, Any]) -> Dict[str, Any]:
        """
        Normalize Binance market anomalies response to unified schema.
        Implements FR-011 (detect_market_anomalies).

        Input format (Binance anomaly detection):
        {
            "quote_stuffing_detected": true,
            "iceberg_orders_detected": false,
            "flash_crash_risk": "LOW",
            "anomalies": [
                {"type": "quote_stuffing", "severity": "HIGH", "timestamp": 1697048400000},
                ...
            ]
        }

        Output format (unified - already in good shape):
        Same as input
        """
        return raw

    def _normalize_binance_microstructure_health(self, raw: Dict[str, Any]) -> Dict[str, Any]:
        """
        Normalize Binance microstructure health response to unified schema.
        Implements FR-012 (get_microstructure_health).

        Input format (Binance microstructure health):
        {
            "overall_health_score": 82.5,
            "spread_health": 85.0,
            "depth_health": 80.0,
            "toxicity_score": 15.5,
            "components": {
                "bid_ask_spread": 0.02,
                "effective_spread": 0.025,
                "adverse_selection": 0.015
            }
        }

        Output format (unified - already in good shape):
        Same as input
        """
        return raw
