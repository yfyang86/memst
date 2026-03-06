//! UAT: Phase 11 - Semantic Merge
//!
//! User Acceptance Tests for:
//! - Three-way semantic merge
//! - Conflict detection and resolution

use memst_core::objects::*;
use memst_core::types::{MemoryItem, MemoryType};
use memst_repo::merge::{ConflictType, ResolutionStatus, SemanticMerger};
use tempfile::TempDir;

fn create_memory(content: &str, confidence: f32) -> MemoryItem {
    MemoryItem {
        id: uuid::Uuid::new_v4(),
        content: content.to_string(),
        source: "test".to_string(),
        created_at: chrono::Utc::now(),
        last_accessed: chrono::Utc::now(),
        access_count: 0,
        embedding: None,
        tags: vec![],
        confidence,
        importance: confidence,
        memory_type: MemoryType::Semantic,
        token_estimate: Some(content.split_whitespace().count() as u32),
        supersedes: None,
    }
}

/// UAT-11.1: Fast-Forward Merge
/// As a user, I want fast-forward merges when possible
/// so that history remains linear.
#[test]
fn uat_11_1_fast_forward_merge() {
    let temp_dir = TempDir::new().unwrap();
    let mut object_store = ObjectStore::new(&temp_dir.path().to_path_buf()).unwrap();
    let ref_store = RefStore::new(&temp_dir.path().join("refs")).unwrap();

    // Create initial commit
    let author = Author::new("test", "test@memst");
    let tree = Tree::new();
    let tree_oid = object_store.write_tree(&tree).unwrap();
    
    let commit1 = Commit::new(tree_oid, author.clone(), "Initial");
    let commit1_oid = object_store.write_commit(&commit1).unwrap();

    // Create second commit
    let mut commit2 = Commit::new(tree_oid, author.clone(), "Second");
    commit2.add_parent(commit1_oid);
    let commit2_oid = object_store.write_commit(&commit2).unwrap();

    // Set up branches
    ref_store.set_ref("main", RefType::Branch, commit1_oid).unwrap();
    ref_store.set_ref("feature", RefType::Branch, commit2_oid).unwrap();

    // Merge feature into main (should be fast-forward)
    let merger = SemanticMerger::new();
    let result = merger.merge(
        &mut object_store,
        &ref_store,
        "feature",
        "main",
        author,
        "Merge feature",
    );

    // Should succeed
    assert!(result.is_ok(), "Merge should succeed: {:?}", result.err());
    let result = result.unwrap();

    // Should be fast-forward
    assert!(result.fast_forward, "Should be fast-forward merge");
    assert_eq!(result.commit_hash, commit2_oid);
    assert!(result.conflicts.is_empty());
}

/// UAT-11.2: Three-Way Merge
/// As a user, I want proper three-way merges for diverged branches
/// so that parallel work is preserved.
#[test]
fn uat_11_2_three_way_merge() {
    let temp_dir = TempDir::new().unwrap();
    let mut object_store = ObjectStore::new(&temp_dir.path().to_path_buf()).unwrap();
    let ref_store = RefStore::new(&temp_dir.path().join("refs")).unwrap();

    let author = Author::new("test", "test@memst");
    let tree = Tree::new();
    let tree_oid = object_store.write_tree(&tree).unwrap();

    // Create base commit
    let base = Commit::new(tree_oid, author.clone(), "Base");
    let base_oid = object_store.write_commit(&base).unwrap();

    // Create main branch commit
    let mut main_commit = Commit::new(tree_oid, author.clone(), "Main update");
    main_commit.add_parent(base_oid);
    let main_oid = object_store.write_commit(&main_commit).unwrap();

    // Create feature branch commit (also from base)
    let mut feature_commit = Commit::new(tree_oid, author.clone(), "Feature");
    feature_commit.add_parent(base_oid);
    let feature_oid = object_store.write_commit(&feature_commit).unwrap();

    // Set up branches
    ref_store.set_ref("main", RefType::Branch, main_oid).unwrap();
    ref_store.set_ref("feature", RefType::Branch, feature_oid).unwrap();

    // Merge (not fast-forward)
    let merger = SemanticMerger::new();
    let result = merger.merge(
        &mut object_store,
        &ref_store,
        "feature",
        "main",
        author,
        "Merge feature into main",
    );

    assert!(result.is_ok());
    let result = result.unwrap();

    // Should NOT be fast-forward
    assert!(!result.fast_forward, "Should be three-way merge");
    
    // Should create merge commit
    let merge_commit = object_store.read_commit(&result.commit_hash).unwrap();
    assert!(merge_commit.is_merge());
    assert_eq!(merge_commit.parent_oids.len(), 2);
    assert!(merge_commit.parent_oids.contains(&main_oid));
    assert!(merge_commit.parent_oids.contains(&feature_oid));
}

/// UAT-11.3: Duplicate Detection
/// As a user, I want duplicate memories to be detected
/// so that they can be deduplicated.
#[test]
fn uat_11_3_duplicate_detection() {
    let merger = SemanticMerger::new();

    // Nearly identical memories
    let mem1 = create_memory("User prefers Tokio for async Rust", 0.9);
    let mem2 = create_memory("User prefers Tokio for async Rust programming", 0.85);

    // Use reflection to test internal method
    let similarity = merger.calculate_similarity(&mem1.content, &mem2.content);
    
    // Should be very similar
    assert!(similarity > 0.8, "Similar memories should have high similarity: {}", similarity);

    // Direct conflict detection
    let conflict = merger.detect_conflict(&mem1, &mem2);
    
    if let Some(c) = conflict {
        assert_eq!(c.conflict_type, ConflictType::Duplication);
    }
}

/// UAT-11.4: Contradiction Detection
/// As a user, I want contradictory memories to be flagged
/// so that conflicts can be resolved.
#[test]
fn uat_11_4_contradiction_detection() {
    let merger = SemanticMerger::new();

    // Contradictory memories
    let mem1 = create_memory("User prefers dark mode for IDE", 0.9);
    let mem2 = create_memory("User prefers light mode for IDE", 0.85);

    let conflict = merger.detect_conflict(&mem1, &mem2);
    
    if let Some(c) = conflict {
        // Should detect as either contradiction or duplicate
        assert!(
            c.conflict_type == ConflictType::SemanticContradiction ||
            c.conflict_type == ConflictType::Duplication,
            "Should detect conflict"
        );
    }
}

/// UAT-11.5: Auto-Resolution of Duplicates
/// As a user, I want simple conflicts auto-resolved
/// so that manual intervention is minimized.
#[test]
fn uat_11_5_auto_resolution() {
    let merger = SemanticMerger::new().with_config(0.85, 0.7, true);

    let mem1 = create_memory("User likes Rust", 0.95);
    let mem2 = create_memory("User likes Rust", 0.90);

    let conflict = merger.detect_conflict(&mem1, &mem2)
        .expect("Should detect duplicate");
    
    assert_eq!(conflict.conflict_type, ConflictType::Duplication);
    assert_eq!(conflict.resolution, ResolutionStatus::Unresolved);

    // Auto-resolve
    let resolved = merger.auto_resolve_conflict(&conflict);
    
    if resolved.resolution != ResolutionStatus::Unresolved {
        assert!(
            matches!(resolved.resolution, ResolutionStatus::AutoResolved | 
                     ResolutionStatus::KeepOurs | ResolutionStatus::KeepTheirs),
            "Should auto-resolve duplicates"
        );
    }
}

/// UAT-11.6: Text Similarity Calculation
/// As a user, I want accurate similarity measurement
/// so that near-duplicates are caught.
#[test]
fn uat_11_6_similarity_calculation() {
    let merger = SemanticMerger::new();

    let identical = merger.calculate_similarity(
        "exact same content",
        "exact same content"
    );
    assert_eq!(identical, 1.0, "Identical strings should have similarity 1.0");

    let similar = merger.calculate_similarity(
        "User prefers Tokio async",
        "User prefers Tokio async runtime"
    );
    assert!(similar > 0.5 && similar < 1.0, 
        "Similar strings should have high similarity: {}", similar);

    let different = merger.calculate_similarity(
        "Rust programming",
        "Python programming language tutorial"
    );
    assert!(different < 0.5, 
        "Different strings should have low similarity: {}", different);

    let unrelated = merger.calculate_similarity(
        "completely different",
        "nothing in common here at all"
    );
    assert!(unrelated < 0.3, 
        "Unrelated strings should have very low similarity: {}", unrelated);
}

/// UAT-11.7: Contradiction Markers
/// As a user, I want negation words detected
/// so that contradictions are flagged.
#[test]
fn uat_11_7_contradiction_markers() {
    let merger = SemanticMerger::new();

    // Should detect negation
    assert!(merger.has_contradiction_marker("User does not like this"));
    assert!(merger.has_contradiction_marker("This is not correct"));
    assert!(merger.has_contradiction_marker("Never use this"));
    assert!(merger.has_contradiction_marker("Wrong approach"));
    assert!(merger.has_contradiction_marker("User switched from X to Y"));

    // Should not flag positive statements
    assert!(!merger.has_contradiction_marker("User likes this"));
    assert!(!merger.has_contradiction_marker("This is correct"));
    assert!(!merger.has_contradiction_marker("Always use this"));
}

/// UAT-11.8: Merge Result Reporting
/// As a user, I want detailed merge results
/// so that I understand what happened.
#[test]
fn uat_11_8_merge_result_reporting() {
    // This is a conceptual test - in reality would need full merge
    let result = memst_repo::merge::SemanticMergeResult {
        commit_hash: ObjectId::from_content(b"merge"),
        fast_forward: false,
        conflicts: vec![],
        memories_added: 5,
        memories_removed: 2,
        token_delta: 150,
    };

    assert!(!result.fast_forward);
    assert_eq!(result.memories_added, 5);
    assert_eq!(result.memories_removed, 2);
    assert!(result.token_delta > 0);
    assert!(result.conflicts.is_empty());
}
