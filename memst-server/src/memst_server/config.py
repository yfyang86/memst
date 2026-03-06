"""Configuration loader for MemSt server.

This module provides configuration management for the MemSt Python server.

Configuration File Format:
    The server loads configuration from TOML files. It searches for config files
    in the following order:
    1. Path specified by MEMST_CONFIG_PATH environment variable
    2. ./config.toml (current directory)
    3. ../config.toml (parent directory)
    4. ./memst-store/config.toml
    5. ~/.config/memst/config.toml

LLM Configuration Compatibility:
    The server supports both 'api_url' and 'base_url' field names in the [llm]
    and [embedding] sections to maintain compatibility with the Rust backend
    configuration format.
    
    Use 'api_url' for Python-native configuration:
    ```toml
    [llm]
    api_url = "http://localhost:8080/v1"
    model = "gpt-4"
    ```
    
    Or use 'base_url' for Rust-compatible configuration:
    ```toml
    [llm]
    base_url = "http://localhost:8080/v1"
    model = "gpt-4"
    ```
    
    Both formats are fully supported and can be used interchangeably.
"""

import os
from pathlib import Path
from typing import Optional
from functools import lru_cache

import toml
from pydantic import BaseModel


class LLMConfig(BaseModel):
    """LLM configuration.
    
    Supports both 'api_url' and 'base_url' field names for compatibility
    with different configuration sources (Rust backend uses base_url).
    """
    type: str = "openai"
    api_url: str = "http://localhost:8080/v1"
    # Alias for api_url to match Rust backend config format
    base_url: Optional[str] = None
    model: str = ""
    timeout: int = 60
    max_tokens: int = 8192
    temperature: float = 0.7
    api_key: str = ""
    
    def get_api_url(self) -> str:
        """Get the effective API URL (supports both api_url and base_url)."""
        return self.base_url if self.base_url else self.api_url


class EmbeddingConfig(BaseModel):
    """Embedding configuration.
    
    Supports both 'api_url' and 'base_url' field names for compatibility
    with different configuration sources (Rust backend uses base_url).
    """
    type: str = "openai"
    api_url: str = "http://localhost:8081/v1/embeddings"
    # Alias for api_url to match Rust backend config format
    base_url: Optional[str] = None
    model: str = "text-embedding-bge_m3"
    timeout: int = 30
    expected_dimension: int = 1024
    
    def get_api_url(self) -> str:
        """Get the effective API URL (supports both api_url and base_url)."""
        return self.base_url if self.base_url else self.api_url


class ServerConfig(BaseModel):
    """Server configuration."""
    host: str = "127.0.0.1"
    port: int = 8192
    debug: bool = False
    store_path: str = "./memst-store"
    cors_origins: list[str] = []  # No CORS by default


class Config(BaseModel):
    """Main configuration."""
    llm: Optional[LLMConfig] = None
    embedding: Optional[EmbeddingConfig] = None
    server: ServerConfig = ServerConfig()


def find_config_file() -> Optional[Path]:
    """Find config.toml file."""
    # Check MEMST_CONFIG_PATH env var
    if os.environ.get("MEMST_CONFIG_PATH"):
        path = Path(os.environ["MEMST_CONFIG_PATH"])
        if path.exists():
            return path

    # Search in current and parent directories
    cwd = Path.cwd()
    for path in [cwd, cwd.parent, cwd.parent.parent]:
        config_path = path / "config.toml"
        if config_path.exists():
            return config_path

    # Check default locations
    default_paths = [
        Path("./memst-store/config.toml"),
        Path.home() / ".config/memst/config.toml",
    ]
    for path in default_paths:
        if path.exists():
            return path

    return None


def _load_llm_config(data: dict) -> LLMConfig:
    """Load LLM config with support for both api_url and base_url."""
    llm_data = data.copy()
    # If base_url is provided but api_url is not, use base_url as api_url
    if "base_url" in llm_data and "api_url" not in llm_data:
        llm_data["api_url"] = llm_data.pop("base_url")
    return LLMConfig(**llm_data)


def _load_embedding_config(data: dict) -> EmbeddingConfig:
    """Load embedding config with support for both api_url and base_url."""
    emb_data = data.copy()
    # If base_url is provided but api_url is not, use base_url as api_url
    if "base_url" in emb_data and "api_url" not in emb_data:
        emb_data["api_url"] = emb_data.pop("base_url")
    return EmbeddingConfig(**emb_data)


def load_config(config_path: Optional[Path] = None) -> Config:
    """Load configuration from TOML file.
    
    Supports both 'api_url' and 'base_url' field names in the [llm] and [embedding]
    sections for compatibility with Rust backend configuration.
    """
    if config_path is None:
        config_path = find_config_file()

    if config_path and config_path.exists():
        data = toml.load(config_path)
        return Config(
            llm=_load_llm_config(data.get("llm", {})),
            embedding=_load_embedding_config(data.get("embedding", {})),
            server=ServerConfig(**data.get("server", {})),
        )

    # Return default config
    return Config()


@lru_cache()
def get_config() -> Config:
    """Get cached configuration."""
    return load_config()
