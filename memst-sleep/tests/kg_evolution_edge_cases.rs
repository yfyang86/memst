//! Edge Case Tests for KG Evolution Engine
//!
//! Tests error handling, boundary conditions, and complex scenarios.

use memst_core::graph::KnowledgeGraph;
use memst_core::types::{Entity, KgEvolutionAction, KgEvolutionConfig, Relationship};
use memst_sleep::kg_evolve::KgEvolutionEngine;

mod common;

fn create_test_graph_with_entities() -> (KnowledgeGraph, Vec<uuid::Uuid>, tempfile::TempDir) {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let mut graph = KnowledgeGraph::new(temp_dir.path()).expect("Failed to create graph");
    let session_id = uuid::Uuid::new_v4();
    
    let mut entity_ids = Vec::new();
    
    let e1 = Entity::new("Rust Programming Language", "technology", session_id);
    entity_ids.push(e1.id);
    graph.add_entity(e1).expect("Failed to add entity");
    
    let e2 = Entity::new("Rust Language", "technology", session_id);
    entity_ids.push(e2.id);
    graph.add_entity(e2).expect("Failed to add entity");
    
    let e3 = Entity::new("Python", "technology", session_id);
    entity_ids.push(e3.id);
    graph.add_entity(e3).expect("Failed to add entity");
    
    (graph, entity_ids, temp_dir)
}

#[test]
fn test_merge_with_missing_keep_entity() {
    common::setup();
    
    let (mut graph, _entity_ids, _temp_dir) = create_test_graph_with_entities();
    let engine = KgEvolutionEngine::new();
    
    let fake_id = uuid::Uuid::new_v4();
    
    // Try to merge with non-existent keep entity
    let action = KgEvolutionAction::MergeEntities {
        keep: fake_id,
        merge: _entity_ids[1],
        reason: "Test merge".to_string(),
    };
    
    let result = engine.apply_action(&mut graph, &action);
    assert!(result.is_err(), "Should fail when keep entity doesn't exist");
    
    // Verify error is EntityNotFound
    let err_str = format!("{:?}", result.unwrap_err());
    assert!(err_str.contains("EntityNotFound"), "Error should be EntityNotFound");
}

#[test]
fn test_merge_with_missing_merge_entity() {
    common::setup();
    
    let (mut graph, entity_ids, _temp_dir) = create_test_graph_with_entities();
    let engine = KgEvolutionEngine::new();
    
    let fake_id = uuid::Uuid::new_v4();
    
    // Try to merge with non-existent merge entity
    let action = KgEvolutionAction::MergeEntities {
        keep: entity_ids[0],
        merge: fake_id,
        reason: "Test merge".to_string(),
    };
    
    let result = engine.apply_action(&mut graph, &action);
    assert!(result.is_err(), "Should fail when merge entity doesn't exist");
}

#[test]
fn test_deprecate_missing_entity() {
    common::setup();
    
    let (mut graph, _entity_ids, _temp_dir) = create_test_graph_with_entities();
    let engine = KgEvolutionEngine::new();
    
    let fake_id = uuid::Uuid::new_v4();
    
    let action = KgEvolutionAction::DeprecateEntity {
        entity_id: fake_id,
        reason: "Test deprecation".to_string(),
        replacement: None,
    };
    
    let result = engine.apply_action(&mut graph, &action);
    assert!(result.is_err(), "Should fail when entity doesn't exist");
}

#[test]
fn test_split_with_empty_new_entities() {
    common::setup();
    
    let (mut graph, entity_ids, _temp_dir) = create_test_graph_with_entities();
    let engine = KgEvolutionEngine::new();
    
    // Try to split with no new entities
    let action = KgEvolutionAction::SplitEntity {
        original: entity_ids[0],
        new_entities: vec![],
        reason: "Test split".to_string(),
    };
    
    let result = engine.apply_action(&mut graph, &action);
    assert!(result.is_err(), "Should fail with empty new_entities");
}

#[test]
fn test_split_missing_original() {
    common::setup();
    
    let (mut graph, _entity_ids, _temp_dir) = create_test_graph_with_entities();
    let engine = KgEvolutionEngine::new();
    
    let session_id = uuid::Uuid::new_v4();
    let fake_id = uuid::Uuid::new_v4();
    
    let new_entity = Entity::new("New Entity", "technology", session_id);
    
    // Try to split non-existent entity
    let action = KgEvolutionAction::SplitEntity {
        original: fake_id,
        new_entities: vec![new_entity],
        reason: "Test split".to_string(),
    };
    
    let result = engine.apply_action(&mut graph, &action);
    assert!(result.is_err(), "Should fail when original entity doesn't exist");
}

#[test]
fn test_update_missing_entity() {
    common::setup();
    
    let (mut graph, _entity_ids, _temp_dir) = create_test_graph_with_entities();
    let engine = KgEvolutionEngine::new();
    
    let fake_id = uuid::Uuid::new_v4();
    
    let action = KgEvolutionAction::UpdateEntity {
        entity_id: fake_id,
        new_name: Some("New Name".to_string()),
        new_type: None,
        attribute_changes: serde_json::json!({}),
        reason: "Test update".to_string(),
    };
    
    let result = engine.apply_action(&mut graph, &action);
    assert!(result.is_err(), "Should fail when entity doesn't exist");
}

#[test]
fn test_update_missing_relationship() {
    common::setup();
    
    let (mut graph, _entity_ids, _temp_dir) = create_test_graph_with_entities();
    let engine = KgEvolutionEngine::new();
    
    let fake_rel_id = uuid::Uuid::new_v4();
    
    let action = KgEvolutionAction::UpdateRelationship {
        relationship_id: fake_rel_id,
        confidence: Some(0.9),
        relevance: None,
        reason: "Test update".to_string(),
    };
    
    let result = engine.apply_action(&mut graph, &action);
    assert!(result.is_err(), "Should fail when relationship doesn't exist");
}

#[test]
fn test_remove_missing_relationship() {
    common::setup();
    
    let (mut graph, _entity_ids, _temp_dir) = create_test_graph_with_entities();
    let engine = KgEvolutionEngine::new();
    
    let fake_rel_id = uuid::Uuid::new_v4();
    
    let action = KgEvolutionAction::RemoveRelationship {
        relationship_id: fake_rel_id,
        reason: "Test removal".to_string(),
    };
    
    // Removing non-existent relationship should return Ok(false)
    let result = engine.apply_action(&mut graph, &action);
    assert!(result.is_ok(), "Should not error on missing relationship");
    assert!(!result.unwrap(), "Should return false when relationship not found");
}

#[test]
fn test_add_relationship_duplicate_entities() {
    common::setup();
    
    let (mut graph, entity_ids, _temp_dir) = create_test_graph_with_entities();
    let engine = KgEvolutionEngine::new();
    
    let session_id = uuid::Uuid::new_v4();
    
    // Try to create self-referencing relationship
    let relationship = Relationship::new(
        entity_ids[0],
        "is_a",
        entity_ids[0],
        session_id,
    );
    
    let action = KgEvolutionAction::AddRelationship {
        relationship,
    };
    
    // Should allow self-relationships (some ontologies use them)
    let result = engine.apply_action(&mut graph, &action);
    assert!(result.is_ok(), "Should allow self-referencing relationships");
}

#[test]
fn test_merge_already_deprecated_entity() {
    common::setup();
    
    let (mut graph, entity_ids, _temp_dir) = create_test_graph_with_entities();
    let engine = KgEvolutionEngine::new();
    
    // First deprecate entity
    if let Some(entity) = graph.get_entity_mut(entity_ids[1]) {
        entity.deprecate("Test deprecation".to_string());
    }
    
    // Try to merge deprecated entity
    let action = KgEvolutionAction::MergeEntities {
        keep: entity_ids[0],
        merge: entity_ids[1],
        reason: "Test merge".to_string(),
    };
    
    // Should still work, just might be unusual
    let result = engine.apply_action(&mut graph, &action);
    assert!(result.is_ok(), "Should allow merging deprecated entities");
}

#[test]
fn test_evolution_with_no_entities() {
    common::setup();
    
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let graph = KnowledgeGraph::new(temp_dir.path()).expect("Failed to create graph");
    
    let engine = KgEvolutionEngine::new();
    
    // Should not panic with empty graph
    let candidates = engine.detect_merge_candidates(&graph);
    assert!(candidates.is_empty(), "Should return empty candidates for empty graph");
    
    let actions = engine.evolve(&graph);
    assert!(actions.is_empty(), "Should return empty actions for empty graph");
}

#[test]
fn test_merge_preserves_attributes() {
    common::setup();
    
    let (mut graph, entity_ids, _temp_dir) = create_test_graph_with_entities();
    let session_id = uuid::Uuid::new_v4();
    
    // Add attributes to keep entity
    if let Some(entity) = graph.get_entity_mut(entity_ids[0]) {
        entity.attributes = serde_json::json!({
            "website": "rust-lang.org",
            "creator": "Graydon Hoare"
        });
    }
    
    // Add different attributes to merge entity
    if let Some(entity) = graph.get_entity_mut(entity_ids[1]) {
        entity.attributes = serde_json::json!({
            "paradigm": "systems programming",
            "memory_model": "ownership"
        });
    }
    
    let engine = KgEvolutionEngine::new();
    let action = KgEvolutionAction::MergeEntities {
        keep: entity_ids[0],
        merge: entity_ids[1],
        reason: "Test merge with attributes".to_string(),
    };
    
    let result = engine.apply_action(&mut graph, &action);
    assert!(result.is_ok());
    
    // Verify keep entity has merged attributes
    let keep_entity = graph.get_entity(entity_ids[0]).unwrap();
    let attrs = keep_entity.attributes.as_object().unwrap();
    assert!(attrs.contains_key("website"), "Should preserve original attributes");
    assert!(attrs.contains_key("paradigm"), "Should add new attributes from merged entity");
}

#[test]
fn test_cascade_operations() {
    common::setup();
    
    let (mut graph, entity_ids, _temp_dir) = create_test_graph_with_entities();
    let engine = KgEvolutionEngine::new();
    let session_id = uuid::Uuid::new_v4();
    
    // Create relationship
    let rel = Relationship::new(entity_ids[0], "uses", entity_ids[2], session_id);
    let rel_id = rel.id;
    graph.add_relationship(rel).expect("Failed to add relationship");
    
    // Merge entity 0 into entity 1
    let merge_action = KgEvolutionAction::MergeEntities {
        keep: entity_ids[1],
        merge: entity_ids[0],
        reason: "Cascade test".to_string(),
    };
    engine.apply_action(&mut graph, &merge_action).expect("Merge failed");
    
    // Verify relationship was migrated
    let rels = graph.get_relationships(entity_ids[1]);
    assert!(!rels.is_empty(), "Relationship should be migrated");
    
    // Get the migrated relationship ID (it should be the same, but let's verify)
    let migrated_rel_id = rels[0].0.id;
    
    // Update the migrated relationship
    let update_action = KgEvolutionAction::UpdateRelationship {
        relationship_id: migrated_rel_id,
        confidence: Some(0.99),
        relevance: None,
        reason: "Update after merge".to_string(),
    };
    let result = engine.apply_action(&mut graph, &update_action);
    assert!(result.is_ok(), "Should be able to update migrated relationship: {:?}", result);
    
    // Remove the relationship
    let remove_action = KgEvolutionAction::RemoveRelationship {
        relationship_id: migrated_rel_id,
        reason: "Cleanup".to_string(),
    };
    let result = engine.apply_action(&mut graph, &remove_action);
    assert!(result.is_ok(), "Should be able to remove relationship");
}
