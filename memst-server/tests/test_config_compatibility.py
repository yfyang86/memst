"""Tests for config compatibility between api_url and base_url."""

import tempfile
import os
from pathlib import Path

import sys
sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

from memst_server.config import load_config, LLMConfig, EmbeddingConfig


def test_llm_config_api_url():
    """Test LLMConfig with api_url field."""
    config = LLMConfig(api_url="http://test1:8080/v1")
    assert config.get_api_url() == "http://test1:8080/v1"


def test_llm_config_base_url():
    """Test LLMConfig with base_url field."""
    config = LLMConfig(base_url="http://test2:8080/v1")
    assert config.get_api_url() == "http://test2:8080/v1"


def test_llm_config_both_urls():
    """Test LLMConfig with both api_url and base_url - base_url takes precedence."""
    config = LLMConfig(api_url="http://api:8080/v1", base_url="http://base:8080/v1")
    # base_url takes precedence when explicitly set
    assert config.get_api_url() == "http://base:8080/v1"


def test_embedding_config_api_url():
    """Test EmbeddingConfig with api_url field."""
    config = EmbeddingConfig(api_url="http://test1:8081/v1/embeddings")
    assert config.get_api_url() == "http://test1:8081/v1/embeddings"


def test_embedding_config_base_url():
    """Test EmbeddingConfig with base_url field."""
    config = EmbeddingConfig(base_url="http://test2:8081/v1/embeddings")
    assert config.get_api_url() == "http://test2:8081/v1/embeddings"


def test_load_config_with_api_url():
    """Test loading config with api_url field."""
    with tempfile.NamedTemporaryFile(mode='w', suffix='.toml', delete=False) as f:
        f.write("""
[llm]
type = "openai"
api_url = "http://api-url-test:8080/v1"
model = "gpt-4"

[embedding]
type = "openai"
api_url = "http://api-url-test:8081/v1/embeddings"
model = "text-embedding-ada-002"
""")
        temp_path = f.name
    
    try:
        config = load_config(Path(temp_path))
        assert config.llm.get_api_url() == "http://api-url-test:8080/v1"
        assert config.embedding.get_api_url() == "http://api-url-test:8081/v1/embeddings"
    finally:
        os.unlink(temp_path)


def test_load_config_with_base_url():
    """Test loading config with base_url field (Rust backend format)."""
    with tempfile.NamedTemporaryFile(mode='w', suffix='.toml', delete=False) as f:
        f.write("""
[llm]
type = "openai"
base_url = "http://base-url-test:8080/v1"
api_key = "test-key"
model = "gpt-4"

[embedding]
type = "openai"
base_url = "http://base-url-test:8081/v1/embeddings"
model = "text-embedding-ada-002"
""")
        temp_path = f.name
    
    try:
        config = load_config(Path(temp_path))
        assert config.llm.get_api_url() == "http://base-url-test:8080/v1"
        assert config.embedding.get_api_url() == "http://base-url-test:8081/v1/embeddings"
    finally:
        os.unlink(temp_path)


def test_load_config_real_world():
    """Test with real-world config format matching user's config."""
    with tempfile.NamedTemporaryFile(mode='w', suffix='.toml', delete=False) as f:
        f.write("""
[server]
port = 8193
cors_origins = ["http://localhost:3000", "http://127.0.0.1:3000"]
store_path = "/tmp/data"

[llm]
base_url = "https://your-endpoint.com/v1"
api_key = "YOUR_API_KEY_HERE"
model = "/workspace/models/openai-mirror/gpt-oss-120b/"
""")
        temp_path = f.name
    
    try:
        config = load_config(Path(temp_path))
        assert config.llm.get_api_url() == "https://your-endpoint.com/v1"
        assert config.llm.api_key == "YOUR_API_KEY_HERE"
        assert config.llm.model == "/workspace/models/openai-mirror/gpt-oss-120b/"
        assert config.server.port == 8193
    finally:
        os.unlink(temp_path)


if __name__ == "__main__":
    print("Running config compatibility tests...")
    
    test_llm_config_api_url()
    print("✓ test_llm_config_api_url")
    
    test_llm_config_base_url()
    print("✓ test_llm_config_base_url")
    
    test_llm_config_both_urls()
    print("✓ test_llm_config_both_urls")
    
    test_embedding_config_api_url()
    print("✓ test_embedding_config_api_url")
    
    test_embedding_config_base_url()
    print("✓ test_embedding_config_base_url")
    
    test_load_config_with_api_url()
    print("✓ test_load_config_with_api_url")
    
    test_load_config_with_base_url()
    print("✓ test_load_config_with_base_url")
    
    test_load_config_real_world()
    print("✓ test_load_config_real_world")
    
    print("\n✅ All config compatibility tests passed!")
