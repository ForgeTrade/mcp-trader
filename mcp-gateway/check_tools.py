#!/usr/bin/env python3
"""Quick check to see what tools are exposed via SSE"""
import asyncio
import httpx
from httpx_sse import aconnect_sse
import json


async def main():
    print("Connecting to SSE server...")

    # Get session_id
    session_id = None
    async with httpx.AsyncClient(timeout=30.0) as client:
        async with aconnect_sse(client, "GET", "http://localhost:3001/sse") as event_source:
            async for sse in event_source.aiter_sse():
                if sse.event == "endpoint":
                    endpoint_url = sse.data
                    if "session_id=" in endpoint_url:
                        session_id = endpoint_url.split("session_id=")[1]
                    elif "sessionId=" in endpoint_url:
                        session_id = endpoint_url.split("sessionId=")[1]
                    print(f"✅ Connected with session_id: {session_id}")
                    break

    if not session_id:
        print("❌ Failed to get session_id")
        return

    # Send initialize
    print("\nSending initialize...")
    url = f"http://localhost:3001/sse/messages?session_id={session_id}"
    async with httpx.AsyncClient(timeout=30.0) as client:
        resp = await client.post(url, json={
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "1.0"}
            }
        })
        print(f"Initialize: {resp.status_code}")

    # List tools
    print("\nListing tools...")
    async with httpx.AsyncClient(timeout=30.0) as client:
        resp = await client.post(url, json={
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        })
        print(f"Tools/list: {resp.status_code}")

        # Since response is async via SSE, let's wait a bit and check logs
        await asyncio.sleep(2)

    print("\n" + "="*80)
    print("Check /tmp/gateway_test.log for the actual response")
    print("Looking for tools/list handler output...")
    print("="*80)


if __name__ == "__main__":
    asyncio.run(main())
