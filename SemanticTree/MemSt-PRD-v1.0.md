# MemSt — Advanced Product Requirements Document
## Memory Management System for LLM Agents

**Project:** `github.com/yfyang86/memst`  
**Version:** 1.0 (Merged & Expanded)  
**Date:** 2026-03-06  
**Status:** Draft — In Review

---

## Table of Contents

- [0. Market Signal & Landscape](#0-market-signal--landscape)
- [1. Problem Statement](#1-problem-statement)
- [2. Goals & Non-Goals](#2-goals--non-goals)
- [3. Two-Lane Memory Architecture](#3-two-lane-memory-architecture)
- [4. MemRepo — Git-Alike Store Design](#4-memrepo--git-alike-store-design)
- [5. Data Models](#5-data-models)
- [6. Design Backlog — Items Needing Polish](#6-design-backlog--items-needing-polish)
- [7. API Surface — Endpoints to Add](#7-api-surface--endpoints-to-add)
- [8. Algorithms to Implement](#8-algorithms-to-implement)
- [9. Phased Roadmap](#9-phased-roadmap)
- [10. Open Questions](#10-open-questions)
- [11. Security & Privacy](#11-security--privacy)
- [Appendix A: Competitor Comparison Matrix](#appendix-a-competitor-comparison-matrix)
- [Appendix B: Letta / MemGPT Deep Dive](#appendix-b-letta--memgpt-deep-dive)
- [Appendix C: Proposed Crate Structure](#appendix-c-proposed-crate-structure)
- [Appendix D: Frontmatter Schema Reference](#appendix-d-frontmatter-schema-reference)

---

## 0. Market Signal & Landscape

### 0.1 Why Agent Memory Is Now Critical

LLM agents have crossed from toy demos into production workflows (Claude Code, Cursor, OpenCode, OpenClaw). The common failure mode in all of them is **context amnesia** — agents forget critical facts across sessions, re-derive expensive computations, and produce inconsistent outputs because their memory is ephemeral or unstructured. Memory has become the unsolved infrastructure layer of the agentic stack.

Key signals from 2025–2026 research:

- **MIRIX** (Wang et al., Jul 2025) demonstrates a six-component modular memory architecture (Core, Episodic, Semantic, Procedural, Resource, Knowledge Vault) that outperforms monolithic buffers on long-horizon tasks.
- **A-MEM** (Xu et al., NeurIPS 2025) applies Zettelkasten interconnected-note principles, achieving state-of-the-art results on long-term conversation benchmarks by dynamically linking memories at insertion time.
- **Memory-R1** (Aug 2025) and **MemSearcher** (Nov 2025) show that RL-trained agents (GRPO/PPO) can learn optimal CRUD policies for memory, outperforming rule-based compaction.
- **Letta sleep-time compute** shows asynchronous memory consolidation during idle periods dramatically improves memory quality and reduces in-conversation latency.
- **HiAgent** (Hu et al., 2024) demonstrates hierarchical working-memory chunking via subgoals, pointing to a multi-granular compaction model.

### 0.2 Competitive Landscape

| System | Storage Model | Versioning | Cross-Agent | Offline/Local | MCP Support | Open Source |
|--------|--------------|------------|-------------|---------------|-------------|-------------|
| **MemSt (ours)** | Tiered bin + git DAG | Git-alike | Planned | ✅ | Planned | ✅ |
| Letta / MemGPT | Postgres blocks | None | ✅ | ❌ | ✅ | ✅ |
| Mem0 | Vector DB | None | ✅ | Partial | Partial | ✅ |
| A-MEM | ChromaDB | None | ❌ | ✅ | ❌ | ✅ |
| Zep | Graph + Vector | None | Partial | ❌ | ✅ | Partial |
| LangMem | In-memory | None | ❌ | ✅ | ❌ | ✅ |
| OpenMemory MCP | SQLite + Vector | None | ❌ | ✅ | ✅ | ✅ |
| Claude Code (native) | Context window | None | ❌ | ✅ | N/A | ❌ |

**MemSt's unique positioning:** The only system combining (a) git-like content-addressable versioning, (b) full offline/local operation, (c) tiered memory with compaction checkpoints, and (d) a multi-backend search system with hybrid RRF fusion.

### 0.3 Key Insights Driving This PRD

1. **Structured > unstructured storage.** Graph-backed or note-linked memories outperform flat retrieval (A-MEM, Zep) on multi-hop queries.
2. **Sleep-time consolidation is a first-class concern**, not an afterthought (Letta, Memory-R1).
3. **Versioning is a competitive moat.** No competitor offers rewind/branch/blame on memory state. This is MemSt's most defensible feature.
4. **Multi-agent scoping** (user/project/org/session/agent) is required for real deployment scenarios.
5. **Context window token budget** must be treated as a first-class constraint during retrieval, not post-retrieval filtering.
6. **DAG-based memory evolution** (similar to A-MEM's Zettelkasten approach) allows memories to retroactively update their own metadata as new experiences accrue — a design MemSt should adopt in the Knowledge Graph layer.

### 0.4 Target Agents

MemSt is designed to serve as the memory backend for:

- **Claude Code / Sonnet 4.x** — coding agents needing cross-session project memory
- **OpenCode** — open-source coding agent
- **OpenClaw** — multi-agent orchestration framework  
- **Nanobot** (current MemSt server integration) — local agentic assistant
- **Any OpenAI-compatible agent** via the REST API

---

## 1. Problem Statement

Current MemSt (Phase 1–12) has solved the **storage and search** planes reasonably well: tiered bincode storage, BM25 + HNSW hybrid search, and git-like content-addressable objects are all in place or planned.

**What remains unsolved:**

1. **No compaction lifecycle.** Working memories are never promoted, expired, or summarized by policy. The tier model exists but transitions between tiers are manual.
2. **No token-budget-aware context assembly.** When an agent needs to build its context window, there is no component that intelligently selects and truncates memories within a token budget.
3. **No sleep-time / asynchronous consolidation.** Consolidation is either triggered manually or happens inline during chat, blocking responses.
4. **No multi-agent identity or memory scoping.** Two subagents in the same project can corrupt each other's working memory.
5. **No semantic merge.** When memories conflict, MemSt has no way to reconcile them beyond last-write-wins.
6. **No skill/procedure memory.** MemSt stores facts and conversations but not learned agent workflows (procedural memory).
7. **Knowledge Graph is stateless.** The KG is regenerated from scratch on each parse, losing relationship history and temporal provenance.

---

## 2. Goals & Non-Goals

### Goals (v1.0)

- ✅ Complete git-alike MemRepo with branch/commit/merge/blame/diff
- ✅ Token-budget-aware context assembly API
- ✅ Compaction lifecycle with lossless rewind via checkpoints
- ✅ Asynchronous sleep-time consolidation pipeline
- ✅ Skill/procedure memory CRUD + matching
- ✅ Multi-agent scoping with worktree isolation
- ✅ Incremental KG maintenance with temporal provenance
- ✅ Three-way semantic merge for conflicting memories
- ✅ MCP server adapter layer (for Claude, Cursor, etc.)
- ✅ Import/export for Letta `.af` format and OpenAI thread format

### Non-Goals (v1.0)

- ❌ CRDT-based conflict-free merging (deferred to Phase 7+)
- ❌ Parametric memory (fine-tuning model weights from MemSt data)
- ❌ Multimodal memory (audio, video) — text and image attachments only
- ❌ Distributed/sharded MemRepo across nodes
- ❌ Real-time streaming KG updates (batch updates only)

---

## 3. Two-Lane Memory Architecture

### 3.1 Lane A — Ephemeral (Session-Scoped)

Lane A is the **hot path**: memory that lives within a single agent session. It is fast, mutable, and directly injected into the context window.

```
┌─────────────────────────────────────┐
│          CONTEXT WINDOW             │
│  ┌──────────┐  ┌──────────────────┐ │
│  │  System  │  │  Working Memory  │ │
│  │ Prompt   │  │  (Lane A, tier 0)│ │
│  └──────────┘  └──────────────────┘ │
│  ┌──────────────────────────────────┤
│  │    Active Conversation Buffer    │
│  └──────────────────────────────────┤
└─────────────────────────────────────┘
         │
         ▼  token budget exhausted / round threshold
┌─────────────────────────────────────┐
│       SHORT-TERM MEMORY (tier 1)    │
│   Summarized turns, tagged facts    │
└─────────────────────────────────────┘
```

**Lane A properties:**
- Stored in `working.bin` / `short.bin`
- Max token budget enforced at assembly time
- Eviction policy: LRU + importance score
- TTL: session lifetime (no cross-session persistence by default)

### 3.2 Lane B — Persistent (Cross-Session)

Lane B is the **cold path**: memory that survives session boundaries, is versioned via MemRepo commits, and is retrieved semantically.

```
┌──────────────────────────────────────────────┐
│            LONG-TERM MEMORY (tier 2)         │
│  ┌───────────────┐  ┌───────────────────────┐│
│  │ Semantic Facts │  │  Episodic Summaries   ││
│  └───────────────┘  └───────────────────────┘│
│  ┌───────────────┐  ┌───────────────────────┐│
│  │  Skill/Proc.  │  │  Knowledge Graph      ││
│  │   Memory      │  │  (Entity + Relations) ││
│  └───────────────┘  └───────────────────────┘│
└──────────────────────────────────────────────┘
         │
         ▼  MemRepo commit
┌──────────────────────────────────────────────┐
│              ARCHIVAL (tier 3)               │
│  Content-addressable packfiles, blob store   │
│  Full history, diff/blame/rewind available   │
└──────────────────────────────────────────────┘
```

**Lane B properties:**
- Versioned via MemRepo (git-alike DAG)
- Retrieved via hybrid BM25 + HNSW search
- Scope: `user` / `project` / `org` / `agent` / `session`
- Compaction checkpoints enable lossless rewind

### 3.3 Tier Model (Orthogonal to Lanes)

The tier model is an *implementation dimension* that cuts across both lanes:

| Tier | Name | Lane | Retention | Token Cost | Update Freq |
|------|------|------|-----------|------------|-------------|
| 0 | Working | A | Session | High (direct inject) | Every turn |
| 1 | Short-Term | A | Session / configurable | Medium (summarized) | Every N turns |
| 2 | Long-Term | B | Permanent | Low (retrieved on demand) | Per consolidation |
| 3 | Archival | B | Permanent | Zero (not in context) | Per commit |

### 3.4 Memory Type Taxonomy

Following MIRIX and cognitive science frameworks, MemSt should distinguish these memory types stored across tiers:

| Type | Description | Primary Tier |
|------|-------------|-------------|
| **Episodic** | "What happened" — conversation turns, event summaries | 0 → 1 |
| **Semantic** | "What is true" — facts, preferences, world knowledge | 1 → 2 |
| **Procedural / Skill** | "How to do X" — learned agent workflows, tool usage patterns | 2 |
| **Resource** | File paths, URLs, external references | 2 |
| **Meta-cognitive** | Agent's beliefs about its own knowledge quality, confidence | 2 |

---

## 4. MemRepo — Git-Alike Store Design

### 4.1 Repository Layout

```
memst-store/
├── config.toml                     # Store config (port, LLM, embedding)
├── manifest.json                   # Session index (grep-friendly)
├── schema_version                  # "1.1.0" (plain text)
├── store.lock                      # Advisory lock
│
├── objects/                        # Content-addressable object store
│   ├── {xx}/                       # 2-char prefix sharding
│   │   └── {rest-of-hash}          # Blake3 hash, zstd-compressed
│   └── pack/                       # Packfiles for compacted objects
│       ├── {pack-id}.pack
│       └── {pack-id}.idx
│
├── refs/
│   ├── heads/                      # Branch tips
│   │   ├── main
│   │   └── {session-id}
│   ├── tags/                       # Annotated snapshots
│   └── agents/                     # Per-agent HEADs
│       └── {agent-id}
│
├── HEAD                            # Current branch ref
├── AGENT_HEAD                      # Current agent ref
│
├── context/                        # Working-tree view (markdown + YAML frontmatter)
│   ├── working/
│   │   └── {session-id}.md
│   ├── skills/
│   │   └── {skill-slug}.md
│   └── entities/
│       └── {entity-id}.md
│
├── sessions/                       # Session runtime data
│   └── {session-id}/
│       ├── metadata.json
│       ├── messages.pack           # Bincode + zstd
│       ├── messages.idx            # Human-readable offset index
│       └── operations.log         # JSON Lines append-only ops log
│
├── memories/                       # Tier storage
│   ├── working.bin
│   ├── short.bin
│   └── long.bin
│
├── search_index/
│   ├── native/
│   └── tantivy/
│
├── vectors/                        # HNSW index persistence
│   └── hnsw.bin
│
└── worktrees/                      # Concurrent subagent isolation
    └── {worktree-id}/
        └── {session-id}/
```

### 4.2 Object Model

Every object is identified by its **Blake3 hash** (upgrade from SHA-256 — 3× faster, parallel).

```rust
pub enum ObjectType {
    Blob,           // Raw bytes: message content, memory fact, file
    Tree,           // Directory: maps name → ObjectId
    Commit,         // Versioned snapshot of a Tree
    Tag,            // Annotated pointer to a Commit
    Skill,          // Procedural memory: trigger + steps + metadata
    ContextFile,    // Markdown file in the `context/` working tree
    Entity,         // Knowledge graph node
    Relation,       // Knowledge graph edge
}

pub struct ObjectId(pub [u8; 32]); // Blake3 digest

pub struct CommitMetadata {
    pub tree:           ObjectId,
    pub parents:        Vec<ObjectId>,
    pub author:         Author,
    pub timestamp:      DateTime<Utc>,
    pub message:        String,
    pub token_delta:    i32,               // Token budget change
    pub confidence:     f32,               // 0.0–1.0
    pub source:         CommitSource,
    pub scope:          MemoryScope,
}

pub enum CommitSource {
    UserExplicit,         // User manually triggered
    AgentInline,          // Agent wrote during conversation
    SleepConsolidation,   // Async consolidation job
    SkillLearning,        // Skill extraction from conversation
    ImportExternal,       // Imported from Letta/.af/OpenAI format
    Merge,                // Three-way semantic merge
}

pub enum MemoryScope {
    User(UserId),
    Project(ProjectId),
    Org(OrgId),
    Session(SessionId),
    Agent(AgentId),
}
```

### 4.3 Context/ Working Tree

The `context/` directory provides a **human-readable view** of the current memory state. Files use markdown with YAML frontmatter for rich metadata. This is what agents write to and read from during normal operation.

```markdown
---
id: "mem-550e8400"
type: semantic
tier: long
scope: project/my-rust-project
tags: [rust, async, tokio]
importance: 0.85
confidence: 0.92
created_at: 2026-01-15T10:22:00Z
updated_at: 2026-03-01T08:14:00Z
commit_hash: "a3f8c2d..."
supersedes: "mem-44aabb11"
token_estimate: 48
embedding_model: text-embedding-bge_m3
embedding_version: "1.0"
source: agent_inline
---

# User prefers Tokio over async-std for async Rust projects

The user has consistently chosen Tokio as the async runtime across
three separate projects (alpha, beta, gamma). They cited ecosystem
compatibility and tower middleware as primary reasons.

**Related:** [[entity/tokio]], [[entity/async-std]], [[skill/rust-async-setup]]
```

### 4.4 Branching Model

| Branch Type | Name Pattern | Purpose |
|------------|-------------|---------|
| Main | `main` | Stable, consolidated memory |
| Session | `session/{id}` | Per-session working branch |
| Agent | `agent/{id}` | Per-agent isolation |
| Feature | `feat/{description}` | Experimental memory additions |
| Sleep | `sleep/{timestamp}` | Consolidation job output |

**Merge policy:**

```
session branches → merge into agent branch on session end (3-way semantic merge)
agent branches  → merge into main at project checkpoints (with conflict resolution)
sleep branches  → fast-forward merge into main (no conflicts by design)
```

### 4.5 Packfile Format

Packfiles bundle loose objects for efficiency:

```
pack-{blake3-id}.pack
├── Header: magic (4B) + version (4B) + object_count (4B)
├── Objects: [type(1B) | delta_flag(1B) | compressed_size(4B) | data(N)]
│            delta objects reference base by ObjectId
└── Trailer: Blake3 checksum of entire pack

pack-{blake3-id}.idx
├── Fan-out table: 256 × u32 (cumulative count per first byte)
├── Sorted ObjectIds: N × 32B
├── CRCs: N × u32
└── Offsets: N × u64
```

---

## 5. Data Models

### 5.1 Core Structs

```rust
pub struct Session {
    pub id:             SessionId,          // UUID v7
    pub name:           String,
    pub model:          String,
    pub tags:           Vec<String>,
    pub user_id:        UserId,
    pub agent_id:       Option<AgentId>,
    pub session_type:   SessionType,
    pub status:         SessionStatus,
    pub scope:          MemoryScope,
    pub branch:         String,             // Active git branch
    pub created_at:     DateTime<Utc>,
    pub updated_at:     DateTime<Utc>,
    pub message_count:  u32,
    pub token_used:     u32,
    pub token_budget:   u32,               // Context window limit
}

pub enum SessionType {
    Chat,
    Agent,
    SleepTime,          // Async consolidation session
    Subagent,           // Spawned sub-agent
    Reflection,         // Meta-cognitive reflection pass
}

pub enum SessionStatus {
    Active,
    Idle,               // No activity, eligible for sleep-time
    Sleeping,           // Sleep-time consolidation in progress
    Archived,
    Error(String),
}

pub struct Message {
    pub id:                 MessageId,
    pub session_id:         SessionId,
    pub role:               Role,
    pub content:            Content,
    pub timestamp:          DateTime<Utc>,
    pub metadata:           MessageMetadata,
}

pub struct MessageMetadata {
    pub model:              Option<String>,
    pub usage:              Option<TokenUsage>,
    pub reasoning_tokens:   Option<u32>,    // For thinking-mode models
    pub is_compacted:       bool,           // True if this is a compaction summary
    pub original_id:        Option<MessageId>, // Points to pre-compaction message
    pub tool_calls:         Vec<ToolCall>,
    pub attachments:        Vec<Attachment>,
}

pub struct MemoryItem {
    pub id:             MemoryId,
    pub session_id:     Option<SessionId>,  // None = cross-session
    pub scope:          MemoryScope,
    pub content:        String,
    pub tier:           MemoryTier,
    pub memory_type:    MemoryType,
    pub tags:           Vec<String>,
    pub importance:     f32,                // 0.0–1.0
    pub confidence:     f32,               // 0.0–1.0
    pub token_estimate: u32,
    pub embedding:      Option<EmbeddingRef>,  // Reference to vector store
    pub commit_hash:    Option<ObjectId>,      // MemRepo commit
    pub supersedes:     Option<MemoryId>,
    pub retracted_by:   Option<MemoryId>,
    pub created_at:     DateTime<Utc>,
    pub last_accessed:  DateTime<Utc>,
    pub access_count:   u32,
}

pub enum MemoryType {
    Episodic,
    Semantic,
    Procedural,
    Resource,
    MetaCognitive,
}

pub struct Skill {
    pub id:             SkillId,
    pub slug:           String,             // human-readable key
    pub name:           String,
    pub description:    String,
    pub trigger_patterns: Vec<String>,      // Regex or semantic triggers
    pub steps:          Vec<SkillStep>,
    pub success_rate:   f32,
    pub usage_count:    u32,
    pub source_session: Option<SessionId>,  // Where it was learned
    pub commit_hash:    ObjectId,
    pub created_at:     DateTime<Utc>,
}

pub struct SkillStep {
    pub order:      u8,
    pub action:     String,
    pub tool:       Option<String>,
    pub conditions: Vec<String>,
    pub on_failure: SkillFailurePolicy,
}

pub enum SkillFailurePolicy {
    Abort,
    Skip,
    Retry(u8),
    Fallback(SkillId),
}

pub struct Entity {
    pub id:             EntityId,
    pub label:          String,
    pub entity_type:    EntityType,
    pub attributes:     HashMap<String, serde_json::Value>,
    pub commit_hash:    ObjectId,
    pub first_seen:     DateTime<Utc>,
    pub last_updated:   DateTime<Utc>,
    pub source_memories: Vec<MemoryId>,
}

pub struct Relation {
    pub id:             RelationId,
    pub from:           EntityId,
    pub to:             EntityId,
    pub label:          String,
    pub weight:         f32,
    pub confidence:     f32,
    pub source_memories: Vec<MemoryId>,
    pub created_at:     DateTime<Utc>,
}
```

### 5.2 Agent Registry

```rust
pub struct AgentRecord {
    pub id:             AgentId,
    pub name:           String,
    pub model:          String,
    pub capabilities:   Vec<String>,
    pub memory_scopes:  Vec<MemoryScope>,   // What this agent can read
    pub write_scope:    MemoryScope,        // What this agent can write
    pub worktree_id:    Option<WorktreeId>,
    pub head_commit:    Option<ObjectId>,
    pub registered_at:  DateTime<Utc>,
    pub last_active:    DateTime<Utc>,
}
```

### 5.3 Consolidation Job

```rust
pub struct ConsolidationJob {
    pub id:             JobId,
    pub scope:          MemoryScope,
    pub source_branch:  String,
    pub target_branch:  String,
    pub status:         JobStatus,
    pub trigger:        ConsolidationTrigger,
    pub memories_in:    u32,
    pub memories_out:   u32,
    pub token_saved:    i32,
    pub created_at:     DateTime<Utc>,
    pub finished_at:    Option<DateTime<Utc>>,
}

pub enum ConsolidationTrigger {
    Scheduled,
    TokenBudgetExceeded,
    SessionIdle(Duration),
    ExplicitAPI,
}
```

---

## 6. Design Backlog — Items Needing Polish

### 6.1 Data Model Gaps

| # | Item | Priority | Status | Notes |
|---|------|----------|--------|-------|
| D1 | Token counting pluggability | P0 | ❌ Missing | Need `TokenCounter` trait; tiktoken + HuggingFace tokenizers |
| D2 | Compaction lifecycle state machine | P0 | ❌ Missing | Define Working→Short→Long→Archival transitions + triggers |
| D3 | Embedding storage format & versioning | P0 | Partial | `expected_dimension` exists; need model version hash for invalidation |
| D4 | Skill representation | P1 | ❌ Missing | Full `Skill` struct, trigger patterns, step DAG |
| D5 | Multi-agent identity & scoping | P1 | ❌ Missing | `AgentRecord`, `MemoryScope`, per-agent write isolation |
| D6 | Session lifecycle state machine | P1 | Partial | `status` field exists; transitions not implemented |
| D7 | Frontmatter schema (context/ files) | P1 | ❌ Missing | YAML spec, required vs optional fields, version |
| D8 | Attachment handling in memories | P2 | ❌ Missing | Binary blobs in MemRepo object store, MIME type tracking |
| D9 | Meta-cognitive memory type | P2 | ❌ Missing | Agent confidence in its own knowledge; feeds retrieval scoring |
| D10 | Relation weight & confidence decay | P2 | ❌ Missing | Time-decay function for KG edge weights |

### 6.2 Storage Layer Gaps

| # | Item | Priority | Status | Notes |
|---|------|----------|--------|-------|
| S1 | Blake3 upgrade (from SHA-256) | P0 | ❌ Missing | 3× faster; parallel hashing for pack generation |
| S2 | Packfile implementation | P0 | Partial | Object store exists; delta compression + idx missing |
| S3 | Write-ahead log (WAL) for crash recovery | P0 | ❌ Missing | Pre-write journal before object store commit |
| S4 | Worktree lifecycle (create/attach/prune) | P1 | ❌ Missing | Subagent isolation; auto-prune on session end |
| S5 | Reflog (per-ref history) | P1 | ❌ Missing | `refs/logs/{branch}` — enables `git reflog`-style recovery |
| S6 | Shallow clone for export | P2 | ❌ Missing | Export last N commits only, for agent hand-off |
| S7 | Object TTL / expiry policy | P2 | ❌ Missing | Configurable TTL per tier; GC sweep |
| S8 | Vector store persistence across restarts | P1 | Partial | HNSW build on startup; need incremental persist |

### 6.3 Existing API Gaps

| # | Item | Priority | Notes |
|---|------|----------|-------|
| A1 | No `POST /memory/consolidate` endpoint | P0 | Trigger compaction manually |
| A2 | No token budget in context assembly | P0 | Retrieval ignores `token_budget` |
| A3 | No worktree management endpoints | P1 | Create/list/merge worktrees |
| A4 | No skill CRUD | P1 | Procedural memory is entirely absent |
| A5 | No agent registry CRUD | P1 | Agents not tracked in the store |
| A6 | No sleep-time job management | P1 | No way to schedule/monitor consolidation |
| A7 | No import/export (Letta/OpenAI format) | P2 | Interoperability |
| A8 | No blame/diff on memory commits | P2 | Audit trail for memory changes |
| A9 | No MCP adapter endpoint | P1 | Needed for Claude Code, Cursor integration |
| A10 | No redaction endpoint | P2 | GDPR/privacy — tombstone PII from history |

---

## 7. API Surface — Endpoints to Add

Base URL: `http://127.0.0.1:8193/api/v1`

### 7.1 MemRepo / Version Control

```http
# Branch operations
GET    /memrepo/branches                          # List all branches
POST   /memrepo/branches                          # Create branch
DELETE /memrepo/branches/{branch}                 # Delete branch
POST   /memrepo/branches/{branch}/checkout        # Checkout branch
POST   /memrepo/merge                             # Merge branch into target

# Commit operations
GET    /memrepo/commits/{ref}                     # Get commit by ref/hash
GET    /memrepo/log?branch=main&limit=20          # Commit history
POST   /memrepo/commit                            # Create a manual commit
GET    /memrepo/diff?from={hash}&to={hash}        # Diff between commits
GET    /memrepo/blame/{memory_id}                 # Per-memory change history
POST   /memrepo/rewind/{hash}                     # Rewind to checkpoint
GET    /memrepo/reflog?ref=main                   # Reflog for a ref

# Worktree operations
GET    /memrepo/worktrees                         # List worktrees
POST   /memrepo/worktrees                         # Create worktree for agent
DELETE /memrepo/worktrees/{id}                    # Remove worktree (merge or discard)
POST   /memrepo/worktrees/{id}/merge              # Merge worktree into main
```

**Create branch request:**
```json
{
  "name": "session/abc-123",
  "from": "main",
  "scope": { "type": "session", "id": "abc-123" }
}
```

**Merge request:**
```json
{
  "source_branch": "session/abc-123",
  "target_branch": "main",
  "strategy": "semantic_3way",
  "author": { "name": "agent-1", "email": "agent@memst" },
  "message": "Merge session abc-123 on completion"
}
```

**Merge response:**
```json
{
  "commit_hash": "a3f8c2d1...",
  "fast_forward": false,
  "conflicts": [
    {
      "memory_id": "mem-xyz",
      "conflict_type": "semantic_contradiction",
      "ours": "User prefers Tokio",
      "theirs": "User prefers async-std",
      "resolution": "merged",
      "merged_content": "User initially preferred async-std but switched to Tokio in 2025"
    }
  ],
  "memories_added": 14,
  "memories_removed": 2,
  "token_delta": 320
}
```

### 7.2 Context Assembly (Token-Budget-Aware)

```http
POST /context/build
```

**Request:**
```json
{
  "session_id": "abc-123",
  "token_budget": 8192,
  "reserved_for_response": 2048,
  "query": "help me debug this Tokio timeout issue",
  "tiers": ["working", "short", "long"],
  "memory_types": ["episodic", "semantic", "procedural"],
  "include_skills": true,
  "include_entities": true,
  "max_memories": 20,
  "recency_weight": 0.3,
  "relevance_weight": 0.5,
  "importance_weight": 0.2
}
```

**Response:**
```json
{
  "context_blocks": [
    {
      "type": "system_context",
      "content": "...",
      "token_count": 512
    },
    {
      "type": "memory",
      "memory_id": "mem-001",
      "tier": "long",
      "memory_type": "semantic",
      "content": "User prefers Tokio...",
      "token_count": 48,
      "score": 0.94,
      "score_breakdown": {
        "relevance": 0.96,
        "recency": 0.82,
        "importance": 0.85,
        "coherence": 0.99
      }
    }
  ],
  "total_tokens": 6144,
  "budget_remaining": 2048,
  "memories_included": 18,
  "memories_skipped": 43,
  "truncated": false,
  "assembly_time_ms": 12
}
```

### 7.3 Compaction & Consolidation

```http
# Trigger consolidation
POST /sessions/{session_id}/memory/consolidate
POST /memory/consolidate                          # Cross-session (scope-based)

# Sleep-time jobs
GET    /sleep/jobs                                # List consolidation jobs
POST   /sleep/jobs                                # Schedule consolidation job
GET    /sleep/jobs/{job_id}                       # Job status
DELETE /sleep/jobs/{job_id}                       # Cancel job
POST   /sleep/jobs/{job_id}/run                   # Force run immediately

# Compaction checkpoints
GET    /memrepo/checkpoints                       # List compaction checkpoints
POST   /memrepo/checkpoints                       # Create checkpoint before compaction
POST   /memrepo/checkpoints/{id}/restore          # Restore from checkpoint
```

**Consolidation request:**
```json
{
  "scope": { "type": "session", "id": "abc-123" },
  "source_tier": "working",
  "target_tier": "short",
  "max_output_tokens": 1024,
  "strategy": "hierarchical_summary",
  "commit_to_repo": true,
  "branch": "sleep/2026-03-06T02:00:00Z"
}
```

### 7.4 Skills (Procedural Memory)

```http
GET    /skills                                    # List skills
POST   /skills                                    # Create skill
GET    /skills/{skill_id}                         # Get skill
PUT    /skills/{skill_id}                         # Update skill
DELETE /skills/{skill_id}                         # Delete skill
POST   /skills/match                              # Find skills matching a query
POST   /skills/{skill_id}/record-outcome          # Record success/failure
POST   /skills/extract                            # Extract skill from session history
```

**Skill match request:**
```json
{
  "query": "set up a new Rust async project with Tokio",
  "context": "Starting a new Rust project",
  "limit": 5,
  "min_confidence": 0.7
}
```

### 7.5 Agent Registry

```http
GET    /agents                                    # List registered agents
POST   /agents                                    # Register agent
GET    /agents/{agent_id}                         # Get agent record
PUT    /agents/{agent_id}                         # Update agent
DELETE /agents/{agent_id}                         # Deregister agent
GET    /agents/{agent_id}/memory                  # Get agent-scoped memories
POST   /agents/{agent_id}/memory/search           # Search within agent scope
POST   /agents/memory/cross-search                # Search across agent scopes
```

### 7.6 Knowledge Graph (Enhanced)

```http
GET    /sessions/{session_id}/knowledge-graph          # Current KG
POST   /sessions/{session_id}/knowledge-graph/parse    # Force regenerate
POST   /knowledge-graph/update                         # Incremental update (append events)
GET    /knowledge-graph/entities/{entity_id}           # Entity details + provenance
GET    /knowledge-graph/entities/{entity_id}/history   # Entity change history
POST   /knowledge-graph/query                          # Multi-hop graph query
GET    /knowledge-graph/diff?from={hash}&to={hash}     # KG diff between commits
```

**Multi-hop query:**
```json
{
  "start_entity": "tokio",
  "relation_path": ["used_in", "has_dependency"],
  "max_hops": 3,
  "min_edge_confidence": 0.7
}
```

### 7.7 MCP Adapter

```http
GET    /mcp/manifest                              # MCP tool manifest for agents
POST   /mcp/tools/memory_read                     # MCP-format memory read
POST   /mcp/tools/memory_write                    # MCP-format memory write
POST   /mcp/tools/memory_search                   # MCP-format search
POST   /mcp/tools/skill_lookup                    # MCP-format skill lookup
```

**MCP manifest response** (consumed by Claude Code, Cursor, etc.):
```json
{
  "tools": [
    {
      "name": "memory_read",
      "description": "Read memories relevant to a query, respecting token budget",
      "input_schema": {
        "type": "object",
        "properties": {
          "query": { "type": "string" },
          "token_budget": { "type": "integer" },
          "tiers": { "type": "array", "items": { "type": "string" } }
        }
      }
    }
  ]
}
```

### 7.8 Import / Export

```http
POST /import/letta                                # Import Letta .af agent file
POST /import/openai-threads                       # Import OpenAI thread export
POST /import/claude-projects                      # Import Claude project memory
GET  /export/letta/{session_id}                   # Export as Letta .af format
GET  /export/openai/{session_id}                  # Export as OpenAI thread format
GET  /export/archive/{scope}                      # Full archive export (tar.zst)
```

### 7.9 Redaction

```http
POST /redact                                      # Redact PII from memory + history
GET  /redact/jobs/{job_id}                        # Redaction job status

# Request
{
  "patterns": ["\\b\\d{3}-\\d{2}-\\d{4}\\b"],   # SSN pattern example
  "scope": { "type": "user", "id": "user-123" },
  "dry_run": true,
  "create_tombstone": true
}
```

### 7.10 Search (Additions to Existing)

```http
POST /search/temporal                             # Time-bounded search
POST /search/graph                                # KG-guided semantic search
GET  /search/strategy?query={q}                   # (existing — already implemented)

# Temporal search request
{
  "query": "database design decisions",
  "after": "2026-01-01T00:00:00Z",
  "before": "2026-03-01T00:00:00Z",
  "search_type": "hybrid",
  "limit": 20
}
```

---

## 8. Algorithms to Implement

### 8.0 Page-Index Concept (Retained from Original)

Before vector retrieval, maintain a **page index** — a sparse inverted index over memory "pages" (logical groupings of related memories by topic). This provides O(1) topic-level filtering before full vector search:

```
page_index = {
  "rust/async":    [mem-001, mem-042, mem-105],
  "project/alpha": [mem-001, mem-017, mem-042],
  "skill/tokio":   [mem-042, mem-105],
}
```

### 8.1 Multi-Signal Retrieval Scoring

Given a query `q` and candidate memory `m`, the composite retrieval score is:

```
score(q, m) = w_r · relevance(q, m)
            + w_t · recency_decay(m.last_accessed)
            + w_i · m.importance
            + w_c · coherence(m, context_window)
            + w_s · m.confidence
            - w_p · provenance_penalty(m.source)
```

Where:
- `relevance(q, m)` = hybrid BM25 + cosine similarity (RRF-fused)
- `recency_decay(t)` = `exp(-λ · (now - t).days)`, λ ∈ [0.01, 0.1]
- `coherence(m, ctx)` = 1 - semantic_similarity(m, ctx_centroid) if centroid is saturated
- `provenance_penalty` = small penalty for `ImportExternal` sources vs `AgentInline`

Default weights: `w_r=0.50, w_t=0.20, w_i=0.20, w_c=0.05, w_s=0.05`

The `coherence` term prevents over-loading the context with near-duplicate memories.

### 8.2 Token-Budget-Aware Progressive Disclosure

```
Algorithm: BuildContext(query, budget, session, tiers)

Input:  query string, budget_tokens int, session SessionId, tiers []Tier
Output: ordered list of ContextBlock

1. reserved  ← budget * RESPONSE_RESERVE_RATIO   # default 0.25
2. available ← budget - reserved - system_prompt_tokens
3. candidates ← MultiSignalRetrieve(query, session, tiers, max=100)
4. selected   ← []
5. used       ← 0

6. For block in candidates sorted by score DESC:
   a. est ← estimate_tokens(block.content)
   b. if used + est > available: continue
   c. if would_create_contradiction(block, selected):
        if block.confidence > conflicting.confidence:
            replace conflicting with block
        else: continue
   d. selected.append(block)
   e. used += est
   f. if len(selected) >= MAX_MEMORIES: break

7. Insert working-memory blocks first (always included up to tier-0-budget)
8. Sort selected by [tier ASC, score DESC]
9. Return selected with usage stats
```

### 8.3 Incremental Memory Consolidation (Sleep-Time)

```
Algorithm: SleepConsolidate(scope, source_tier, target_tier, budget)

1. job ← CreateJob(scope, source_tier, target_tier)
2. checkpoint ← CreateCheckpoint(scope)          # Lossless rewind point
3. branch ← CreateBranch("sleep/" + timestamp, from="main")

4. memories ← LoadTier(scope, source_tier)
5. groups   ← TopicCluster(memories)             # k-means on embeddings

6. For group in groups:
   a. sorted ← SortByImportance(group)
   b. if TokenSum(sorted) < CONSOLIDATION_THRESHOLD: continue
   
   c. summary ← LLM.Summarize(sorted,
        prompt="Distill these memories preserving key facts. Max {budget/groups} tokens.")
   
   d. new_mem ← MemoryItem {
        content:     summary,
        tier:        target_tier,
        importance:  max(m.importance for m in sorted),
        confidence:  mean(m.confidence for m in sorted),
        supersedes:  [m.id for m in sorted],
      }
   
   e. CommitMemory(new_mem, branch)
   f. TombstoneMemories(sorted, branch)          # Soft-delete originals

7. merge_result ← SemanticMerge(branch, "main")
8. job.finish(memories_in=len(memories), memories_out=len(groups))
9. Return merge_result
```

### 8.4 Three-Way Semantic Merge

When merging a session branch into main, conflicts arise when two branches diverge from the same base commit.

```
Algorithm: SemanticMerge(source, target, base_commit)

1. base_mems   ← LoadMemoriesAtCommit(base_commit)
2. source_mems ← LoadMemoriesAtCommit(source)
3. target_mems ← LoadMemoriesAtCommit(target)

4. added_in_source  ← source_mems - base_mems
5. added_in_target  ← target_mems - base_mems
6. deleted_in_source ← base_mems - source_mems
7. deleted_in_target ← base_mems - target_mems

8. # Detect semantic conflicts (not just ID conflicts)
   For s in added_in_source:
     For t in added_in_target:
       if cosine_sim(s.embedding, t.embedding) > CONFLICT_THRESHOLD:
         if Contradicts(s.content, t.content):  # LLM-based check
           conflicts.append(Conflict(s, t, type=CONTRADICTION))
         elif Duplicates(s, t):
           conflicts.append(Conflict(s, t, type=DUPLICATION))

9. For conflict in conflicts:
     if conflict.type == DUPLICATION:
       keep ← argmax(s.confidence, t.confidence)
       discard ← the other
     elif conflict.type == CONTRADICTION:
       if abs(s.timestamp - t.timestamp) < 7 days:
         merged ← LLM.Reconcile(s, t)           # "User preferred X then switched to Y"
       else:
         keep ← more_recent                      # Recency wins for factual updates

10. Commit merged state to target
11. Return MergeResult { fast_forward, conflicts_found, conflicts_resolved, commit_hash }
```

### 8.5 Compaction with Lossless Rewind

```
Algorithm: CompactTier(scope, tier, max_output_tokens)

Pre-condition: Checkpoint exists at current HEAD

1. orig_commit ← HEAD
2. objects     ← LooseObjects(tier)
3. if len(objects) < PACK_THRESHOLD: return  # Nothing to compact

4. # Generate delta-compressed packfile
   sorted ← TopoSort(objects, by=dependency)
   base_candidates ← SelectBases(sorted)      # Most-referenced objects
   
   For obj in sorted:
     best_base ← FindBestDelta(obj, base_candidates)
     if DeltaRatio(obj, best_base) > 0.5:
       pack.append(DeltaObject(obj, base=best_base))
     else:
       pack.append(FullObject(obj))
       base_candidates.add(obj)
   
5. pack_id ← WritePack(pack)
   WriteIndex(pack_id)
   DeleteLooseObjects(objects)

6. # Verify: every object still reachable
   assert Verify(pack_id, orig_commit)
```

### 8.6 Knowledge Graph Incremental Update

```
Algorithm: KGUpdate(new_memories, existing_graph)

1. For mem in new_memories:
   a. entities ← ExtractEntities(mem.content)   # NER via LLM or spaCy
   b. relations ← ExtractRelations(mem.content, entities)

   c. For entity in entities:
        existing ← graph.FindSimilar(entity, threshold=0.9)
        if existing:
          MergeEntity(existing, entity)          # Update attributes
        else:
          graph.AddEntity(entity)

   d. For relation in relations:
        existing ← graph.FindRelation(rel.from, rel.to, rel.label)
        if existing:
          existing.weight   = ewma(existing.weight, relation.weight)
          existing.confidence = max(existing.confidence, relation.confidence)
          existing.source_memories.append(mem.id)
        else:
          graph.AddRelation(relation)

   e. # Contradiction detection
      For entity in entities:
        conflicting ← graph.FindContradictions(entity)
        if conflicting:
          EmitConflict(entity, conflicting, source=mem.id)

2. Commit KG snapshot to MemRepo as ContextFile objects
```

### 8.7 Skill Extraction

```
Algorithm: ExtractSkills(session_id, min_success_rate)

1. messages ← GetSessionMessages(session_id)
2. ops      ← GetSessionTrace(session_id)

3. # Find successful tool sequences
   sequences ← FindToolSequences(ops, outcome=success)

4. For seq in sequences:
   a. pattern ← LLM.GenerateTrigger(seq.context, seq.inputs)
   b. steps   ← LLM.AbstractSteps(seq.tool_calls)
   c. similar ← MatchExistingSkill(pattern, threshold=0.85)
   
   d. if similar:
        similar.usage_count += 1
        similar.success_rate = ewma(similar.success_rate, 1.0)
        CommitSkillUpdate(similar)
      else:
        skill ← Skill { trigger_patterns=[pattern], steps=steps, ... }
        CommitNewSkill(skill)

5. Return extracted_skills
```

### 8.8 A-MEM-Inspired Memory Evolution

When a new memory is added, it should retroactively update **existing** related memories' attributes (following the Zettelkasten / A-MEM pattern):

```
Algorithm: EvolveMemoryNetwork(new_mem, store)

1. embedding   ← Embed(new_mem.content)
2. neighbors   ← HNSW.Search(embedding, k=10, threshold=0.7)

3. For neighbor in neighbors:
   a. relation ← LLM.ClassifyRelation(new_mem, neighbor)
      # Returns: SUPPORTS | CONTRADICTS | ELABORATES | SUPERSEDES | UNRELATED

   b. if relation == SUPPORTS:
        neighbor.confidence = min(1.0, neighbor.confidence + 0.05)
        new_mem.confidence  = min(1.0, new_mem.confidence + 0.03)

   c. elif relation == CONTRADICTS:
        EmitConflict(new_mem, neighbor)
        # Resolution deferred to next merge or explicit API call

   d. elif relation == ELABORATES:
        Link(new_mem.id → neighbor.id, label="elaborates")
        neighbor.context_note += f"\nElaborated by: {new_mem.id}"

   e. elif relation == SUPERSEDES:
        new_mem.supersedes  = neighbor.id
        neighbor.retracted_by = new_mem.id
        DowngradeTier(neighbor)                # Moved to archival

4. CommitEvolution(new_mem, affected_neighbors)
```

### 8.9 Safety Filters (Retained from Original)

Before any memory is committed:

```
function SafetyFilter(memory: MemoryItem) → FilterResult:
  1. PIIDetect(memory.content)      # Regex + NER: SSN, CC, phone, email
  2. ConfidentialityCheck(memory)   # Tagged secrets / API keys
  3. HallucinationRisk(memory)      # Flag low-confidence factual claims
  4. RedundancyCheck(memory, store) # Cosine sim > 0.95 → deduplicate
  5. ToxicityFilter(memory.content) # Harmful content classifier
  
  if any check fails:
    return FilterResult.Reject(reason, suggested_redaction)
  return FilterResult.Allow
```

---

## 9. Phased Roadmap

| Phase | Theme | Key Deliverables | Target |
|-------|-------|-----------------|--------|
| **P1** (done) | Core Storage | Sessions, messages, binary store, manifest | ✅ |
| **P2** (done) | Search | Native BM25, Tantivy backend | ✅ |
| **P3** (done) | Memory Tiers | Working/short/long tiers, memory CRUD | ✅ |
| **P4** (done) | Web GUI | FastAPI server, React UI, streaming chat | ✅ |
| **P5** (done) | Python Bindings | PyO3 bindings, maturin build | ✅ |
| **P6** (done) | Semantic Search | HNSW vector index, hybrid RRF search | ✅ |
| **P7** (done) | Git Objects | Blob/Tree/Commit/Tag, BranchOps, MergeOps | ✅ |
| **P8** | Data Model | Blake3 upgrade, WAL, compaction lifecycle, Frontmatter schema | Q2 2026 |
| **P9** | Context Assembly | Token-budget API (§7.2), multi-signal scoring (§8.1–8.2) | Q2 2026 |
| **P10** | Sleep-Time | Async consolidation jobs, sleep-time sessions (§7.3, §8.3) | Q3 2026 |
| **P11** | Semantic Merge | Three-way merge (§8.4), KG incremental update (§8.6) | Q3 2026 |
| **P12** | Skills | Procedural memory, skill extraction (§7.4, §8.7) | Q3 2026 |
| **P13** | Multi-Agent | Agent registry, worktrees, cross-agent search (§7.5) | Q4 2026 |
| **P14** | MCP + Interop | MCP adapter (§7.7), Letta import/export (§7.8) | Q4 2026 |
| **P15** | Memory Evolution | A-MEM-style network evolution (§8.8), KG decay | Q1 2027 |

---

## 10. Open Questions

| # | Question | Impact | Owner |
|---|----------|--------|-------|
| Q1 | Should `messages.pack` use the MemRepo object store or remain independent? Unifying reduces code but increases startup cost. | High | Architecture |
| Q2 | Token counting: integrate `tiktoken` (Python dep) or build a pure-Rust approximation? Approximations are ±5% but avoid FFI. | Medium | Performance |
| Q3 | For the LLM-dependent steps (contradiction detection, skill extraction), should MemSt call out to the configured LLM, or require an external "memory agent" process? | High | Architecture |
| Q4 | CRDT for cross-agent merging: vector clocks add complexity. Is eventual consistency acceptable for v1, with merge conflicts surfaced to the user? | High | Consistency model |
| Q5 | Should the `context/` working-tree be writable by agents directly (risky, unversioned writes) or only writable via the commit API? | High | Safety |
| Q6 | Embedding model versioning: if the model changes, all HNSW vectors are invalid. Need a migration strategy — full re-embed or dual-index? | High | Ops |
| Q7 | Sleep-time scheduling: cron-like (time-based) vs. event-based (token budget exceeded, session idle)? Probably both, but priority? | Medium | UX |
| Q8 | How should MemSt handle multi-modal memories (images in conversation)? Defer entirely, or add image-embedding support? | Low (v1) | Scope |

---

## 11. Security & Privacy

### 11.1 Memory Isolation

- Per-agent write scopes enforced at the API layer — an agent cannot write to a scope it was not registered with.
- Worktree isolation ensures concurrent subagent writes never corrupt the main branch.
- `store.lock` prevents concurrent process access to the same store.

### 11.2 PII Handling

- Safety filter (§8.9) runs on every `POST /memory` write path.
- Redaction API (§7.9) enables tombstoning of PII with audit trail.
- MemRepo history preserves tombstones; actual content is zeroed in the object store.

### 11.3 At-Rest Encryption

- Objects store should support optional AES-256-GCM encryption keyed by a user-provided passphrase.
- Key derivation: Argon2id with salt stored in `config.toml`.
- This is a P15+ feature; not required for local-only deployments.

### 11.4 Transport Security

- REST API defaults to HTTP (localhost only); TLS should be enabled for networked deployments.
- Planned: API key auth for multi-user server deployments (beyond current single-user model).

---

## Appendix A: Competitor Comparison Matrix

Detailed feature comparison across memory systems as of Q1 2026:

| Feature | MemSt | Letta | Mem0 | A-MEM | Zep | OpenMemory MCP |
|---------|-------|-------|------|-------|-----|----------------|
| Git-like versioning | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Offline/local | ✅ | ❌ | Partial | ✅ | ❌ | ✅ |
| Tiered memory | ✅ | ✅ | ❌ | ❌ | Partial | ❌ |
| Hybrid BM25+Vector | ✅ | ❌ | Partial | ❌ | ✅ | ❌ |
| Knowledge Graph | ✅ | ❌ | ❌ | ✅ | ✅ | ❌ |
| Sleep-time consolidation | Planned | ✅ | ❌ | ❌ | ❌ | ❌ |
| Skill/procedural memory | Planned | Partial | ❌ | ❌ | ❌ | ❌ |
| MCP adapter | Planned | ✅ | Partial | ❌ | ✅ | ✅ |
| Rust core | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Python bindings | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| Multi-agent scoping | Planned | ✅ | Partial | ❌ | ❌ | ❌ |
| Semantic 3-way merge | Planned | ❌ | ❌ | ❌ | ❌ | ❌ |
| Memory evolution (A-MEM) | Planned | ❌ | ❌ | ✅ | ❌ | ❌ |
| Import/export | Planned | ✅ | ❌ | ❌ | ❌ | ❌ |

---

## Appendix B: Letta / MemGPT Deep Dive

**What Letta does well that MemSt should learn from:**

1. **Memory blocks as first-class objects.** Letta's structured context blocks (persona, human, system, agent) map cleanly to MemSt's tier model. MemSt should expose similar named-block semantics in the context assembly API.

2. **Sleep-time compute pattern.** Letta's async consolidation pattern (idle agent processes memories during downtime) is the correct architecture. MemSt's `ConsolidationJob` model mirrors this with the addition of MemRepo commits for history.

3. **Agent File (.af) format.** Letta's serialized agent state format is the closest thing to a portability standard. MemSt should support import/export to establish interoperability.

**Where MemSt improves on Letta:**

1. **Versioning.** Letta has no memory history. MemSt's git-alike DAG gives full rewind, diff, blame.
2. **Local-first.** Letta requires a hosted service. MemSt runs entirely on-device.
3. **Search depth.** Letta has no BM25 backend. MemSt's dual-backend + HNSW + RRF is significantly more capable.
4. **Procedural memory.** Letta has nascent tool-suggestion features. MemSt's `Skill` model is more structured.

---

## Appendix C: Proposed Crate Structure

```
memst/
├── memst-core/          # Core data structures, storage, search
│   ├── src/
│   │   ├── objects/     # Blake3, Blob, Tree, Commit, Tag, Skill, Entity
│   │   ├── store/       # SessionStore, object store, WAL
│   │   ├── memory/      # Tier management, compaction lifecycle
│   │   ├── search/      # Native BM25, Tantivy, HNSW, hybrid RRF
│   │   ├── kg/          # Knowledge graph, incremental update
│   │   ├── context/     # Token-budget-aware context assembly
│   │   └── config/      # MemStConfig, TokenCounter trait
│
├── memst-repo/          # NEW: Git-alike MemRepo backend (split from core)
│   ├── src/
│   │   ├── pack/        # Packfile generation, delta compression
│   │   ├── refs/        # RefStore, RefLog, worktrees
│   │   ├── merge/       # Three-way semantic merge
│   │   └── wal/         # Write-ahead log for crash recovery
│
├── memst-sleep/         # NEW: Async consolidation pipeline
│   ├── src/
│   │   ├── jobs/        # ConsolidationJob scheduler
│   │   ├── consolidate/ # Clustering + LLM summarization
│   │   └── evolve/      # A-MEM-style memory evolution
│
├── memst-mcp/           # NEW: MCP adapter layer
│   ├── src/
│   │   └── adapter/     # MCP tool manifest + handler
│
├── memst-server/        # FastAPI Python server (existing)
├── memst-ui/            # React TypeScript frontend (existing)
├── memst-py/            # PyO3 Python bindings (existing)
└── memst-cli/           # CLI binary (existing)
```

---

## Appendix D: Frontmatter Schema Reference

All files in the `context/` working tree use the following YAML frontmatter schema:

```yaml
# Required fields
id: string                   # MemoryId (UUID)
type: episodic|semantic|procedural|resource|metacognitive
tier: working|short|long|archival
created_at: datetime (ISO 8601)

# Recommended fields
scope:
  type: user|project|org|session|agent
  id: string
tags: [string]
importance: float (0.0–1.0)
confidence: float (0.0–1.0)
token_estimate: integer
commit_hash: string (Blake3 hex, 7-char abbrev OK)

# Optional fields
updated_at: datetime
source: user_explicit|agent_inline|sleep_consolidation|skill_learning|import_external|merge
supersedes: string (MemoryId)
retracted_by: string (MemoryId)
embedding_model: string
embedding_version: string
access_count: integer
last_accessed: datetime
linked_skills: [SkillId]
linked_entities: [EntityId]
```

**Validation rules:**
- `importance` and `confidence` must be in [0.0, 1.0]
- `supersedes` chain must be acyclic
- `retracted_by` and `supersedes` cannot both be set on the same item
- `token_estimate` should be kept current; stale estimates degrade context assembly quality
- Files without valid frontmatter are treated as unversioned drafts and not indexed

---

*Document maintained in `docs/PRD.md`. For the implementation tracker, see GitHub Issues labeled `prd-item`.*
