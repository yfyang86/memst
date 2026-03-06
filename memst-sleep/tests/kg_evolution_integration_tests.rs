//! Integration Tests for KG Evolution Engine
//!
//! Tests the KgEvolutionEngine with real graph operations.

use memst_core::graph::KnowledgeGraph;
use memst_core::types::{Entity, KgEvolutionAction, KgEvolutionConfig, TemporalRelevance};
use memst_sleep::kg_evolve::KgEvolutionEngine;
use std::collections::HashSet;

mod common;

use memst_core::types::Relationship;

fn create_test_graph_with_entities() -> (KnowledgeGraph, Vec<uuid::Uuid>, tempfile::TempDir) {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let mut graph = KnowledgeGraph::new(temp_dir.path()).expect("Failed to create graph");
    let session_id = uuid::Uuid::new_v4();
    
    let mut entity_ids = Vec::new();
    
    // Create VERY similar entities that should be merged (high word overlap)
    // "Rust Programming Language" vs "Rust Language" = intersection/rust, language = 2, union = 3 = 0.67
    // Still not enough for default 0.85 threshold, but better
    let e1 = Entity::new("Rust Programming Language", "technology", session_id);
    entity_ids.push(e1.id);
    graph.add_entity(e1).expect("Failed to add entity");
    
    let e2 = Entity::new("Rust Language", "technology", session_id);
    entity_ids.push(e2.id);
    graph.add_entity(e2).expect("Failed to add entity");
    
    // Create a different entity that should NOT be merged
    let e3 = Entity::new("Python", "technology", session_id);
    entity_ids.push(e3.id);
    graph.add_entity(e3).expect("Failed to add entity");
    
    // Create an entity with low relevance (should be deprecated)
    let mut e4 = Entity::new("Old Framework", "technology", session_id);
    e4.relevance = 0.02; // Very low relevance
    entity_ids.push(e4.id);
    graph.add_entity(e4).expect("Failed to add entity");
    
    (graph, entity_ids, temp_dir)
}

#[test]
fn test_evolution_detect_merge_candidates() {
    common::setup();
    
    let (graph, _entity_ids, _temp_dir) = create_test_graph_with_entities();
    
    // Use a lower threshold for testing
    let config = KgEvolutionConfig {
        merge_threshold: 0.4, // Lower threshold to catch "Rust Language" similarity
        split_threshold: 0.7,
        min_auto_apply_confidence: 0.5,
        require_approval_for_merge: false,
        max_comparison_batch: 100,
    };
    let engine = KgEvolutionEngine::with_config(config);
    
    let candidates = engine.detect_merge_candidates(&graph);
    
    eprintln!("Found {} merge candidates", candidates.len());
    for (e1, e2, sim) in &candidates {
        let ent1 = graph.get_entity(*e1).unwrap();
        let ent2 = graph.get_entity(*e2).unwrap();
        eprintln!("  Candidate: '{}' vs '{}' = {:.2}", ent1.name, ent2.name, sim);
    }
    
    // Should detect Rust entities as similar
    assert!(!candidates.is_empty(), "Should find at least one merge candidate");
    
    // Check that candidates are sorted by similarity (highest first)
    for i in 1..candidates.len() {
        assert!(candidates[i-1].2 >= candidates[i].2, 
            "Candidates should be sorted by similarity descending");
    }
}

#[test]
fn test_evolution_apply_merge() {
    common::setup();
    
    let (mut graph, entity_ids, _temp_dir) = create_test_graph_with_entities();
    let engine = KgEvolutionEngine::new();
    
    // Find the two Rust entities
    let rust_programming = entity_ids[0];
    let rust_lang = entity_ids[1];
    
    // Apply merge
    let action = KgEvolutionAction::MergeEntities {
        keep: rust_programming,
        merge: rust_lang,
        reason: "Similar names, same technology".to_string(),
    };
    
    let result = engine.apply_action(&mut graph, &action);
    
    assert!(result.is_ok(), "Merge should succeed: {:?}", result.err());
    assert!(result.unwrap(), "Merge should make changes");
    
    // Verify the merged entity is deprecated
    let merged = graph.get_entity(rust_lang).expect("Merged entity should exist");
    assert!(merged.is_deprecated, "Merged entity should be deprecated");
    assert!(merged.deprecation_reason.as_ref().unwrap().contains("Merged"));
    
    // Verify the keep entity has updated attributes
    let keep = graph.get_entity(rust_programming).expect("Keep entity should exist");
    assert!(!keep.is_deprecated, "Keep entity should not be deprecated");
    
    eprintln!("✓ Merge application test passed");
}

#[test]
fn test_evolution_merge_with_relationships() {
    common::setup();
    
    let (mut graph, entity_ids, _temp_dir) = create_test_graph_with_entities();
    let session_id = uuid::Uuid::new_v4();
    
    // Add some relationships before merging
    // rust_programming (entity_ids[0]) --uses--> python (entity_ids[2])
    graph.add_relationship(
        memst_core::types::Relationship::new(
            entity_ids[0], "uses", entity_ids[2], session_id
        )
    ).expect("Failed to add relationship");
    
    // rust_lang (entity_ids[1]) --created_by--> rust_programming (entity_ids[0])
    graph.add_relationship(
        memst_core::types::Relationship::new(
            entity_ids[1], "created_by", entity_ids[0], session_id
        )
    ).expect("Failed to add relationship");
    
    let engine = KgEvolutionEngine::new();
    
    let rust_programming = entity_ids[0];
    let rust_lang = entity_ids[1];
    
    // Count relationships before merge
    let rels_before = graph.get_relationships(rust_programming).len();
    eprintln!("Relationships before merge: {}", rels_before);
    
    // Apply merge
    let action = KgEvolutionAction::MergeEntities {
        keep: rust_programming,
        merge: rust_lang,
        reason: "Similar names, same technology".to_string(),
    };
    
    let result = engine.apply_action(&mut graph, &action);
    assert!(result.is_ok(), "Merge should succeed");
    
    // Verify relationships were migrated
    let rels_after = graph.get_relationships(rust_programming).len();
    eprintln!("Relationships after merge: {}", rels_after);
    
    // Should have more relationships now (the created_by from rust_lang)
    assert!(rels_after >= rels_before, "Relationships should be migrated to keep entity");
    
    // Verify the deprecated entity has no more relationships as subject
    let deprecated_rels = graph.get_relationships(rust_lang);
    assert!(deprecated_rels.is_empty() || 
            deprecated_rels.iter().all(|(r, _, _)| r.subject_id != rust_lang),
            "Deprecated entity should not have relationships as subject");
    
    eprintln!("✓ Merge with relationship migration test passed");
}

#[test]
fn test_evolution_apply_deprecation() {
    common::setup();
    
    let (mut graph, entity_ids, _temp_dir) = create_test_graph_with_entities();
    let engine = KgEvolutionEngine::new();
    
    let entity_to_deprecate = entity_ids[3]; // The low-relevance entity
    
    let action = KgEvolutionAction::DeprecateEntity {
        entity_id: entity_to_deprecate,
        reason: "Low relevance".to_string(),
        replacement: None,
    };
    
    let result = engine.apply_action(&mut graph, &action);
    
    assert!(result.is_ok(), "Deprecation should succeed");
    assert!(result.unwrap(), "Deprecation should make changes");
    
    // Verify entity is deprecated
    let entity = graph.get_entity(entity_to_deprecate).expect("Entity should exist");
    assert!(entity.is_deprecated, "Entity should be deprecated");
    assert_eq!(entity.deprecation_reason, Some("Low relevance".to_string()));
    
    eprintln!("✓ Deprecation test passed");
}

#[test]
fn test_evolution_entity_update() {
    common::setup();
    
    let (mut graph, entity_ids, _temp_dir) = create_test_graph_with_entities();
    let engine = KgEvolutionEngine::new();
    
    let entity_to_update = entity_ids[0];
    
    // Get original name
    let original_name = graph.get_entity(entity_to_update).unwrap().name.clone();
    
    let action = KgEvolutionAction::UpdateEntity {
        entity_id: entity_to_update,
        new_name: Some("Rust Language".to_string()),
        new_type: None,
        attribute_changes: serde_json::json!({
            "version": "1.70",
            "paradigm": "systems"
        }),
        reason: "Better naming".to_string(),
    };
    
    let result = engine.apply_action(&mut graph, &action);
    
    assert!(result.is_ok(), "Update should succeed");
    
    // Verify update
    let entity = graph.get_entity(entity_to_update).expect("Entity should exist");
    assert_eq!(entity.name, "Rust Language");
    assert_ne!(entity.name, original_name);
    
    // Check attributes were merged
    let attrs = entity.attributes.as_object().expect("Should have attributes");
    assert!(attrs.contains_key("version"), "Should have version attribute");
    assert!(attrs.contains_key("paradigm"), "Should have paradigm attribute");
    
    eprintln!("✓ Entity update test passed");
}

#[test]
fn test_evolution_full_cycle() {
    common::setup();
    
    let (mut graph, _entity_ids, _temp_dir) = create_test_graph_with_entities();
    
    let config = KgEvolutionConfig {
        merge_threshold: 0.3, // Lower threshold for testing
        split_threshold: 0.8,
        min_auto_apply_confidence: 0.5,
        require_approval_for_merge: false,
        max_comparison_batch: 100,
    };
    
    let engine = KgEvolutionEngine::with_config(config);
    
    // Run evolution cycle
    let actions = engine.evolve(&graph);
    
    eprintln!("Evolution proposed {} actions", actions.len());
    
    // Apply the actions
    let stats = engine.apply_evolution_actions(&mut graph, &actions);
    
    eprintln!("Applied: {}, Failed: {}", stats.actions_applied, stats.actions_failed);
    
    // Stats should be consistent
    assert_eq!(stats.actions_applied + stats.actions_failed, actions.len(),
        "All actions should be accounted for");
    
    eprintln!("✓ Full evolution cycle test passed");
}

#[test]
fn test_evolution_with_custom_config() {
    common::setup();
    
    let (graph, _entity_ids, _temp_dir) = create_test_graph_with_entities();
    
    // Very strict config - should find fewer candidates
    let strict_config = KgEvolutionConfig {
        merge_threshold: 0.9,
        split_threshold: 0.95,
        min_auto_apply_confidence: 0.95,
        require_approval_for_merge: true,
        max_comparison_batch: 100,
    };
    
    let strict_engine = KgEvolutionEngine::with_config(strict_config);
    let strict_candidates = strict_engine.detect_merge_candidates(&graph);
    
    // Lenient config - should find more candidates
    let lenient_config = KgEvolutionConfig {
        merge_threshold: 0.1,
        split_threshold: 0.5,
        min_auto_apply_confidence: 0.1,
        require_approval_for_merge: false,
        max_comparison_batch: 100,
    };
    
    let lenient_engine = KgEvolutionEngine::with_config(lenient_config);
    let lenient_candidates = lenient_engine.detect_merge_candidates(&graph);
    
    eprintln!("Strict config: {} candidates, Lenient config: {} candidates",
        strict_candidates.len(), lenient_candidates.len());
    
    // Lenient should find at least as many as strict
    assert!(lenient_candidates.len() >= strict_candidates.len(),
        "Lenient config should find more or equal candidates");
    
    eprintln!("✓ Custom config test passed");
}

#[test]
fn test_evolution_stats_tracking() {
    common::setup();
    
    let (mut graph, entity_ids, _temp_dir) = create_test_graph_with_entities();
    let engine = KgEvolutionEngine::new();
    
    // Create a mix of actions
    let actions = vec![
        KgEvolutionAction::MergeEntities {
            keep: entity_ids[0],
            merge: entity_ids[1],
            reason: "Test merge".to_string(),
        },
        KgEvolutionAction::DeprecateEntity {
            entity_id: entity_ids[3],
            reason: "Low relevance".to_string(),
            replacement: None,
        },
        // This one should fail (split not implemented)
        KgEvolutionAction::SplitEntity {
            original: entity_ids[2],
            new_entities: vec![],
            reason: "Test split".to_string(),
        },
    ];
    
    let stats = engine.apply_evolution_actions(&mut graph, &actions);
    
    eprintln!("Stats: applied={}, failed={}", stats.actions_applied, stats.actions_failed);
    
    // Should have 2 applied (merge + deprecate) and 1 failed (split)
    assert_eq!(stats.actions_applied, 2, "Should apply 2 actions");
    assert_eq!(stats.actions_failed, 1, "Should fail 1 action (split not implemented)");
    
    eprintln!("✓ Stats tracking test passed");
}

#[test]
fn test_entity_similarity_calculation() {
    common::setup();
    
    // Note: text_similarity is a private method, so we test it indirectly
    // through detect_merge_candidates with a low threshold
    
    let (graph, _entity_ids, _temp_dir) = create_test_graph_with_entities();
    
    // Use a low threshold so we can see the similarity scores
    let config = KgEvolutionConfig {
        merge_threshold: 0.3, // Low threshold to capture more candidates
        split_threshold: 0.7,
        min_auto_apply_confidence: 0.5,
        require_approval_for_merge: false,
        max_comparison_batch: 100,
    };
    let engine = KgEvolutionEngine::with_config(config);
    
    // Get candidates - this uses text_similarity internally
    let candidates = engine.detect_merge_candidates(&graph);
    
    eprintln!("Candidates found with threshold 0.3:");
    for (e1, e2, sim) in &candidates {
        let ent1 = graph.get_entity(*e1).unwrap();
        let ent2 = graph.get_entity(*e2).unwrap();
        eprintln!("  '{}' vs '{}' = {:.2}", ent1.name, ent2.name, sim);
    }
    
    // The two Rust entities should be detected as similar
    let has_rust_candidates = candidates.iter().any(|(e1, e2, sim)| {
        let ent1 = graph.get_entity(*e1).unwrap();
        let ent2 = graph.get_entity(*e2).unwrap();
        (ent1.name.contains("Rust") && ent2.name.contains("Rust")) && *sim > 0.3
    });
    
    assert!(has_rust_candidates, "Should detect Rust entities as similar");
    
    eprintln!("✓ Similarity calculation test passed");
}


#[test]
fn test_evolution_add_relationship() {
    common::setup();
    
    let (mut graph, entity_ids, _temp_dir) = create_test_graph_with_entities();
    let engine = KgEvolutionEngine::new();
    
    let session_id = uuid::Uuid::new_v4();
    
    // Create a relationship between two entities
    let relationship = Relationship::new(
        entity_ids[0],
        "related_to",
        entity_ids[2],
        session_id,
    );
    
    let action = KgEvolutionAction::AddRelationship {
        relationship: relationship.clone(),
    };
    
    let result = engine.apply_action(&mut graph, &action);
    assert!(result.is_ok(), "Should successfully add relationship: {:?}", result);
    assert!(result.unwrap(), "Should return true for successful add");
    
    // Verify relationship was added
    let relationships = graph.get_relationships(entity_ids[0]);
    assert_eq!(relationships.len(), 1, "Should have one relationship");
    assert_eq!(relationships[0].0.predicate, "related_to");
    
    eprintln!("✓ Add relationship test passed");
}

#[test]
fn test_evolution_add_relationship_invalid_entity() {
    common::setup();
    
    let (mut graph, _entity_ids, _temp_dir) = create_test_graph_with_entities();
    let engine = KgEvolutionEngine::new();
    
    let session_id = uuid::Uuid::new_v4();
    let fake_id = uuid::Uuid::new_v4();
    
    // Try to create a relationship with non-existent entity
    let relationship = Relationship::new(
        fake_id,
        "related_to",
        uuid::Uuid::new_v4(),
        session_id,
    );
    
    let action = KgEvolutionAction::AddRelationship {
        relationship,
    };
    
    let result = engine.apply_action(&mut graph, &action);
    assert!(result.is_err(), "Should fail when entity doesn't exist");
    
    eprintln!("✓ Add relationship validation test passed");
}

#[test]
fn test_evolution_update_relationship() {
    common::setup();
    
    let (mut graph, entity_ids, _temp_dir) = create_test_graph_with_entities();
    let engine = KgEvolutionEngine::new();
    
    let session_id = uuid::Uuid::new_v4();
    
    // First add a relationship
    let relationship = Relationship::new(
        entity_ids[0],
        "uses",
        entity_ids[2],
        session_id,
    );
    let rel_id = relationship.id;
    graph.add_relationship(relationship).expect("Failed to add relationship");
    
    // Now update its confidence
    let action = KgEvolutionAction::UpdateRelationship {
        relationship_id: rel_id,
        confidence: Some(0.95),
        relevance: None,
        reason: "Increased confidence".to_string(),
    };
    
    let result = engine.apply_action(&mut graph, &action);
    assert!(result.is_ok(), "Should successfully update relationship: {:?}", result);
    assert!(result.unwrap(), "Should return true for successful update");
    
    // Verify relationship was updated using get_relationship
    let updated = graph.get_relationship(rel_id);
    assert!(updated.is_some(), "Relationship should exist");
    assert_eq!(updated.unwrap().0.confidence, 0.95, "Confidence should be updated");
    
    eprintln!("✓ Update relationship test passed");
}

#[test]
fn test_evolution_split_entity() {
    common::setup();
    
    let (mut graph, entity_ids, _temp_dir) = create_test_graph_with_entities();
    let engine = KgEvolutionEngine::new();
    
    let session_id = uuid::Uuid::new_v4();
    
    // Create relationships for the entity to be split
    let rel = Relationship::new(
        entity_ids[0],
        "uses",
        entity_ids[2],
        session_id,
    );
    graph.add_relationship(rel).expect("Failed to add relationship");
    
    // Create new entities to split into
    let new_entity1 = Entity::new("Rust Language Core", "technology", session_id);
    let new_entity2 = Entity::new("Rust Ecosystem", "technology", session_id);
    let new_entity1_id = new_entity1.id;
    
    let action = KgEvolutionAction::SplitEntity {
        original: entity_ids[0],
        new_entities: vec![new_entity1, new_entity2],
        reason: "Split for clarity".to_string(),
    };
    
    let result = engine.apply_action(&mut graph, &action);
    assert!(result.is_ok(), "Should successfully split entity: {:?}", result);
    assert!(result.unwrap(), "Should return true for successful split");
    
    // Verify original entity is deprecated
    let original = graph.get_entity(entity_ids[0]).expect("Original should exist");
    assert!(original.is_deprecated, "Original entity should be deprecated");
    
    // Verify new entities exist
    assert!(graph.get_entity(new_entity1_id).is_some(), "New entity 1 should exist");
    
    // Verify relationships were migrated
    let new_rels = graph.get_relationships(new_entity1_id);
    assert!(!new_rels.is_empty(), "Relationships should be migrated to first new entity");
    
    eprintln!("✓ Split entity test passed");
}

#[test]
fn test_evolution_remove_relationship() {
    common::setup();
    
    let (mut graph, entity_ids, _temp_dir) = create_test_graph_with_entities();
    let engine = KgEvolutionEngine::new();
    
    let session_id = uuid::Uuid::new_v4();
    
    // Add a relationship
    let relationship = Relationship::new(
        entity_ids[0],
        "uses",
        entity_ids[2],
        session_id,
    );
    let rel_id = relationship.id;
    graph.add_relationship(relationship).expect("Failed to add relationship");
    
    // Verify it exists
    let relationships_before = graph.get_relationships(entity_ids[0]);
    assert_eq!(relationships_before.len(), 1, "Should have one relationship before removal");
    
    // Remove the relationship
    let action = KgEvolutionAction::RemoveRelationship {
        relationship_id: rel_id,
        reason: "No longer relevant".to_string(),
    };
    
    let result = engine.apply_action(&mut graph, &action);
    assert!(result.is_ok(), "Should successfully remove relationship: {:?}", result);
    assert!(result.unwrap(), "Should return true for successful removal");
    
    // Verify it was removed
    let relationships_after = graph.get_relationships(entity_ids[0]);
    assert!(relationships_after.is_empty(), "Should have no relationships after removal");
    
    eprintln!("✓ Remove relationship test passed");
}
