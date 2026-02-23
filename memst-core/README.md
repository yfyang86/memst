# memst-core

Core library for MemSt - Git-like memory architecture for LLM session management.

## Overview

`memst-core` provides the foundational components for building session-aware memory systems:

- **Session Store**: File-based storage with Git-like object model
- **Search**: BM25 text search with Tantivy backend option
- **Vector Search**: HNSW index for semantic similarity search
- **Hybrid Search**: RRF fusion combining keyword and semantic results
- **Memory Tiers**: Working, short-term, and long-term memory management
- **LLM Integration**: Fact and entity extraction via OpenAI-compatible APIs

## Quick Start

```rust
use memst_core::store::SessionStore;
use memst_core::types::{SessionMetadata, Role};

// Initialize store
let store = SessionStore::init("./my-store")?;

// Create a session
let metadata = SessionMetadata {
    name: "My Chat".to_string(),
    model: "gpt-4".to_string(),
    tags: vec!["work".to_string()],
    ..Default::default()
};
let session_id = store.create_session(metadata)?;

// Add messages
store.add_message(session_id, Role::User, "Hello!".to_string())?;
store.add_message(session_id, Role::Assistant, "Hi there!".to_string())?;

// Search
let results = store.search("hello", Some(10))?;
```

## Configuration

Set environment variables for LLM/embedding services:

```bash
export MEMST_LLM_API_URL="http://localhost:8080/v1"
export MEMST_LLM_MODEL="gpt-4"
export MEMST_EMBEDDING_API_URL="http://localhost:8081/v1/embeddings"
export MEMST_EMBEDDING_MODEL="text-embedding-ada-002"
```

Or use a `config.toml` file - see `config::EXAMPLE_CONFIG` for format.

## Features

- `tantivy` - Enable Tantivy search backend (default)

## License

MIT OR Apache-2.0
