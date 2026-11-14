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
- **Knowledge Graph** for entity and relationship tracking, stored in embedded HelixDB with BM25 and vector entity search ([details](docs/graph-backend-helix.md))
- **KG decay** - Temporal relevance decay for entities/relationships
- **KG evolution** - Automatic entity merging and deprecation
- **KG extraction v2** - SQLite-backed, LLM-powered extraction with domain ontologies
- **Procedural memory (Skills)** for learned workflows

## What's New in v1.0

### Semantic Memory Architecture
- **Blake3 content-addressable storage** - 3× faster hashing than SHA-256
- **Write-Ahead Log (WAL)** - Crash recovery for durability
- **Memory lifecycle** - Automatic tier transitions with checkpoints
- **A-MEM style evolution** - Retroactive memory network updates
- **Semantic merge** - Fast-forward merges and conflict detection between memories (loading memories from commits for a full three-way merge is not implemented yet)

### Multi-Agent Support
- **Worktrees** - Isolated environments for concurrent agents
- **Agent registry** - Track agent identity and permissions
- **Cross-agent search** - Search across agent memory scopes

### Context Assembly
- **Token-budget-aware retrieval** - Respects LLM context limits
- **Multi-signal scoring** - Relevance, recency, importance, confidence
- **Progressive disclosure** - Selective memory inclusion

### MCP Integration
- **MCP tool definitions and request handling** for Claude Code, Cursor, and other MCP clients. The tool handlers are placeholders and are not yet connected to a store
- **Tools**: memory_read, memory_write, memory_search, skill_lookup, context_build

### KG Extraction v2 (New)
- **SQLite storage** for ontologies, jobs and results
- **Domain-specific ontologies**: 80+ intelligence domains
- **LLM-powered extraction**: Chain-of-thought entity/relationship extraction
- **Entity linking**: Automatic disambiguation and canonicalization

## Workspace Layout

### Core Libraries
- **memst-core**: Core library (store/search/vector/hybrid/context/memory)
- **memst-repo**: Git-alike MemRepo backend (WAL, merge, worktrees)
- **memst-sleep**: Async sleep-time consolidation pipeline
- **memst-extract-v2**: KG extraction with domain ontologies (SQLite storage)
- **memst-mcp**: MCP (Model Context Protocol) adapter
- **memst-cli**: `memst` CLI binary
- **memst-py**: Python extension module (`import memst`)
- **memst-lib**: Shared library/FFI glue
- **memst-graph-helix**: Knowledge-graph backend on embedded [HelixDB](https://github.com/yfyang86/helix-db) (graph + vector + BM25)
- **memst-embed**: Local CPU text embeddings (BGE-M3), no API key needed

### Applications (`apps/`, outside the Rust workspace)
- **apps/memst-server**: Python FastAPI backend server with nanobot agent integration
- **apps/memst-ui**: React/TypeScript web UI and optional Tauri desktop shell
- **apps/third/nanobot**: Nanobot AI agent framework (git submodule)

See [apps/README.md](apps/README.md). Nothing in the engine depends on `apps/`.

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

### Web server and UI

The FastAPI server, the React UI and the Tauri desktop shell live in
[`apps/`](apps/README.md) and are isolated from the Rust workspace. See
[apps/README.md](apps/README.md) for setup, and
[apps/memst-server-api.md](apps/memst-server-api.md) for the REST API.

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

The CLI includes the embedded HelixDB graph engine, which is built from source
(a few minutes on a clean build). For a smaller binary that only reads and
writes `graph.json`:

```bash
cargo install --path memst-cli --no-default-features
```

### Build Python bindings (local dev)

```bash
unset CONDA_PREFIX            # maturin refuses to run with both conda and a venv active
uv run maturin build --release -m memst-py/Cargo.toml
uv pip install target/wheels/memst_py-*.whl
python -c "import memst; print(memst.__version__)"
```

## Configuration

MemSt loads LLM + embedding settings from `config.toml` by default (and falls back to environment variables if no config is found).

- Example config: `config.example.toml`
- Override path explicitly: `MEMST_CONFIG_PATH=/path/to/config.toml`
- Discovery: searches the current directory and its parent directories for `config.toml` and `memst-store/config.toml`, then falls back to `~/.config/memst/config.toml`

**Server Configuration (apps/memst-server/config.toml):**

```toml
[server]
## Server port (default: 8192)
port = 8193

## Session data storage path
store_path = "/tmp/data"

## CORS origins (comma-separated list, or "*" for all)
cors_origins = ["http://localhost:3000", "http://127.0.0.1:3000"]

[llm]
type = "openai"
api_url = "http://localhost:8080/v1"
model = "gpt-4"
timeout = 60
max_tokens = 8192
temperature = 0.7
api_key = ""

[embedding]
type = "openai"
api_url = "http://localhost:8081/v1/embeddings"
model = "text-embedding-bge_m3"
timeout = 30
expected_dimension = 1024

[kg_extraction]
## Enable KG Extraction v2
enabled = true
## Database path (null/empty = in-memory)
# db_path = "./memst-store/kg.db"
## Default ontology ID
# default_ontology = "lingyuqingbao-lei-kejiqingbao-rengongzhineng"
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

# Python binding tests (using uv)
uv run pytest memst-py/tests -v

# Or with maturin develop
maturin develop --release && python -m pytest memst-py/tests -v
```

## Documentation

- User manual: `UserManual.md`
- Knowledge-graph backend (HelixDB): `docs/graph-backend-helix.md`
- Research context (agent memory, 2025–2026): `docs/research/`
- Latest project review: `CodeReview/CR-20261114.md`
- Roadmap and sprint status: `docs/ROADMAP.md`
- Web server and UI: `apps/README.md`

## Feature Roadmap

| Phase | Feature | Status |
|-------|---------|--------|
| P1-P7 | Core storage, search, semantic search, git objects | ✅ Complete |
| P8 | Data Model (Blake3, WAL, lifecycle, frontmatter) | ✅ Complete |
| P9 | Context Assembly (token-budget, multi-signal scoring) | ✅ Complete |
| P10 | Sleep-Time (async consolidation, A-MEM evolution) | ✅ Complete |
| P11 | Semantic Merge (fast-forward, conflict detection) | ⚠️ Partial: three-way merge does not load memories from commits |
| P12 | Skills (procedural memory) | ✅ Complete |
| P13 | Multi-Agent (worktrees, agent registry) | ✅ Complete |
| P14 | MCP Adapter | ⚠️ Partial: handlers are placeholders |
| P15 | Memory Evolution, KG decay | ✅ Complete |
| P16 | KG Extraction v2 (SQLite) | ✅ Complete |

## KG Extraction v2 Quick Start

### Rust

```rust
use memst_extract_v2::{ExtractionService, KgStorage};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create storage (SQLite by default)
    let storage = KgStorage::new_in_memory().await?;
    
    // Create extraction service
    let service = ExtractionService::new(storage).await?;
    
    // Extract entities from text
    let job = service.extract_entities(
        "doc-001",
        "OpenAI released GPT-4 Turbo in 2023.",
        "tech-ai"
    ).await?;
    
    println!("Extracted {} entities", job.entity_count);
    Ok(())
}
```

### Python

```python
import memst

# Create storage (in-memory for testing, or use file-based)
storage = memst.KgStorage.new_in_memory()

# Create extraction service
service = memst.ExtractionService(storage)

# Load ontologies from schema JSON
schema_json = '''[
    {
        "top_category": "领域情报类",
        "first_category": "科技情报", 
        "second_category": "人工智能",
        "chinese_name": "科技情报-人工智能",
        "english_name": "Tech Intelligence-AI",
        "overview": "监测AI技术发展"
    }
]'''

ontology_ids = service.load_ontologies(schema_json)

# Extract entities
job = service.extract_entities(
    doc_id="doc-001",
    text="OpenAI released GPT-4 Turbo in 2023. Sam Altman is the CEO.",
    ontology_id=ontology_ids[0]
)

print(f"Status: {job.status}")
print(f"Entities: {job.entity_count}")
print(f"Tokens: {job.tokens_used}")
```

### Storage

KG extraction stores ontologies, jobs and results in SQLite (the default
`sqlite` feature). There is no other backend: the `duckdb` feature that used to
be listed was a stub that never built, and has been removed.

## License

Apache License 2.0. See `LICENSE`.
