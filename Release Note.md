# MemSt Release Notes

## v0.1.0 - Initial Release Candidate (RC1)

*Release Date: 2026-03-02*

---

## Overview

MemSt v0.1.0 RC1 is the first release candidate for a hybrid, searchable session memory system designed for LLM applications. This release provides core functionality for persistent session storage, full-text search, and memory tier management with support for multiple interfaces (CLI, Python, REST API, Web UI).

---

## What's New in This Release

### Core Features

#### Session Management
- **Persistent Session Store**: Create and manage chat sessions with metadata (name, model, tags)
- **Message Storage**: Binary storage with bincode + zstd compression for efficient message persistence
- **Three-Tier Memory System**: Working, Short-Term, and Long-Term memory organization
- **Operations Logging**: Append-only trace log for auditing and debugging

#### Search Capabilities
- **Native Backend**: Custom Rust inverted index with BM25 scoring (k1=1.2, b=0.75)
- **Tantivy Backend**: Optional advanced full-text search with phrase search, fuzzy matching, and regex support
- **Semantic Search**: HNSW-based vector similarity search (requires embedding API configuration)
- **Hybrid Search**: Combines keyword and semantic search using Reciprocal Rank Fusion (RRF)

#### Storage Architecture
- **Content-Addressable Storage**: Git-like object primitives (Blob, Tree, Commit, Tag) at library level
- **Grep-Friendly Index Files**: Human-readable message.idx for easy debugging
- **Reference System**: Branch and tag operations with merge support

### Interfaces

#### CLI (`memst-cli`)
```
memst init ./data                    # Initialize store
memst session new --name "Chat"     # Create session
memst session list                   # List sessions
memst message add <id> --role user --content "Hello"  # Add message
memst search "query" --limit 20      # Full-text search
memst memory add <id> --tier working --content "Fact" # Add memory
memst trace <id>                     # View operation history
memst stats                          # Storage statistics
```

#### Python Bindings (`memst-py`)
```python
import memst

store = memst.SessionStore("./data")
session = store.create_session("My Chat", "gpt-4")
store.add_message(session.id, memst.Role.User, "Hello")
messages = store.get_session_messages(session.id)
results = store.search("hello", limit=10)
```

#### REST API (`memst-server`)
- Full CRUD for sessions and messages
- Streaming chat endpoints for LLM integration
- Search API with multiple backends
- Agent session support with autonomous capabilities

#### Web UI (`memst-ui`)
- React-based graphical interface
- Session management and chat view
- Knowledge graph visualization
- Real-time search with multiple modes (Text, Semantic, Regex, Hybrid)

---

## Architecture Highlights

### Directory Structure
```
store/
├── manifest.json              # Global session index
├── schema_version             # Format version
├── sessions/
│   └── {session_id}/
│       ├── metadata.json      # Session config
│       ├── messages.bin       # Compressed messages
│       ├── messages.idx       # Message index
│       └── operations.log     # Operations trace
├── search_index/
│   ├── native/                # Native inverted index
│   └── tantivy/              # Tantivy full-text index
└── memories/
    ├── working.bin            # Active memories
    ├── short.bin              # Short-term memories
    └── long.bin               # Long-term memories
```

### Supported Configurations
- **LLM Providers**: OpenAI, Claude, LM Studio, Ollama (OpenAI-compatible APIs)
- **Embedding Models**: Any OpenAI-compatible embedding endpoint (e.g., bge-m3)
- **Python Version**: 3.8+ (3.12 recommended)
- **Rust Version**: 1.93+ (1.88+ for Tantivy backend)

---

## Component Versions

| Component | Version | Description |
|-----------|---------|-------------|
| memst-core | 0.1.0 | Core Rust library |
| memst-cli | 0.1.0 | Command-line interface |
| memst-py | 0.1.0 | Python bindings (PyO3) |
| memst-lib | 0.1.0 | FFI/shared library |
| memst-server | - | FastAPI backend |
| memst-ui | - | React + TypeScript frontend |

### Key Dependencies
- **serde**: 1.0+ (serialization)
- **bincode**: 1.3 (binary encoding)
- **zstd**: 0.13 (compression)
- **tantivy**: 0.25 (optional, full-text search)
- **pyo3**: 0.22 (Python bindings)
- **reqwest**: 0.11 (HTTP client)

---

## Known Limitations

### Not Implemented in This Release
- **Git-backed version control**: Git-like primitives available at library level, but CLI commands for branch/merge not shipped
- **Search across sessions**: Basic multi-session search works; advanced cross-session analytics deferred
- **Message pagination**: Limited offset-based pagination
- **CRDT synchronization**: Multi-device sync deferred to future release
- **Packfiles & GC**: Storage optimization features deferred

### Known Issues
- Python 3.13 has compatibility issues; use Python 3.12 for Python bindings
- Large stores may experience slower search performance without Tantivy backend
- Semantic search requires external embedding API configuration

---

## Upgrading from Pre-RC Versions

If you were using a pre-release version:

1. **Backup your data**: Copy your store directory before upgrading
2. **Rebuild Python bindings**: `cd memst-py && maturin build --release`
3. **Update configuration**: Ensure `config.toml` follows the current format
4. **Test search**: Run a few searches to verify index compatibility

---

## Getting Started

### Quick Install
```bash
# Clone repository
git clone https://github.com/yfyang86/memst.git
cd memst

# Build workspace
cargo build --release

# Install CLI
cargo install --path memst-cli

# Initialize a store
memst init ./my-store
```

### Python Setup
```bash
cd memst-server
uv venv --python 3.12
uv pip install fastapi uvicorn duckdb python-dotenv pydantic pydantic-settings httpx toml
uv pip install -e ../third/nanobot

cd ../memst-py
VIRTUAL_ENV=../memst-server/.venv ../memst-server/.venv/bin/maturin develop
```

### Web UI Setup
```bash
# Backend
cd memst-server
./server.sh --start

# Frontend (separate terminal)
cd memst-ui
npm run dev
```

---

## Changelog (Since Initial Commit)

### Core Development
- `704501d` - DOC: add API document
- `21d804a` - Add: Server maintenance script
- `d88893a` - Add future work
- `f7d1502` - Bug fix: nanobot patch with environment fix
- `c502fda` - Search Skill example
- `7222d0e` - Frontend
- `6b072f8` - Memst: Python wrapper Backend
- `881551c` - Memst: Core RUST Backend

### Features Added
- Session-based message storage with binary compression
- Native and Tantivy full-text search backends
- HNSW vector index for semantic search
- Hybrid search with RRF fusion
- Three-tier memory system (Working/ShortTerm/LongTerm)
- Python bindings via PyO3
- FastAPI REST server
- React web UI with knowledge graph
- Git-like object primitives (Blob, Tree, Commit, Tag)
- Branch and merge operations
- Operation tracing and audit logs

---

## What's Next (Preview)

Future releases (v0.2.0+) will focus on:
- Full Git-like CLI with branch/merge commands
- Cross-session search analytics
- CRDT-based multi-device synchronization
- Packfiles and garbage collection for storage optimization
- Enhanced pagination and filtering
- Performance improvements for large-scale deployments

---

## Support & Feedback

- **Issues**: https://github.com/yfyang86/memst/issues
- **Discussions**: https://github.com/yfyang86/memst/discussions
- **License**: Apache 2.0

---

*Thank you for trying MemSt RC1! We welcome your feedback to help shape the 1.0 release.*
