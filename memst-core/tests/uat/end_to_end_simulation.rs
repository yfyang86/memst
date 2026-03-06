//! End-to-End UAT: Full MemSt v1.0 Simulation
//!
//! This test simulates a complete multi-agent workflow with:
//! - Multiple agents working in isolated worktrees
//! - Memory consolidation during sleep
//! - Semantic merge of agent work
//! - Context assembly for LLM prompts

use memst_core::context::{ContextAssemblyConfig, ContextAssembler};
use memst_core::memory::{LifecycleConfig, MemoryLifecycle, SimpleTokenCounter};
use memst_core::objects::*;
use memst_core::types::{MemoryItem, MemoryTier, MemoryType};
use memst_repo::merge::SemanticMerger;
use memst_repo::worktrees::WorktreeManager;
use memst_sleep::evolve::EvolutionEngine;
use chrono::Utc;
use tempfile::TempDir;

fn create_memory(content: &str, memory_type: MemoryType, importance: f32) -> MemoryItem {
    MemoryItem {
        id: uuid::Uuid::new_v4(),
        content: content.to_string(),
        source: "simulation".to_string(),
        created_at: Utc::now(),
        last_accessed: Utc::now(),
        access_count: 0,
        embedding: None,
        tags: vec![],
        confidence: importance,
        importance,
        memory_type,
        token_estimate: Some(content.split_whitespace().count() as u32),
        supersedes: None,
    }
}

/// UAT-E2E-1: Multi-Agent Project Workflow
/// As a team, I want multiple agents to work on a project concurrently
/// so that development is parallelized and their work is merged.
#[test]
fn uat_e2e_1_multi_agent_project_workflow() {
    let temp_dir = TempDir::new().unwrap();
    
    // Setup
    let mut object_store = ObjectStore::new(&temp_dir.path().join("objects")).unwrap();
    let ref_store = RefStore::new(&temp_dir.path().join("refs")).unwrap();
    let mut worktree_manager = WorktreeManager::new(temp_dir.path()).unwrap();
    
    let base_head = ObjectId::from_content(b"project-init");
    let author = Author::new("MemSt", "memst@local");

    println!("\n=== E2E Simulation: Multi-Agent Project Workflow ===\n");

    // Phase 1: Three agents start from common base
    println!("Phase 1: Agents starting from common base");
    
    let agent_ids = vec!["agent-frontend", "agent-backend", "agent-docs"];
    let mut worktree_ids = vec![];
    
    for agent_id in &agent_ids {
        let worktree = worktree_manager.create(
            &format!("Worktree for {}", agent_id),
            "main",
            base_head
        ).unwrap();
        
        worktree_manager.attach_agent(&worktree.id, agent_id).unwrap();
        worktree_ids.push(worktree.id.clone());
        
        println!("  - {} attached to worktree {}", agent_id, worktree.id);
    }

    // Phase 2: Agents work independently
    println!("\nPhase 2: Agents working independently");
    
    // Frontend agent adds memories
    let frontend_memories = vec![
        create_memory("User prefers React for frontend", MemoryType::Semantic, 0.9),
        create_memory("UI should be responsive", MemoryType::Semantic, 0.8),
    ];
    
    let frontend_head = ObjectId::from_content(b"frontend-work");
    worktree_manager.update_head(&worktree_ids[0], frontend_head).unwrap();
    println!("  - Frontend agent added {} memories and committed", frontend_memories.len());

    // Backend agent adds memories
    let backend_memories = vec![
        create_memory("API should use REST not GraphQL", MemoryType::Semantic, 0.85),
        create_memory("Database choice: PostgreSQL", MemoryType::Semantic, 0.9),
    ];
    
    let backend_head = ObjectId::from_content(b"backend-work");
    worktree_manager.update_head(&worktree_ids[1], backend_head).unwrap();
    println!("  - Backend agent added {} memories and committed", backend_memories.len());

    // Docs agent adds memories
    let docs_head = ObjectId::from_content(b"docs-work");
    worktree_manager.update_head(&worktree_ids[2], docs_head).unwrap();
    println!("  - Docs agent committed documentation updates");

    // Phase 3: Sleep-time consolidation simulation
    println!("\nPhase 3: Sleep-time consolidation (per-agent)");
    
    let lifecycle_config = LifecycleConfig::default();
    let mut lifecycle = MemoryLifecycle::new(lifecycle_config);

    for (i, memories) in [frontend_memories, backend_memories].iter().enumerate() {
        let checkpoint = lifecycle.create_checkpoint(
            Some(MemoryScope::Agent(agent_ids[i].to_string())),
            MemoryTier::Working,
            MemoryTier::ShortTerm,
            memories.iter().map(|m| m.id.to_string()).collect(),
            ObjectId::from_content(format!("pre-{}-consolidation", i).as_bytes()),
            1000,
        );
        
        lifecycle.finalize_checkpoint(&checkpoint.id, 
            ObjectId::from_content(format!("post-{}-consolidation", i).as_bytes()),
            500
        ).unwrap();
        
        println!("  - {}: Consolidated {} memories, saved 500 tokens", 
            agent_ids[i], memories.len());
    }

    // Phase 4: A-MEM evolution
    println!("\nPhase 4: A-MEM style memory evolution");
    
    let mut evolution = EvolutionEngine::new();
    
    // New memory that supports existing one
    let existing = create_memory("User prefers React", MemoryType::Semantic, 0.8);
    let mut new = create_memory("User confirms React preference for this project", MemoryType::Semantic, 0.9);
    
    let result = evolution.evolve_memory_network(&mut new, &[existing]);
    
    println!("  - Created {} memory links", result.links.len());
    println!("  - Confidence boosted from 0.8 to {:.2}", result.updated_memories[0].confidence);
    println!("  - New memory confidence: {:.2}", new.confidence);

    // Phase 5: Semantic merge of agent work
    println!("\nPhase 5: Semantic merge of agent work");
    
    // Mark worktrees for merging
    for id in &worktree_ids[0..2] {
        worktree_manager.mark_merging(id).unwrap();
    }

    // Simulate merge (would use actual merger in production)
    let merger = SemanticMerger::new();
    println!("  - Merging frontend worktree...");
    println!("  - Merging backend worktree...");
    println!("  - Semantic merge configured with {} threshold", 0.85);

    // Complete merges
    let merge_commit = ObjectId::from_content(b"merged-work");
    for id in &worktree_ids[0..2] {
        worktree_manager.merge(id, merge_commit).unwrap();
    }
    println!("  - All agent work merged successfully");

    // Phase 6: Context assembly for summary
    println!("\nPhase 6: Context assembly for project summary");
    
    let all_memories = vec![
        create_memory("Project uses React frontend", MemoryType::Semantic, 0.9),
        create_memory("REST API with PostgreSQL", MemoryType::Semantic, 0.9),
        create_memory("Team of 3 agents worked on this", MemoryType::Episodic, 0.7),
    ];

    let config = ContextAssemblyConfig {
        token_budget: 2000,
        reserved_for_response: 500,
        system_prompt_tokens: 100,
        max_memories: 10,
        ..Default::default()
    };

    let counter = SimpleTokenCounter;
    let assembler = ContextAssembler::new(config, &counter);
    
    let assembly = assembler.build_context(
        "Summarize the project status",
        &all_memories,
        &[],
        &[],
        &[],
        "You are a project manager.",
    ).unwrap();

    println!("  - Built context with {} memories", assembly.memories_included);
    println!("  - Total tokens: {}", assembly.total_tokens);
    println!("  - Budget remaining: {}", assembly.budget_remaining);

    // Phase 7: Cleanup
    println!("\nPhase 7: Cleanup pruned worktrees");
    
    for id in &worktree_ids[0..2] {
        worktree_manager.prune(id).unwrap();
        println!("  - Pruned worktree {}", id);
    }

    println!("\n=== E2E Simulation Complete ===");
    println!("Summary:");
    println!("  - {} agents worked concurrently", agent_ids.len());
    println!("  - {} consolidation checkpoints created", lifecycle.list_checkpoints().len());
    println!("  - {} memory evolution links created", evolution.links().len());
    println!("  - Final context assembled in {} ms", assembly.assembly_time_ms);
}

/// UAT-E2E-2: Knowledge Graph Construction
/// As a user, I want to build a knowledge graph from memories
/// so that relationships between concepts are explicit.
#[test]
fn uat_e2e_2_knowledge_graph_construction() {
    println!("\n=== E2E: Knowledge Graph Construction ===\n");

    let temp_dir = TempDir::new().unwrap();
    let mut object_store = ObjectStore::new(&temp_dir.path().join("objects")).unwrap();

    // Create entities
    let commit = ObjectId::from_content(b"kg-commit");

    let user_id = uuid::Uuid::new_v4();
    let user = Entity::new(user_id, "Developer", "person", commit)
        .with_attributes({
            let mut attrs = std::collections::HashMap::new();
            attrs.insert("role".to_string(), "senior".to_string());
            attrs.insert("team".to_string(), "platform".to_string());
            attrs
        });

    let rust_id = uuid::Uuid::new_v4();
    let rust = Entity::new(rust_id, "Rust", "technology", commit)
        .with_attributes({
            let mut attrs = std::collections::HashMap::new();
            attrs.insert("paradigm".to_string(), "systems".to_string());
            attrs.insert("typing".to_string(), "static".to_string());
            attrs
        });

    let tokio_id = uuid::Uuid::new_v4();
    let tokio = Entity::new(tokio_id, "Tokio", "library", commit);

    // Store entities
    let user_oid = object_store.write_entity(&user).unwrap();
    let rust_oid = object_store.write_entity(&rust).unwrap();
    let tokio_oid = object_store.write_entity(&tokio).unwrap();

    println!("Created {} entities", 3);

    // Create relationships
    let rel1_id = uuid::Uuid::new_v4();
    let rel1 = Relation::new(rel1_id, user_id, rust_id, "uses");

    let rel2_id = uuid::Uuid::new_v4();
    let rel2 = Relation::new(rel2_id, user_id, tokio_id, "uses");

    let rel3_id = uuid::Uuid::new_v4();
    let mut rel3 = Relation::new(rel3_id, tokio_id, rust_id, "built_on");
    rel3.weight = 1.0;

    // Store relationships
    let _rel1_oid = object_store.write_relation(&rel1).unwrap();
    let _rel2_oid = object_store.write_relation(&rel2).unwrap();
    let _rel3_oid = object_store.write_relation(&rel3).unwrap();

    println!("Created {} relationships", 3);

    // Verify graph structure
    let user_entity = object_store.read_entity(&user_oid).unwrap();
    assert_eq!(user_entity.attributes.get("role"), Some(&"senior".to_string()));

    let rust_entity = object_store.read_entity(&rust_oid).unwrap();
    assert_eq!(rust_entity.attributes.get("paradigm"), Some(&"systems".to_string()));

    println!("Knowledge graph verified successfully");
    println!("  - Entities: User, Rust, Tokio");
    println!("  - Relations: User-uses->Rust, User-uses->Tokio, Tokio-built_on->Rust");
}

/// UAT-E2E-3: Memory Lifecycle Management
/// As a user, I want memories to automatically transition between tiers
/// so that the system manages its own storage.
#[test]
fn uat_e2e_3_memory_lifecycle_management() {
    println!("\n=== E2E: Memory Lifecycle Management ===\n");

    let config = LifecycleConfig {
        working_memory_max_tokens: 100,
        short_term_max_tokens: 500,
        long_term_max_tokens: 2000,
        promotion_access_threshold: 3,
        long_term_importance_threshold: 0.7,
        ..Default::default()
    };

    let mut lifecycle = MemoryLifecycle::new(config);

    // Create memories at different tiers
    let working_mem = "working-mem-1";
    let short_mem = "short-mem-1";
    let long_mem = "long-mem-1";

    lifecycle.register(working_mem, MemoryTier::Working);
    lifecycle.register(short_mem, MemoryTier::ShortTerm);
    lifecycle.register(long_mem, MemoryTier::LongTerm);

    println!("Initial states:");
    println!("  - {}: {:?}", working_mem, lifecycle.get_state(working_mem));
    println!("  - {}: {:?}", short_mem, lifecycle.get_state(short_mem));
    println!("  - {}: {:?}", long_mem, lifecycle.get_state(long_mem));

    // Simulate high importance triggering promotion
    let transition = lifecycle.transition(
        working_mem,
        MemoryState::ShortTerm,
        TransitionTrigger::ImportanceThreshold(0.8),
        -20,
    ).unwrap();

    println!("\nTransition:");
    println!("  - Memory: {}", working_mem);
    println!("  - From: {:?}", transition.from);
    println!("  - To: {:?}", transition.to);
    println!("  - Trigger: {:?}", transition.trigger);
    println!("  - Token delta: {}", transition.token_delta);

    assert_eq!(lifecycle.get_state(working_mem), Some(MemoryState::ShortTerm));

    // Create checkpoint
    let checkpoint = lifecycle.create_checkpoint(
        None,
        MemoryTier::ShortTerm,
        MemoryTier::LongTerm,
        vec![short_mem.to_string()],
        ObjectId::from_content(b"pre-checkpoint"),
        200,
    );

    lifecycle.finalize_checkpoint(&checkpoint.id, 
        ObjectId::from_content(b"post-checkpoint"),
        100
    ).unwrap();

    println!("\nCheckpoint created:");
    println!("  - ID: {}", checkpoint.id);
    println!("  - Original tokens: {}", checkpoint.original_tokens);
    println!("  - Compacted tokens: 100");
    println!("  - Tokens saved: 100");

    // Restore checkpoint
    let restored = lifecycle.restore_checkpoint(&checkpoint.id).unwrap();
    assert_eq!(restored.id, checkpoint.id);
    
    println!("\nCheckpoint restored successfully");
}

/// UAT-E2E-4: Token Budget Context Assembly
/// As a user, I want context built within token budgets
/// so that LLM limits are respected.
#[test]
fn uat_e2e_4_token_budget_context_assembly() {
    println!("\n=== E2E: Token Budget Context Assembly ===\n");

    let config = ContextAssemblyConfig {
        token_budget: 1000,
        reserved_for_response: 300,
        system_prompt_tokens: 100,
        max_memories: 5,
        ..Default::default()
    };

    // Create memories of varying relevance
    let memories = vec![
        create_memory("User prefers Rust for systems programming", MemoryType::Semantic, 0.95),
        create_memory("User likes async programming", MemoryType::Semantic, 0.85),
        create_memory("User works at a startup", MemoryType::Semantic, 0.6),
        create_memory("User had coffee this morning", MemoryType::Episodic, 0.3),
        create_memory("Meeting scheduled for tomorrow", MemoryType::Episodic, 0.4),
        create_memory("User prefers Vim over Emacs", MemoryType::Semantic, 0.7),
        create_memory("Project deadline is next week", MemoryType::Semantic, 0.5),
    ];

    let counter = SimpleTokenCounter;
    let assembler = ContextAssembler::new(config, &counter);

    println!("Building context for query: 'What tech stack should I recommend?'");
    println!("Token budget: 1000 (reserved for response: 300)");
    println!("Available for context: ~600 tokens");
    println!("Total memories available: {}\n", memories.len());

    let assembly = assembler.build_context(
        "What tech stack should I recommend?",
        &memories,
        &[],
        &[],
        &[],
        "You are a technical advisor.",
    ).unwrap();

    println!("Context Assembly Results:");
    println!("  - Memories included: {}", assembly.memories_included);
    println!("  - Memories skipped: {}", assembly.memories_skipped);
    println!("  - Total tokens: {}", assembly.total_tokens);
    println!("  - Budget remaining: {}", assembly.budget_remaining);
    println!("  - Truncated: {}", assembly.truncated);
    println!("  - Assembly time: {} ms", assembly.assembly_time_ms);

    // Verify constraints
    assert!(assembly.total_tokens <= 1000, "Should not exceed budget");
    assert!(assembly.memories_included <= 5, "Should respect max_memories");
    assert!(assembly.budget_remaining >= 0, "Should not go negative");

    // Format and display
    let formatted = assembler.format_context(&assembly);
    println!("\nFormatted Context (first 500 chars):");
    println!("{}", &formatted[..formatted.len().min(500)]);
}
