"""Configuration loader for MemSt server."""

import os
from pathlib import Path
from typing import Optional
from functools import lru_cache

import toml
from pydantic import BaseModel


class LLMConfig(BaseModel):
    """LLM configuration."""
    type: str = "openai"
    api_url: str = "http://localhost:8080/v1"
    model: str = ""
    timeout: int = 60
    max_tokens: int = 8192
    temperature: float = 0.7
    api_key: str = ""


class EmbeddingConfig(BaseModel):
    """Embedding configuration."""
    type: str = "openai"
    api_url: str = "http://localhost:8081/v1/embeddings"
    model: str = "text-embedding-bge_m3"
    timeout: int = 30
    expected_dimension: int = 1024


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


def load_config(config_path: Optional[Path] = None) -> Config:
    """Load configuration from TOML file."""
    if config_path is None:
        config_path = find_config_file()

    if config_path and config_path.exists():
        data = toml.load(config_path)
        return Config(
            llm=LLMConfig(**data.get("llm", {})),
            embedding=EmbeddingConfig(**data.get("embedding", {})),
            server=ServerConfig(**data.get("server", {})),
        )

    # Return default config
    return Config()


@lru_cache()
def get_config() -> Config:
    """Get cached configuration."""
    return load_config()
