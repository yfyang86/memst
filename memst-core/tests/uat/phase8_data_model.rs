//! UAT: Phase 8 - Data Model
//!
//! User Acceptance Tests for:
//! - Blake3 hashing
//! - WAL (Write-Ahead Log)
//! - Memory lifecycle
//! - New object types (Skill, Entity, Relation, ContextFile)

use memst_core::memory::{LifecycleConfig, MemoryLifecycle, MemoryState, TransitionTrigger};
use memst_core::objects::*;
use memst_core::types::{MemoryItem, MemoryTier, MemoryType};
use std::collections::HashMap;
use tempfile::TempDir;

/// UAT-8.1: Blake3 Content Addressing
/// As a user, I want objects to be identified by Blake3 hashes
/// so that I get faster hashing with the same collision resistance.
#[test]
fn uat_8_1_blake3_hashing() {
    let content1 = b"Hello, World!";
    let content2 = b"Hello, World!";
    let content3 = b"Different content";

    let oid1 = ObjectId::from_content(content1);
    let oid2 = ObjectId::from_content(content2);
    let oid3 = ObjectId::from_content(content3);

    // Same content produces same hash (deduplication)
    assert_eq!(oid1, oid2, "Same content should produce same Blake3 hash");

    // Different content produces different hash
    assert_ne!(oid1, oid3, "Different content should produce different hash");

    // Verify hex representation
    let hex = oid1.to_hex();
    assert_eq!(hex.len(), 64, "Blake3 produces 256-bit (64 hex char) hashes");

    // Roundtrip through hex
    let oid_from_hex = ObjectId::from_hex(&hex).expect("Should parse valid hex");
    assert_eq!(oid1, oid_from_hex, "Hex roundtrip should preserve ObjectId");
}

/// UAT-8.2: Write-Ahead Log Recovery
/// As a user, I want durability guarantees so that crashes don't corrupt memory.
#[test]
fn uat_8_2_wal_durability() {
    use memst_repo::wal::{WriteAheadLog, WalOp};

    let temp_dir = TempDir::new().unwrap();
    let mut wal = WriteAheadLog::open(temp_dir.path()).unwrap();

    // Simulate multiple memory operations
    let operations = vec![
        WalOp::WriteObject {
            oid: ObjectId::from_content(b"memory1"),
            object_type: "blob".to_string(),
            data: b"User likes Rust".to_vec(),
        },
        WalOp::UpdateRef {
            name: "main".to_string(),
            ref_type: "branch".to_string(),
            old_oid: None,
            new_oid: ObjectId::from_content(b"commit1"),
        },
        WalOp::WriteObject {
            oid: ObjectId::from_content(b"memory2"),
            object_type: "blob".to_string(),
            data: b"User prefers Tokio".to_vec(),
        },
    ];

    // Append batch atomically
    let entries = wal.append_batch(operations.clone()).unwrap();
    assert_eq!(entries.len(), 3, "All operations should be logged");

    // Simulate crash recovery by reopening WAL
    drop(wal);
    let wal = WriteAheadLog::open(temp_dir.path()).unwrap();
    let recovered = wal.recover().unwrap();

    assert_eq!(recovered.len(), 3, "Should recover all entries after crash");
    
    // Verify content
    match &recovered[0].operation {
        WalOp::WriteObject { data, .. } => {
            assert_eq!(data, b"User likes Rust");
        }
        _ => panic!("Expected WriteObject operation"),
    }
}

/// UAT-8.3: Memory Lifecycle State Machine
/// As a user, I want memories to transition between tiers automatically
/// so that important information is preserved and less important info is archived.
#[test]
fn uat_8_3_memory_lifecycle_transitions() {
    let config = LifecycleConfig {
        working_memory_max_tokens: 100,
        short_term_max_tokens: 500,
        long_term_max_tokens: 10000,
        promotion_access_threshold: 3,
        long_term_importance_threshold: 0.7,
        ..Default::default()
    };

    let mut lifecycle = MemoryLifecycle::new(config);

    // Register a new memory in working tier
    let mem_id = "mem-001";
    let state = lifecycle.register(mem_id, MemoryTier::Working);
    assert_eq!(state, MemoryState::Active);

    // Simulate high importance triggering promotion
    let transition = lifecycle.transition(
        mem_id,
        MemoryState::ShortTerm,
        TransitionTrigger::ImportanceThreshold(0.8),
        -50, // Token delta
    ).unwrap();

    assert_eq!(transition.from, MemoryState::Active);
    assert_eq!(transition.to, MemoryState::ShortTerm);
    assert_eq!(transition.token_delta, -50);

    // Verify state tracking
    assert_eq!(lifecycle.get_state(mem_id), Some(MemoryState::ShortTerm));

    // Check transition history
    let history = lifecycle.transitions();
    assert_eq!(history.len(), 1);
    assert!(matches!(&history[0].trigger, TransitionTrigger::ImportanceThreshold(0.8)));
}

/// UAT-8.4: Compaction Checkpoints
/// As a user, I want lossless rewind capability
/// so that I can undo compaction if needed.
#[test]
fn uat_8_4_compaction_checkpoints() {
    let config = LifecycleConfig::default();
    let mut lifecycle = MemoryLifecycle::new(config);

    let pre_commit = ObjectId::from_content(b"pre-compaction");
    let memory_ids = vec!["mem-1".to_string(), "mem-2".to_string(), "mem-3".to_string()];

    // Create checkpoint before compaction
    let checkpoint = lifecycle.create_checkpoint(
        None, // scope
        MemoryTier::Working,
        MemoryTier::ShortTerm,
        memory_ids.clone(),
        pre_commit,
        1000, // original tokens
    );

    assert!(checkpoint.id.starts_with("chk-"));
    assert_eq!(checkpoint.memory_ids, memory_ids);
    assert_eq!(checkpoint.original_tokens, 1000);
    assert!(checkpoint.post_commit.is_none()); // Not finalized yet

    // Finalize after compaction
    let post_commit = ObjectId::from_content(b"post-compaction");
    lifecycle.finalize_checkpoint(&checkpoint.id, post_commit, 500).unwrap();

    // Verify checkpoint is retrievable
    let restored = lifecycle.restore_checkpoint(&checkpoint.id).unwrap();
    assert_eq!(restored.post_commit, Some(post_commit));
    assert_eq!(restored.compacted_tokens, 500);
    assert_eq!(restored.token_saved(), 500);
}

/// UAT-8.5: Skill Object CRUD
/// As an agent, I want to store and retrieve procedural knowledge
/// so that I can apply learned workflows.
#[test]
fn uat_8_5_skill_object_lifecycle() {
    // Create a skill
    let mut skill = Skill::new(
        "rust-async-setup",
        "Setup Rust Async Project",
        "Initialize a new Rust project with Tokio",
    )
    .with_trigger("set up rust async")
    .with_trigger("initialize tokio project");

    // Add steps
    skill.steps.push(SkillStep {
        order: 1,
        action: "Run cargo new project".to_string(),
        tool: Some("shell".to_string()),
        conditions: vec!["directory exists".to_string()],
        on_failure: SkillFailurePolicy::Abort,
    });

    skill.steps.push(SkillStep {
        order: 2,
        action: "Add tokio dependency".to_string(),
        tool: Some("cargo".to_string()),
        conditions: vec![], // No conditions
        on_failure: SkillFailurePolicy::Retry(3),
    });

    // Verify skill structure
    assert_eq!(skill.slug, "rust-async-setup");
    assert_eq!(skill.trigger_patterns.len(), 2);
    assert_eq!(skill.steps.len(), 2);
    assert_eq!(skill.usage_count, 0);
    assert_eq!(skill.success_rate, 1.0);

    // Simulate usage with success
    skill.record_outcome(true);
    assert_eq!(skill.usage_count, 1);
    assert!(skill.success_rate > 0.99);

    // Simulate usage with failure
    skill.record_outcome(false);
    assert_eq!(skill.usage_count, 2);
    assert!(skill.success_rate < 1.0);
    assert!(skill.success_rate > 0.0);
}

/// UAT-8.6: Entity and Relation Objects
/// As a user, I want to build a knowledge graph
/// so that I can understand relationships between concepts.
#[test]
fn uat_8_6_knowledge_graph_objects() {
    let commit = ObjectId::from_content(b"commit");

    // Create entities
    let user_id = uuid::Uuid::new_v4();
    let user = Entity::new(user_id, "User", "person", commit);

    let tokio_id = uuid::Uuid::new_v4();
    let tokio = Entity::new(tokio_id, "Tokio", "technology", commit)
        .with_attributes({
            let mut attrs = HashMap::new();
            attrs.insert("ecosystem".to_string(), "async".to_string());
            attrs.insert("language".to_string(), "rust".to_string());
            attrs
        });

    // Create relationship
    let relation_id = uuid::Uuid::new_v4();
    let relation = Relation::new(relation_id, user_id, tokio_id, "uses");

    // Verify structure
    assert_eq!(user.label, "User");
    assert_eq!(tokio.entity_type, "technology");
    assert_eq!(tokio.attributes.get("ecosystem"), Some(&"async".to_string()));
    
    assert_eq!(relation.label, "uses");
    assert_eq!(relation.from, user_id);
    assert_eq!(relation.to, tokio_id);
    assert_eq!(relation.weight, 1.0);
}

/// UAT-8.7: ContextFile with Frontmatter
/// As a user, I want human-readable memory storage
/// so that I can inspect and edit memories directly.
#[test]
fn uat_8_7_context_file_format() {
    use chrono::Utc;

    let frontmatter = Frontmatter::new(
        "mem-550e8400",
        MemoryType::Semantic,
        MemoryTier::LongTerm,
    );

    let context_file = ContextFile::new(
        frontmatter,
        "# User prefers Tokio over async-std\n\n\
         The user has consistently chosen Tokio as the async runtime \
         across three separate projects.",
        "context/entities/tokio.md",
    );

    // Serialize to markdown
    let markdown = context_file.to_markdown();
    
    // Verify format
    assert!(markdown.starts_with("---"));
    assert!(markdown.contains("\"id\": \"mem-550e8400\""));
    assert!(markdown.contains("\"type\": \"Semantic\""));
    assert!(markdown.contains("# User prefers Tokio over async-std"));

    // Parse back
    let parsed = ContextFile::from_markdown(&markdown, "context/entities/tokio.md").unwrap();
    assert_eq!(parsed.frontmatter.id, "mem-550e8400");
    assert_eq!(parsed.frontmatter.memory_type, MemoryType::Semantic);
    assert_eq!(parsed.content.trim(), context_file.content.trim());
}

/// UAT-8.8: Object Store with New Types
/// As a user, I want to persist all object types
/// so that my knowledge graph survives restarts.
#[test]
fn uat_8_8_object_store_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let mut store = ObjectStore::new(&temp_dir.path().to_path_buf()).unwrap();

    // Store a skill
    let skill = Skill::new("test-skill", "Test Skill", "A test skill");
    let skill_oid = store.write_skill(&skill).unwrap();

    // Store an entity
    let entity_id = uuid::Uuid::new_v4();
    let entity = Entity::new(entity_id, "Test Entity", "test", skill_oid);
    let entity_oid = store.write_entity(&entity).unwrap();

    // Store a relation
    let relation_id = uuid::Uuid::new_v4();
    let relation = Relation::new(relation_id, entity_id, entity_id, "self-ref");
    let relation_oid = store.write_relation(&relation).unwrap();

    // Store a context file
    let frontmatter = Frontmatter::new("ctx-1", MemoryType::Semantic, MemoryTier::Working);
    let context_file = ContextFile::new(frontmatter, "Test content", "test.md");
    let context_oid = store.write_context_file(&context_file).unwrap();

    // Read them back
    let read_skill = store.read_skill(&skill_oid).unwrap();
    assert_eq!(read_skill.slug, "test-skill");

    let read_entity = store.read_entity(&entity_oid).unwrap();
    assert_eq!(read_entity.label, "Test Entity");

    let read_relation = store.read_relation(&relation_oid).unwrap();
    assert_eq!(read_relation.label, "self-ref");

    let read_context = store.read_context_file(&context_oid).unwrap();
    assert_eq!(read_context.frontmatter.id, "ctx-1");
}

/// Extension trait for checkpoint
trait CheckpointExt {
    fn token_saved(&self) -> u32;
}

impl CheckpointExt for memst_core::memory::CompactionCheckpoint {
    fn token_saved(&self) -> u32 {
        self.original_tokens.saturating_sub(self.compacted_tokens)
    }
}
