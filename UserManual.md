# MemSt User Manual

MemSt is a hybrid storage system for LLM session memory with full-text search, operations logging, and memory tier management.

## Table of Contents

1. [Installation](#installation)
2. [Quick Start](#quick-start)
3. [Architecture](#architecture)
4. [CLI Commands](#cli-commands)
5. [Search Features](#search-features)
6. [Semantic & Hybrid Search](#semantic--hybrid-search-phase-12)
7. [Configuration](#configuration)
8. [Web GUI](#web-gui)
9. [Git-Like Architecture](#git-like-architecture-phase-8)
10. [Python Bindings](#python-bindings-phase-9)

---

## Installation

### Prerequisites

- Rust 1.93 (see rust-toolchain.toml)
- Cargo
- Python 3.8+ (for Python bindings)
- uv (recommended) or pip

### Install via uv (Recommended for Python)

```bash
# Install uv if not already installed
curl -LsSf https://astral.sh/uv/install.sh | sh

# Clone and build
git clone https://github.com/yfyang86/memst.git
cd memst

# Build and install Python package
cd memst-py
maturin build --release
uv pip install ../target/wheels/memst-*.whl

# Verify installation
memst --help
```

### Building

```bash
# Clone the repository
git clone https://github.com/yfyang86/memst.git
cd memst

# Build with both search backends (default)
cargo build --release

# Build with native backend only
cargo build --release --no-default-features --features native-backend

# Build with Tantivy backend only
cargo build --release --no-default-features --features tantivy-backend
```

### Installation

```bash
# Install to ~/.cargo/bin
cargo install --path memst-cli

# Or copy manually
cp target/release/memst ~/.local/bin/
```

---

## Quick Start

### Initialize a Store

```bash
memst init ./my-store
```

This creates:
```
my-store/
├── manifest.json              # Global config, session index
├── schema_version             # Format: "1.0.0"
├── sessions/                  # Session directories
├── search_index/              # Full-text search index
├── memories/                  # Memory tier storage
└── store.lock                 # Advisory lock file
```

### Create a Session

```bash
# Create a new session
memst session new --name "Rust Project" --model "gpt-4" --tags "rust,programming"

# List all sessions
memst session list

# Show session details
memst session show 550e8400-e5b2-4c8f-9f0a-1a2b3c4d5e6f
```

### Add Messages

```bash
# Add a user message
memst message add 550e8400-e5b2-4c8f-9f0a-1a2b3c4d5e6f --role user --content "Hello, I need help with Rust async"

# Add an assistant message
memst message add 550e8400-e5b2-4c8f-9f0a-1a2b3c4d5e6f --role assistant --content "I'd be happy to help! What specific async question do you have?"
```

### View Messages

```bash
# View all messages in a session
memst message list 550e8400-e5b2-4c8f-9f0a-1a2b3c4d5e6f

# View messages with pagination
memst message list 550e8400 --limit 50 --offset 0
```

---

## Architecture

### Directory Structure

```
store/
├── manifest.json              # Global session index (JSON, grep-friendly)
├── schema_version             # "1.0.0" (plain text)
├── sessions/
│   └── {session_id}/
│       ├── metadata.json      # Session config (JSON)
│       ├── messages.bin       # Bincode + zstd compressed messages
│       ├── messages.idx       # Message index (grep-friendly text)
│       └── operations.log     # Append-only operations log (JSON Lines)
├── search_index/
│   ├── native/                # Native Rust inverted index
│   └── tantivy/               # Tantivy full-text index
└── memories/
    ├── working.bin            # Active working memories
    ├── short.bin              # Short-term memories
    └── long.bin               # Long-term memories
```

### Data Storage Formats

#### Messages (Binary)
- Format: Bincode-serialized Message structs
- Compression: zstd
- Access: Random access via messages.idx

#### Index Files (Human-Readable)
```
# messages.idx format (grep-friendly)
# message_id byte_offset byte_length timestamp role
msg-001 0 256 2026-01-31T10:00:00Z user
msg-002 256 312 2026-01-31T10:01:00Z assistant
```

#### Operations Log (JSON Lines)
```json
{"id":"op-001","timestamp":"2026-01-31T10:00:00Z","type":"tool_call","input":{"name":"web_search"},"duration_ms":1250}
{"id":"op-002","timestamp":"2026-01-31T10:02:00Z","type":"thinking_step","input":{"step":1},"duration_ms":50}
```

---

## CLI Commands

### Global Options

```bash
--store PATH, -s # Store directory (default: ./data)
--help, -h       # Show help
--version        # Show version
```

### CLI Overview

The `memst` command is the Rust CLI binary (memst-cli) for common operations:

```bash
memst --help
memst init ./my-store                    # Initialize store
memst session new --name "Chat"          # Create session
memst session list                       # List sessions
memst message add <id> --role user --content "Hello"  # Add message
memst memory add <id> --tier working --content "Fact" # Add memory
memst search "query" --limit 20          # Search
memst stats                              # Show statistics
```

### Init

Initialize a new MemSt store.

```bash
memst init [PATH]

# Examples
memst init ./my-store           # Create store in my-store/
memst init                      # Create in ./data/
```

### Session

Session management commands.

```bash
# Create a new session
memst session new [OPTIONS]

Options:
  --name TEXT       Session name
  --model TEXT      LLM model name (e.g., "gpt-4")
  --tags TAGS       Comma-separated tags

# Examples
memst session new --name "Project Alpha" --model "claude-3"
memst session new --name "Debug Session" --tags "debug,rust" --model "gpt-4"

# List all sessions
memst session list

# Show session details
memst session show SESSION_ID

# Delete a session
memst session delete SESSION_ID
```

### Message

Message management within sessions.

```bash
# Add a message
memst message add SESSION_ID [OPTIONS]

Options:
  --role ROLE       Message role: user, assistant, system, tool
    --content TEXT    Message content (optional if --file is used)
    --file PATH       Read message content from stdin or file

# Examples
memst message add 550e8400 --role user --content "Hello world"
echo "Hi there!" | memst message add 550e8400 --role assistant --file -

# List messages
memst message list SESSION_ID [OPTIONS]

Options:
  --limit NUM       Maximum messages (default: 100)
  --offset NUM      Skip first N messages (default: 0)
```

### Trace

View operation history (from operations.log).

```bash
memst trace show SESSION_ID [OPTIONS]

Options:
    --op-type TYPE    Filter by operation type
  --limit NUM       Maximum operations (default: 100)
  --json            JSON output

# Examples
memst trace 550e8400                          # All operations
memst trace show 550e8400 --op-type tool_call  # Tool calls only
memst trace 550e8400 --json | jq '.'
```

### Memory

Memory tier management.

```bash
# Add a memory
memst memory add SESSION_ID [OPTIONS]

Options:
    --tier TIER       Memory tier: working, short, long
    --content TEXT    Memory content
    --tags TAGS       Comma-separated tags
    --confidence NUM  Confidence score (0.0-1.0)

# Examples
memst memory add 550e8400 --tier working --content "User prefers dark mode" --tags "preference,ui"
memst memory add 550e8400 --tier short --content "Project deadline: March 1" --tags "project"

# List memories in a tier
memst memory list SESSION_ID [--tier TIER]

# Retrieve relevant memories
memst memory retrieve SESSION_ID QUERY [--limit NUM]
```

### Search

Full-text search across sessions.

```bash
memst search QUERY [OPTIONS]

Options:
  --session ID      Filter by session ID
    --doc-type TYPE   Filter by document type: message, memory
  --limit NUM       Maximum results (default: 20)
  --backend BACKEND Search backend: native, tantivy (auto-detect)
  --json            JSON output

# Examples
memst search "rust async"                                    # Basic search
memst search "debug error" --session 550e8400               # Session filter
memst search "api" --doc-type message --limit 50            # Type filter
memst search "pattern" --backend tantivy                    # Force Tantivy
memst search "rust" --json | jq '.results[].snippet'       # JSON output
```

### Stats

Show storage statistics.

```bash
memst stats [--store PATH]

# Output example
Sessions: 5
Messages: 1,234
Operations: 5,678
Memories: 89
  - Working: 45
  - Short-term: 30
  - Long-term: 14
Search Index: 1,234 documents
Storage Size: 12.5 MB
```

---

## Search Features

### Backends

MemSt supports two search backends:

#### Native Backend (Default)
- No external dependencies
- Custom Rust inverted index
- BM25 scoring with k1=1.2, b=0.75
- Case-insensitive tokenization
- Bincode + zstd compression

#### Tantivy Backend (Optional)
- Advanced full-text search
- Phrase search: `"exact phrase"`
- Fuzzy matching: `~2` suffix for edit distance
- Regex queries: `/pattern/`
- Requires Rust 1.88+

```bash
# Use specific backend
memst search "query" --backend native
memst search "query" --backend tantivy
```

### Semantic & Hybrid Search (Phase 12)

MemSt supports semantic search using vector embeddings and hybrid search combining keyword and semantic results.

#### Semantic Search

Semantic search finds conceptually similar content using vector embeddings (cosine similarity).

**Configuration** (in config.toml):
```toml
[embedding]
api_url = "http://127.0.0.1:1378/v1/embeddings"
model = "text-embedding-bge_m3"
dimension = 1024
```

**How it works**:
1. Text content is converted to 1024-dimensional vectors using the embedding model
2. Queries are similarly converted to vectors
3. Similarity is computed using cosine similarity
4. Results are ranked by similarity score (0.0 to 1.0)

```bash
# Semantic search finds conceptually similar content
memst search "machine learning concepts" --semantic

# Hybrid search combines keyword + semantic (default)
memst search "python async programming" --hybrid

# Force keyword-only search
memst search "exact error message" --keyword
```

#### Hybrid Search

Hybrid search combines keyword (BM25) and semantic (cosine) search using Reciprocal Rank Fusion (RRF).

**Fusion Strategies**:
- **RRF (Default)**: Reciprocal Rank Fusion - combines rankings from both methods
- **Weighted**: Weighted sum of keyword and semantic scores
- **Interleave**: Round-robin interleaving of results

**Configuration**:
```toml
[search.hybrid]
keyword_weight = 0.5        # Weight for keyword search (0.0-1.0)
semantic_weight = 0.5       # Weight for semantic search (0.0-1.0)
fusion_strategy = "rrf"     # rrf, weighted, interleave
max_results = 20            # Maximum results to return
min_score = 0.1             # Minimum fusion score threshold
```

#### Query Router

The QueryRouter automatically selects the best search strategy based on query characteristics:

| Query Type | Strategy | Reason |
|------------|----------|--------|
| Short (< 3 words) | Keyword | Exact matches better |
| Contains quotes | Keyword | Phrase search |
| Long (> 5 words) | Semantic | Conceptual matching |
| Medium (3-5 words) | Hybrid | Balance of both |

```bash
# Query router automatically selects strategy
memst search "What are the best practices for Rust error handling?" --auto

# Explain routing decision
memst search "python web framework" --explain
```

#### HNSW Vector Index

The HNSW (Hierarchical Navigable Small World) algorithm provides fast approximate nearest neighbor search.

**Configuration**:
```toml
[vector]
m = 16                    # Number of neighbors per node
ef_construction = 200     # Search width during construction
ef_search = 100           # Search width during query
similarity_threshold = 0.5 # Minimum similarity to return
```

### Search Query Syntax

#### Basic Search
```bash
# Search for any term
memst search rust

# Search for multiple terms (AND)
memst search rust async debugging
```

#### Session Filter
```bash
# Search in specific session
memst search "error" --session 550e8400-e5b2-4c8f-9f0a-1a2b3c4d5e6f
```

#### Type Filter
```bash
# Messages only
memst search "debug" --type message

# Memories only
memst search "preference" --type memory
```

#### Combining Filters
```bash
# Session and type filter
memst search "api" --session 550e8400 --type message --limit 10
```

### Search Results

Search results include:
- **score**: BM25 relevance score (0.0 - 100.0)
- **snippet**: Text preview with matched terms
- **session_id**: Source session
- **doc_type**: message or memory
- **timestamp**: Document timestamp

```json
{
  "id": "msg-123",
  "session_id": "550e8400-e5b2-4c8f-9f0a-1a2b3c4d5e6f",
  "doc_type": "message",
  "score": 12.5,
  "snippet": "...debugging Rust async code requires...",
  "timestamp": "2026-01-31T10:00:00Z"
}
```

---

## Configuration

### Feature Flags

Edit `memst-core/Cargo.toml` to customize:

```toml
[features]
# Enable/disable backends
tantivy-backend = ["tantivy"]  # Set to disable Tantivy
native-backend = []            # Set to disable native backend

default = ["native-backend", "tantivy-backend"]
```

### Environment Variables

```bash
# Config file override
MEMST_CONFIG_PATH=/path/to/config.toml

# LLM configuration (fallbacks when no config.toml is found)
MEMST_LLM_API_URL=http://localhost:8080/v1
MEMST_LLM_MODEL=gpt-4

# Embedding configuration (fallbacks when no config.toml is found)
# api_url can be either a base URL (.../v1) or a full embeddings endpoint (.../v1/embeddings)
MEMST_EMBEDDING_API_URL=http://localhost:8081/v1
MEMST_EMBEDDING_MODEL=text-embedding-bge_m3

# Enable long-running / network integration tests
MEMST_RUN_INTEGRATION_TESTS=1
```

### Tantivy Configuration

When using Tantivy backend, configure in code:

```rust
use memst_core::search::TantivySearchConfig;

let config = TantivySearchConfig {
    k1: 1.2,           // BM25 term frequency saturation
    b: 0.75,           // BM25 length normalization
    enable_phrase_search: true,
    enable_fuzzy: true,
    enable_regex: true,
    snippet_length: 100,
    ..Default::default()
};
```

### Config.toml Support

MemSt supports loading LLM and embedding settings from a `config.toml` file. The configuration file is searched in the following order:

1. `MEMST_CONFIG_PATH` (if set)
2. Search the current directory and parent directories for `config.toml`
3. Search the current directory and parent directories for `memst-store/config.toml`
4. `~/.config/memst/config.toml` (user config directory)

#### Configuration File Format

Create a `config.toml` file with your LLM and embedding settings:

```toml
# MemSt Configuration File
# Copy this to config.toml in your project root or memst-store directory

[llm]
## LLM provider type: openai, claude, lmstudio, ollama
type = "openai"
## API endpoint URL (OpenAI-compatible)
## This is a [VLLM] example with GPT-OSS
api_url = "http://127.0.0.1:1378/v1"
## Model name or path
model = "/workspace/models/openai-mirror/gpt-oss-120b/"
## Request timeout in seconds
timeout = 60
## Maximum tokens to generate
max_tokens = 8192
## Temperature (0.0-2.0)
temperature = 0.7
## API key (can be empty/null for local LLMs)
api_key = ""

[embedding]
## Embedding provider type: openai, claude, lmstudio, ollama
type = "lmstudio"
## API endpoint URL
## This is an [LM studio] example:
api_url = "http://127.0.0.1:1378/v1/embeddings"
## Model name
model = "text-embedding-bge_m3"
## Request timeout in seconds
timeout = 30
## Expected embedding dimension (for validation, optional)
expected_dimension = 1024

[server]
## Session data storage path
store_path = "/tmp/data"

```

#### Loading Configuration in Rust

```rust
use memst_core::config::MemStConfig;

let config = MemStConfig::load_from_file(&path)?;

// Convert to LLM config
let llm_config = config.to_llm_config();

// Convert to embedding config
let embedding_config = config.to_embedding_config();
```

#### Running Tests with config.toml (LLM + Embedding)

By default, `cargo test --workspace` only runs offline/unit tests.

Network integration tests (LLM calls + embedding calls) are gated behind `MEMST_RUN_INTEGRATION_TESTS=1`.

To ensure the test binaries always find your configuration regardless of their working directory, prefer setting `MEMST_CONFIG_PATH` to an absolute (or `$PWD`-prefixed) path.

```bash
# Option A: copy the template
cp config.example.toml config.toml

# Run all tests (offline + integration)
MEMST_RUN_INTEGRATION_TESTS=1 \
MEMST_CONFIG_PATH="$PWD/config.toml" \
cargo test --workspace -- --nocapture
```

```bash
# Option B: point directly at the example file
MEMST_RUN_INTEGRATION_TESTS=1 \
MEMST_CONFIG_PATH="$PWD/config.example.toml" \
cargo test --workspace -- --nocapture
```

Notes:

- For embeddings, `api_url` may be either a base URL like `http://host:port/v1` or the full embeddings endpoint like `http://host:port/v1/embeddings`.
- If you don't set `MEMST_CONFIG_PATH`, MemSt searches `config.toml` and `memst-store/config.toml` by walking up parent directories from the current working directory.

---

## Web GUI

MemSt includes a web-based graphical interface built with React and TypeScript for easy session management, chat, and search operations.

### Setup and Installation

#### 1. Backend Server Setup

```bash
# Navigate to the server directory
cd memst-server

# Create a virtual environment with Python 3.12 (recommended)
uv venv --python 3.12

# Install dependencies
uv pip install fastapi uvicorn duckdb python-dotenv pydantic pydantic-settings httpx toml

# Install nanobot (local dependency)
uv pip install -e ../third/nanobot

# Install maturin for building Python bindings
uv pip install maturin

# Build and install memst-py module
cd ../memst-py
VIRTUAL_ENV=../memst-server/.venv ../memst-server/.venv/bin/maturin develop

# Return to server directory
cd ../memst-server
```

#### 2. Configure CORS

Edit `memst-server/config.toml` to add your frontend origin:

```toml
[server]
## Server port
port = 8193
## Session data storage path
store_path = "/tmp/data"
## CORS origins (comma-separated list of allowed origins, or "*" for all)
cors_origins = ["http://localhost:3000", "http://127.0.0.1:3000"]
```

#### 3. Start the Backend

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
PYTHONPATH=src .venv/bin/python -m memst_server.main

# Or with custom config
MEMST_CONFIG_PATH=/path/to/config.toml PYTHONPATH=src .venv/bin/python -m memst_server.main
```

#### 4. Start the Frontend

```bash
# In a new terminal
cd memst-ui

# Configure the API backend URL (optional)
# By default, frontend connects to http://127.0.0.1:8193
# Create .env.local to override defaults:

echo 'VITE_API_HOST=127.0.0.1' > .env.local
echo 'VITE_API_PORT=8193' >> .env.local

# For a different backend server:
# echo 'VITE_API_HOST=192.168.1.100' > .env.local
# echo 'VITE_API_PORT=8193' >> .env.local
# echo 'VITE_DEV_PORT=3001' >> .env.local

# Start the development server
npm run dev
```

The web UI will be available at `http://localhost:3000` and will communicate with the API at `http://127.0.0.1:8193`.

**Frontend Configuration:**

Create `.env.local` in the `memst-ui` directory to override default settings:

```
# .env.local in memst-ui directory
VITE_API_HOST=127.0.0.1
VITE_API_PORT=8193
VITE_DEV_PORT=3000
```

Available environment variables:
- `VITE_API_HOST` - Backend API host (default: `127.0.0.1`)
- `VITE_API_PORT` - Backend API port (default: `8193`)
- `VITE_DEV_PORT` - Frontend dev server port (default: `3000`)

Note: `.env.local` is automatically ignored by git.

### Troubleshooting

#### Common Issues

1. **ModuleNotFoundError: No module named 'memst_server'**
   - Ensure `PYTHONPATH=src` is set before running the server

2. **CORS Policy Error**
   - Add your frontend origin to `cors_origins` in `config.toml`
   - Restart the server after modifying config

3. **Frontend Can't Connect to Backend**
   - Check the API URL in `memst-ui/src/config.local.ts`
   - Make sure the backend server is running
   - Ensure CORS is configured on the server

4. **SIGABRT / Library not loaded: libpython3.13.dylib**
   - Recreate the venv with Python 3.12: `uv venv --python 3.12`
   - Python 3.13 has known compatibility issues with some packages

4. **memst module not available**
   - Ensure memst-py is installed: `VIRTUAL_ENV=../memst-server/.venv ../memst-server/.venv/bin/maturin develop`

### Web GUI Features

#### Sidebar Navigation

The sidebar provides access to:

| Tab | Description |
|-----|-------------|
| **Sessions** | Active chat sessions with quick access |
| **Resources** | Recent files and documents |
| **History** | Chronological session history grouped by date |

**Session Management:**
- Click a session to load it in the chat view
- New sessions can be created via the "+ New Session" button
- Delete sessions with the ellipsis menu
- Sessions are grouped: Today, Yesterday, This Week, This Month, Earlier

#### Chat Interface

The main chat area supports:

- **Streaming Responses**: Real-time message streaming from LLM
- **Markdown Rendering**: Code blocks, lists, formatting with `react-markdown`
- **Message Actions**: Copy messages to clipboard
- **Auto-scroll**: Automatic scroll to new messages
- **Input History**: Previous prompts available via arrow keys

**Session Types:**
| Type | Icon | Use Case |
|------|------|----------|
| `chat` | Comment | General conversation |
| `task` | Tasks | Task-oriented workflows |
| `search` | Search | Information retrieval |
| `recommend` | Lightbulb | Recommendations |

#### Search Panel

The search panel offers four search modes:

| Mode | Description |
|------|-------------|
| **Text** | Keyword matching across all messages |
| **Semantic** | Vector-based similarity search |
| **Regex** | Regular expression pattern matching |
| **Hybrid** | Combined text + semantic (RRF fusion) |

**Search Results Display:**
- Session ID and document type
- Relevance score
- Text snippet with highlighted matches
- Click result to navigate to source message

#### Knowledge Graph Visualization

MemSt includes an interactive knowledge graph (D3.js powered):

- **Nodes**: Entities extracted from conversations
- **Edges**: Relationships between entities
- **Interactions**: Zoom, pan, and drag support
- **Filtering**: Filter by entity type or relationship strength

**Node Types:**
- `concept`: Abstract ideas
- `entity`: Named entities (people, places, things)
- `action`: Actions or operations
- `memory`: Stored memories

### API Integration

The web GUI connects to MemSt via REST API:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/sessions` | GET | List all sessions |
| `/api/v1/sessions` | POST | Create new session |
| `/api/v1/sessions/{id}` | GET | Get session details |
| `/api/v1/sessions/{id}/messages` | GET | Get session messages |
| `/api/v1/sessions/{id}/messages` | POST | Add message |
| `/api/v1/sessions/{id}/chat` | POST | Chat with streaming |
| `/api/v1/sessions/{id}/memory` | GET | Get memories by tier |
| `/api/v1/sessions/{id}/knowledge-graph` | GET | Get knowledge graph |
| `/api/v1/search` | GET | Search all sessions |
| `/api/v1/settings` | GET/PUT | Settings management |
| `/api/v1/users` | GET/POST | User management |
| `/api/v1/agent/sessions` | GET/POST | Agent session management |
| `/api/v1/agent/chat/{id}` | POST | Agent chat (streaming/non-streaming) |

### Agent Sessions

MemSt supports agent-based conversations with autonomous capabilities:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/agent/sessions` | POST | Create new agent session |
| `/api/v1/agent/sessions` | GET | List agent sessions |
| `/api/v1/agent/sessions/{id}` | GET | Get agent session details |
| `/api/v1/agent/sessions/{id}/history` | GET | Get agent message history |
| `/api/v1/agent/chat/{id}` | POST | Send message to agent (non-streaming) |
| `/api/v1/agent/chat/{id}/stream` | POST | Send message to agent (streaming) |

**Agent Session Types:**
- `agent`: Autonomous agent with tool use capabilities

**Creating an Agent Session:**
```bash
curl -X POST http://127.0.0.1:8193/api/v1/agent/sessions \
  -H "Content-Type: application/json" \
  -d '{"name": "Research Agent", "model": "gpt-4"}'
```

**Chatting with an Agent:**
```bash
# Non-streaming response
curl -X POST http://127.0.0.1:8193/api/v1/agent/chat/{session_id} \
  -H "Content-Type: application/json" \
  -d '{"message": "Research the latest in Rust async"}'

# Streaming response
curl -X POST http://127.0.0.1:8193/api/v1/agent/chat/{session_id}/stream \
  -H "Content-Type: application/json" \
  -d '{"message": "Write a summary of Rust async"}'
```

**Agent Memory Synchronization:**
Agent conversations automatically sync to memory tiers:
- Messages are stored with correct `user` and `assistant` roles
- Query-response pairs are preserved as conversation units in working memory
- Memory tiers (working, short-term, long-term) populate correctly

### Settings Configuration

The web GUI supports real-time settings updates:

```json
{
  "llm": {
    "type": "openai",
    "api_url": "http://localhost:8080/v1",
    "model": "gpt-4",
    "temperature": 0.7,
    "max_tokens": 8192
  },
  "embedding": {
    "type": "openai",
    "api_url": "http://localhost:8081/v1/embeddings",
    "model": "text-embedding-bge_m3",
    "expected_dimension": 1024
  }
}
```

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Cmd/Ctrl + K` | Open search dialog |
| `Up Arrow` | Previous input in history |
| `Down Arrow` | Next input in history |
| `Enter` | Send message |
| `Shift + Enter` | New line in input |

### Architecture

The web stack consists of:

```
memst-ui/          # React + TypeScript frontend
├── src/
│   ├── components/    # UI components
│   ├── context/       # React context providers
│   ├── hooks/         # Custom React hooks
│   ├── types/         # TypeScript type definitions
│   └── api/           # API client

memst-server/       # FastAPI backend
├── src/
│   └── memst_server/
│       ├── api/          # REST API routes
│       ├── db.py         # DuckDB user/settings storage
│       ├── memst_client.py  # MemSt Rust library wrapper
│       └── config.py     # Configuration management
```

### Development

```bash
# Frontend development with hot reload
cd memst-ui

# Configure API URL (optional - defaults to http://127.0.0.1:8193)
cp src/config.ts src/config.local.ts
# Edit config.local.ts to change the backend URL

npm run dev

# Build for production
cd memst-ui
npm run build

# Backend development
cd memst-server
PYTHONPATH=src .venv/bin/python -m memst_server.main --reload
```

**Note:** Always set `PYTHONPATH=src` when running the backend to ensure the `memst_server` module can be found.

---

## Git-Like Architecture (Phase 8)

MemSt includes optional Git-like version control features for content-addressable storage and session branching.

### Content-Addressable Storage

Objects are identified by their SHA-256 content hash (64 hex characters):

```rust
use memst_core::objects::{ObjectId, Blob, Tree, Commit, Tag};

// Create ObjectId from content
let content = b"Hello, World!";
let oid = ObjectId::from_content(content);
println!("Object ID: {}", oid.to_hex()); // 64 char hex string

// Abbreviated ID for display
let short = oid.abbreviate(); // 7 char prefix
```

### Object Types

| Type | Description |
|------|-------------|
| `Blob` | Raw content (memory facts, messages) |
| `Tree` | Directory structure with entries |
| `Commit` | Snapshot with parent references |
| `Tag` | Annotated or lightweight tags |

```rust
// Create a blob
let blob = Blob::new(b"User prefers dark mode");
let blob_oid = store.write_blob(&blob).unwrap();

// Create a tree with entries
let mut tree = Tree::new();
tree.add_entry(TreeEntry::new(TreeEntry::MODE_FILE, blob_oid, "fact.txt"));
let tree_oid = store.write_tree(&tree).unwrap();

// Create a commit
let author = Author::new("User", "user@example.com");
let commit = Commit::new(tree_oid, &author, "Add fact");
let commit_oid = store.write_commit(&commit).unwrap();

// Create an annotated tag
let tag = Tag::new(commit_oid, "v1.0.0", author, "Release 1.0.0");
let tag_oid = store.write_tag(&tag).unwrap();
```

### Branch Operations

```rust
use memst_core::objects::{BranchOps, RefType};

// Create branch operations
let branches = BranchOps::new(&store, &refs);

// Create a branch at a commit
branches.create("feature", commit_oid).unwrap();

// Check if branch exists
if branches.exists("feature").unwrap() {
    println!("Branch exists");
}

// Get branch tip OID
let branch_oid = branches.get("feature").unwrap();

// Check if commit is ancestor of branch tip
if branches.is_ancestor("main", commit_oid).unwrap() {
    println!("Commit is ancestor of main");
}

// Rename branch
branches.rename("feature", "new-feature").unwrap();

// Delete branch
branches.delete("new-feature").unwrap();
```

### Merge Operations

```rust
use memst_core::objects::{MergeOps, MergeOptions, MergeStrategy};

// Create merge operations
let mut merges = MergeOps::new(&mut store, &refs);

// Perform merge with default (recursive) strategy
let result = merges.merge("main", feature_oid, author.clone(), MergeOptions::default()).unwrap();

println!("Fast-forward: {}", result.fast_forward);
println!("Up-to-date: {}", result.up_to_date);
println!("Conflicts: {}", result.conflicts.len());

// Use specific merge strategy
let options = MergeOptions {
    strategy: MergeStrategy::Ours,
    ..Default::default()
};
let result = merges.merge("main", feature_oid, author, options).unwrap();
```

### Merge Strategies

| Strategy | Description |
|----------|-------------|
| `Recursive` | Three-way merge with recursive ancestor handling |
| `Resolve` | Simple resolve using common ancestor |
| `Octopus` | Multi-branch merge (2+ heads) |
| `Ours` | Keep ours, discard theirs |
| `Theirs` | Keep theirs, discard ours |

### Commit History

```rust
use memst_core::objects::CommitHistory;

// Get history starting from a commit
let history = CommitHistory::new(&store, commit_oid);

// Get all ancestors
let ancestors = history.ancestors(None).unwrap();

// Get limited ancestry (last N commits)
let recent = history.ancestors(Some(10)).unwrap();

// Find merge base between two commits
let merge_base = history.merge_base(other_oid).unwrap();

// Check if at initial commit
let is_initial = history.is_initial().unwrap();

// Get parent count
let parent_count = history.parent_count().unwrap();
```

### Ref Store

Low-level reference operations:

```rust
use memst_core::objects::RefStore;

// Create ref store
let refs = RefStore::new(&base_path).unwrap();

// Set branch ref
refs.set_ref("main", RefType::Branch, commit_oid).unwrap();

// Get branch ref
let branch_oid = refs.get_ref("main", RefType::Branch).unwrap();

// List all branches
let branches = refs.list_refs(RefType::Branch).unwrap();

// Delete branch
refs.delete_ref("main", RefType::Branch).unwrap();

// HEAD operations
refs.set_head(commit_oid).unwrap();
let head = refs.get_head().unwrap();
refs.set_head_to_branch("main").unwrap();
let branch = refs.get_branch_name().unwrap();
```

---

## Python Bindings (Phase 9)

MemSt provides Python bindings via PyO3 for seamless integration with Python applications.

### Installation

```bash
# From source (requires Rust toolchain)
cd memst-py
maturin build --release
pip install target/wheels/memst-*.whl

# Or from PyPI (when published)
pip install memst
```

### Core Classes

| Class | Description |
|-------|-------------|
| `SessionStore` | Main store for managing sessions |
| `Session` | Session with id, name, model, created_at |
| `Message` | Message with id, role, content, timestamp |
| `MemoryItem` | Memory with content, importance, confidence, tags |
| `Role` | Enum: System, User, Assistant, Tool |
| `MemoryTier` | Enum: Working, ShortTerm, LongTerm |

### Phase 8: Git-Like Architecture Classes

| Class | Description |
|-------|-------------|
| `ObjectId` | Content-addressable SHA-256 identifier |
| `Blob` | Raw content storage |
| `Tree` | Directory structure with entries |
| `TreeEntry` | Entry in a tree (mode, oid, name) |
| `Commit` | Commit with tree, author, message, parents |
| `Tag` | Annotated or lightweight tags |
| `Author` | Author with name, email, timestamp |
| `RefType` | Branch or Tag reference type |
| `MergeStrategy` | Merge strategy enum |
| `MergeResult` | Result of merge operation |

### Quick Start

```python
import tempfile
from memst import SessionStore, Role, MemoryTier

# Create a store
store = SessionStore("/tmp/my-store")

# Create a session
session = store.create_session("My Chat", "gpt-4")
print(f"Session: {session.id}")

# Add messages
store.add_message(session.id, Role.User, "Hello, I need help with Rust!")
store.add_message(session.id, Role.Assistant, "I'd be happy to help with Rust!")

# List sessions
sessions = store.list_sessions()
for s in sessions:
    print(f"  - {s['name']} ({s['model']})")

# Add memories to tiers
store.add_memory(session.id, MemoryTier.Working, "User is learning Rust", tags=["learning"])
store.add_memory(session.id, MemoryTier.ShortTerm, "Project deadline: March 1", tags=["project"])

# Search
results = store.search("Rust", limit=10)
for r in results:
    print(f"  {r['content'][:80]}...")
```

### Phase 8 Git-Like Python API

```python
from memst import ObjectId, Blob, Tree, Commit, Tag, Author

# Create ObjectId from content
oid = ObjectId(b"Hello, World!")
print(f"Object ID: {oid.hex}")  # 64 char hex string
print(f"Abbreviated: {oid.abbreviate()}")  # 7 char prefix
print(f"Is nil: {oid.is_nil()}")

# Create Author
author = Author("Test User", "test@example.com")
print(f"Author: {author.name} <{author.email}>")

# Create Tree with entries
tree = Tree()
tree.add_entry(0o100644, oid.hex, "memory.txt")

# Create Commit
commit = Commit(oid.hex, author, "Initial commit")
print(f"Commit message: {commit.message}")
print(f"Is merge: {commit.is_merge()}")
print(f"Parents: {len(commit.parent_oids)}")

# Create Tag
tag = Tag(oid.hex, "v1.0.0", author, "Release 1.0.0")
print(f"Tag: {tag.name}, Lightweight: {tag.is_lightweight}")

# Use enums
from memst import RefType, MergeStrategy

print(f"Branch ref: {RefType.Branch}")
print(f"Merge strategy: {MergeStrategy.Recursive}")
```

### Phase 12: Advanced Search (Semantic & Hybrid) Python API

MemSt provides Python bindings for the HNSW vector index and hybrid search system.

#### Phase 12 Classes

| Class | Description |
|-------|-------------|
| `HnswConfig` | HNSW configuration (m, ef_construction, ef_search, threshold) |
| `HnswIndex` | Vector index for ANN search with add/search/delete |
| `DocumentInfo` | Document metadata (id, session_id, doc_type, content, timestamp) |
| `VectorSearchResult` | Search result with cosine similarity score |
| `FusionStrategy` | RRF, Weighted, Interleave fusion strategies |
| `HybridSearchConfig` | Configurable weights and fusion settings |
| `HybridSearchResult` | Combined keyword + semantic results |
| `QueryRouter` | Analyzes queries for optimal search strategy |
| `SearchStrategy` | Keyword, Semantic, Hybrid recommendations |

#### HNSW Vector Index API

```python
import memst

# Create HNSW index with 1024 dimensions (for bge-m3)
index = memst.HnswIndex(1024)

# Or with custom configuration
config = memst.HnswConfig()
config.m = 16                    # Connections per node
config.ef_construction = 200     # Search width during construction
config.ef_search = 100           # Search width during query
config.similarity_threshold = 0.5 # Minimum similarity

index = memst.HnswIndex(1024, config)

# Add documents with embeddings
index.add_document(
    id='msg-001',
    vector=[0.1, 0.2, 0.3, ...],  # 1024-dimensional vector
    session_id='session-123',
    doc_type='message',
    content='Hello, I need help with Rust async'
)

# Search for similar documents
results = index.search(query=[0.1, 0.2, 0.3, ...], limit=10)

# Search with session filter
results = index.search_filtered(
    query=[0.1, 0.2, 0.3, ...],
    limit=10,
    session_filter='session-123'
)

# Delete a document
deleted = index.delete('msg-001')

# Get index statistics
print(f"Documents: {index.len()}")
print(f"Dimension: {index.dimension()}")
```

#### Search Result Types

```python
# DocumentInfo - document metadata
doc = memst.DocumentInfo(
    id='msg-001',
    session_id='session-123',
    doc_type='message',
    content='Hello, I need help with Rust async',
    timestamp='2024-01-01T00:00:00Z'
)
print(f"ID: {doc.id}")
print(f"Session: {doc.session_id}")
print(f"Type: {doc.doc_type}")
print(f"Content: {doc.content}")

# VectorSearchResult - search result with score
result = memst.VectorSearchResult(
    id='msg-001',
    score=0.95,  # Cosine similarity (0.0-1.0)
    document=doc
)
print(f"ID: {result.id}")
print(f"Score: {result.score}")
print(f"Content: {result.document.content}")
```

#### Fusion Strategies

```python
from memst import FusionStrategy, HybridSearchConfig

# Default: RRF (Reciprocal Rank Fusion)
config = memst.HybridSearchConfig()
print(f"Keyword weight: {config.keyword_weight}")  # 0.5
print(f"Semantic weight: {config.semantic_weight}")  # 0.5
print(f"Fusion strategy: {config.fusion_strategy}")  # Rrf

# Use different fusion strategy
config = memst.HybridSearchConfig()
config.keyword_weight = 0.7
config.semantic_weight = 0.3
config.fusion_strategy = FusionStrategy.Weighted

# Interleave strategy
config.fusion_strategy = FusionStrategy.Interleave
```

#### Query Router

```python
from memst import QueryRouter, SearchStrategy

router = memst.QueryRouter()

# Analyze query to get recommended strategy
strategy = router.analyze_query("hello")
print(f"Strategy: {strategy}")  # Keyword (short query)

strategy = router.analyze_query("What are the key principles of effective software architecture?")
print(f"Strategy: {strategy}")  # Semantic (long query)

strategy = router.analyze_query("Rust async programming")
print(f"Strategy: {strategy}")  # Hybrid (medium query)

# Get explanation for recommendation
explanation = router.explain_recommendation("hello", "Keyword")
print(f"Explanation: {explanation}")
```

#### Search Strategy Enum

```python
from memst import SearchStrategy

print(f"Keyword: {SearchStrategy.Keyword}")
print(f"Semantic: {SearchStrategy.Semantic}")
print(f"Hybrid: {SearchStrategy.Hybrid}")
```

### SessionStore Methods

```python
# Constructor / factory
store = SessionStore(path)  # Create or open store

# Session management
session = store.create_session(name, model)
sessions = store.list_sessions()
session = store.get_session(session_id)
store.delete_session(session_id)

# Messages
store.add_message(session_id, Role, content)
messages = store.get_session_messages(session_id)

# Memory
store.add_memory(session_id, MemoryTier, content, tags=None)
memories = store.get_session_memory(session_id, MemoryTier)

# Search
results = store.search(query, limit=10)

# Statistics
# (No dedicated stats method in Python bindings yet; use the CLI `memst stats`)
```

---

## Best Practices

---

### 1. Store Location
- Use absolute paths for reliability
- Backup store directory regularly
- Use version control for manifest.json

### 2. Session Management
- Use descriptive session names
- Add relevant tags for filtering
- Delete inactive sessions periodically

### 3. Search Optimization
- Use session filters for faster searches
- Limit results with `--limit` for large datasets
- Use Tantivy backend for complex queries

### 4. Memory Tiers
- Working tier: Active conversation context
- Short tier: Session-relevant memories
- Long tier: Persistent knowledge

---

## Troubleshooting

### Common Issues

#### 1. Python Module Not Found

If you get `ModuleNotFoundError: No module named 'memst_server'`:

```bash
# Ensure PYTHONPATH is set
cd memst-server
PYTHONPATH=src .venv/bin/python -m memst_server.main
```

#### 2. CORS Policy Errors

If you see CORS errors in the browser console:

```
Access to fetch at 'http://127.0.0.1:8193/api/...' 
from origin 'http://localhost:3000' has been blocked by CORS policy
```

**Fix:** Add your frontend origin to `config.toml`:

```toml
[server]
cors_origins = ["http://localhost:3000", "http://127.0.0.1:3000"]
```

Then restart the server.

#### 3. Python 3.13 Compatibility Issues

If you get `SIGABRT` or `Library not loaded: libpython3.13.dylib`:

```bash
# Recreate the virtual environment with Python 3.12
cd memst-server
rm -rf .venv
uv venv --python 3.12
uv pip install fastapi uvicorn duckdb python-dotenv pydantic pydantic-settings httpx toml
uv pip install -e ../third/nanobot
```

#### 4. memst Module Not Available

If you see "Warning: memst module not available":

```bash
# Build and install memst-py
cd memst-py
VIRTUAL_ENV=../memst-server/.venv ../memst-server/.venv/bin/maturin develop
```

#### 5. Port Already in Use

If port 8192/8193 is already in use:

```bash
# Check what's using the port
lsof -i :8192

# Change port in config.toml
[server]
port = 8194
```

### Lock Errors

If you see `store.lock` errors:
```bash
# Check for running processes
lsof | grep store.lock

# Remove stale lock (only if no other process is using)
rm store/store.lock
```

### Corrupted Index

Rebuild search index:
```bash
# Delete and recreate
rm -rf search_index/
memst search "test"  # Auto-recreates on first search
```

### Rust Version

Tantivy backend requires Rust 1.88+:
```bash
rustc --version
rustup install 1.88
rustup default 1.88
```

---

## API Reference

### Rust Library

```rust
use memst_core::{SessionStore, Message, Role, SearchQuery};

// Open store
let store = SessionStore::open("./my-store")?;

// Create session
let session_id = store.create_session(SessionMetadata {
    name: "My Session".into(),
    model: "gpt-4".into(),
    tags: vec!["tag1".into()],
    ..Default::default()
})?;

// Add message
store.append_message(session_id, Message {
    role: Role::User,
    content: Content::Text("Hello".into()),
    ..Default::default()
})?;

// Search
let results = store.search(SearchQuery {
    terms: vec!["hello".into()],
    limit: 10,
    ..Default::default()
})?;
```

### Python Bindings

The `memst` Python package provides full access to MemSt functionality.

#### Installation

```bash
# From source (requires Rust toolchain)
cd memst-py
maturin build --release
pip install target/wheels/memst-*.whl

# Or from PyPI (when published)
pip install memst
```

#### Quick Start

```python
import tempfile
from memst import SessionStore, Role, MemoryTier

# Create a store
store = SessionStore("/tmp/my-store")

# Create a session
session = store.create_session("My Chat", "gpt-4")
print(f"Session: {session.id}")

# Add messages
store.add_message(session.id, Role.User, "Hello, I need help with Rust!")
store.add_message(session.id, Role.Assistant, "I'd be happy to help with Rust!")

# List sessions
sessions = store.list_sessions()
for s in sessions:
    print(f"  - {s['name']} ({s['model']})")

# Get messages
messages = store.get_session_messages(session.id)
for msg in messages:
    print(f"[{msg['role']}] {msg['content']}")

# Add memories to tiers
store.add_memory(session.id, MemoryTier.Working, "User is learning Rust", tags=["learning"])
store.add_memory(session.id, MemoryTier.ShortTerm, "Project deadline: March 1", tags=["project"])

# Search
results = store.search("Rust", limit=10)
for r in results:
    print(f"  {r['content'][:80]}...")
```

#### API Reference

| Class | Description |
|-------|-------------|
| `SessionStore` | Main store for managing sessions |
| `Session` | Session with id, name, model, created_at |
| `Message` | Message with id, role, content, timestamp |
| `MemoryItem` | Memory with content, importance, confidence, tags |
| `Role` | Enum: System, User, Assistant, Tool |
| `MemoryTier` | Enum: Working, ShortTerm, LongTerm |

#### SessionStore Methods

```python
# Constructor / factory
store = SessionStore(path)  # Create or open store

# Session management
session = store.create_session(name, model)
sessions = store.list_sessions()
session = store.get_session(session_id)
store.delete_session(session_id)

# Messages
store.add_message(session_id, Role, content)
messages = store.get_session_messages(session_id)

# Memory
store.add_memory(session_id, MemoryTier, content, tags=None)
memories = store.get_session_memory(session_id, MemoryTier)

# Search
results = store.search(query, limit=10)

# Statistics
stats = store.stats()
```

---

## License

MemSt is licensed under the Apache License 2.0. See `LICENSE` in the repository root.
