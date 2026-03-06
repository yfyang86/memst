# MemSt Server Configuration

This document describes the configuration options for the MemSt Python server.

## Configuration File Location

The server searches for `config.toml` in the following order:

1. Path specified by `MEMST_CONFIG_PATH` environment variable
2. `./config.toml` (current working directory)
3. `../config.toml` (parent directory)
4. `./memst-store/config.toml`
5. `~/.config/memst/config.toml`

## Configuration Format

The configuration file uses TOML format with the following sections:

### Server Section

```toml
[server]
host = "127.0.0.1"      # Server bind address
port = 8192             # Server port
debug = false           # Debug mode
store_path = "./memst-store"  # Data storage path
cors_origins = [        # Allowed CORS origins
    "http://localhost:3000",
    "http://127.0.0.1:3000"
]
```

### LLM Section

The LLM section supports both `api_url` and `base_url` field names for compatibility:

**Option 1: Using `api_url` (Python native)**
```toml
[llm]
type = "openai"
api_url = "http://localhost:8080/v1"
model = "gpt-4"
api_key = "your-api-key"
timeout = 60
max_tokens = 8192
temperature = 0.7
```

**Option 2: Using `base_url` (Rust backend compatible)**
```toml
[llm]
type = "openai"
base_url = "http://localhost:8080/v1"
model = "gpt-4"
api_key = "your-api-key"
timeout = 60
max_tokens = 8192
temperature = 0.7
```

Both formats are fully supported. Use whichever matches your setup:
- Use `api_url` if you're configuring the Python server independently
- Use `base_url` if you're sharing configuration with the Rust backend

### Embedding Section

Similarly, the embedding section supports both field names:

```toml
[embedding]
type = "openai"
base_url = "http://localhost:8081/v1/embeddings"
model = "text-embedding-ada-002"
timeout = 30
expected_dimension = 1024
```

## Complete Example

```toml
[server]
port = 8193
cors_origins = ["http://localhost:3000", "http://127.0.0.1:3000"]
store_path = "/tmp/data"

[llm]
type = "openai"
base_url = "https://your-endpoint.com/v1"
api_key = "YOUR_API_KEY_HERE"
model = "/workspace/models/openai-mirror/gpt-oss-120b/"

[embedding]
type = "openai"
base_url = "http://localhost:8081/v1/embeddings"
model = "text-embedding-bge_m3"
```

## Environment Variables

- `MEMST_CONFIG_PATH`: Override the config file path
- `MEMST_LLM_API_URL`: Override the LLM API URL
- `MEMST_LLM_MODEL`: Override the LLM model
- `MEMST_LLM_API_KEY`: Override the LLM API key

## API Compatibility

The Python server maintains full API compatibility with the Rust backend:

- Both use the same configuration file format
- Both support `base_url` field name for LLM configuration
- Both search for config in the same locations
- The Python server adds support for `api_url` as an alternative field name

This allows you to use a single `~/.config/memst/config.toml` file for both the Rust CLI tools and the Python web server.
