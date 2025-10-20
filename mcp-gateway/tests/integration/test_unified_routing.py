"""
Integration tests for unified tool routing (T024).
Verifies venue-based routing and UnifiedToolRouter functionality.
Tests FR-003 (venue-based routing) and FR-018 (fallback).
"""
import pytest
from unittest.mock import AsyncMock, MagicMock, patch
from mcp_gateway.adapters.unified_router import UnifiedToolRouter
from mcp_gateway.adapters.grpc_client import ProviderGRPCClient


class TestUnifiedToolRouting:
    """Test suite for unified tool routing."""

    @pytest.fixture
    def mock_binance_client(self):
        """Create mock Binance gRPC client."""
        client = MagicMock(spec=ProviderGRPCClient)
        client.provider_name = "binance"
        client.is_healthy = MagicMock(return_value=True)
        client.invoke = AsyncMock()
        return client

    @pytest.fixture
    def mock_okx_client(self):
        """Create mock OKX gRPC client."""
        client = MagicMock(spec=ProviderGRPCClient)
        client.provider_name = "okx"
        client.is_healthy = MagicMock(return_value=True)
        client.invoke = AsyncMock()
        return client

    @pytest.fixture
    def router_single_provider(self, mock_binance_client):
        """Create router with single provider."""
        return UnifiedToolRouter({"binance": mock_binance_client})

    @pytest.fixture
    def router_multi_provider(self, mock_binance_client, mock_okx_client):
        """Create router with multiple providers."""
        return UnifiedToolRouter({
            "binance": mock_binance_client,
            "okx": mock_okx_client
        })

    @pytest.mark.asyncio
    async def test_route_ticker_to_binance(self, router_single_provider, mock_binance_client):
        """Test routing market.get_ticker to Binance provider (FR-003)."""
        # Setup mock response
        mock_binance_client.invoke.return_value = {
            "result": {
                "symbol": "BTCUSDT",
                "bidPrice": "43250.50",
                "askPrice": "43251.00",
                "volume": "12345.67",
                "closeTime": 1697048400000
            }
        }

        # Call unified tool
        result = await router_single_provider.route_tool_call(
            unified_tool_name="market.get_ticker",
            arguments={"venue": "binance", "instrument": "BTCUSDT"},
            correlation_id="test-123",
            timeout=5.0
        )

        # Verify routing
        mock_binance_client.invoke.assert_called_once()
        call_args = mock_binance_client.invoke.call_args
        assert call_args.kwargs["tool_name"] == "binance.get_ticker"
        assert call_args.kwargs["payload"]["symbol"] == "BTCUSDT"

        # Verify result includes routing metadata
        assert "routing_info" in result
        assert result["routing_info"]["unified_tool"] == "market.get_ticker"
        assert result["routing_info"]["venue"] == "binance"

    @pytest.mark.asyncio
    async def test_route_orderbook_to_binance(self, router_single_provider, mock_binance_client):
        """Test routing market.get_orderbook_l1 to Binance provider (FR-003)."""
        mock_binance_client.invoke.return_value = {
            "result": {
                "lastUpdateId": 123456789,
                "bids": [["43250.50", "1.234"]],
                "asks": [["43251.00", "0.987"]]
            }
        }

        result = await router_single_provider.route_tool_call(
            unified_tool_name="market.get_orderbook_l1",
            arguments={"venue": "binance", "instrument": "ETHUSDT"},
            correlation_id="test-456",
            timeout=5.0
        )

        # Verify tool name mapping
        mock_binance_client.invoke.assert_called_once()
        call_args = mock_binance_client.invoke.call_args
        assert call_args.kwargs["tool_name"] == "binance.orderbook_l1"
        assert call_args.kwargs["payload"]["symbol"] == "ETHUSDT"

    @pytest.mark.asyncio
    async def test_route_to_okx_provider(self, router_multi_provider, mock_okx_client):
        """Test routing to OKX when multiple providers available (FR-003)."""
        mock_okx_client.invoke.return_value = {
            "result": {
                "symbol": "BTC-USDT",
                "best_bid": "43250.50",
                "best_ask": "43251.00"
            }
        }

        result = await router_multi_provider.route_tool_call(
            unified_tool_name="market.get_ticker",
            arguments={"venue": "okx", "instrument": "BTC-USDT"},
            correlation_id="test-789",
            timeout=5.0
        )

        # Verify OKX was called, not Binance
        mock_okx_client.invoke.assert_called_once()
        assert result["routing_info"]["venue"] == "okx"

    @pytest.mark.asyncio
    async def test_instrument_parameter_mapping(self, router_single_provider, mock_binance_client):
        """Test that 'instrument' parameter is mapped to 'symbol' (FR-003)."""
        mock_binance_client.invoke.return_value = {"result": {}}

        await router_single_provider.route_tool_call(
            unified_tool_name="market.get_ticker",
            arguments={"venue": "binance", "instrument": "SOLUSDT"},
            correlation_id="test-param",
            timeout=5.0
        )

        # Verify 'instrument' was renamed to 'symbol'
        call_args = mock_binance_client.invoke.call_args
        assert "symbol" in call_args.kwargs["payload"]
        assert call_args.kwargs["payload"]["symbol"] == "SOLUSDT"
        assert "instrument" not in call_args.kwargs["payload"]
        assert "venue" not in call_args.kwargs["payload"]

    @pytest.mark.asyncio
    async def test_missing_venue_parameter(self, router_single_provider):
        """Test error when venue parameter is missing."""
        with pytest.raises(ValueError, match="Missing required 'venue' parameter"):
            await router_single_provider.route_tool_call(
                unified_tool_name="market.get_ticker",
                arguments={"instrument": "BTCUSDT"},  # Missing venue
                correlation_id="test-error",
                timeout=5.0
            )

    @pytest.mark.asyncio
    async def test_unknown_venue(self, router_single_provider):
        """Test error when venue doesn't exist."""
        with pytest.raises(ValueError, match="Unknown venue"):
            await router_single_provider.route_tool_call(
                unified_tool_name="market.get_ticker",
                arguments={"venue": "nonexistent", "instrument": "BTCUSDT"},
                correlation_id="test-unknown",
                timeout=5.0
            )

    @pytest.mark.asyncio
    async def test_unsupported_unified_tool(self, router_single_provider):
        """Test error when unified tool is not supported."""
        with pytest.raises(ValueError, match="Unsupported unified tool"):
            await router_single_provider.route_tool_call(
                unified_tool_name="market.unsupported_tool",
                arguments={"venue": "binance", "instrument": "BTCUSDT"},
                correlation_id="test-unsupported",
                timeout=5.0
            )

    @pytest.mark.asyncio
    async def test_provider_failure_propagates(self, router_single_provider, mock_binance_client):
        """Test that provider failures are propagated with context."""
        # Setup mock to raise error
        mock_binance_client.invoke.side_effect = Exception("Connection timeout")

        with pytest.raises(RuntimeError, match="Provider binance failed"):
            await router_single_provider.route_tool_call(
                unified_tool_name="market.get_ticker",
                arguments={"venue": "binance", "instrument": "BTCUSDT"},
                correlation_id="test-failure",
                timeout=5.0
            )

    @pytest.mark.asyncio
    async def test_latency_tracking(self, router_single_provider, mock_binance_client):
        """Test that latency is tracked and included in response."""
        mock_binance_client.invoke.return_value = {"result": {"data": "test"}}

        result = await router_single_provider.route_tool_call(
            unified_tool_name="market.get_ticker",
            arguments={"venue": "binance", "instrument": "BTCUSDT"},
            correlation_id="test-latency",
            timeout=5.0
        )

        # Verify latency metadata
        assert "routing_info" in result
        assert "latency_ms" in result["routing_info"]
        assert isinstance(result["routing_info"]["latency_ms"], float)
        assert result["routing_info"]["latency_ms"] >= 0

        # Verify latency is also added to result
        assert "latency_ms" in result["result"]

    @pytest.mark.asyncio
    async def test_venue_added_to_result(self, router_single_provider, mock_binance_client):
        """Test that venue is added to result."""
        mock_binance_client.invoke.return_value = {"result": {"data": "test"}}

        result = await router_single_provider.route_tool_call(
            unified_tool_name="market.get_ticker",
            arguments={"venue": "binance", "instrument": "BTCUSDT"},
            correlation_id="test-venue",
            timeout=5.0
        )

        assert result["result"]["venue"] == "binance"

    @pytest.mark.asyncio
    async def test_unhealthy_provider_warning(self, router_single_provider, mock_binance_client):
        """Test that unhealthy providers still get requests but log warning (FR-018)."""
        # Mark provider as unhealthy
        mock_binance_client.is_healthy.return_value = False
        mock_binance_client.invoke.return_value = {"result": {"data": "test"}}

        # Should still work, just with warning
        result = await router_single_provider.route_tool_call(
            unified_tool_name="market.get_ticker",
            arguments={"venue": "binance", "instrument": "BTCUSDT"},
            correlation_id="test-unhealthy",
            timeout=5.0
        )

        # Verify request was still made
        mock_binance_client.invoke.assert_called_once()
        assert "result" in result

    def test_get_available_venues(self, router_multi_provider):
        """Test getting list of available venues."""
        venues = router_multi_provider.get_available_venues("market.get_ticker")

        assert "binance" in venues
        assert "okx" in venues
        assert len(venues) == 2

    def test_get_supported_tools(self, router_single_provider):
        """Test getting list of supported unified tools."""
        tools = router_single_provider.get_supported_tools()

        assert "market.get_ticker" in tools
        assert "market.get_orderbook_l1" in tools
        assert "market.get_orderbook_l2" in tools
        assert "market.get_klines" in tools

    def test_get_tool_metadata(self, router_single_provider):
        """Test getting metadata about a unified tool."""
        metadata = router_single_provider.get_tool_metadata("market.get_ticker")

        assert metadata is not None
        assert metadata["name"] == "market.get_ticker"
        assert metadata["provider_pattern"] == "{venue}.get_ticker"
        assert "binance" in metadata["available_venues"]
        assert metadata["supported"] is True

    def test_get_tool_metadata_unsupported(self, router_single_provider):
        """Test getting metadata for unsupported tool."""
        metadata = router_single_provider.get_tool_metadata("market.nonexistent")
        assert metadata is None

    @pytest.mark.asyncio
    async def test_correlation_id_propagation(self, router_single_provider, mock_binance_client):
        """Test that correlation ID is propagated to provider."""
        mock_binance_client.invoke.return_value = {"result": {}}
        correlation_id = "test-correlation-123"

        await router_single_provider.route_tool_call(
            unified_tool_name="market.get_ticker",
            arguments={"venue": "binance", "instrument": "BTCUSDT"},
            correlation_id=correlation_id,
            timeout=5.0
        )

        call_args = mock_binance_client.invoke.call_args
        assert call_args.kwargs["correlation_id"] == correlation_id

    @pytest.mark.asyncio
    async def test_timeout_propagation(self, router_single_provider, mock_binance_client):
        """Test that timeout is propagated to provider."""
        mock_binance_client.invoke.return_value = {"result": {}}
        custom_timeout = 10.0

        await router_single_provider.route_tool_call(
            unified_tool_name="market.get_ticker",
            arguments={"venue": "binance", "instrument": "BTCUSDT"},
            correlation_id="test-timeout",
            timeout=custom_timeout
        )

        call_args = mock_binance_client.invoke.call_args
        assert call_args.kwargs["timeout"] == custom_timeout
