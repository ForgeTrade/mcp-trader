"""
Unified schema base structures for multi-exchange responses.
These schemas provide common fields that all unified tool responses must include.
"""
from pydantic import BaseModel, Field
from typing import Any


class UnifiedResponseBase(BaseModel):
    """
    Base model for all unified tool responses.
    Provides common fields that must be present in every response.
    """

    venue: str = Field(
        ...,
        description="Exchange/venue identifier (e.g., 'binance', 'okx')",
        examples=["binance", "okx", "coinbase"]
    )

    timestamp: int = Field(
        ...,
        description="Response timestamp in milliseconds since epoch",
        examples=[1697048400000]
    )

    latency_ms: float = Field(
        ...,
        description="Request latency in milliseconds (gateway to provider)",
        examples=[12.5, 45.2],
        ge=0.0
    )

    class Config:
        """Pydantic model configuration."""
        json_schema_extra = {
            "description": "Base structure for all unified exchange responses"
        }


class UnifiedErrorResponse(BaseModel):
    """
    Unified error response for failed tool invocations.
    Provides structured error information across all providers.
    """

    error: str = Field(
        ...,
        description="Error message describing what went wrong",
        examples=["Symbol not found", "Rate limit exceeded"]
    )

    error_code: str | None = Field(
        None,
        description="Error code for programmatic handling",
        examples=["SYMBOL_NOT_FOUND", "RATE_LIMIT"]
    )

    venue: str | None = Field(
        None,
        description="Exchange/venue where error occurred",
        examples=["binance", "okx"]
    )

    timestamp: int = Field(
        ...,
        description="Error timestamp in milliseconds since epoch",
        examples=[1697048400000]
    )

    alternatives: list[str] | None = Field(
        None,
        description="Suggested alternatives or corrections (e.g., valid symbols)",
        examples=[["BTCUSDT", "ETHUSDT"]]
    )

    class Config:
        """Pydantic model configuration."""
        json_schema_extra = {
            "description": "Unified error response for failed operations"
        }
