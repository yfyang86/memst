"""Tests for config compatibility between api_url and base_url, and KG extraction config."""

import tempfile
import os
from pathlib import Path

import sys
sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

from memst_server.config import (
    load_config, LLMConfig, EmbeddingConfig, 
    KGExtractionConfig, ServerConfig, Config
)


# =============================================================================
# LLM Config Tests
# =============================================================================

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


def test_llm_config_defaults():
    """Test LLMConfig default values."""
    config = LLMConfig()
    assert config.type == "openai"
    assert config.model == ""
    assert config.timeout == 60
    assert config.max_tokens == 8192
    assert config.temperature == 0.7
    assert config.api_key == ""


# =============================================================================
# Embedding Config Tests
# =============================================================================

def test_embedding_config_api_url():
    """Test EmbeddingConfig with api_url field."""
    config = EmbeddingConfig(api_url="http://test1:8081/v1/embeddings")
    assert config.get_api_url() == "http://test1:8081/v1/embeddings"


def test_embedding_config_base_url():
    """Test EmbeddingConfig with base_url field."""
    config = EmbeddingConfig(base_url="http://test2:8081/v1/embeddings")
    assert config.get_api_url() == "http://test2:8081/v1/embeddings"


def test_embedding_config_defaults():
    """Test EmbeddingConfig default values."""
    config = EmbeddingConfig()
    assert config.type == "openai"
    assert config.model == "text-embedding-bge_m3"
    assert config.timeout == 30
    assert config.expected_dimension == 1024


# =============================================================================
# KG Extraction Config Tests
# =============================================================================

def test_kg_extraction_config_defaults():
    """Test KGExtractionConfig default values."""
    config = KGExtractionConfig()
    assert config.enabled is True
    assert config.db_path is None
    assert config.default_ontology is None


def test_kg_extraction_config_custom():
    """Test KGExtractionConfig with custom values."""
    config = KGExtractionConfig(
        enabled=False,
        db_path="./test-kg.db",
        default_ontology="test-ontology"
    )
    assert config.enabled is False
    assert config.db_path == "./test-kg.db"
    assert config.default_ontology == "test-ontology"


# =============================================================================
# Server Config Tests
# =============================================================================

def test_server_config_defaults():
    """Test ServerConfig default values."""
    config = ServerConfig()
    assert config.host == "127.0.0.1"
    assert config.port == 8192
    assert config.debug is False
    assert config.store_path == "./memst-store"
    assert config.cors_origins == []


def test_server_config_custom():
    """Test ServerConfig with custom values."""
    config = ServerConfig(
        host="0.0.0.0",
        port=9000,
        debug=True,
        store_path="/tmp/test",
        cors_origins=["http://localhost:3000"]
    )
    assert config.host == "0.0.0.0"
    assert config.port == 9000
    assert config.debug is True
    assert config.store_path == "/tmp/test"
    assert config.cors_origins == ["http://localhost:3000"]


# =============================================================================
# Load Config Tests
# =============================================================================

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


def test_load_config_full():
    """Test loading full config with all sections."""
    with tempfile.NamedTemporaryFile(mode='w', suffix='.toml', delete=False) as f:
        f.write("""
[server]
host = "0.0.0.0"
port = 9000
debug = true
store_path = "/tmp/test-data"
cors_origins = ["http://localhost:3000"]

[llm]
type = "openai"
api_url = "http://llm-test:8080/v1"
model = "gpt-4"
timeout = 120
max_tokens = 4096
temperature = 0.5
api_key = "test-api-key"

[embedding]
type = "lmstudio"
api_url = "http://embed-test:8081/v1/embeddings"
model = "text-embedding-bge_m3"
timeout = 60
expected_dimension = 1024

[kg_extraction]
enabled = true
db_path = "./test-kg.db"
default_ontology = "test-ontology-1"
""")
        temp_path = f.name
    
    try:
        config = load_config(Path(temp_path))
        
        # Server
        assert config.server.host == "0.0.0.0"
        assert config.server.port == 9000
        assert config.server.debug is True
        assert config.server.store_path == "/tmp/test-data"
        assert config.server.cors_origins == ["http://localhost:3000"]
        
        # LLM
        assert config.llm.type == "openai"
        assert config.llm.get_api_url() == "http://llm-test:8080/v1"
        assert config.llm.model == "gpt-4"
        assert config.llm.timeout == 120
        assert config.llm.max_tokens == 4096
        assert config.llm.temperature == 0.5
        assert config.llm.api_key == "test-api-key"
        
        # Embedding
        assert config.embedding.type == "lmstudio"
        assert config.embedding.get_api_url() == "http://embed-test:8081/v1/embeddings"
        assert config.embedding.model == "text-embedding-bge_m3"
        assert config.embedding.timeout == 60
        assert config.embedding.expected_dimension == 1024
        
        # KG Extraction
        assert config.kg_extraction.enabled is True
        assert config.kg_extraction.db_path == "./test-kg.db"
        assert config.kg_extraction.default_ontology == "test-ontology-1"
        
    finally:
        os.unlink(temp_path)


def test_load_config_kg_extraction_defaults():
    """Test that KG extraction config has defaults when not specified."""
    with tempfile.NamedTemporaryFile(mode='w', suffix='.toml', delete=False) as f:
        f.write("""
[llm]
api_url = "http://test:8080/v1"
model = "gpt-4"
""")
        temp_path = f.name
    
    try:
        config = load_config(Path(temp_path))
        # KG extraction should have defaults
        assert config.kg_extraction.enabled is True
        assert config.kg_extraction.db_path is None
        assert config.kg_extraction.default_ontology is None
    finally:
        os.unlink(temp_path)


def test_load_config_empty_file():
    """Test loading empty config file returns defaults."""
    with tempfile.NamedTemporaryFile(mode='w', suffix='.toml', delete=False) as f:
        f.write("")
        temp_path = f.name
    
    try:
        config = load_config(Path(temp_path))
        assert config.llm is not None
        assert config.embedding is not None
        assert config.server is not None
        assert config.kg_extraction is not None
        # Check defaults
        assert config.server.port == 8192
        assert config.kg_extraction.enabled is True
    finally:
        os.unlink(temp_path)


def test_load_config_no_file():
    """Test loading when no config file exists returns defaults."""
    config = load_config(Path("/nonexistent/config.toml"))
    # When no config file exists, llm and embedding are None (user must configure)
    assert config.llm is None
    assert config.embedding is None
    # But server and kg_extraction have defaults
    assert config.server is not None
    assert config.kg_extraction is not None
    assert config.server.port == 8192
    assert config.kg_extraction.enabled is True


# =============================================================================
# Config Model Tests
# =============================================================================

def test_config_model_creation():
    """Test creating Config model directly."""
    config = Config(
        llm=LLMConfig(api_url="http://test:8080/v1", model="gpt-4"),
        embedding=EmbeddingConfig(api_url="http://test:8081/v1/embeddings", model="bge-m3"),
        server=ServerConfig(port=9000),
        kg_extraction=KGExtractionConfig(enabled=False, db_path="./kg.db")
    )
    
    assert config.llm.get_api_url() == "http://test:8080/v1"
    assert config.llm.model == "gpt-4"
    assert config.embedding.get_api_url() == "http://test:8081/v1/embeddings"
    assert config.embedding.model == "bge-m3"
    assert config.server.port == 9000
    assert config.kg_extraction.enabled is False
    assert config.kg_extraction.db_path == "./kg.db"


if __name__ == "__main__":
    print("Running config compatibility tests...")
    
    # LLM tests
    test_llm_config_api_url()
    print("✓ test_llm_config_api_url")
    
    test_llm_config_base_url()
    print("✓ test_llm_config_base_url")
    
    test_llm_config_both_urls()
    print("✓ test_llm_config_both_urls")
    
    test_llm_config_defaults()
    print("✓ test_llm_config_defaults")
    
    # Embedding tests
    test_embedding_config_api_url()
    print("✓ test_embedding_config_api_url")
    
    test_embedding_config_base_url()
    print("✓ test_embedding_config_base_url")
    
    test_embedding_config_defaults()
    print("✓ test_embedding_config_defaults")
    
    # KG Extraction tests
    test_kg_extraction_config_defaults()
    print("✓ test_kg_extraction_config_defaults")
    
    test_kg_extraction_config_custom()
    print("✓ test_kg_extraction_config_custom")
    
    # Server tests
    test_server_config_defaults()
    print("✓ test_server_config_defaults")
    
    test_server_config_custom()
    print("✓ test_server_config_custom")
    
    # Load config tests
    test_load_config_with_api_url()
    print("✓ test_load_config_with_api_url")
    
    test_load_config_with_base_url()
    print("✓ test_load_config_with_base_url")
    
    test_load_config_real_world()
    print("✓ test_load_config_real_world")
    
    test_load_config_full()
    print("✓ test_load_config_full")
    
    test_load_config_kg_extraction_defaults()
    print("✓ test_load_config_kg_extraction_defaults")
    
    test_load_config_empty_file()
    print("✓ test_load_config_empty_file")
    
    test_load_config_no_file()
    print("✓ test_load_config_no_file")
    
    # Model tests
    test_config_model_creation()
    print("✓ test_config_model_creation")
    
    print("\n✅ All config compatibility tests passed!")
