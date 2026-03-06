# MemSt

Hybrid, searchable session memory for LLM applications with **semantic memory architecture** (v1.0).

Author: Yifan Yang <yfyang.86@hotmail.com>

[![Rust](https://img.shields.io/badge/rust-1.93%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

**MemSt v1.0** is a comprehensive memory management system for LLM agents featuring:

- **Git-like version control** for memory with branching and merging
- **Tiered memory architecture** (Working → Short-term → Long-term → Archival)
- **Semantic search** with HNSW vector index and hybrid RRF fusion
- **Context assembly** with token-budget-aware retrieval
- **Sleep-time consolidation** for automatic memory maintenance
- **Multi-agent support** with isolated worktrees
- **MCP (Model Context Protocol)** adapter for Claude/Cursor integration
- **Knowledge Graph** for entity and relationship tracking
- **KG decay** - Temporal relevance decay for entities/relationships
- **KG evolution** - Automatic entity merging and deprecation
- **Procedural memory (Skills)** for learned workflows

## What's New in v1.0

### Semantic Memory Architecture
- **Blake3 content-addressable storage** - 3× faster hashing than SHA-256
- **Write-Ahead Log (WAL)** - Crash recovery for durability
- **Memory lifecycle** - Automatic tier transitions with checkpoints
- **A-MEM style evolution** - Retroactive memory network updates
- **Three-way semantic merge** - Conflict detection and resolution

### Multi-Agent Support
- **Worktrees** - Isolated environments for concurrent agents
- **Agent registry** - Track agent identity and permissions
- **Cross-agent search** - Search across agent memory scopes

### Context Assembly
- **Token-budget-aware retrieval** - Respects LLM context limits
- **Multi-signal scoring** - Relevance, recency, importance, confidence
- **Progressive disclosure** - Selective memory inclusion

### MCP Integration
- **Full MCP protocol support** for Claude Code, Cursor, and other MCP clients
- **Tools**: memory_read, memory_write, memory_search, skill_lookup, context_build

## Workspace Layout

- **memst-core**: Core library (store/search/vector/hybrid/context/memory)
- **memst-repo**: Git-alike MemRepo backend (WAL, merge, worktrees)
- **memst-sleep**: Async sleep-time consolidation pipeline
- **memst-mcp**: MCP (Model Context Protocol) adapter
- **memst-cli**: `memst` CLI binary
- **memst-py**: Python extension module (`import memst`)
- **memst-lib**: Shared library/FFI glue

## Quick Start

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

# Build context within token budget (v1.0)
memst context build <session-uuid> --query "help with async" --budget 4000
```

### Python

```python
import memst

store = memst.SessionStore("./data")
session = store.create_session("My Chat", "gpt-4")

store.add_message(session.id, memst.Role.User, "Hello")
store.add_message(session.id, memst.Role.Assistant, "Hi!")

# Context assembly with token budget (v1.0)
context = store.build_context(
    query="help with Rust async",
    token_budget=4000,
    include_memories=True
)

messages = store.get_session_messages(session.id)
print(len(messages))
```

### Web Server (FastAPI + React UI)

![ui-chat](./assets/figures/memst-ui-chat.png)

```bash
# Terminal 1: Start the backend server
cd memst-server
PYTHONPATH=src .venv/bin/python -m memst_server.main

# Terminal 2: Start the frontend (from memst-ui directory)
cd memst-ui

# Configure the frontend API URL (optional - see below)
# By default, frontend connects to http://127.0.0.1:8193
cp src/config.ts src/config.local.ts
# Edit config.local.ts to change the API URL if needed

npm run dev
```

The frontend will be available at `http://localhost:3000` and will communicate with the backend at `http://127.0.0.1:8193`.

**Frontend Configuration:**

The frontend reads the backend API URL from environment variables. Create a `.env.local` file in the `memst-ui` directory to override defaults:

```bash
cd memst-ui
echo 'VITE_API_HOST=127.0.0.1' > .env.local
echo 'VITE_API_PORT=8193' >> .env.local
echo 'VITE_DEV_PORT=3000' >> .env.local
```

Example `.env.local` for a remote backend:
```
VITE_API_HOST=192.168.1.100
VITE_API_PORT=8193
VITE_DEV_PORT=3001
```

Available environment variables:
- `VITE_API_HOST` - Backend API host (default: `127.0.0.1`)
- `VITE_API_PORT` - Backend API port (default: `8193`)
- `VITE_DEV_PORT` - Frontend dev server port (default: `3000`)

**Note:** Ensure `cors_origins` is set in server's `config.toml` to allow frontend access.

### API Documentation

For detailed API documentation, see [memst-server-api.md](memst-server-api.md).

### Rust

```rust
use memst_core::store::SessionStore;
use memst_core::types::{Role, SessionMetadata};

// Basic store operations
let store = SessionStore::init("./data")?;
let session_id = store.create_session(SessionMetadata::new("My Chat", "gpt-4"))?;
store.append_message(session_id, memst_core::types::Message::new(Role::User, "Hello".into()))?;

// Context assembly with token budget (v1.0)
use memst_core::context::{ContextAssemblyConfig, ContextAssembler};
use memst_core::memory::SimpleTokenCounter;

let config = ContextAssemblyConfig {
    token_budget: 4000,
    reserved_for_response: 1000,
    ..Default::default()
};
let counter = SimpleTokenCounter;
let assembler = ContextAssembler::new(config, &counter);
let assembly = assembler.build_context(
    "help with async",
    &memories,
    &[],
    &[],
    &[],
    "You are a helpful assistant."
)?;
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

The Python bindings require a working Python environment. There are some known issues with certain Python versions.

**Setup using uv (recommended):**

```bash
# Create a fresh virtual environment with Python 3.12 (avoids Python 3.13 issues)
cd memst-server
uv venv --python 3.12
uv pip install fastapi uvicorn duckdb python-dotenv pydantic pydantic-settings httpx toml
uv pip install -e ../third/nanobot

# Build and install memst-py using maturin
cd ../memst-py
VIRTUAL_ENV=../memst-server/.venv ../memst-server/.venv/bin/maturin develop

# Verify
cd ../memst-server
PYTHONPATH=src .venv/bin/python -c "import memst; print(memst.__version__)"
```

**Running the server:**

```bash
# Option 1: Using the management script (recommended)
./server.sh --start      # Start server
./server.sh --stop       # Stop server
./server.sh --restart    # Restart server
./server.sh --status     # Check status
./server.sh --check      # Validate configuration
./server.sh --maintain   # Run maintenance checks (config validation, status, logs)
./server.sh --logs       # View server logs

# Option 2: Manual start
cd memst-server

# Set PYTHONPATH to include the src directory
PYTHONPATH=src .venv/bin/python -m memst_server.main
```

**Configuration:**

Create or edit `config.toml` in the memst-server directory:

```toml
[server]
port = 8193
store_path = "/tmp/data"
# Add CORS origins for frontend access
cors_origins = ["http://localhost:3000", "http://127.0.0.1:3000"]
```

**Troubleshooting:**

- If you get `SIGABRT` / `Library not loaded: libpython3.13.dylib`, recreate the venv with Python 3.12: `uv venv --python 3.12`
- If you get `ModuleNotFoundError: No module named 'memst_server'`, ensure `PYTHONPATH=src` is set
- If frontend CORS errors occur, add your frontend origin to `cors_origins` in config.toml
- If frontend can't connect to backend, check the API URL in `memst-ui/src/config.local.ts`

## Configuration

MemSt loads LLM + embedding settings from `config.toml` by default (and falls back to environment variables if no config is found).

- Example config: `config.example.toml`
- Override path explicitly: `MEMST_CONFIG_PATH=/path/to/config.toml`
- Discovery: searches the current directory and its parent directories for `config.toml` and `memst-store/config.toml`, then falls back to `~/.config/memst/config.toml`

**Server Configuration (memst-server/config.toml):**

```toml
[server]
## Server port (default: 8192)
port = 8193

## Session data storage path
store_path = "/tmp/data"

## CORS origins (comma-separated list, or "*" for all)
## Required for frontend access
cors_origins = ["http://localhost:3000", "http://127.0.0.1:3000"]
```

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

# Run UAT (User Acceptance Tests)
cargo test --test uat_phase8_data_model
cargo test --test uat_phase9_context_assembly
cargo test --test uat_phase10_sleep_time
cargo test --test uat_phase11_semantic_merge
cargo test --test uat_phase12_skills
cargo test --test uat_phase13_multi_agent
cargo test --test uat_phase14_mcp
cargo test --test end_to_end_simulation

# Enable network integration tests (LLM/embedding)
MEMST_RUN_INTEGRATION_TESTS=1 cargo test --workspace

# Python binding tests
python -m pytest memst-py/tests
```

## Documentation

- User manual: `UserManual.md`
- Architecture PRD: `SemanticTree/MemSt-PRD-v1.0.md`

## Feature Roadmap

| Phase | Feature | Status |
|-------|---------|--------|
| P1-P7 | Core storage, search, semantic search, git objects | ✅ Complete |
| P8 | Data Model (Blake3, WAL, lifecycle, frontmatter) | ✅ Complete |
| P9 | Context Assembly (token-budget, multi-signal scoring) | ✅ Complete |
| P10 | Sleep-Time (async consolidation, A-MEM evolution) | ✅ Complete |
| P11 | Semantic Merge (3-way merge, conflict detection) | ✅ Complete |
| P12 | Skills (procedural memory) | ✅ Complete |
| P13 | Multi-Agent (worktrees, agent registry) | ✅ Complete |
| P14 | MCP Adapter | ✅ Complete |
| P15 | Memory Evolution, KG decay | ✅ Complete |

## License

Apache License 2.0. See `LICENSE`.
