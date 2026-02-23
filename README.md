# MemSt

Hybrid, searchable session memory for LLM applications.

Author: Yifan Yang <yfyang.86@hotmail.com>

[![Rust](https://img.shields.io/badge/rust-1.93%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

MemSt is a Rust workspace that provides:

- A persistent session store for chat history and tiered memories
- Keyword search (native / Tantivy)
- Semantic search (HNSW) and hybrid search utilities
- Python bindings via PyO3 (for embedding into existing Python apps)

## Workspace layout

- memst-core: core library (store/search/vector/hybrid)
- memst-cli: `memst` CLI binary
- memst-py: Python extension module (`import memst`)
- memst-lib: shared library/FFI glue

## Quick start

### CLI

```bash
# Initialize a store
memst init ./data

# Create a session
memst --store ./data session new --name "My Chat" --model "gpt-4"

# List sessions
memst --store ./data session list

# Add messages
memst --store ./data message add <session-uuid> --role user --content "Hello"
memst --store ./data message add <session-uuid> --role assistant --content "Hi!"

# Search
memst --store ./data search "hello" --doc-type message --limit 20
```

### Python

```python
import memst

store = memst.SessionStore("./data")
session = store.create_session("My Chat", "gpt-4")

store.add_message(session.id, memst.Role.User, "Hello")
store.add_message(session.id, memst.Role.Assistant, "Hi!")

messages = store.get_session_messages(session.id)
print(len(messages))
```

### Rust

```rust
use memst_core::store::SessionStore;
use memst_core::types::{Role, SessionMetadata};

let store = SessionStore::init("./data")?;
let session_id = store.create_session(SessionMetadata::new("My Chat", "gpt-4"))?;
store.append_message(session_id, memst_core::types::Message::new(Role::User, "Hello".into()))?;
```

## Installation

### Build from source (workspace)

```bash
cargo build --release
```

### Install CLI

```bash
cargo install --path memst-cli
memst --help
```

### Build Python bindings (local dev)

```bash
cd memst-py
maturin develop --release
python -c "import memst; print(memst.__version__)"
```

## Configuration

MemSt loads LLM + embedding settings from `config.toml` by default (and falls back to environment variables if no config is found).

- Example config: `config.example.toml`
- Override path explicitly: `MEMST_CONFIG_PATH=/path/to/config.toml`
- Discovery: searches the current directory and its parent directories for `config.toml` and `memst-store/config.toml`, then falls back to `~/.config/memst/config.toml`

Environment variables (fallbacks):

```bash
export MEMST_LLM_API_URL="http://localhost:8080/v1"
export MEMST_LLM_MODEL="gpt-4"

export MEMST_EMBEDDING_API_URL="http://localhost:8081/v1"            # or .../v1/embeddings
export MEMST_EMBEDDING_MODEL="text-embedding-bge_m3"
```

Notes:

- `embedding.api_url` accepts either a base URL like `.../v1` or the full endpoint `.../v1/embeddings`.

## Testing

```bash
# Fast, offline-friendly unit tests
cargo test --workspace

# Enable network integration tests (LLM/embedding)
MEMST_RUN_INTEGRATION_TESTS=1 cargo test --workspace

# Python binding tests
python -m pytest memst-py/tests
```

## Documentation

- User manual: `UserManual.md`

## License

Apache License 2.0. See `LICENSE`.
