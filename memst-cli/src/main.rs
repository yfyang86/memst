use anyhow::Result;
use clap::{Parser, Subcommand};
use memst_core::graph::{Direction, EntitySearch, KnowledgeGraph, SearchMethod};
use memst_core::search::SearchQuery;
use memst_core::store::SessionStore;
use memst_core::types::{
    Entity, EntityQuery, MemoryItem, MemoryQuery, MemoryTier, Message, OperationQuery, Relationship,
    Role, SessionMetadata,
};
use serde_json::json;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(name = "memst")]
#[command(author = "MemSt Team")]
#[command(version = "0.1.0")]
#[command(about = "Git-like memory architecture for LLM session management", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,

    /// Store directory path
    #[arg(short, long, default_value = "./memst-store")]
    store: PathBuf,
}

#[derive(Subcommand, Debug)]
enum Command {
    #[command(about = "Initialize a new memory store")]
    Init { path: Option<PathBuf> },

    #[command(about = "Session management")]
    Session {
        #[command(subcommand)]
        command: SessionCommand,
    },

    #[command(about = "Message operations")]
    Message {
        #[command(subcommand)]
        command: MessageCommand,
    },

    #[command(about = "Trace operation history")]
    Trace {
        #[command(subcommand)]
        command: TraceCommand,
    },

    #[command(about = "Memory tier management")]
    Memory {
        #[command(subcommand)]
        command: MemoryCommand,
    },

    #[command(about = "Search across sessions")]
    Search {
        /// Search query terms
        query: Vec<String>,

        /// Filter by session ID
        #[arg(short, long)]
        session: Option<Uuid>,

        /// Filter by document type (message, memory)
        #[arg(short, long)]
        doc_type: Option<String>,

        /// Maximum results
        #[arg(short, long, default_value = "20")]
        limit: usize,

        /// Use specific backend (native, tantivy)
        #[arg(long)]
        backend: Option<String>,

        /// JSON output
        #[arg(long)]
        json: bool,
    },

    #[command(about = "Show storage statistics")]
    Stats,

    #[command(about = "Knowledge graph operations")]
    Graph {
        /// Storage backend: auto, json or helix.
        ///
        /// `auto` uses the store's shared HelixDB graph. A session that still
        /// has its own graph from an older MemSt keeps using it until
        /// `graph migrate` has been run.
        #[arg(long, global = true, default_value = "auto")]
        backend: String,

        /// Whose graph to use. Owners share the store and never see each
        /// other's entities.
        #[arg(long, global = true, default_value = "default")]
        owner: String,

        /// For query and search: include entities from every session, not
        /// only the given one.
        #[arg(long, global = true)]
        all_sessions: bool,

        /// What computes embeddings: auto, local, http or none.
        ///
        /// `auto` uses the embedding API from config.toml or
        /// MEMST_EMBEDDING_API_URL when one is set, and otherwise the local
        /// BGE-M3 model (downloaded on first use, about 2.2 GB).
        #[arg(long, global = true, default_value = "auto")]
        embedder: String,

        #[command(subcommand)]
        command: GraphCommand,
    },

    #[command(about = "Serve one owner's memory to MCP clients, and review what they wrote")]
    Mcp {
        /// Whose memory. Required: a server is always bound to one owner,
        /// and clients cannot name another.
        #[arg(long, global = true)]
        owner: Option<String>,

        #[command(subcommand)]
        command: McpCommand,
    },
}

#[derive(Subcommand, Debug)]
enum McpCommand {
    /// Run an MCP server on standard input and output (for Claude Code,
    /// Cursor and other MCP clients). It opens no network port.
    Serve {
        /// Offer only the tools that read. Nothing in the store is changed.
        #[arg(long)]
        read_only: bool,

        /// What to do with a note that reads like an instruction to a model:
        /// quarantine (keep it aside until you approve it), reject or off.
        #[arg(long, default_value = "quarantine")]
        screening: String,
    },
    /// List the notes set aside for review.
    Review,
    /// Release a note from review, or mark any note as approved by you.
    Approve {
        /// The note's id
        id: Uuid,
    },
    /// Delete a note.
    Forget {
        /// The note's id
        id: Uuid,
    },
    /// Show the most recent entries of the audit log.
    Audit {
        /// How many entries
        #[arg(long, default_value_t = 20)]
        last: usize,
    },
    /// Print the index: what every session is given at its start.
    Index,
    /// List the notes.
    Notes,
    /// Print a note, as it is now or as it was at an earlier revision.
    Show {
        /// The note's id
        id: Uuid,
        /// A revision id (or its first characters) from `history`
        #[arg(long)]
        at: Option<String>,
    },
    /// List the changes made to the notes, newest first.
    History {
        /// How many changes
        #[arg(long, default_value_t = 20)]
        last: usize,
    },
    /// Put the notes back as they were at a revision. Nothing is lost: the
    /// states in between stay in the history.
    Restore {
        /// A revision id (or its first characters) from `history`
        revision: String,
    },
    /// Review memory offline: check the notes against what was said, add
    /// what was missed, and build pages. Without `--apply` the proposal is
    /// shown and kept for you to look at; nothing changes.
    ///
    /// Uses the model configured in config.toml or MEMST_LLM_API_URL,
    /// MEMST_LLM_MODEL and MEMST_LLM_API_KEY.
    Dream {
        /// Apply the waiting proposal, or draft one and apply it at once.
        #[arg(long)]
        apply: bool,
        /// Throw the waiting proposal away.
        #[arg(long, conflicts_with = "apply")]
        discard: bool,
        /// Extra fields for the model request, as JSON (for example
        /// '{"thinking":{"type":"disabled"}}').
        #[arg(long)]
        llm_extra: Option<String>,
        /// Most transcript text to read in one pass, in characters.
        #[arg(long, default_value_t = 60_000)]
        max_chars: usize,
    },
    /// Put an approved note at the top of the index.
    Pin {
        /// The note's id
        id: Uuid,
    },
    /// Take a note off the top of the index.
    Unpin {
        /// The note's id
        id: Uuid,
    },
}

#[derive(Subcommand, Debug)]
enum SessionCommand {
    #[command(about = "Create a new session")]
    New {
        /// Session name
        #[arg(short, long)]
        name: Option<String>,

        /// Model name
        #[arg(short, long, default_value = "gpt-4")]
        model: String,

        /// Comma-separated tags
        #[arg(short, long)]
        tags: Option<String>,
    },

    #[command(about = "List all sessions")]
    List,

    #[command(about = "Show session details")]
    Show {
        /// Session ID
        id: Uuid,
    },

    #[command(about = "Delete a session")]
    Delete {
        /// Session ID
        id: Uuid,
    },
}

#[derive(Subcommand, Debug)]
enum MessageCommand {
    #[command(about = "Add a message to a session")]
    Add {
        /// Session ID
        session_id: Uuid,

        /// Message role (user, assistant, system, tool)
        #[arg(short, long, default_value = "user")]
        role: String,

        /// Message content
        #[arg(short, long)]
        content: Option<String>,

        /// Read from stdin or file
        #[arg(long)]
        file: Option<PathBuf>,
    },

    #[command(about = "List messages in a session")]
    List {
        /// Session ID
        session_id: Uuid,

        /// Maximum messages
        #[arg(short, long, default_value = "100")]
        limit: u32,

        /// Skip first N messages
        #[arg(long, default_value = "0")]
        offset: u32,
    },
}

#[derive(Subcommand, Debug)]
enum TraceCommand {
    #[command(about = "Show operation history")]
    Show {
        /// Session ID
        session_id: Uuid,

        /// Filter by operation type
        #[arg(short, long)]
        op_type: Option<String>,

        /// Maximum results
        #[arg(short, long, default_value = "100")]
        limit: usize,

        /// JSON output
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum MemoryCommand {
    #[command(about = "Add a memory to a tier")]
    Add {
        /// Session ID
        session_id: Uuid,

        /// Memory tier (working, short, long)
        #[arg(short, long, default_value = "working")]
        tier: String,

        /// Memory content
        #[arg(short, long)]
        content: String,

        /// Comma-separated tags
        #[arg(short, long)]
        tags: Option<String>,

        /// Confidence score (0.0-1.0)
        #[arg(short, long, default_value = "1.0")]
        confidence: f32,
    },

    #[command(about = "List memories in a tier")]
    List {
        /// Session ID
        session_id: Uuid,

        /// Memory tier
        #[arg(short, long)]
        tier: Option<String>,

        /// JSON output
        #[arg(long)]
        json: bool,
    },

    #[command(about = "Retrieve relevant memories")]
    Retrieve {
        /// Session ID
        session_id: Uuid,

        /// Search query
        query: String,

        /// Maximum results
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// JSON output
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum GraphCommand {
    #[command(about = "Add an entity")]
    AddEntity {
        /// Session ID
        session_id: Uuid,

        /// Entity name
        name: String,

        /// Entity type (person, technology, location, etc.)
        entity_type: String,

        /// JSON attributes
        #[arg(short, long)]
        attributes: Option<String>,

        /// Always create a new entity, even if one with the same name and
        /// type already exists
        #[arg(long)]
        new: bool,
    },

    #[command(about = "Add a relationship")]
    AddRel {
        /// Session ID
        session_id: Uuid,

        /// Subject entity ID
        subject: Uuid,

        /// Relationship predicate (knows, uses, located_in, etc.)
        predicate: String,

        /// Object entity ID
        object: Uuid,

        /// Confidence score (0.0-1.0)
        #[arg(short, long, default_value = "1.0")]
        confidence: f32,
    },

    #[command(about = "Query entities")]
    Query {
        /// Session ID
        session_id: Uuid,

        /// Entity type filter
        #[arg(short, long)]
        entity_type: Option<String>,

        /// Maximum results
        #[arg(short, long, default_value = "50")]
        limit: usize,

        /// JSON output
        #[arg(long)]
        json: bool,
    },

    #[command(about = "Show graph statistics")]
    Stats {
        /// Session ID
        session_id: Uuid,
    },

    #[command(about = "List entities within N hops of an entity")]
    Neighbors {
        /// Session ID
        session_id: Uuid,

        /// Entity to start from
        entity: Uuid,

        /// Maximum number of hops
        #[arg(long, default_value = "2")]
        hops: usize,

        /// Edges to follow: out, in or both
        #[arg(long, default_value = "both")]
        direction: String,

        /// JSON output
        #[arg(long)]
        json: bool,
    },

    #[command(about = "Search entities by keywords (BM25) and, with --semantic, by meaning")]
    Search {
        /// Session ID
        session_id: Uuid,

        /// Search text
        query: String,

        /// Maximum results
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// Also embed the query and search by vector similarity.
        /// Needs an embedding API (config.toml or MEMST_EMBEDDING_API_URL)
        /// and entity embeddings (`graph embed`); without them the search
        /// continues with keywords only.
        #[arg(long)]
        semantic: bool,

        /// Include deprecated entities
        #[arg(long)]
        include_deprecated: bool,

        /// Search relationships ("alice uses rust") instead of entities
        #[arg(long)]
        relationships: bool,

        /// Also print the current relationships around each hit
        #[arg(long)]
        expand: bool,

        /// JSON output
        #[arg(long)]
        json: bool,
    },

    #[command(about = "Embed entities that are new or have changed (for --semantic search)")]
    Embed {
        /// Session ID
        session_id: Uuid,
    },

    #[command(about = "Extract entities and relationships from a session's messages into the graph")]
    Ingest {
        /// Session ID
        session_id: Uuid,

        /// Re-read every message, not only those added since the last ingest
        #[arg(long)]
        all: bool,

        /// Characters of conversation sent to the LLM per extraction call
        #[arg(long, default_value = "6000")]
        batch_chars: usize,
    },

    #[command(about = "Copy every per-session graph of this store into the shared graph")]
    Migrate,
}

/// The embedder the `--embedder` option selects, or `None` for "none".
#[cfg(feature = "local-embeddings")]
fn select_embedder(kind: &str) -> Result<Option<Box<dyn memst_core::embed::Embedder>>> {
    Ok(memst_embed::select_embedder(kind, |dir| {
        eprintln!(
            "note: downloading the BGE-M3 embedding model (about 2.2 GB) to {}; this happens once",
            dir.display()
        );
    })?)
}

/// The embedder the `--embedder` option selects (built without the local model).
#[cfg(not(feature = "local-embeddings"))]
fn select_embedder(kind: &str) -> Result<Option<Box<dyn memst_core::embed::Embedder>>> {
    use memst_core::embed::HttpEmbedder;
    match kind {
        "none" => Ok(None),
        "http" | "auto" => Ok(Some(Box::new(HttpEmbedder::from_config()?))),
        "local" => anyhow::bail!("this memst was built without the 'local-embeddings' feature"),
        other => anyhow::bail!("unknown embedder '{other}' (expected auto, local, http or none)"),
    }
}

/// Plain text of a message, for extraction.
fn message_text(message: &memst_core::types::Message) -> String {
    use memst_core::types::{Content, ContentPart};
    match &message.content {
        Content::Text(text) => text.clone(),
        Content::MultiPart(parts) => parts
            .iter()
            .filter_map(|part| match part {
                ContentPart::Text(text) => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

/// Open the knowledge graph a session of this store uses.
///
/// With `read_ok`, a graph that another process is writing is opened
/// read-only instead of failing.
#[cfg(feature = "helix")]
fn open_graph(
    store_dir: &std::path::Path,
    session_dir: &std::path::Path,
    backend: &str,
    owner: &str,
    read_ok: bool,
) -> Result<KnowledgeGraph> {
    use memst_graph_helix::{GraphLocation, StoreGraphOptions};

    let mut options = StoreGraphOptions {
        choice: backend.parse().map_err(anyhow::Error::msg)?,
        owner: owner.to_string(),
        read_only: false,
    };
    let (graph, location) = match memst_graph_helix::open_store(store_dir, session_dir, &options) {
        Err(memst_core::error::Error::LockError(reason)) if read_ok => {
            eprintln!("note: {reason}");
            eprintln!("note: reading without the lock; very recent writes may not be visible yet");
            options.read_only = true;
            memst_graph_helix::open_store(store_dir, session_dir, &options)?
        }
        other => other?,
    };
    if location == GraphLocation::LegacySession && backend == "auto" {
        eprintln!(
            "note: this session uses its own graph from an older MemSt; run `memst graph migrate` to move it into the shared graph"
        );
    }
    Ok(graph)
}

/// Open a session's knowledge graph (built without HelixDB: JSON only).
#[cfg(not(feature = "helix"))]
fn open_graph(
    _store_dir: &std::path::Path,
    session_dir: &std::path::Path,
    backend: &str,
    _owner: &str,
    _read_ok: bool,
) -> Result<KnowledgeGraph> {
    match backend {
        "auto" | "json" => Ok(KnowledgeGraph::new(session_dir)?),
        other => anyhow::bail!(
            "graph backend '{other}' is not available: this memst was built without the 'helix' feature"
        ),
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    match &args.command {
        Command::Init { path } => {
            let store_path = path.as_ref().unwrap_or(&args.store);
            println!("Initializing memory store at: {}", store_path.display());
            SessionStore::init(store_path)?;
            println!("Store initialized successfully!");
        }

        Command::Session { command } => {
            let store = SessionStore::open(&args.store)?;
            match command {
                SessionCommand::New { name, model, tags } => {
                    let name = name
                        .clone()
                        .unwrap_or_else(|| "Unnamed Session".to_string());
                    let tags_vec: Vec<String> = tags
                        .as_ref()
                        .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
                        .unwrap_or_default();
                    let session_id = store
                        .create_session(SessionMetadata::new(&name, model).with_tags(tags_vec))?;
                    println!("Created session: {}", session_id);
                }

                SessionCommand::List => {
                    let sessions = store.list_sessions()?;
                    if sessions.is_empty() {
                        println!("No sessions found.");
                    } else {
                        println!("Sessions:");
                        for s in sessions {
                            println!(
                                "  {} - {} ({}) [{} msgs]",
                                s.id, s.name, s.model, s.message_count
                            );
                        }
                    }
                }

                SessionCommand::Show { id } => {
                    if let Some(metadata) = store.get_session(*id)? {
                        println!("Session: {}", id);
                        println!("  Name: {}", metadata.name);
                        println!("  Model: {}", metadata.model);
                        println!("  Tags: {:?}", metadata.tags);
                        println!("  Created: {}", metadata.created_at);
                        println!("  Last Activity: {}", metadata.last_activity);
                        println!("  Messages: {}", metadata.message_count);
                    } else {
                        println!("Session not found: {}", id);
                    }
                }

                SessionCommand::Delete { id } => {
                    store.delete_session(*id)?;
                    println!("Deleted session: {}", id);
                }
            }
        }

        Command::Message { command } => match command {
            MessageCommand::Add {
                session_id,
                role,
                content,
                file,
            } => {
                let store = SessionStore::open(&args.store)?;

                // Get content from file or argument
                let content_text = if let Some(ref file_path) = file {
                    std::fs::read_to_string(file_path)?
                } else {
                    content.clone().unwrap_or_default()
                };

                let role = match role.to_lowercase().as_str() {
                    "system" => Role::System,
                    "assistant" => Role::Assistant,
                    "tool" => Role::Tool,
                    _ => Role::User,
                };

                let message = Message::new(role, content_text);
                let message_id = message.id;
                store.append_message(*session_id, message.clone())?;

                // Index the message
                store.index_message(*session_id, &message)?;

                println!("Added message to session {}: {}", session_id, message_id);
            }

            MessageCommand::List {
                session_id,
                limit,
                offset,
            } => {
                let store = SessionStore::open(&args.store)?;
                let messages = store.get_messages_range(
                    *session_id,
                    *offset as usize,
                    (*offset + *limit) as usize,
                )?;

                if messages.is_empty() {
                    println!("No messages found in session: {}", session_id);
                } else {
                    for (i, msg) in messages.iter().enumerate() {
                        let role_str = match msg.role {
                            Role::System => "[SYSTEM]",
                            Role::User => "[USER]",
                            Role::Assistant => "[ASSISTANT]",
                            Role::Tool => "[TOOL]",
                        };
                        let content = match &msg.content {
                            memst_core::types::Content::Text(s) => s.clone(),
                            memst_core::types::Content::MultiPart(parts) => parts
                                .iter()
                                .map(|p| match p {
                                    memst_core::types::ContentPart::Text(t) => t.clone(),
                                    _ => String::from("[non-text]"),
                                })
                                .collect::<Vec<_>>()
                                .join("\n"),
                        };
                        println!(
                            "\n{}{} {}:",
                            role_str,
                            i + *offset as usize,
                            msg.timestamp.format("%Y-%m-%d %H:%M")
                        );
                        println!("{}", content);
                    }
                }
            }
        },

        Command::Trace { command } => match command {
            TraceCommand::Show {
                session_id,
                op_type,
                limit,
                json,
            } => {
                let store = SessionStore::open(&args.store)?;

                let query = OperationQuery::new().with_limit(*limit);
                let ops = store.query_operations(*session_id, query)?;

                let filtered: Vec<_> = if let Some(ref ot) = op_type {
                    let filter_type = match ot.as_str() {
                        "tool_call" => memst_core::types::OperationType::ToolCall {
                            name: String::new(),
                        },
                        "function_call" => memst_core::types::OperationType::FunctionCall {
                            name: String::new(),
                        },
                        "web_search" => memst_core::types::OperationType::WebSearch,
                        "thinking_step" => memst_core::types::OperationType::ThinkingStep,
                        "memory_retrieval" => memst_core::types::OperationType::MemoryRetrieval,
                        "knowledge_graph_query" => {
                            memst_core::types::OperationType::KnowledgeGraphQuery
                        }
                        _ => memst_core::types::OperationType::Custom(ot.clone()),
                    };
                    ops.into_iter()
                        .filter(|op| {
                            std::mem::discriminant(&op.op_type)
                                == std::mem::discriminant(&filter_type)
                        })
                        .collect()
                } else {
                    ops
                };

                if *json {
                    println!("{}", serde_json::to_string_pretty(&filtered)?);
                } else {
                    if filtered.is_empty() {
                        println!("No operations found.");
                    } else {
                        for op in filtered {
                            println!(
                                "{} {} {:>15} ({}ms)",
                                op.timestamp.format("%Y-%m-%d %H:%M:%S"),
                                op.id.to_string().chars().take(8).collect::<String>(),
                                op.op_type,
                                op.duration_ms
                            );
                        }
                    }
                }
            }
        },

        Command::Memory { command } => match command {
            MemoryCommand::Add {
                session_id,
                tier,
                content,
                tags,
                confidence,
            } => {
                let store = SessionStore::open(&args.store)?;

                let tier = match tier.to_lowercase().as_str() {
                    "short" | "short_term" => MemoryTier::ShortTerm,
                    "long" | "long_term" => MemoryTier::LongTerm,
                    _ => MemoryTier::Working,
                };

                let tags_vec: Vec<String> = tags
                    .as_ref()
                    .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default();

                let memory = MemoryItem::new(content, "cli")
                    .with_tags(tags_vec)
                    .with_confidence(*confidence);

                let added = store.add_memory(*session_id, tier.clone(), memory)?;

                // Index the memory
                store.index_memory(*session_id, &added)?;

                println!(
                    "Added memory to session {} tier: {}",
                    session_id,
                    tier.to_string()
                );
            }

            MemoryCommand::List {
                session_id,
                tier,
                json,
            } => {
                let store = SessionStore::open(&args.store)?;

                let tiers: Vec<MemoryTier> = if let Some(ref t) = tier {
                    let parsed = match t.to_lowercase().as_str() {
                        "short" | "short_term" => MemoryTier::ShortTerm,
                        "long" | "long_term" => MemoryTier::LongTerm,
                        _ => MemoryTier::Working,
                    };
                    vec![parsed]
                } else {
                    vec![
                        MemoryTier::Working,
                        MemoryTier::ShortTerm,
                        MemoryTier::LongTerm,
                    ]
                };

                let mut all_memories: Vec<(String, MemoryItem)> = Vec::new();
                for t in &tiers {
                    for m in store.get_tier(*session_id, t.clone())? {
                        all_memories.push((t.to_string(), m));
                    }
                }

                if *json {
                    let output: Vec<_> = all_memories
                        .iter()
                        .map(|(t, m)| {
                            json!({
                                "tier": t,
                                "id": m.id.to_string(),
                                "content": m.content,
                                "tags": m.tags,
                                "confidence": m.confidence,
                                "importance": m.importance,
                                "access_count": m.access_count,
                                "created_at": m.created_at.to_rfc3339()
                            })
                        })
                        .collect();
                    println!("{}", serde_json::to_string_pretty(&output)?);
                } else {
                    if all_memories.is_empty() {
                        println!("No memories found.");
                    } else {
                        for (tier, m) in &all_memories {
                            println!(
                                "[{}] {:>6} {} - {}",
                                m.id.to_string().chars().take(8).collect::<String>(),
                                tier,
                                format!("{:.2}", m.importance),
                                m.content.chars().take(60).collect::<String>()
                            );
                        }
                    }
                }
            }

            MemoryCommand::Retrieve {
                session_id,
                query,
                limit,
                json,
            } => {
                let store = SessionStore::open(&args.store)?;

                let mem_query = MemoryQuery::new().with_keyword(query).with_limit(*limit);

                let results = store.retrieve_memories(*session_id, mem_query)?;

                if *json {
                    let output: Vec<_> = results
                        .iter()
                        .map(|m| {
                            json!({
                                "id": m.id.to_string(),
                                "content": m.content,
                                "tags": m.tags,
                                "confidence": m.confidence,
                                "importance": m.importance
                            })
                        })
                        .collect();
                    println!("{}", serde_json::to_string_pretty(&output)?);
                } else {
                    println!("Retrieved {} memories:", results.len());
                    for m in &results {
                        println!(
                            "  {} [{}] {}",
                            m.id.to_string().chars().take(8).collect::<String>(),
                            format!("{:.2}", m.importance),
                            m.content.chars().take(70).collect::<String>()
                        );
                    }
                }
            }
        },

        Command::Search {
            query,
            session,
            doc_type,
            limit,
            backend: _,
            json,
        } => {
            let store = SessionStore::open(&args.store)?;

            let mut search_query = SearchQuery::new()
                .with_terms(query.clone())
                .with_limit(*limit);

            if let Some(sid) = *session {
                search_query = search_query.with_session(sid);
            }

            if let Some(ref dt) = *doc_type {
                search_query = search_query.with_doc_type(dt.clone());
            }

            let results = store.search(search_query)?;

            if *json {
                let output: Vec<_> = results
                    .iter()
                    .map(|r| {
                        json!({
                            "id": r.id,
                            "session_id": r.session_id.to_string(),
                            "doc_type": r.doc_type,
                            "score": r.score,
                            "snippet": r.snippet,
                            "timestamp": r.timestamp.to_rfc3339()
                        })
                    })
                    .collect();
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                if results.is_empty() {
                    println!("No results found for: {}", query.join(" "));
                } else {
                    println!("Found {} results:", results.len());
                    for r in &results {
                        println!(
                            "\n[{:>4.1}] {} ({}) {}",
                            r.score,
                            r.doc_type,
                            r.session_id.to_string().chars().take(8).collect::<String>(),
                            r.snippet.chars().take(80).collect::<String>()
                        );
                    }
                }
            }
        }

        Command::Stats => {
            let store = SessionStore::open(&args.store)?;

            let sessions = store.list_sessions()?;
            let total_messages: u32 = sessions.iter().map(|s| s.message_count).sum();

            println!("MemSt Store Statistics");
            println!("======================");
            println!("Sessions: {}", sessions.len());
            println!("Total Messages: {}", total_messages);

            let search_count = store.search_index_count()?;
            println!("Indexed Documents: {}", search_count);

            // Memory counts per tier
            let mut working = 0;
            let mut short_term = 0;
            let mut long_term = 0;

            for s in &sessions {
                working += store.get_tier(s.id, MemoryTier::Working)?.len();
                short_term += store.get_tier(s.id, MemoryTier::ShortTerm)?.len();
                long_term += store.get_tier(s.id, MemoryTier::LongTerm)?.len();
            }

            println!("Memories: {}", working + short_term + long_term);
            println!("  Working: {}", working);
            println!("  Short-term: {}", short_term);
            println!("  Long-term: {}", long_term);
        }

        Command::Mcp { owner, command } => {
            use memst_mcp::service::MemoryService;
            use memst_mcp::{McpAdapter, McpServer, Scope, Screening, WritePolicy};
            use memst_repo::versioned_dir::{Change, Past};

            let Some(owner) = owner else {
                anyhow::bail!("--owner is required: say whose memory this is (for example --owner alice)");
            };
            let open = |read_only: bool, policy: WritePolicy| -> Result<MemoryService> {
                let scope = Scope::new(owner, read_only).map_err(anyhow::Error::msg)?;
                MemoryService::open(&args.store, scope, policy).map_err(|e| anyhow::anyhow!("{e}"))
            };
            let refused = |e: memst_mcp::Refusal| anyhow::anyhow!("{e}");
            // The owner's own commands: full access, default limits.
            let mine = || open(false, WritePolicy::default());
            match command {
                McpCommand::Serve { read_only, screening } => {
                    let screening = match screening.as_str() {
                        "quarantine" => Screening::Quarantine,
                        "reject" => Screening::Reject,
                        "off" => Screening::Off,
                        other => anyhow::bail!("unknown screening '{other}' (expected quarantine, reject or off)"),
                    };
                    let service = open(*read_only, WritePolicy { screening, ..WritePolicy::default() })?;
                    // Standard output carries the protocol and nothing else.
                    eprintln!(
                        "memst: serving {} memory for owner '{}' from {} over stdio",
                        if *read_only { "read-only" } else { "read-write" },
                        owner,
                        args.store.display()
                    );
                    let server = McpServer::new(McpAdapter::connected(std::sync::Arc::new(service)))?;
                    server.serve(std::io::stdin().lock(), std::io::stdout().lock())?;
                }
                McpCommand::Review => {
                    let waiting = mine()?.quarantined().map_err(refused)?;
                    if waiting.is_empty() {
                        println!("Nothing is waiting for review.");
                    }
                    for note in waiting {
                        println!("{}  written {} by {}", note.id, note.updated.format("%Y-%m-%d %H:%M"), note.source);
                        for reason in memst_mcp::policy::instruction_like(&note.body) {
                            println!("  set aside because it {reason}");
                        }
                        if !note.tags.is_empty() {
                            println!("  tags: {}", note.tags.join(", "));
                        }
                        // Shown escaped, so hidden characters are visible.
                        println!("  {}\n", note.body.escape_default());
                    }
                }
                McpCommand::Approve { id } => {
                    mine()?.approve(*id).map_err(refused)?;
                    println!("Approved {id}.");
                }
                McpCommand::Forget { id } => {
                    mine()?.forget(*id).map_err(refused)?;
                    println!("Forgot {id}, and every earlier version of it.");
                }
                McpCommand::Audit { last } => {
                    let log = std::fs::read_to_string(mine()?.audit_path()).unwrap_or_default();
                    let lines: Vec<&str> = log.lines().collect();
                    for line in &lines[lines.len().saturating_sub(*last)..] {
                        println!("{line}");
                    }
                }
                McpCommand::Index => print!("{}", mine()?.index().map_err(refused)?),
                McpCommand::Notes => {
                    let memory = mine()?;
                    let mut notes = memory.search_all().map_err(refused)?;
                    notes.extend(memory.quarantined().map_err(refused)?);
                    if notes.is_empty() {
                        println!("No notes.");
                    }
                    for note in notes {
                        let state = if note.status == memst_mcp::note::Status::Quarantined {
                            "waiting for review"
                        } else {
                            memst_mcp::service::trust_of(&note)
                        };
                        println!("{}  {}  {}  {}{}", note.id, note.updated.format("%Y-%m-%d"), state, if note.pinned { "[pinned] " } else { "" }, note.title());
                    }
                }
                McpCommand::Show { id, at } => match at {
                    None => {
                        let memory = mine()?;
                        let mut all = memory.search_all().map_err(refused)?;
                        all.extend(memory.quarantined().map_err(refused)?);
                        let note = all.into_iter().find(|n| n.id == *id).ok_or_else(|| anyhow::anyhow!("no note with that id"))?;
                        print!("{}", note.to_markdown());
                    }
                    Some(revision) => match mine()?.note_at(*id, revision).map_err(refused)? {
                        Past::Content(text) => print!("{text}"),
                        Past::Absent => println!("The note did not exist at that revision."),
                        Past::Forgotten => println!("The note existed at that revision; its content has since been forgotten."),
                    },
                },
                McpCommand::History { last } => {
                    let revisions = mine()?.history(*last).map_err(refused)?;
                    if revisions.is_empty() {
                        println!("No changes yet.");
                    }
                    for revision in revisions {
                        println!("{}  {}  {}  {}", &revision.id.to_hex()[..12], revision.at.format("%Y-%m-%d %H:%M"), revision.author, revision.message);
                        for change in &revision.changes {
                            let (word, path) = match change {
                                Change::Added(p) => ("added", p),
                                Change::Modified(p) => ("changed", p),
                                Change::Removed(p) => ("removed", p),
                            };
                            println!("    {word} {path}");
                        }
                    }
                }
                McpCommand::Restore { revision } => {
                    let skipped = mine()?.restore(revision).map_err(refused)?;
                    println!("The notes are as they were at {revision}.");
                    if skipped > 0 {
                        println!("{skipped} note(s) that were forgotten since then were not brought back.");
                    }
                }
                McpCommand::Dream { apply, discard, llm_extra, max_chars } => {
                    use memst_mcp::dream::{self, DreamConfig, Proposal};
                    let memory = mine()?;
                    if *discard {
                        println!("{}", if memory.discard_pending().map_err(refused)? { "The waiting proposal was thrown away." } else { "No proposal was waiting." });
                        return Ok(());
                    }
                    let show = |proposal: &Proposal| {
                        for c in &proposal.corrections {
                            println!("correct {}\n    to:      {}\n    because the user said: \"{}\" ({})", c.note, c.text, c.evidence.quote, c.evidence.source.key());
                        }
                        for n in &proposal.notes {
                            println!("add a note ({})\n    {}\n    from: \"{}\" ({})", n.kind, n.text, n.evidence.quote, n.evidence.source.key());
                        }
                        for page in &proposal.pages {
                            println!("write the page \"{}\" with {} fact(s)", page.title, page.facts.len());
                            for fact in &page.facts {
                                println!("    - {}  [{}]", fact.statement, fact.source.key());
                            }
                        }
                        for (id, reason) in &proposal.pins {
                            println!("suggestion: pin {id} ({reason})");
                        }
                        for dropped in &proposal.dropped {
                            println!("dropped: {dropped}");
                        }
                    };
                    let proposal = match memory.pending().map_err(refused)? {
                        Some(waiting) if *apply => waiting,
                        _ => {
                            let mut config = DreamConfig { max_transcript_chars: *max_chars, ..DreamConfig::default() };
                            if let Some(extra) = llm_extra {
                                config.request_extra = serde_json::from_str(extra).map_err(|e| anyhow::anyhow!("--llm-extra is not a JSON object: {e}"))?;
                            }
                            let llm = memst_core::llm::LlmLoader::default_provider()?;
                            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
                            runtime.block_on(dream::propose(&memory, llm, &config)).map_err(refused)?
                        }
                    };
                    show(&proposal);
                    if proposal.is_empty() {
                        println!("Nothing to change.");
                    }
                    if *apply {
                        let done = dream::apply(&memory, &proposal).map_err(refused)?;
                        println!(
                            "Applied: {} corrected, {} disputed, {} added, {} page(s); {} set aside for review.",
                            done.corrected, done.disputed, done.added, done.pages, done.quarantined
                        );
                        for skipped in &done.skipped {
                            println!("left out: {skipped}");
                        }
                        if let (Some(_), Some(before)) = (&done.revision, &done.before) {
                            println!("To undo this pass: memst mcp --owner {owner} restore {}", &before[..12]);
                        }
                    } else if !proposal.is_empty() {
                        memory.save_pending(&proposal).map_err(refused)?;
                        println!("Nothing has changed. Apply with `dream --apply`, or throw it away with `dream --discard`.");
                    }
                }
                McpCommand::Pin { id } => {
                    mine()?.pin(*id, true).map_err(refused)?;
                    println!("Pinned {id}.");
                }
                McpCommand::Unpin { id } => {
                    mine()?.pin(*id, false).map_err(refused)?;
                    println!("Unpinned {id}.");
                }
            }
        }

        Command::Graph { backend, owner, all_sessions, embedder, command } => {
            let store = SessionStore::open(&args.store)?;
            
            if let GraphCommand::Migrate = command {
                #[cfg(feature = "helix")]
                {
                    let report = memst_graph_helix::migrate_store(&args.store, owner)?;
                    if report.sessions == 0 {
                        println!("Nothing to migrate: no session has a graph of its own left.");
                    } else {
                        println!(
                            "Migrated {} session graph(s): {} entities and {} relationships, into the shared graph for owner '{}'.",
                            report.sessions, report.entities, report.relationships, owner
                        );
                        println!("The per-session graphs were kept as a backup.");
                    }
                    return Ok(());
                }
                #[cfg(not(feature = "helix"))]
                anyhow::bail!("this memst was built without the 'helix' feature");
            }

            let session_id = match &command {
                GraphCommand::AddEntity { session_id, .. } => *session_id,
                GraphCommand::AddRel { session_id, .. } => *session_id,
                GraphCommand::Query { session_id, .. } => *session_id,
                GraphCommand::Stats { session_id } => *session_id,
                GraphCommand::Neighbors { session_id, .. } => *session_id,
                GraphCommand::Search { session_id, .. } => *session_id,
                GraphCommand::Embed { session_id, .. } => *session_id,
                GraphCommand::Ingest { session_id, .. } => *session_id,
                GraphCommand::Migrate => unreachable!("handled above"),
            };
            let reads_only = matches!(
                command,
                GraphCommand::Query { .. }
                    | GraphCommand::Stats { .. }
                    | GraphCommand::Neighbors { .. }
                    | GraphCommand::Search { .. }
            );

            let session_path = store.session_path(session_id);
            let mut graph = open_graph(&args.store, &session_path, backend, owner, reads_only)?;

            match command {
                GraphCommand::AddEntity {
                    session_id,
                    name,
                    entity_type,
                    attributes,
                    new,
                } => {
                    let mut entity = Entity::new(name, entity_type, *session_id);
                    if let Some(attrs) = attributes {
                        if let Ok(attrs_map) = serde_json::from_str::<serde_json::Value>(attrs) {
                            entity = entity.with_attributes(attrs_map);
                        }
                    }
                    // The graph is shared across sessions, so the same thing
                    // mentioned again is the same entity unless --new is given.
                    let (added, created) = if *new {
                        (graph.add_entity(entity)?, true)
                    } else {
                        graph.resolve_or_add_entity(entity)?
                    };
                    if created {
                        println!("Added entity: {} (type: {})", added.id, added.entity_type);
                    } else {
                        println!(
                            "Known entity: {} (type: {}); now also linked to this session",
                            added.id, added.entity_type
                        );
                    }
                }

                GraphCommand::AddRel {
                    session_id,
                    subject,
                    predicate,
                    object,
                    confidence,
                } => {
                    let relationship = Relationship {
                        id: Uuid::new_v4(),
                        subject_id: *subject,
                        predicate: predicate.clone(),
                        object_id: *object,
                        confidence: *confidence,
                        session_id: *session_id,
                        source_message_id: None,
                        created_at: chrono::Utc::now(),
                        relevance: 1.0,
                        half_life_days: 30.0,
                        is_stable: false,
                        last_decay_at: chrono::Utc::now(),
                        valid_from: None,
                        valid_to: None,
                        superseded_by: None,
                        valid_precision: None,
                        stated_at: None,
                    };
                    let _added = graph.add_relationship(relationship)?;
                    println!(
                        "Added relationship: {} --[{}]--> {}",
                        subject, predicate, object
                    );
                }

                GraphCommand::Query {
                    session_id,
                    entity_type,
                    limit,
                    json,
                } => {
                    // The type filter is part of the query, so the limit
                    // applies to matching entities (it used to be applied
                    // before filtering, which could return too few).
                    let mut query = EntityQuery::new().with_limit(*limit);
                    if !*all_sessions {
                        query = query.with_session(*session_id);
                    }
                    if let Some(et) = entity_type {
                        query = query.with_type(et.clone());
                    }
                    let filtered = graph.find_entities(query)?;

                    if *json {
                        let output: Vec<_> = filtered
                            .iter()
                            .map(|e| {
                                json!({
                                    "id": e.id.to_string(),
                                    "name": e.name,
                                    "entity_type": e.entity_type,
                                    "confidence": e.confidence,
                                    "access_count": e.access_count
                                })
                            })
                            .collect();
                        println!("{}", serde_json::to_string_pretty(&output)?);
                    } else {
                        if filtered.is_empty() {
                            println!("No entities found.");
                        } else {
                            println!("Found {} entities:", filtered.len());
                            for e in &filtered {
                                println!(
                                    "  {} - {} [{}] (accesses: {})",
                                    e.id.to_string().chars().take(8).collect::<String>(),
                                    e.name,
                                    e.entity_type,
                                    e.access_count
                                );
                            }
                        }
                    }
                }

                GraphCommand::Stats { session_id } => {
                    let stats = graph.stats()?;
                    let in_session = graph
                        .find_entities(EntityQuery::new().with_session(*session_id))?
                        .len();
                    println!("Graph Statistics");
                    println!("==============================");
                    println!("Backend: {}", graph.backend_kind());
                    println!("Owner: {}", owner);
                    println!("Entities in this session: {}", in_session);
                    println!("Total Entities: {}", stats.entity_count);
                    println!("Total Relationships: {}", stats.relationship_count);
                    println!("Entity Types: {}", stats.entity_types.len());
                }

                GraphCommand::Neighbors {
                    session_id: _,
                    entity,
                    hops,
                    direction,
                    json,
                } => {
                    let direction = match direction.as_str() {
                        "out" => Direction::Outgoing,
                        "in" => Direction::Incoming,
                        "both" => Direction::Both,
                        other => anyhow::bail!("unknown direction '{other}' (expected out, in or both)"),
                    };
                    let hits = graph.neighborhood(*entity, *hops, direction)?;
                    if *json {
                        let output: Vec<_> = hits
                            .iter()
                            .map(|(e, distance)| {
                                json!({
                                    "id": e.id.to_string(),
                                    "name": e.name,
                                    "entity_type": e.entity_type,
                                    "hops": distance
                                })
                            })
                            .collect();
                        println!("{}", serde_json::to_string_pretty(&output)?);
                    } else if hits.is_empty() {
                        println!("No entities within {} hops.", hops);
                    } else {
                        for (e, distance) in &hits {
                            println!("  {} hop(s)  {}  {} [{}]", distance, e.id, e.name, e.entity_type);
                        }
                    }
                }

                GraphCommand::Search {
                    session_id,
                    query,
                    limit,
                    semantic,
                    include_deprecated,
                    relationships,
                    expand,
                    json,
                } => {
                    if *relationships {
                        let result = graph.search_relationships(query, *limit)?;
                        let hits: Vec<_> = result
                            .hits
                            .iter()
                            .filter(|((r, _, _), _)| *all_sessions || r.session_id == *session_id)
                            .collect();
                        if *json {
                            let rows: Vec<_> = hits
                                .iter()
                                .map(|((r, s, o), score)| {
                                    json!({
                                        "id": r.id.to_string(),
                                        "subject": s.name,
                                        "predicate": r.predicate,
                                        "object": o.name,
                                        "confidence": r.confidence,
                                        "score": score
                                    })
                                })
                                .collect();
                            println!("{}", serde_json::to_string_pretty(&rows)?);
                        } else {
                            if let Some(reason) = &result.fallback_reason {
                                println!("(fallback search: {reason})");
                            }
                            if hits.is_empty() {
                                println!("No relationships found.");
                            }
                            for ((r, s, o), score) in hits {
                                println!("  {:.3}  {} --[{}]--> {}", score, s.name, r.predicate, o.name);
                            }
                        }
                        return Ok(());
                    }

                    let mut search = EntitySearch::text(query.clone()).with_limit(*limit);
                    search.include_deprecated = *include_deprecated;
                    if !*all_sessions {
                        search.session_id = Some(*session_id);
                    }
                    if *semantic {
                        // A missing or failing embedder must not stop the
                        // search: carry on with keywords only.
                        let embedded = select_embedder(embedder).and_then(|chosen| match chosen {
                            Some(chosen) => Ok(chosen.embed(std::slice::from_ref(query))?.pop()),
                            None => Ok(None),
                        });
                        match embedded {
                            Ok(vector) => search.embedding = vector,
                            Err(error) => eprintln!(
                                "warning: could not embed the query ({error}); searching by keywords only"
                            ),
                        }
                    }
                    let context = if *expand {
                        graph.search_with_context(&search)?
                    } else {
                        memst_core::graph::SearchContext {
                            result: graph.search_entities(&search)?,
                            relationships: Vec::new(),
                        }
                    };
                    let around: Vec<_> = context
                        .relationships
                        .iter()
                        .map(|(r, s, o)| format!("{} --[{}]--> {}", s.name, r.predicate, o.name))
                        .collect();
                    let result = context.result;
                    let method = match result.method {
                        SearchMethod::Native => graph.backend_kind(),
                        SearchMethod::Fallback => "fallback",
                    };
                    if *json {
                        let hits: Vec<_> = result
                            .hits
                            .iter()
                            .map(|hit| {
                                json!({
                                    "id": hit.entity.id.to_string(),
                                    "name": hit.entity.name,
                                    "entity_type": hit.entity.entity_type,
                                    "score": hit.score
                                })
                            })
                            .collect();
                        let output = json!({
                            "method": method,
                            "fallback_reason": result.fallback_reason,
                            "hits": hits,
                            "relationships": around
                        });
                        println!("{}", serde_json::to_string_pretty(&output)?);
                    } else {
                        if let Some(reason) = &result.fallback_reason {
                            println!("(fallback search: {reason})");
                        }
                        if result.hits.is_empty() {
                            println!("No entities found.");
                        }
                        for hit in &result.hits {
                            println!(
                                "  {:.3}  {}  {} [{}]",
                                hit.score, hit.entity.id, hit.entity.name, hit.entity.entity_type
                            );
                        }
                        if !around.is_empty() {
                            println!("Around these:");
                            for line in &around {
                                println!("  {line}");
                            }
                        }
                    }
                }

                GraphCommand::Embed { session_id: _ } => {
                    let Some(chosen) = select_embedder(embedder)? else {
                        anyhow::bail!("--embedder none: nothing to embed with");
                    };
                    let embedded = graph.embed_pending(chosen.as_ref())?;
                    if embedded == 0 {
                        println!("Every entity is already embedded with {}.", chosen.id());
                    } else {
                        println!("Embedded {} entities with {}.", embedded, chosen.id());
                    }
                }

                GraphCommand::Ingest {
                    session_id,
                    all,
                    batch_chars,
                } => {
                    use memst_sleep::kg_extract::KgExtractionService;
                    use memst_sleep::kg_ingest::GraphIngestor;

                    // How many of the session's messages earlier runs covered.
                    let marker = session_path.join("graph.ingested");
                    let done: usize = if *all {
                        0
                    } else {
                        std::fs::read_to_string(&marker)
                            .ok()
                            .and_then(|text| text.trim().parse().ok())
                            .unwrap_or(0)
                    };
                    let messages = store.get_messages(*session_id)?;
                    let pending: Vec<memst_sleep::kg_ingest::SourceMessage> = messages
                        .iter()
                        .skip(done)
                        .map(|m| memst_sleep::kg_ingest::SourceMessage {
                            id: m.id,
                            text: message_text(m),
                            timestamp: Some(m.timestamp),
                        })
                        .filter(|m| !m.text.trim().is_empty())
                        .collect();
                    if pending.is_empty() {
                        println!("No new messages to ingest ({} already covered).", done.min(messages.len()));
                        return Ok(());
                    }

                    let llm = memst_core::llm::LlmLoader::default_provider()?;
                    // Extraction is the point; a missing embedder only means
                    // the new entities are not searchable by meaning yet.
                    let chosen = match select_embedder(embedder) {
                        Ok(chosen) => chosen,
                        Err(error) => {
                            eprintln!("warning: no embedder ({error}); run `memst graph embed` later");
                            None
                        }
                    };
                    let ingestor = GraphIngestor::new(KgExtractionService::new(llm));
                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
                    let report = runtime.block_on(ingestor.ingest_dated_messages(
                        &mut graph,
                        *session_id,
                        &pending,
                        *batch_chars,
                        chosen.as_deref(),
                    ))?;
                    std::fs::write(&marker, messages.len().to_string())?;

                    println!("Ingested {} message(s).", pending.len());
                    println!(
                        "Entities: {} new, {} already known.",
                        report.entities_created, report.entities_reused
                    );
                    println!(
                        "Relationships: {} new, {} seen again, {} skipped (unknown endpoint).",
                        report.relationships_created,
                        report.relationships_reinforced,
                        report.relationships_skipped
                    );
                    if report.relationships_superseded > 0 {
                        println!(
                            "Updated facts: {} earlier relationship(s) superseded.",
                            report.relationships_superseded
                        );
                    }
                    println!("Embedded: {}.", report.entities_embedded);
                }

                GraphCommand::Migrate => unreachable!("handled before the graph is opened"),
            }
        }
    }

    Ok(())
}
