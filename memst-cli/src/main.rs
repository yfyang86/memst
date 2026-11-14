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
        /// `auto` uses whatever the session already has (HelixDB, else
        /// graph.json) and creates new graphs in HelixDB.
        #[arg(long, global = true, default_value = "auto")]
        backend: String,

        #[command(subcommand)]
        command: GraphCommand,
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

        /// JSON output
        #[arg(long)]
        json: bool,
    },

    #[command(about = "Compute and store embeddings for entities (for --semantic search)")]
    Embed {
        /// Session ID
        session_id: Uuid,

        /// Re-embed entities that already have an embedding
        #[arg(long)]
        all: bool,
    },

    #[command(about = "Copy a session's graph.json into HelixDB")]
    Migrate {
        /// Session ID
        session_id: Uuid,
    },
}

/// Text an entity is embedded from: name, type and attributes.
fn entity_embedding_text(entity: &Entity) -> String {
    let has_attributes = entity.attributes.as_object().is_some_and(|m| !m.is_empty());
    if has_attributes {
        format!("{} ({}) {}", entity.name, entity.entity_type, entity.attributes)
    } else {
        format!("{} ({})", entity.name, entity.entity_type)
    }
}

/// Embed texts with the configured embedding API.
fn embed_texts(texts: &[String]) -> Result<Vec<Vec<f32>>> {
    let client = memst_core::llm::EmbeddingClient::new(memst_core::llm::EmbeddingConfig::default())?;
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
    Ok(runtime.block_on(client.embed_batch(texts))?)
}

/// Open a session's knowledge graph with the requested backend.
#[cfg(feature = "helix")]
fn open_graph(dir: &std::path::Path, backend: &str) -> Result<KnowledgeGraph> {
    let choice: memst_graph_helix::BackendChoice = backend.parse().map_err(anyhow::Error::msg)?;
    Ok(memst_graph_helix::open(dir, choice)?)
}

/// Open a session's knowledge graph (built without HelixDB: JSON only).
#[cfg(not(feature = "helix"))]
fn open_graph(dir: &std::path::Path, backend: &str) -> Result<KnowledgeGraph> {
    match backend {
        "auto" | "json" => Ok(KnowledgeGraph::new(dir)?),
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

        Command::Graph { backend, command } => {
            let store = SessionStore::open(&args.store)?;
            
            // Extract session_id from the command
            let session_id = match &command {
                GraphCommand::AddEntity { session_id, .. } => *session_id,
                GraphCommand::AddRel { session_id, .. } => *session_id,
                GraphCommand::Query { session_id, .. } => *session_id,
                GraphCommand::Stats { session_id } => *session_id,
                GraphCommand::Neighbors { session_id, .. } => *session_id,
                GraphCommand::Search { session_id, .. } => *session_id,
                GraphCommand::Embed { session_id, .. } => *session_id,
                GraphCommand::Migrate { session_id } => *session_id,
            };

            let session_path = store.session_path(session_id);

            if let GraphCommand::Migrate { .. } = command {
                #[cfg(feature = "helix")]
                {
                    let (entities, relationships) =
                        memst_graph_helix::migrate_json_to_helix(&session_path)?;
                    println!(
                        "Migrated {} entities and {} relationships to HelixDB ({})",
                        entities,
                        relationships,
                        session_path.join(memst_graph_helix::HELIX_DIR).display()
                    );
                    println!("graph.json was kept as a backup.");
                    return Ok(());
                }
                #[cfg(not(feature = "helix"))]
                anyhow::bail!("this memst was built without the 'helix' feature");
            }

            let mut graph = open_graph(&session_path, backend)?;

            match command {
                GraphCommand::AddEntity {
                    session_id,
                    name,
                    entity_type,
                    attributes,
                } => {
                    let mut entity = Entity::new(name, entity_type, *session_id);
                    if let Some(attrs) = attributes {
                        if let Ok(attrs_map) = serde_json::from_str::<serde_json::Value>(attrs) {
                            entity = entity.with_attributes(attrs_map);
                        }
                    }
                    let added = graph.add_entity(entity)?;
                    println!("Added entity: {} (type: {})", added.id, added.entity_type);
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
                    };
                    let _added = graph.add_relationship(relationship)?;
                    println!(
                        "Added relationship: {} --[{}]--> {}",
                        subject, predicate, object
                    );
                }

                GraphCommand::Query {
                    session_id: _,
                    entity_type,
                    limit,
                    json,
                } => {
                    // The type filter is part of the query, so the limit
                    // applies to matching entities (it used to be applied
                    // before filtering, which could return too few).
                    let mut query = EntityQuery::new().with_limit(*limit);
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

                GraphCommand::Stats { session_id: _ } => {
                    let stats = graph.stats()?;
                    println!("Graph Statistics for Session");
                    println!("==============================");
                    println!("Backend: {}", graph.backend_kind());
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
                    session_id: _,
                    query,
                    limit,
                    semantic,
                    include_deprecated,
                    json,
                } => {
                    let mut search = EntitySearch::text(query.clone()).with_limit(*limit);
                    search.include_deprecated = *include_deprecated;
                    if *semantic {
                        // A missing or failing embedding API must not stop
                        // the search: carry on with keywords only.
                        match embed_texts(std::slice::from_ref(query)) {
                            Ok(mut vectors) => search.embedding = vectors.pop(),
                            Err(error) => eprintln!(
                                "warning: could not embed the query ({error}); searching by keywords only"
                            ),
                        }
                    }
                    let result = graph.search_entities(&search)?;
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
                            "hits": hits
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
                    }
                }

                GraphCommand::Embed { session_id: _, all } => {
                    let embedded = graph.embedded_entity_ids()?;
                    let pending: Vec<Entity> = graph
                        .all_entities()?
                        .into_iter()
                        .filter(|e| *all || !embedded.contains(&e.id))
                        .collect();
                    if pending.is_empty() {
                        println!("All {} entities already have embeddings.", embedded.len());
                        return Ok(());
                    }
                    let texts: Vec<String> = pending.iter().map(entity_embedding_text).collect();
                    let vectors = embed_texts(&texts)?;
                    for (entity, vector) in pending.iter().zip(&vectors) {
                        graph.set_entity_embedding(entity.id, vector)?;
                    }
                    println!("Embedded {} entities.", pending.len());
                }

                GraphCommand::Migrate { .. } => unreachable!("handled before the graph is opened"),
            }
        }
    }

    Ok(())
}
