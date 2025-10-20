"""
Configuration module for MCP Gateway.
Handles loading and validation of gateway configuration including:
- Tool filtering (expose_unified_only, expose_provider_tools)
- Per-provider rate limits
- Provider settings
"""
from pydantic import BaseModel, Field
from typing import Dict, List
import yaml
import logging
from pathlib import Path

logger = logging.getLogger(__name__)


# Feature 014: Venue name mapping (FR-004, FR-005, FR-006)
# Maps public-facing venue names to provider IDs (and tool prefixes)
VENUE_MAPPING: Dict[str, str] = {
    "binance": "binance",  # Public name → Provider ID (which matches tool prefix)
}

# Feature 014: List of public venue names (FR-004, FR-010)
# Only these names are exposed to API users
PUBLIC_VENUES: List[str] = list(VENUE_MAPPING.keys())


class RateLimitConfig(BaseModel):
    """Rate limit configuration for a provider."""

    requests_per_second: float = Field(
        default=10.0,
        description="Maximum requests per second",
        ge=0.1
    )

    burst_size: int = Field(
        default=20,
        description="Maximum burst size for rate limiter",
        ge=1
    )

    rate_limit_group: str | None = Field(
        default=None,
        description="Rate limit group identifier for shared limits"
    )


class ProviderConfig(BaseModel):
    """Configuration for a single provider."""

    name: str = Field(
        ...,
        description="Provider name (e.g., 'binance', 'okx')"
    )

    address: str = Field(
        ...,
        description="gRPC address (host:port)"
    )

    enabled: bool = Field(
        default=True,
        description="Whether provider is enabled"
    )

    rate_limit: RateLimitConfig | None = Field(
        default=None,
        description="Provider-specific rate limit configuration"
    )


class GatewayConfig(BaseModel):
    """
    Main gateway configuration.
    Supports FR-026, FR-027, FR-029.
    """

    # Tool filtering (FR-026, FR-027)
    expose_unified_only: bool = Field(
        default=True,
        description="Only expose unified tools (market.*, trade.*) to clients. "
                    "When true, provider-specific tools are hidden by default."
    )

    expose_provider_tools: List[str] = Field(
        default_factory=list,
        description="Whitelist of provider tool patterns to expose even when "
                    "expose_unified_only=true. Supports glob patterns like 'binance.get_*'."
    )

    # Provider configuration
    providers: List[ProviderConfig] = Field(
        default_factory=list,
        description="List of provider configurations"
    )

    # Global rate limits (FR-029)
    default_rate_limit: RateLimitConfig = Field(
        default_factory=lambda: RateLimitConfig(
            requests_per_second=10.0,
            burst_size=20
        ),
        description="Default rate limit for providers without specific configuration"
    )

    # Advanced settings
    enable_caching: bool = Field(
        default=True,
        description="Enable response caching"
    )

    cache_ttl_seconds: float = Field(
        default=5.0,
        description="Default cache TTL in seconds"
    )

    health_check_interval_seconds: int = Field(
        default=30,
        description="Interval between provider health checks"
    )

    class Config:
        """Pydantic model configuration."""
        json_schema_extra = {
            "example": {
                "expose_unified_only": True,
                "expose_provider_tools": ["binance.get_exchange_info"],
                "providers": [
                    {
                        "name": "binance",
                        "address": "localhost:50051",
                        "enabled": True,
                        "rate_limit": {
                            "requests_per_second": 20.0,
                            "burst_size": 40
                        }
                    }
                ],
                "default_rate_limit": {
                    "requests_per_second": 10.0,
                    "burst_size": 20
                }
            }
        }


def load_config(config_path: str | Path) -> GatewayConfig:
    """
    Load gateway configuration from YAML file.

    Args:
        config_path: Path to configuration file (YAML format)

    Returns:
        Validated GatewayConfig instance

    Raises:
        FileNotFoundError: If config file doesn't exist
        ValueError: If config validation fails
    """
    config_path = Path(config_path)

    if not config_path.exists():
        raise FileNotFoundError(f"Configuration file not found: {config_path}")

    try:
        with open(config_path, 'r') as f:
            config_data = yaml.safe_load(f)

        if not config_data:
            logger.warning(f"Empty config file at {config_path}, using defaults")
            return GatewayConfig()

        # Validate and parse configuration
        config = GatewayConfig(**config_data)
        logger.info(f"Loaded gateway configuration from {config_path}")
        logger.info(f"  - Unified only: {config.expose_unified_only}")
        logger.info(f"  - Provider whitelist: {config.expose_provider_tools}")
        logger.info(f"  - Providers: {len(config.providers)} configured")

        return config

    except yaml.YAMLError as e:
        raise ValueError(f"Failed to parse YAML configuration: {e}")
    except Exception as e:
        raise ValueError(f"Failed to load configuration: {e}")


def get_default_config() -> GatewayConfig:
    """
    Get default gateway configuration.

    Returns:
        GatewayConfig with sensible defaults
    """
    return GatewayConfig(
        expose_unified_only=True,
        expose_provider_tools=[],
        providers=[],
        default_rate_limit=RateLimitConfig(
            requests_per_second=10.0,
            burst_size=20
        )
    )
