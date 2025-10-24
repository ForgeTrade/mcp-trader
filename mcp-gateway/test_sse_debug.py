#!/usr/bin/env python3
"""Debug SSE endpoint"""
import asyncio
import httpx
from httpx_sse import aconnect_sse


async def main():
    print("Connecting to SSE endpoint...")

    async with httpx.AsyncClient() as client:
        async with aconnect_sse(client, "GET", "http://localhost:3001/sse") as event_source:
            print("Connected! Reading events...")
            count = 0
            async for sse in event_source.aiter_sse():
                count += 1
                print(f"\nEvent #{count}:")
                print(f"  event: {sse.event!r}")
                print(f"  data: {sse.data!r}")
                print(f"  id: {sse.id!r}")
                print(f"  retry: {sse.retry!r}")

                if count >= 5:  # Stop after 5 events
                    break

    print("\nDone!")


if __name__ == "__main__":
    asyncio.run(main())
