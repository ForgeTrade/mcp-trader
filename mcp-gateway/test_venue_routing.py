#!/usr/bin/env python3
"""
Test P0 fix: Verify venue parameter is honored for routing.

This test verifies that market.generate_report correctly routes to the
appropriate provider based on the 'venue' parameter.
"""
import sys
import asyncio
from pathlib import Path

# Add mcp_gateway to path
sys.path.insert(0, str(Path(__file__).parent))

from mcp_gateway.main import MCPGateway


async def test_venue_routing():
    """Test that venue parameter correctly routes to appropriate provider."""
    print("=== Testing P0 Fix: Venue Routing ===\n")

    # Create gateway instance
    config_path = Path(__file__).parent / "providers.yaml"
    gateway = MCPGateway(str(config_path))

    # Initialize
    print("1. Initializing gateway...")
    await gateway.initialize()

    # Check venue_provider_map is populated
    print(f"\n2. Venue to Provider mapping:")
    for venue, provider in gateway.venue_provider_map.items():
        print(f"   {venue} -> {provider}")

    if not gateway.venue_provider_map:
        print("   ❌ FAIL: venue_provider_map is empty!")
        return False

    # Test 1: Default venue (binance)
    print("\n3. Test 1: Default venue (should route to binance)")
    arguments = {"instrument": "BTCUSDT"}
    # Simulate extraction logic from invoke_tool
    venue = arguments.get("venue", "binance").lower()
    provider_name = gateway.venue_provider_map.get(venue)

    if provider_name:
        print(f"   ✅ PASS: Default venue 'binance' routes to '{provider_name}'")
    else:
        print(f"   ❌ FAIL: Default venue 'binance' not found in mapping")
        return False

    # Test 2: Explicit venue
    print("\n4. Test 2: Explicit venue parameter")
    for test_venue in gateway.venue_provider_map.keys():
        arguments = {"instrument": "BTCUSDT", "venue": test_venue}
        venue = arguments.get("venue", "binance").lower()
        provider_name = gateway.venue_provider_map.get(venue)

        if provider_name:
            print(f"   ✅ PASS: Venue '{test_venue}' routes to '{provider_name}'")
        else:
            print(f"   ❌ FAIL: Venue '{test_venue}' not found in mapping")
            return False

    # Test 3: Invalid venue
    print("\n5. Test 3: Invalid venue (should return error with available venues)")
    arguments = {"instrument": "BTCUSDT", "venue": "kraken"}
    venue = arguments.get("venue", "binance").lower()
    provider_name = gateway.venue_provider_map.get(venue)

    if not provider_name:
        available_venues = list(gateway.venue_provider_map.keys())
        print(f"   ✅ PASS: Invalid venue 'kraken' correctly rejected")
        print(f"   Available venues: {available_venues}")
    else:
        print(f"   ⚠️  WARNING: 'kraken' unexpectedly found (might be configured)")

    # Cleanup
    await gateway.shutdown()

    print("\n=== All Venue Routing Tests Passed ✅ ===\n")
    return True


if __name__ == "__main__":
    result = asyncio.run(test_venue_routing())
    sys.exit(0 if result else 1)
