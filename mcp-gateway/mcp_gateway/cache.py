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
    """Simple in-memory cache with TTL."""

    def __init__(self, ttl_seconds: float = 5.0):
        """
        Initialize cache.

        Args:
            ttl_seconds: Time-to-live for cache entries in seconds
        """
        self.ttl_seconds = ttl_seconds
        self._cache: Dict[str, CacheEntry] = {}

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

        # Check if expired
        age = time.time() - entry.timestamp
        if age > self.ttl_seconds:
            # Remove expired entry
            del self._cache[key]
            logger.debug(f"Cache miss (expired): {key}")
            return None

        logger.debug(f"Cache hit: {key} (age: {age:.2f}s)")
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
        """Remove all expired entries."""
        current_time = time.time()
        expired_keys = [
            key for key, entry in self._cache.items()
            if (current_time - entry.timestamp) > self.ttl_seconds
        ]

        for key in expired_keys:
            del self._cache[key]

        if expired_keys:
            logger.debug(f"Cleaned up {len(expired_keys)} expired cache entries")

    def get_stats(self) -> Dict[str, Any]:
        """Get cache statistics."""
        current_time = time.time()
        valid_count = sum(
            1 for entry in self._cache.values()
            if (current_time - entry.timestamp) <= self.ttl_seconds
        )

        return {
            "total_entries": len(self._cache),
            "valid_entries": valid_count,
            "expired_entries": len(self._cache) - valid_count,
            "ttl_seconds": self.ttl_seconds,
        }


# Global cache instance for market data
# TTL of 5 seconds balances freshness with performance
market_data_cache = SimpleCache(ttl_seconds=5.0)
