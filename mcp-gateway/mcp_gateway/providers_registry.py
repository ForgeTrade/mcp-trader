"""
Provider registry module for managing provider connections and capabilities.
"""
import yaml
from typing import Dict, List, Any
from pathlib import Path


class ProviderConfig:
    """Configuration for a single provider."""

    def __init__(self, name: str, type: str, address: str, enabled: bool = True, metadata: Dict[str, Any] = None):
        self.name = name
        self.type = type
        self.address = address
        self.enabled = enabled
        self.metadata = metadata or {}


class ProviderRegistry:
    """
    Manages provider registration, discovery, and capability caching.
    """

    def __init__(self):
        self.providers: Dict[str, ProviderConfig] = {}
        self.capabilities_cache: Dict[str, Dict[str, Any]] = {}

    def load_providers(self, config_path: str = "providers.yaml") -> List[ProviderConfig]:
        """
        Load provider configurations from YAML file.

        Args:
            config_path: Path to providers.yaml configuration file

        Returns:
            List of ProviderConfig objects
        """
        config_file = Path(config_path)
        if not config_file.exists():
            raise FileNotFoundError(f"Provider configuration not found: {config_path}")

        with open(config_file, 'r') as f:
            config = yaml.safe_load(f)

        providers_list = []
        for provider_data in config.get('providers', []):
            provider = ProviderConfig(
                name=provider_data['name'],
                type=provider_data['type'],
                address=provider_data['address'],
                enabled=provider_data.get('enabled', True),
                metadata=provider_data.get('metadata', {})
            )
            if provider.enabled:
                self.providers[provider.name] = provider
                providers_list.append(provider)

        return providers_list

    def get_provider(self, name: str) -> ProviderConfig:
        """Get provider configuration by name."""
        if name not in self.providers:
            raise KeyError(f"Provider not found: {name}")
        return self.providers[name]

    def cache_capabilities(self, provider_name: str, capabilities: Dict[str, Any]):
        """Cache capabilities for a provider."""
        self.capabilities_cache[provider_name] = capabilities

    def get_cached_capabilities(self, provider_name: str) -> Dict[str, Any]:
        """Get cached capabilities for a provider."""
        return self.capabilities_cache.get(provider_name, {})

    def get_all_capabilities(self) -> Dict[str, Dict[str, Any]]:
        """Get all cached capabilities from all providers."""
        return self.capabilities_cache.copy()
