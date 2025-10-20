"""
Integration tests for unified ticker normalization (T023).
Verifies Binance ticker response normalization to unified schema.
Tests FR-008 (ticker normalization).
"""
import pytest
from mcp_gateway.adapters.schema_adapter import SchemaAdapter


class TestUnifiedTickerNormalization:
    """Test suite for Binance ticker normalization."""

    @pytest.fixture
    def schema_adapter(self):
        """Create schema adapter instance."""
        return SchemaAdapter()

    @pytest.fixture
    def sample_binance_ticker(self):
        """Sample Binance ticker response."""
        return {
            "symbol": "BTCUSDT",
            "bidPrice": "43250.50",
            "askPrice": "43251.00",
            "lastPrice": "43250.75",
            "volume": "12345.67",
            "quoteVolume": "534567890.12",
            "priceChangePercent": "2.45",
            "closeTime": 1697048400000
        }

    def test_ticker_normalization_basic_fields(self, schema_adapter, sample_binance_ticker):
        """Test that all mandatory fields are present and correctly normalized."""
        normalized = schema_adapter.normalize(
            venue="binance",
            data_type="ticker",
            raw_response=sample_binance_ticker
        )

        # Verify mandatory fields (FR-008)
        assert "bid" in normalized
        assert "ask" in normalized
        assert "mid" in normalized
        assert "spread_bps" in normalized
        assert "volume" in normalized
        assert "timestamp" in normalized
        assert "venue_symbol" in normalized

        # Verify types
        assert isinstance(normalized["bid"], float)
        assert isinstance(normalized["ask"], float)
        assert isinstance(normalized["mid"], float)
        assert isinstance(normalized["spread_bps"], float)
        assert isinstance(normalized["volume"], float)
        assert isinstance(normalized["timestamp"], int)

    def test_ticker_price_conversion(self, schema_adapter, sample_binance_ticker):
        """Test that string prices are converted to floats (FR-008)."""
        normalized = schema_adapter.normalize(
            venue="binance",
            data_type="ticker",
            raw_response=sample_binance_ticker
        )

        # Verify price conversion
        assert normalized["bid"] == 43250.50
        assert normalized["ask"] == 43251.00

    def test_ticker_mid_calculation(self, schema_adapter, sample_binance_ticker):
        """Test mid-price calculation (FR-008)."""
        normalized = schema_adapter.normalize(
            venue="binance",
            data_type="ticker",
            raw_response=sample_binance_ticker
        )

        expected_mid = (43250.50 + 43251.00) / 2.0
        assert normalized["mid"] == expected_mid
        assert normalized["mid"] == 43250.75

    def test_ticker_spread_bps_calculation(self, schema_adapter, sample_binance_ticker):
        """Test spread in basis points calculation (FR-008)."""
        normalized = schema_adapter.normalize(
            venue="binance",
            data_type="ticker",
            raw_response=sample_binance_ticker
        )

        # Calculate expected spread_bps
        bid = 43250.50
        ask = 43251.00
        mid = (bid + ask) / 2.0
        expected_spread_bps = ((ask - bid) / mid) * 10000.0

        assert abs(normalized["spread_bps"] - expected_spread_bps) < 0.01
        assert normalized["spread_bps"] > 0

    def test_ticker_optional_fields(self, schema_adapter, sample_binance_ticker):
        """Test that optional fields are included when present."""
        normalized = schema_adapter.normalize(
            venue="binance",
            data_type="ticker",
            raw_response=sample_binance_ticker
        )

        # Verify optional fields
        assert "last" in normalized
        assert normalized["last"] == 43250.75

        assert "quote_volume" in normalized
        assert normalized["quote_volume"] == 534567890.12

        assert "price_change_percent" in normalized
        assert normalized["price_change_percent"] == 2.45

    def test_ticker_timestamp_preservation(self, schema_adapter, sample_binance_ticker):
        """Test that timestamp is preserved from Binance response."""
        normalized = schema_adapter.normalize(
            venue="binance",
            data_type="ticker",
            raw_response=sample_binance_ticker
        )

        assert normalized["timestamp"] == 1697048400000

    def test_ticker_venue_symbol_preservation(self, schema_adapter, sample_binance_ticker):
        """Test that original symbol is preserved."""
        normalized = schema_adapter.normalize(
            venue="binance",
            data_type="ticker",
            raw_response=sample_binance_ticker
        )

        assert normalized["venue_symbol"] == "BTCUSDT"

    def test_ticker_additional_fields(self, schema_adapter, sample_binance_ticker):
        """Test that additional fields are merged correctly."""
        additional_fields = {
            "latency_ms": 12.5,
            "venue": "binance"
        }

        normalized = schema_adapter.normalize(
            venue="binance",
            data_type="ticker",
            raw_response=sample_binance_ticker,
            additional_fields=additional_fields
        )

        assert normalized["latency_ms"] == 12.5
        assert normalized["venue"] == "binance"

    def test_ticker_wide_spread(self, schema_adapter):
        """Test ticker with wide spread."""
        wide_spread_ticker = {
            "symbol": "TESTUSDT",
            "bidPrice": "100.00",
            "askPrice": "105.00",  # 5% spread
            "volume": "1000.00",
            "closeTime": 1697048400000
        }

        normalized = schema_adapter.normalize(
            venue="binance",
            data_type="ticker",
            raw_response=wide_spread_ticker
        )

        # Wide spread should result in high spread_bps
        assert normalized["spread_bps"] > 400  # > 4%

    def test_ticker_tight_spread(self, schema_adapter):
        """Test ticker with tight spread."""
        tight_spread_ticker = {
            "symbol": "BTCUSDT",
            "bidPrice": "50000.00",
            "askPrice": "50000.10",  # 0.002% spread
            "volume": "10000.00",
            "closeTime": 1697048400000
        }

        normalized = schema_adapter.normalize(
            venue="binance",
            data_type="ticker",
            raw_response=tight_spread_ticker
        )

        # Tight spread should result in low spread_bps
        assert normalized["spread_bps"] < 5  # < 0.05%

    def test_ticker_missing_optional_fields(self, schema_adapter):
        """Test ticker with only required Binance fields."""
        minimal_ticker = {
            "symbol": "ETHUSDT",
            "bidPrice": "2500.00",
            "askPrice": "2500.50",
            "volume": "5000.00",
            "closeTime": 1697048400000
        }

        normalized = schema_adapter.normalize(
            venue="binance",
            data_type="ticker",
            raw_response=minimal_ticker
        )

        # Mandatory fields should still be present
        assert normalized["bid"] == 2500.00
        assert normalized["ask"] == 2500.50
        assert "mid" in normalized
        assert "spread_bps" in normalized

        # Optional fields should be absent
        assert "last" not in normalized
        assert "quote_volume" not in normalized
        assert "price_change_percent" not in normalized

    def test_ticker_normalization_error_handling(self, schema_adapter):
        """Test error handling for invalid ticker data."""
        invalid_ticker = {
            "symbol": "BTCUSDT",
            # Missing required fields
        }

        with pytest.raises(Exception):
            schema_adapter.normalize(
                venue="binance",
                data_type="ticker",
                raw_response=invalid_ticker
            )

    def test_unsupported_venue(self, schema_adapter, sample_binance_ticker):
        """Test that unsupported venue raises error."""
        with pytest.raises(ValueError, match="No normalizer available"):
            schema_adapter.normalize(
                venue="unsupported_exchange",
                data_type="ticker",
                raw_response=sample_binance_ticker
            )

    def test_unsupported_data_type(self, schema_adapter, sample_binance_ticker):
        """Test that unsupported data type raises error."""
        with pytest.raises(ValueError, match="No normalizer available"):
            schema_adapter.normalize(
                venue="binance",
                data_type="unsupported_type",
                raw_response=sample_binance_ticker
            )
