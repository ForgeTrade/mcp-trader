"""
Simple caching layer for frequently requested market data.
Uses time-based TTL (5 seconds) to reduce load on Binance provider.
"""
import time
import logging
from typing import Dict, Any, Optional, Tuple
from dataclasses import dataclass

logger = logging.getLogger(__name__)


@dataclass
class CacheEntry:
    """Cache entry with TTL."""
    data: Any
    timestamp: float


class SimpleCache:
    """Simple in-memory cache with per-tool TTL support."""

    def __init__(self, ttl_seconds: float = 5.0, tool_ttls: Optional[Dict[str, float]] = None):
        """
        Initialize cache.

        Args:
            ttl_seconds: Default time-to-live for cache entries in seconds
            tool_ttls: Optional mapping of tool name patterns to specific TTLs (FR-049)
                      e.g., {"ticker": 1.0, "orderbook": 0.5, "klines": 5.0}
        """
        self.default_ttl = ttl_seconds
        self.tool_ttls = tool_ttls or {}
        self._cache: Dict[str, CacheEntry] = {}

    def _get_ttl_for_key(self, key: str) -> float:
        """
        Determine the appropriate TTL for a cache key based on tool type.

        Args:
            key: Cache key (typically contains tool name)

        Returns:
            TTL in seconds for this key
        """
        # Check if any tool pattern matches the key
        for tool_pattern, ttl in self.tool_ttls.items():
            if tool_pattern in key:
                return ttl
        return self.default_ttl

    def get(self, key: str) -> Optional[Any]:
        """
        Get value from cache if not expired.

        Args:
            key: Cache key

        Returns:
            Cached value or None if expired/missing
        """
        entry = self._cache.get(key)
        if entry is None:
            return None

        # FR-049: Use per-tool TTL based on key pattern
        ttl = self._get_ttl_for_key(key)

        # Check if expired
        age = time.time() - entry.timestamp
        if age > ttl:
            # Remove expired entry
            del self._cache[key]
            logger.debug(f"Cache miss (expired): {key} (ttl={ttl}s, age={age:.2f}s)")
            return None

        logger.debug(f"Cache hit: {key} (age: {age:.2f}s, ttl={ttl}s)")
        return entry.data

    def set(self, key: str, value: Any):
        """
        Set value in cache.

        Args:
            key: Cache key
            value: Value to cache
        """
        self._cache[key] = CacheEntry(data=value, timestamp=time.time())
        logger.debug(f"Cache set: {key}")

    def invalidate(self, key: str):
        """
        Invalidate a specific cache entry.

        Args:
            key: Cache key to invalidate
        """
        if key in self._cache:
            del self._cache[key]
            logger.debug(f"Cache invalidated: {key}")

    def clear(self):
        """Clear all cache entries."""
        count = len(self._cache)
        self._cache.clear()
        logger.info(f"Cache cleared: {count} entries removed")

    def cleanup_expired(self):
        """Remove all expired entries using per-key TTLs."""
        current_time = time.time()
        expired_keys = []

        for key, entry in self._cache.items():
            ttl = self._get_ttl_for_key(key)
            if (current_time - entry.timestamp) > ttl:
                expired_keys.append(key)

        for key in expired_keys:
            del self._cache[key]

        if expired_keys:
            logger.debug(f"Cleaned up {len(expired_keys)} expired cache entries")

    def get_stats(self) -> Dict[str, Any]:
        """Get cache statistics with per-tool TTL awareness."""
        current_time = time.time()
        valid_count = 0

        for key, entry in self._cache.items():
            ttl = self._get_ttl_for_key(key)
            if (current_time - entry.timestamp) <= ttl:
                valid_count += 1

        return {
            "total_entries": len(self._cache),
            "valid_entries": valid_count,
            "expired_entries": len(self._cache) - valid_count,
            "default_ttl_seconds": self.default_ttl,
            "tool_ttls": self.tool_ttls,
        }


# Global cache instance for market data with per-tool TTLs (FR-049)
# Different data types have different freshness requirements:
# - ticker: 1s (highly volatile price data)
# - orderbook: 0.5s (most volatile, critical for trading)
# - klines: 5s (historical data, less volatile)
# - instrument_metadata: 5min (rarely changes)
market_data_cache = SimpleCache(
    ttl_seconds=5.0,  # Default TTL for uncategorized data
    tool_ttls={
        "ticker": 1.0,
        "orderbook": 0.5,
        "orderbook_l1": 0.5,
        "orderbook_l2": 0.5,
        "klines": 5.0,
        "instrument": 300.0,  # 5 minutes
        "exchange_info": 300.0,  # 5 minutes
    }
)
