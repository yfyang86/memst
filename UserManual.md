# MemSt User Manual

MemSt is a hybrid, searchable session memory library for LLM applications, written in Rust with Python bindings and a REST API server.

Author: Yifan Yang <yfyang.86@hotmail.com>

Date: 2025

---

## Table of Contents

1. [Overview](#overview)
2. [Installation](#installation)
3. [Core Concepts](#core-concepts)
4. [Sessions](#sessions)
5. [Messages](#messages)
6. [Memory Items](#memory-items)
7. [Search](#search)
8. [Git-Like Architecture (Phase 8)](#git-like-architecture-phase-8)
9. [Context Assembly (Phase 9)](#context-assembly-phase-9)
10. [Sleep-Time Consolidation (Phase 10)](#sleep-time-consolidation-phase-10)
11. [Semantic Merge (Phase 11)](#semantic-merge-phase-11)
12. [Skills (Phase 12)](#skills-phase-12)
13. [Multi-Agent Support (Phase 13)](#multi-agent-support-phase-13)
14. [MCP Adapter (Phase 14)](#mcp-adapter-phase-14)
15. [Python Bindings](#python-bindings)
16. [Configuration](#configuration)
17. [CLI Reference](#cli-reference)
18. [API Reference](#api-reference)

---

## Overview

MemSt provides a session-based memory system for LLM applications with the following key features:

| Feature | Description |
|---------|-------------|
| Session Storage | Store and retrieve chat sessions with metadata |
| Message History | Append messages with roles (System, User, Assistant, Tool) |
| Hybrid Search | Combine BM25 text search with HNSW vector search |
| Git-Like Objects | Content-addressable storage with branching and merging |
| Context Assembly | Token-budget-aware memory retrieval |
| Sleep-Time Consolidation | Async background memory maintenance |
| Multi-Agent Support | Isolated worktrees for concurrent agents |
| MCP Protocol | Native integration with Claude/Cursor |

---

## Installation

### From Source (Rust)

```bash
# Clone the repository
git clone <repository-url>
cd memst

# Build the workspace
cargo build --release

# Install CLI
cargo install --path memst-cli
```

### Python Bindings

```bash
cd memst-py
maturin build --release
pip install target/wheels/memst-*.whl
```

---

## Core Concepts

### Memory Tiers

MemSt organizes memories into three tiers:

| Tier | Description | Access Speed | Capacity |
|------|-------------|--------------|----------|
| **Working** | Active conversation context | Fastest | Limited |
| **Short-term** | Recent facts and user preferences | Fast | Moderate |
| **Long-term** | Important knowledge and historical data | Slower | Large |
| **Archival** | Rarely accessed historical data | Slowest | Unlimited |

Memories automatically transition between tiers based on access patterns, importance, and age.

### Memory Types

| Type | Description | Use Case |
|------|-------------|----------|
| **Episodic** | Specific events and experiences | "User said...", "Error occurred..." |
| **Semantic** | Facts and concepts | "User prefers dark mode", "API endpoint is..." |
| **Procedural** | How-to knowledge (Skills) | "To deploy, run...", "Debug steps: ..." |
| **Resource** | External references | URLs, file paths, documentation |
| **MetaCognitive** | System reflections | "Model consistently makes..." |

---

## Sessions

A session represents a conversation with an LLM, identified by a UUID and associated with metadata.

### Creating Sessions

```rust
use memst_core::store::SessionStore;
use memst_core::types::SessionMetadata;

let store = SessionStore::init("./data")?;

let metadata = SessionMetadata::new("Project Planning", "gpt-4");
let session_id = store.create_session(metadata)?;
println!("Session ID: {}", session_id);
```

### Session Metadata

| Field | Type | Description |
|-------|------|-------------|
| `name` | `String` | Human-readable session name |
| `model` | `String` | LLM model identifier |
| `system_prompt` | `Option<String>` | System prompt for the session |
| `tags` | `Vec<String>` | Searchable session tags |
| `user_preferences` | `Vec<String>` | Session-specific preferences |
| `suspended` | `bool` | Whether session is paused |

### Listing Sessions

```rust
// List all non-archived sessions
let sessions = store.list_sessions()?;

for session in sessions {
    println!("{}: {} ({})", session.id, session.name, session.model);
}

// List with filters
let filters = SessionFilters {
    model: Some("gpt-4".to_string()),
    tags: vec!["project".to_string()],
    suspended: Some(false),
    created_after: Some(start_time),
    ..Default::default()
};
let filtered = store.list_sessions_filtered(&filters)?;
```

### Suspending Sessions

```rust
// Suspend a session (pauses background tasks)
store.suspend_session(session_id)?;

// Resume a suspended session
store.resume_session(session_id)?;

// Check suspension status
if store.is_session_suspended(session_id)? {
    println!("Session is suspended");
}
```

---

## Messages

Messages represent the conversation history within a session.

### Message Structure

| Field | Type | Description |
|-------|------|-------------|
| `id` | `Uuid` | Unique message identifier |
| `session_id` | `Uuid` | Parent session |
| `role` | `Role` | System, User, Assistant, or Tool |
| `content` | `String` | Message content |
| `created_at` | `DateTime<Utc>` | Timestamp |
| `run_id` | `Option<Uuid>` | Associated run ID |
| `metadata` | `Option<Value>` | Additional metadata |

### Adding Messages

```rust
use memst_core::types::{Message, Role};

// Simple message
let msg = Message::new(Role::User, "Hello!".into());
store.append_message(session_id, msg)?;

// Message with run_id
let mut msg = Message::new(Role::Assistant, "Processing...".into());
msg.run_id = Some(run_id);
store.append_message(session_id, msg)?;

// Tool message
let tool_msg = Message {
    role: Role::Tool,
    content: "{\"result\": 42}".into(),
    run_id: Some(run_id),
    ..Default::default()
};
store.append_message(session_id, tool_msg)?;
```

### Retrieving Messages

```rust
// Get all messages for a session (newest first)
let messages = store.get_session_messages(session_id)?;

// Get limited number
let recent = store.get_session_messages_with_limit(session_id, 10)?;

// Check message count
let count = store.get_message_count(session_id)?;
```

---

## Memory Items

Memory items store important information extracted from conversations or added explicitly.

### Memory Item Structure

| Field | Type | Description |
|-------|------|-------------|
| `id` | `Uuid` | Unique memory identifier |
| `content` | `String` | Memory content |
| `source` | `String` | Origin of the memory |
| `created_at` | `DateTime<Utc>` | Creation timestamp |
| `last_accessed` | `DateTime<Utc>` | Last access time |
| `access_count` | `u32` | Number of times accessed |
| `embedding` | `Option<Vec<f32>>` | Vector embedding |
| `tags` | `Vec<String>` | Searchable tags |
| `confidence` | `f32` | Confidence score (0.0-1.0) |
| `importance` | `f32` | Importance score (0.0-1.0) |
| `memory_type` | `MemoryType` | Episodic, Semantic, Procedural, etc. |
| `tier` | `MemoryTier` | Working, ShortTerm, LongTerm |
| `token_estimate` | `Option<u32>` | Estimated token count |
| `supersedes` | `Option<Uuid>` | Previous version of this memory |
| `retracted_by` | `Option<Uuid>` | Retraction memory ID |

### Adding Memories

```rust
use memst_core::types::{MemoryItem, MemoryType, MemoryTier};

let memory = MemoryItem::new(
    "User prefers dark mode".into(),
    "conversation".into(),
);

// With custom fields
let mut memory = MemoryItem::new(
    "API key: sk-xxx".into(),
    "user_input".into(),
);
memory.tags = vec!["api".into(), "credentials".into()];
memory.confidence = 0.95;
memory.importance = 0.8;
memory.memory_type = MemoryType::Semantic;
memory.tier = MemoryTier::LongTerm;

store.add_memory(memory)?;
```

### Updating Access

```rust
// Record access to update scores
store.update_memory_access(memory_id)?;
```

### Importance Scoring

```rust
// Get memories sorted by importance
let important = store.get_important_memories(10)?;

// Update importance based on access patterns
let decay_config = DecayConfig {
    half_life_days: 7.0,
    min_relevance: 0.1,
    recompute_threshold: 100,
};
store.decay_memories(&decay_config)?;
```

---

## Search

MemSt provides three search modes: full-text (BM25), vector (HNSW), and hybrid.

### Search Document Types

| Type | Description |
|------|-------------|
| `Session` | Session metadata |
| `Message` | Conversation messages |
| `Memory` | Memory items |

### Full-Text Search (BM25)

```rust
use memst_core::search::DocumentType;

let results = store.search(
    "error handling",
    DocumentType::Message,
    10,
)?;

for (doc, score) in results {
    println!("{}: {:.4}", doc.id, score);
}
```

### Vector Search (HNSW)

Requires embedding configuration in `config.toml`.

```rust
use memst_core::semantic::SemanticSearch;

let semantic = SemanticSearch::new(&store)?;

// Vector search with embedding
let results = semantic.vector_search(
    "async programming",
    DocumentType::Memory,
    10,
    Some(&filter),
)?;

for (doc, score) in results {
    println!("{}: {:.4}", doc.id, score);
}
```

### Hybrid Search (RRF)

Combines BM25 and vector search using Reciprocal Rank Fusion.

```rust
// Hybrid search with weights
let results = semantic.hybrid_search(
    "error handling patterns",
    DocumentType::Memory,
    10,
    Some(&filter),
    0.3,  // BM25 weight
    0.7,  // Vector weight
)?;

for (doc, score) in results {
    println!("{}: {:.4}", doc.id, score);
}
```

### Search Filters

```rust
use memst_core::search::DocumentFilter;

let filter = DocumentFilter {
    doc_types: Some(vec![DocumentType::Memory]),
    tags: Some(vec!["rust".into()]),
    date_from: Some(start_date),
    date_to: Some(end_date),
};
```

---

## Git-Like Architecture (Phase 8)

MemSt v1.0 includes Git-like version control features for content-addressable storage and session branching, powered by **Blake3** for 3× faster hashing than SHA-256.

### Content-Addressable Storage

Objects are identified by their Blake3 content hash (64 hex characters):

```rust
use memst_core::objects::{ObjectId, Blob, Tree, Commit, Tag, ObjectType};

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
| `Skill` | Procedural memory with trigger patterns |
| `ContextFile` | Markdown with JSON frontmatter |
| `Entity` | Knowledge graph node |
| `Relation` | Knowledge graph edge |

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

### Write-Ahead Log (WAL)

For crash recovery, MemSt uses a WAL:

```rust
use memst_repo::wal::{Wal, WalEntry, WalOp};

let mut wal = Wal::open("./data/wal")?;

// Append operation
wal.append(WalEntry {
    op: WalOp::Insert,
    object_id: oid,
    data: content,
    timestamp: Utc::now(),
})?;

// Recover uncommitted operations
let uncommitted = wal.recover()?;
```

### Memory Lifecycle

```rust
use memst_repo::lifecycle::{MemoryLifecycle, LifecycleConfig};

let config = LifecycleConfig {
    working_to_short_term_threshold: 10,
    short_term_to_long_term_threshold: 5,
    long_term_to_archival_days: 90,
    checkpoint_interval_minutes: 60,
};

let lifecycle = MemoryLifecycle::new(config);
lifecycle.transition_tier(&mut memory, MemoryTier::LongTerm)?;
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

## Context Assembly (Phase 9)

MemSt v1.0 introduces token-budget-aware context assembly for optimal LLM context window usage.

### Context Builder

```rust
use memst_core::context::{ContextBuilder, BuildContextConfig};
use memst_core::types::Skill;

let config = BuildContextConfig {
    token_budget: 8000,
    reserved_for_response: 2000,
    system_prompt_tokens: 500,
    relevance_weight: 1.0,
    recency_weight: 0.5,
    importance_weight: 0.3,
    confidence_threshold: 0.5,
    max_memories: 20,
    deduplicate: true,
};

let builder = ContextBuilder::new(token_counter, config);

let assembly = builder.build_context(
    query,
    &memories,
    &skills,
    &entities,
)?;

println!("Total tokens: {}", assembly.total_tokens);
println!("Budget remaining: {}", assembly.budget_remaining);
```

### Multi-Signal Scoring

Memories are scored using multiple signals:

| Signal | Weight | Description |
|--------|--------|-------------|
| Relevance | 1.0 | Semantic similarity to query |
| Recency | 0.5 | Time since last access |
| Importance | 0.3 | Explicit importance score |
| Confidence | 0.2 | Memory confidence |

### Context Block Types

| Block Type | Description |
|------------|-------------|
| `Memory` | Retrieved memory item |
| `Skill` | Matching procedural memory |
| `Entity` | Knowledge graph node |
| `SessionSummary` | Compressed conversation history |

### Progressive Disclosure

```rust
// Progressive disclosure loads more memories as needed
let assembly = builder.build_context_with_disclosure(
    query,
    &memories,
    &skills,
    &entities,
    DisclosureLevel::Standard,  // Standard, Minimal, Exhaustive
)?;
```

### Token Counter

```rust
use memst_core::context::SimpleTokenCounter;

// Simple whitespace-based token estimation
let counter = SimpleTokenCounter;
let tokens = counter.count("Hello, world!");

// Or use a more sophisticated tokenizer
let counter = TiktokenCounter::new("cl100k_base");
```

---

## Sleep-Time Consolidation (Phase 10)

MemSt v1.0 includes async background processes for memory maintenance.

### Consolidation Engine

```rust
use memst_sleep::consolidate::{ConsolidationEngine, ConsolidationConfig};

let config = ConsolidationConfig {
    cluster_similarity_threshold: 0.85,
    min_cluster_size: 3,
    max_summary_length: 200,
};

let engine = ConsolidationEngine::new(config);

// Cluster memories by topic
let clusters = engine.cluster_memories(memories);

// Generate summaries
for cluster in clusters {
    if let Some(summary) = engine.summarize_cluster(&cluster).await {
        println!("Summary: {}", summary.content);
    }
}
```

### Topic Clusters

```rust
pub struct TopicCluster {
    pub id: Uuid,
    pub memories: Vec<MemoryItem>,
    pub centroid: Vec<f32>,
    pub topic_label: String,
    pub coherence_score: f32,
}
```

### Evolution Engine (A-MEM)

```rust
use memst_sleep::evolve::{EvolutionEngine, MemoryRelation};

let mut engine = EvolutionEngine::new();

// Evolve memory network with new memory
let result = engine.evolve_memory_network(&mut new_memory, &existing_memories);

// Result includes:
// - relations: Vec<(Uuid, MemoryRelation)> - links to existing memories
// - updated_memories: Vec<MemoryItem> - memories with updated confidence
// - conflicts: Vec<(Uuid, Uuid)> - detected contradictions

for (memory_id, relation) in result.relations {
    match relation {
        MemoryRelation::Supports => println!("Supports memory {}", memory_id),
        MemoryRelation::Contradicts => println!("Contradicts memory {}", memory_id),
        MemoryRelation::Elaborates => println!("Elaborates memory {}", memory_id),
        MemoryRelation::Supersedes => println!("Supersedes memory {}", memory_id),
        _ => {}
    }
}
```

### Memory Relations

| Relation | Description |
|----------|-------------|
| `Supports` | New memory strengthens existing memory |
| `Contradicts` | New memory conflicts with existing memory |
| `Elaborates` | New memory provides additional detail |
| `Supersedes` | New memory replaces outdated information |
| `Unrelated` | No significant relationship |

### Job Scheduler

```rust
use memst_sleep::scheduler::{JobScheduler, ConsolidationJob, JobPriority};

let scheduler = JobScheduler::new();

// Schedule consolidation job
let job_id = scheduler.schedule(
    ConsolidationJob::new(session_id),
    JobPriority::Normal,
).await?;

// Check job status
let status = scheduler.get_status(job_id).await;
println!("Job status: {:?}", status);
```

---

## Semantic Merge (Phase 11)

MemSt v1.0 provides intelligent three-way merge with semantic conflict detection.

### Semantic Merger

```rust
use memst_repo::merge::{SemanticMerger, SemanticMergeResult};

let merger = SemanticMerger::new();

// Perform three-way semantic merge
let result = merger.merge(
    source_oid,
    target_oid,
    &mut object_store,
    &ref_store,
    author,
    "Merge feature branch",
)?;

println!("Commit: {}", result.commit_hash);
println!("Conflicts: {}", result.conflicts.len());
println!("Added: {}", result.memories_added);
println!("Removed: {}", result.memories_removed);
```

### Conflict Detection

```rust
// Conflicts are detected via semantic similarity
let conflicts = merger.detect_conflicts(&source_memories, &target_memories);

for conflict in conflicts {
    match conflict.conflict_type {
        ConflictType::Duplicate => {
            println!("Duplicate: {} and {}", 
                conflict.memory_a, conflict.memory_b);
        }
        ConflictType::Contradiction => {
            println!("Contradiction: {} contradicts {}",
                conflict.memory_a, conflict.memory_b);
        }
    }
}
```

### Conflict Resolution

```rust
use memst_repo::merge::ConflictResolution;

// Auto-resolve strategies
let resolution = ConflictResolution::Auto {
    strategy: ResolutionStrategy::NewestWins,
};

// Or manual resolution
let resolution = ConflictResolution::Manual {
    keep_a: vec![memory_a_id],
    keep_b: vec![memory_b_id],
    merge: vec![(memory_a_id, memory_b_id)],
};

// Apply resolution
merger.resolve_conflicts(&conflicts, &resolution)?;
```

### Resolution Strategies

| Strategy | Description |
|----------|-------------|
| `NewestWins` | Keep the most recently created memory |
| `HighestConfidence` | Keep the memory with highest confidence |
| `KeepBoth` | Mark as related but keep both |
| `Manual` | User specifies which to keep/merge |

---

## Skills (Phase 12)

MemSt v1.0 supports procedural memory through Skills.

### Skill Structure

```rust
use memst_core::objects::{Skill, SkillStep, SkillFailurePolicy};

let skill = Skill {
    id: "deploy-service".into(),
    slug: "deploy-service".into(),
    name: "Deploy Microservice".into(),
    description: "Deploy a service to Kubernetes".into(),
    trigger_patterns: vec![
        "deploy".into(),
        "push to production".into(),
    ],
    steps: vec![
        SkillStep {
            order: 1,
            action: "Run tests".into(),
            tool: Some("pytest".into()),
            conditions: vec!["tests_pass".into()],
            on_failure: SkillFailurePolicy::Stop,
        },
        SkillStep {
            order: 2,
            action: "Build image".into(),
            tool: Some("docker".into()),
            conditions: vec![],
            on_failure: SkillFailurePolicy::Retry { max_attempts: 3 },
        },
        SkillStep {
            order: 3,
            action: "Deploy to k8s".into(),
            tool: Some("kubectl".into()),
            conditions: vec![],
            on_failure: SkillFailurePolicy::Rollback,
        },
    ],
    success_rate: 0.95,
    usage_count: 42,
};

// Store skill
let skill_oid = object_store.write_skill(&skill)?;
```

### Skill Failure Policies

| Policy | Behavior |
|--------|----------|
| `Stop` | Halt execution on failure |
| `Continue` | Log error and continue |
| `Retry { max_attempts }` | Retry up to N times |
| `Rollback` | Undo previous steps |
| `Fallback { skill_id }` | Switch to alternative skill |

### Trigger Patterns

```rust
// Skills are matched by trigger patterns
let matched_skills = skill_registry.match_skills("deploy the api service");

for skill in matched_skills {
    println!("Matched: {}", skill.name);
    println!("Success rate: {:.1}%", skill.success_rate * 100.0);
}
```

### Skill Execution

```rust
use memst_sleep::skills::SkillExecutor;

let executor = SkillExecutor::new();

// Execute skill with context
let result = executor.execute(&skill, &context).await?;

// Update success rate based on outcome
skill_registry.record_outcome(&skill.id, result.success).await?;
```

---

## Multi-Agent Support (Phase 13)

MemSt v1.0 supports concurrent agents through isolated worktrees.

### Agent Registry

```rust
use memst_repo::multi_agent::{AgentRegistry, AgentInfo};

let registry = AgentRegistry::new();

// Register agent
let agent = AgentInfo {
    id: "agent-001".into(),
    name: "Code Reviewer".into(),
    permissions: vec!["read".into(), "write".into()],
    parent_id: None,
};
registry.register(agent)?;

// Get agent info
let agent = registry.get("agent-001")?;
```

### Worktrees

```rust
use memst_repo::multi_agent::{WorktreeManager, Worktree};

let worktree_mgr = WorktreeManager::new(repo_path);

// Create worktree for agent
let worktree = worktree_mgr.create(
    "agent-001",
    "feature-branch",
    Some(commit_oid),
)?;

// Worktree isolation
// Each agent sees only their worktree
let agent_store = worktree.get_store()?;

// List agent memories
let memories = agent_store.search("async", DocumentType::Memory, 10)?;
```

### Cross-Agent Search

```rust
// Search across all agent worktrees (with permission)
let results = worktree_mgr.cross_agent_search(
    "error handling",
    DocumentType::Memory,
    &requesting_agent_id,
)?;

for (agent_id, memories) in results {
    println!("Agent {}: {} memories", agent_id, memories.len());
}
```

### Session Scopes

| Scope | Description |
|-------|-------------|
| `user` | User-level memories |
| `project` | Project-level memories |
| `org` | Organization-level |
| `session` | Current session only |
| `agent` | Agent-specific |

---

## MCP Adapter (Phase 14)

MemSt v1.0 includes a native MCP (Model Context Protocol) adapter for Claude Code, Cursor, and other MCP clients.

### MCP Manifest

```rust
use memst_mcp::adapter::McpAdapter;

let manifest = McpAdapter::manifest();

println!("Tools available:");
for tool in &manifest.tools {
    println!("  - {}", tool.name);
}
```

### MCP Tools

| Tool | Description |
|------|-------------|
| `memory_read` | Read memories by ID or search query |
| `memory_write` | Store new memory |
| `memory_search` | Semantic search across memories |
| `skill_lookup` | Find skills by trigger pattern |
| `context_build` | Assemble context within token budget |

### Tool Examples

```rust
// memory_read
let request = McpRequest {
    tool: "memory_read".into(),
    params: json!({
        "memory_id": "uuid-here",
        "include_embedding": false,
    }),
};
let response = adapter.handle_memory_read(request).await?;

// memory_search
let request = McpRequest {
    tool: "memory_search".into(),
    params: json!({
        "query": "async error handling",
        "limit": 10,
        "memory_type": "Semantic",
    }),
};
let response = adapter.handle_memory_search(request).await?;

// context_build
let request = McpRequest {
    tool: "context_build".into(),
    params: json!({
        "query": "help with Rust",
        "token_budget": 4000,
        "include_skills": true,
    }),
};
let response = adapter.handle_context_build(request).await?;
```

### Integration with Claude Code

```json
// mcp.json configuration
{
  "tools": [
    {
      "name": "memst",
      "command": "memst",
      "args": ["mcp", "serve"],
      "env": {
        "MEMST_STORE_PATH": "./data"
      }
    }
  ]
}
```

---

## Python Bindings

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

### Basic Operations

```python
import memst

# Initialize store
store = memst.SessionStore("./data")

# Create session
session = store.create_session("My Chat", "gpt-4")
print(f"Session ID: {session.id}")

# Add messages
store.add_message(session.id, memst.Role.USER, "Hello!")
store.add_message(session.id, memst.Role.ASSISTANT, "Hi there!")

# Get messages
messages = store.get_session_messages(session.id)
for msg in messages:
    print(f"{msg.role}: {msg.content}")

# Add memory
memory = memst.MemoryItem(
    content="User prefers Python",
    source="conversation",
    tags=["preference", "python"],
    confidence=0.9,
    importance=0.7,
)
store.add_memory(memory)

# Search
results = store.search("python", memst.DocumentType.MEMORY, limit=10)
for doc, score in results:
    print(f"{doc.content}: {score}")

# Context assembly (v1.0)
assembly = store.build_context(
    query="help with async",
    token_budget=4000,
    include_memories=True,
)
print(f"Total tokens: {assembly.total_tokens}")
```

### Context Assembly (Python)

```python
# Build context with configuration
config = memst.ContextAssemblyConfig(
    token_budget=8000,
    reserved_for_response=2000,
    relevance_weight=1.0,
    recency_weight=0.5,
    importance_weight=0.3,
)

assembly = store.build_context_with_config(
    query="error handling patterns",
    config=config,
)

for block in assembly.context_blocks:
    print(f"Block: {block.content[:100]}...")
```

### Memory Tiers (Python)

```python
# Create memory in specific tier
memory = memst.MemoryItem(
    content="Important API key",
    source="user_input",
    tier=memst.MemoryTier.LONG_TERM,
    memory_type=memst.MemoryType.SEMANTIC,
)
store.add_memory(memory)

# Get memories by tier
working_memories = store.get_memories_by_tier(memst.MemoryTier.WORKING)
```

---

## Configuration

MemSt loads LLM + embedding settings from `config.toml` by default (and falls back to environment variables if no config is found).

- Example config: `config.example.toml`
- Override path explicitly: `MEMST_CONFIG_PATH=/path/to/config.toml`
- Discovery: searches the current directory and its parent directories for `config.toml` and `memst-store/config.toml`, then falls back to `~/.config/memst/config.toml`

### Server Configuration

```toml
[server]
port = 8193
store_path = "/tmp/data"
cors_origins = ["http://localhost:3000", "http://127.0.0.1:3000"]
```

### LLM Configuration

```toml
[llm]
api_url = "http://localhost:8080/v1"
model = "gpt-4"
api_key = "sk-xxx"  # Or use LLM_API_KEY env var
timeout_seconds = 30
max_retries = 3
```

### Embedding Configuration

```toml
[embedding]
api_url = "http://localhost:8081/v1"
model = "text-embedding-bge_m3"
api_key = "sk-xxx"  # Or use EMBEDDING_API_KEY env var
dimension = 1024
batch_size = 32
```

### Memory Configuration

```toml
[memory]
working_capacity = 10
short_term_capacity = 100
long_term_capacity = 10000
consolidation_threshold = 0.8
```

### Environment Variables

```bash
# LLM settings
export MEMST_LLM_API_URL="http://localhost:8080/v1"
export MEMST_LLM_MODEL="gpt-4"
export LLM_API_KEY="sk-xxx"

# Embedding settings
export MEMST_EMBEDDING_API_URL="http://localhost:8081/v1"
export MEMST_EMBEDDING_MODEL="text-embedding-bge_m3"
export EMBEDDING_API_KEY="sk-xxx"

# Store path
export MEMST_STORE_PATH="./data"
```

---

## CLI Reference

### Global Options

| Option | Description |
|--------|-------------|
| `--store <PATH>` | Path to store directory |
| `--config <PATH>` | Path to config file |
| `-v, --verbose` | Enable verbose output |
| `-h, --help` | Print help |

### Session Commands

```bash
# Create session
memst session new --name "My Chat" --model "gpt-4"

# List sessions
memst session list
memst session list --model "gpt-4" --tag "project"

# Get session
memst session get <session-id>

# Suspend/Resume
memst session suspend <session-id>
memst session resume <session-id>

# Rename
memst session rename <session-id> "New Name"

# Archive
memst session archive <session-id>
```

### Message Commands

```bash
# Add message
memst message add <session-id> --role user --content "Hello"

# List messages
memst message list <session-id>
memst message list <session-id> --limit 10

# Get message count
memst message count <session-id>
```

### Memory Commands

```bash
# Add memory
memst memory add "User prefers dark mode" --tag preference --importance 0.8

# Search memories
memst search "async" --doc-type memory --limit 20

# Update access
memst memory touch <memory-id>

# Decay memories
memst memory decay --half-life-days 7
```

### Context Commands (v1.0)

```bash
# Build context
memst context build <session-id> --query "help with async" --budget 4000

# Build with skills
memst context build <session-id> --query "deploy service" --include-skills
```

### Repository Commands (v1.0)

```bash
# Initialize MemRepo
memst repo init

# Create commit
memst repo commit --message "Add user preferences"

# Branch operations
memst repo branch create feature-x
memst repo branch list
memst repo branch switch feature-x

# Merge
memst repo merge feature-x --into main

# Log
memst repo log --limit 10
```

### Skill Commands (v1.0)

```bash
# List skills
memst skill list

# Trigger skill
memst skill trigger "deploy the api"

# Execute skill
memst skill execute <skill-id>
```

### MCP Commands (v1.0)

```bash
# Start MCP server
memst mcp serve

# Get manifest
memst mcp manifest
```

---

## API Reference

See [memst-server-api.md](memst-server-api.md) for detailed REST API documentation.

### Quick Reference

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/sessions` | GET | List sessions |
| `/sessions` | POST | Create session |
| `/sessions/{id}` | GET | Get session |
| `/sessions/{id}/messages` | GET | List messages |
| `/sessions/{id}/messages` | POST | Add message |
| `/search` | POST | Search documents |
| `/context/build` | POST | Build context (v1.0) |
| `/memrepo/commit` | POST | Create commit (v1.0) |
| `/memrepo/merge` | POST | Merge branches (v1.0) |
| `/sleep/jobs` | GET | List consolidation jobs (v1.0) |
| `/skills` | GET | List skills (v1.0) |
| `/mcp/tools` | GET | List MCP tools (v1.0) |

---

## Error Handling

MemSt uses structured errors throughout:

```rust
use memst_core::store::StoreError;

match result {
    Ok(value) => println!("Success: {}", value),
    Err(StoreError::NotFound(id)) => println!("Not found: {}", id),
    Err(StoreError::InvalidInput(msg)) => println!("Invalid: {}", msg),
    Err(StoreError::Io(e)) => println!("IO error: {}", e),
    Err(e) => println!("Error: {:?}", e),
}
```

### Common Error Types

| Error | Description |
|-------|-------------|
| `NotFound` | Requested item doesn't exist |
| `InvalidInput` | Invalid parameters provided |
| `AlreadyExists` | Duplicate unique identifier |
| `Io` | File system or network error |
| `Serialization` | JSON/Bincode encoding error |
| `Embedding` | Vector embedding generation failed |

---

## Best Practices

### Memory Management

1. **Use appropriate memory types**: Mark facts as `Semantic`, events as `Episodic`, procedures as `Procedural`.

2. **Set importance explicitly**: Important memories should have high importance scores (>0.7).

3. **Tag consistently**: Use consistent tag naming for better searchability.

4. **Let tier transitions happen naturally**: Don't manually force tier changes unless necessary.

### Context Assembly

1. **Reserve tokens for response**: Always reserve ~25% of context budget for the LLM response.

2. **Adjust weights for use case**: Increase recency weight for time-sensitive queries.

3. **Use progressive disclosure**: Start with `Standard` level, escalate to `Exhaustive` if needed.

### Multi-Agent

1. **Use worktrees for isolation**: Each agent should have its own worktree.

2. **Define clear permissions**: Use the agent registry to control access.

3. **Share via cross-agent search**: Explicitly enable sharing when needed.

### Git-Like Operations

1. **Commit frequently**: Small, focused commits are easier to merge.

2. **Use semantic merge**: Prefer semantic merge over standard merge for memory conflicts.

3. **Tag important states**: Use tags for milestones or releases.

---

## Troubleshooting

### Common Issues

| Issue | Solution |
|-------|----------|
| Session not found | Check UUID format and store path |
| Search returns empty | Verify embedding service is running |
| Context too large | Reduce token budget or max_memories |
| Merge conflicts | Use semantic merge with manual resolution |
| High memory usage | Run consolidation or archive old sessions |

### Debug Logging

```bash
# Enable debug logging
RUST_LOG=debug memst --verbose session list
```

### Performance Tuning

```toml
# config.toml
[performance]
vector_index_cache_size = 1000
search_batch_size = 100
max_concurrent_jobs = 4
```

---

## Migration Notes

### From v0.x to v1.0

1. **Update config format**: Add new sections for context assembly and sleep-time settings.

2. **Re-index embeddings**: v1.0 uses improved embedding storage.

3. **Update Python imports**: Some class names may have changed.

4. **Enable new features**: Context assembly and sleep-time features are opt-in.

```toml
# Add to config.toml for v1.0 features
[context]
enabled = true
token_budget = 8000

[sleep]
enabled = true
consolidation_interval = 3600
```

---

## Appendix: Feature Summary

| Phase | Feature | Status |
|-------|---------|--------|
| 1 | SQLite session storage | ✅ |
| 2 | Message CRUD | ✅ |
| 3 | BM25 full-text search | ✅ |
| 4 | HNSW vector search | ✅ |
| 5 | Hybrid RRF search | ✅ |
| 6 | Git-like objects (Blob/Tree/Commit) | ✅ |
| 7 | Git-like merge (Branch/Merge) | ✅ |
| 8 | Data Model (Blake3, WAL, lifecycle) | ✅ |
| 9 | Context Assembly | ✅ |
| 10 | Sleep-Time Consolidation | ✅ |
| 11 | Semantic Merge | ✅ |
| 12 | Skills | ✅ |
| 13 | Multi-Agent Support | ✅ |
| 14 | MCP Adapter | ✅ |

---

*Document version: 1.0*
*Last updated: 2025*
