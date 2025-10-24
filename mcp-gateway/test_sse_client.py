#!/usr/bin/env python3
"""
Test client for SSE MCP Gateway - emulates ChatGPT/LLM requests.
Tests the market.generate_report tool via SSE transport.
"""
import asyncio
import json
import sys
import httpx
from httpx_sse import aconnect_sse


class MCPSSEClient:
    """Simple MCP client using SSE transport."""

    def __init__(self, base_url: str = "http://localhost:3001"):
        self.base_url = base_url
        self.session_id: str | None = None
        self.message_id = 0

    async def connect(self):
        """Establish SSE connection and initialize MCP session."""
        print(f"🔌 Connecting to SSE endpoint: {self.base_url}/sse")

        # Open SSE connection (GET request) and get session_id
        async with httpx.AsyncClient(timeout=30.0) as client:
            async with aconnect_sse(client, "GET", f"{self.base_url}/sse") as event_source:
                async for sse in event_source.aiter_sse():
                    if sse.event == "endpoint":
                        # Data is the endpoint URL: /sse/messages?session_id=xxx
                        endpoint_url = sse.data
                        # Parse session_id from URL
                        if "session_id=" in endpoint_url:
                            self.session_id = endpoint_url.split("session_id=")[1]
                        elif "sessionId=" in endpoint_url:
                            self.session_id = endpoint_url.split("sessionId=")[1]
                        print(f"✅ Connected with session_id: {self.session_id}")
                        break

        if not self.session_id:
            raise Exception("Failed to get session_id from SSE connection")

        # Send initialize request
        await self._send_message({
            "jsonrpc": "2.0",
            "id": self._next_id(),
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-sse-client",
                    "version": "1.0.0"
                }
            }
        })

    async def list_tools(self):
        """List available tools."""
        print("\n📋 Listing available tools...")

        response = await self._send_message({
            "jsonrpc": "2.0",
            "id": self._next_id(),
            "method": "tools/list",
            "params": {}
        })

        if "result" in response and "tools" in response["result"]:
            tools = response["result"]["tools"]
            print(f"✅ Found {len(tools)} tool(s):")
            for tool in tools:
                print(f"  - {tool['name']}: {tool.get('description', 'No description')}")
            return tools
        else:
            print(f"❌ Unexpected response: {response}")
            return []

    async def generate_market_report(self, instrument: str, venue: str = "binance", options: dict | None = None):
        """Generate market intelligence report."""
        print(f"\n📊 Generating market report for {instrument} on {venue}...")

        arguments = {
            "venue": venue,
            "instrument": instrument
        }

        if options:
            arguments["options"] = options

        response = await self._send_message({
            "jsonrpc": "2.0",
            "id": self._next_id(),
            "method": "tools/call",
            "params": {
                "name": "market.generate_report",
                "arguments": arguments
            }
        })

        if "result" in response:
            result = response["result"]
            if "content" in result and len(result["content"]) > 0:
                content = result["content"][0]
                if content.get("type") == "text":
                    # Parse the text content (should be JSON)
                    try:
                        report_data = json.loads(content["text"])
                        print("✅ Market report generated successfully!")
                        print(f"\n{'='*80}")
                        print(report_data.get("content", report_data))
                        print(f"{'='*80}\n")
                        return report_data
                    except json.JSONDecodeError:
                        # If not JSON, just print the text
                        print("✅ Market report generated successfully!")
                        print(f"\n{'='*80}")
                        print(content["text"])
                        print(f"{'='*80}\n")
                        return content["text"]
            else:
                print(f"❌ Unexpected result format: {result}")
                return None
        elif "error" in response:
            error = response["error"]
            print(f"❌ Error: {error.get('message', error)}")
            return None
        else:
            print(f"❌ Unexpected response: {response}")
            return None

    async def _send_message(self, message: dict) -> dict:
        """Send JSON-RPC message via SSE POST and wait for response."""
        if not self.session_id:
            raise Exception("Not connected - call connect() first")

        url = f"{self.base_url}/sse/messages?session_id={self.session_id}"

        print(f"📤 Sending: {message['method']}")

        async with httpx.AsyncClient() as client:
            response = await client.post(
                url,
                json=message,
                timeout=30.0
            )

            # Accept both 200 OK and 202 Accepted
            if response.status_code not in [200, 202]:
                raise Exception(f"Request failed: {response.status_code} {response.text}")

            # Parse response (if available)
            if response.text:
                try:
                    result = response.json()
                    method_name = result.get('result', {}).get('_meta', {}).get('progressToken', message.get('method', 'response'))
                    print(f"📥 Received: {method_name}")
                    return result
                except Exception:
                    # Response might be empty for 202
                    print(f"📥 Accepted (no immediate response)")
                    return {"status": "accepted"}
            else:
                print(f"📥 Accepted (no content)")
                return {"status": "accepted"}

    def _next_id(self) -> int:
        """Get next message ID."""
        self.message_id += 1
        return self.message_id


async def main():
    """Run test scenarios."""
    print("=== MCP SSE Client Test - Market Report Feature 018 ===\n")

    # Check if server is running
    try:
        async with httpx.AsyncClient() as client:
            health_response = await client.get("http://localhost:3001/health", timeout=5.0)
            if health_response.status_code == 200:
                print("✅ Server health check passed")
            else:
                print(f"⚠️  Server returned {health_response.status_code}")
    except Exception as e:
        print(f"❌ Server not responding: {e}")
        print("Please start the SSE server first:")
        print("  cd mcp-gateway && uv run python -m mcp_gateway.sse_server")
        sys.exit(1)

    # Create client and connect
    client = MCPSSEClient()

    try:
        await client.connect()

        # List available tools
        tools = await client.list_tools()

        # Test 1: Generate basic report for BTCUSDT
        print("\n" + "="*80)
        print("TEST 1: Basic Report Generation (BTCUSDT)")
        print("="*80)
        await client.generate_market_report("BTCUSDT")

        # Test 2: Generate report with custom options
        print("\n" + "="*80)
        print("TEST 2: Custom Options Report (ETHUSDT with specific sections)")
        print("="*80)
        await client.generate_market_report(
            "ETHUSDT",
            options={
                "include_sections": ["price_overview", "orderbook_metrics"],
                "orderbook_levels": 10
            }
        )

        # Test 3: Invalid symbol error handling
        print("\n" + "="*80)
        print("TEST 3: Error Handling (Invalid Symbol)")
        print("="*80)
        await client.generate_market_report("INVALID")

        print("\n✅ All tests completed!")

    except Exception as e:
        print(f"\n❌ Test failed: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    asyncio.run(main())
