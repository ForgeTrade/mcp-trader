"""
gRPC client adapter for provider communication.
Implements connection pooling and fail-fast timeout strategy.
"""
import grpc.aio
import json
import logging
import time
from typing import Dict, List, Any, Optional
from google.protobuf import empty_pb2

from mcp_gateway.generated import provider_pb2, provider_pb2_grpc

logger = logging.getLogger(__name__)


class ProviderGRPCClient:
    """
    gRPC client for communicating with providers.
    Implements connection pooling (15 channels per provider) and fail-fast timeouts.
    """

    def __init__(self, provider_name: str, address: str, num_channels: int = 15):
        """
        Initialize gRPC client with connection pooling.

        Args:
            provider_name: Name of the provider
            address: Provider gRPC address (host:port)
            num_channels: Number of channels in the pool (default: 15)
        """
        self.provider_name = provider_name
        self.address = address
        self.channels: List[grpc.aio.Channel] = []
        self.current_idx = 0

        # Health check state (T011)
        self._is_healthy = True
        self._last_health_check: Optional[float] = None
        self._consecutive_failures = 0

        # Create channel pool with unique IDs
        for i in range(num_channels):
            channel = grpc.aio.insecure_channel(
                address,
                options=[
                    ("grpc.channel_pool_id", i),
                    ("grpc.keepalive_time_ms", 55000),
                    ("grpc.keepalive_timeout_ms", 10000),
                ]
            )
            self.channels.append(channel)

        logger.info(f"Created {num_channels} gRPC channels for provider {provider_name} at {address}")

    def get_channel(self) -> grpc.aio.Channel:
        """Get a channel from the pool using round-robin."""
        channel = self.channels[self.current_idx]
        self.current_idx = (self.current_idx + 1) % len(self.channels)
        return channel

    async def list_capabilities(self, timeout: float = 2.5) -> Dict[str, Any]:
        """
        Call ListCapabilities RPC on provider.

        Args:
            timeout: Request timeout in seconds (default: 2.5s)

        Returns:
            Capabilities dictionary

        Raises:
            grpc.RpcError: On communication failure
        """
        channel = self.get_channel()
        stub = provider_pb2_grpc.ProviderStub(channel)

        try:
            response = await stub.ListCapabilities(empty_pb2.Empty(), timeout=timeout)

            # Convert protobuf response to dictionary
            capabilities = {
                "tools": [
                    {
                        "name": tool.name,
                        "description": tool.description,
                        "input_schema": json.loads(tool.input_schema.value.decode('utf-8')) if tool.input_schema.value else None,
                        "output_schema": json.loads(tool.output_schema.value.decode('utf-8')) if tool.output_schema.value else None,
                    }
                    for tool in response.tools
                ],
                "resources": [
                    {
                        "uri_scheme": resource.uri_scheme,
                        "description": resource.description,
                        "mime_type": resource.mime_type,
                    }
                    for resource in response.resources
                ],
                "prompts": [
                    {
                        "name": prompt.name,
                        "description": prompt.description,
                        "args_schema": json.loads(prompt.args_schema.value.decode('utf-8')) if prompt.args_schema.value else None,
                    }
                    for prompt in response.prompts
                ],
                "provider_version": response.provider_version,
            }

            logger.info(f"Retrieved capabilities from {self.provider_name}: {len(capabilities['tools'])} tools")
            return capabilities

        except grpc.RpcError as e:
            logger.error(f"Failed to list capabilities from {self.provider_name}: {e.code()} - {e.details()}")
            raise

    async def invoke(self, tool_name: str, payload: Dict[str, Any], correlation_id: str, timeout: float = 15.0) -> Dict[str, Any]:
        """
        Invoke a tool on the provider.

        Args:
            tool_name: Name of the tool to invoke
            payload: Tool arguments
            correlation_id: Correlation ID for tracing
            timeout: Request timeout in seconds (default: 15.0s for analytics-heavy operations)

        Returns:
            Tool result dictionary with 'result' or 'error' key

        Raises:
            grpc.RpcError: On communication failure
        """
        channel = self.get_channel()
        stub = provider_pb2_grpc.ProviderStub(channel)

        # Serialize payload to JSON bytes
        payload_bytes = json.dumps(payload).encode('utf-8')

        request = provider_pb2.InvokeRequest(
            tool_name=tool_name,
            payload=provider_pb2.Json(value=payload_bytes),
            correlation_id=correlation_id,
        )

        try:
            response = await stub.Invoke(request, timeout=timeout)

            if response.error:
                logger.warning(f"Tool invocation failed: {tool_name} - {response.error}")
                return {"error": response.error}

            # Parse result JSON
            result = json.loads(response.result.value.decode('utf-8'))
            return {"result": result}

        except grpc.RpcError as e:
            logger.error(f"gRPC error invoking {tool_name} on {self.provider_name}: {e.code()} - {e.details()}")
            raise

    async def read_resource(self, uri: str, correlation_id: str, timeout: float = 2.5) -> bytes:
        """
        Read a resource from the provider.

        Args:
            uri: Resource URI
            correlation_id: Correlation ID for tracing
            timeout: Request timeout in seconds

        Returns:
            Resource content as bytes

        Raises:
            grpc.RpcError: On communication failure
        """
        channel = self.get_channel()
        stub = provider_pb2_grpc.ProviderStub(channel)

        request = provider_pb2.ResourceRequest(
            uri=uri,
            correlation_id=correlation_id,
        )

        try:
            response = await stub.ReadResource(request, timeout=timeout)
            if response.error:
                raise ValueError(f"Resource read failed: {response.error}")
            return response.content
        except grpc.RpcError as e:
            logger.error(f"gRPC error reading resource {uri} from {self.provider_name}: {e.code()} - {e.details()}")
            raise

    async def get_prompt(self, prompt_name: str, arguments: Dict[str, Any], correlation_id: str, timeout: float = 2.5) -> List[Dict[str, str]]:
        """
        Get a prompt from the provider.

        Args:
            prompt_name: Name of the prompt
            arguments: Prompt arguments
            correlation_id: Correlation ID for tracing
            timeout: Request timeout in seconds

        Returns:
            List of prompt messages

        Raises:
            grpc.RpcError: On communication failure
        """
        channel = self.get_channel()
        stub = provider_pb2_grpc.ProviderStub(channel)

        arguments_bytes = json.dumps(arguments).encode('utf-8')
        request = provider_pb2.PromptRequest(
            prompt_name=prompt_name,
            arguments=provider_pb2.Json(value=arguments_bytes),
            correlation_id=correlation_id,
        )

        try:
            response = await stub.GetPrompt(request, timeout=timeout)
            if response.error:
                raise ValueError(f"Prompt retrieval failed: {response.error}")

            return [
                {"role": msg.role, "content": msg.content}
                for msg in response.messages
            ]
        except grpc.RpcError as e:
            logger.error(f"gRPC error getting prompt {prompt_name} from {self.provider_name}: {e.code()} - {e.details()}")
            raise

    async def health_check(self, timeout: float = 1.0) -> bool:
        """
        Perform health check on provider by calling ListCapabilities.

        Args:
            timeout: Health check timeout in seconds (default: 1.0s)

        Returns:
            True if provider is healthy, False otherwise
        """
        try:
            await self.list_capabilities(timeout=timeout)
            self._is_healthy = True
            self._consecutive_failures = 0
            self._last_health_check = time.time()
            logger.debug(f"Health check passed for provider {self.provider_name}")
            return True
        except Exception as e:
            self._consecutive_failures += 1
            self._is_healthy = False
            self._last_health_check = time.time()
            logger.warning(
                f"Health check failed for provider {self.provider_name} "
                f"(consecutive failures: {self._consecutive_failures}): {e}"
            )
            return False

    def is_healthy(self) -> bool:
        """Check if provider is currently marked as healthy."""
        return self._is_healthy

    def get_health_status(self) -> Dict[str, Any]:
        """
        Get detailed health status information.

        Returns:
            Dictionary with health status details
        """
        return {
            "provider": self.provider_name,
            "address": self.address,
            "healthy": self._is_healthy,
            "last_check": self._last_health_check,
            "consecutive_failures": self._consecutive_failures,
            "channels": len(self.channels),
        }

    async def close(self):
        """Close all channels in the pool."""
        for channel in self.channels:
            await channel.close()
        logger.info(f"Closed {len(self.channels)} channels for provider {self.provider_name}")
