# memst-cli

Command-line interface for MemSt - Git-like memory architecture for LLM sessions.

## Installation

```bash
cargo install --path .
```

## Quick Start

```bash
# Initialize a new store
memst init

# Create a session
memst session new --name "My Project" --model "gpt-4"

# Add messages
memst message add <session-id> --role user --content "What is Rust?"
memst message add <session-id> --role assistant --content "Rust is a systems programming language..."

# List sessions
memst session list

# Search across all sessions
memst search "Rust programming"

# View session details
memst session show <session-id>

# Export a session
memst session export <session-id> --format json
```

## Commands

### Store Management
- `memst init [path]` - Initialize a new memory store
- `memst stats [session-id]` - Show statistics

### Session Management
- `memst session new` - Create a new session
- `memst session list` - List all sessions
- `memst session show <id>` - Show session details
- `memst session delete <id>` - Delete a session
- `memst session export <id>` - Export session data

### Message Management
- `memst message add <session-id> <role> <content>` - Add a message
- `memst message list <session-id>` - List messages in a session

### Memory Management
- `memst memory add <session-id> <content>` - Add a memory
- `memst memory list <session-id>` - List memories
- `memst memory promote <session-id> <memory-id>` - Promote memory tier

### Search
- `memst search <query>` - Search across sessions

## Configuration

The CLI uses the store path `./memst-store` by default. Override with `--store`:

```bash
memst --store /path/to/store session list
```

## Environment Variables

```bash
# LLM API configuration
export MEMST_LLM_API_URL="http://localhost:8080/v1"
export MEMST_LLM_MODEL="gpt-4"

# Embedding API configuration  
export MEMST_EMBEDDING_API_URL="http://localhost:8081/v1/embeddings"
export MEMST_EMBEDDING_MODEL="text-embedding-ada-002"
```

## License

Apache-2.0
