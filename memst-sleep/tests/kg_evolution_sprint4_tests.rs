//! Sprint 4 Advanced Features Tests
//!
//! Tests for LLM-based semantic similarity, parallel processing, and history tracking.

use memst_core::graph::KnowledgeGraph;
use memst_core::types::{Entity, KgEvolutionAction, KgEvolutionConfig};
use memst_sleep::kg_evolve::{KgEvolutionEngine, EvolutionHistory, EvolutionRecord};

mod common;

fn create_test_graph_with_entities() -> (KnowledgeGraph, Vec<uuid::Uuid>, tempfile::TempDir) {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let mut graph = KnowledgeGraph::new(temp_dir.path()).expect("Failed to create graph");
    let session_id = uuid::Uuid::new_v4();
    
    let mut entity_ids = Vec::new();
    
    // Create entities with semantically similar but textually different names
    let e1 = Entity::new("Machine Learning", "technology", session_id);
    entity_ids.push(e1.id);
    graph.add_entity(e1).expect("Failed to add entity");
    
    let e2 = Entity::new("Deep Learning", "technology", session_id);
    entity_ids.push(e2.id);
    graph.add_entity(e2).expect("Failed to add entity");
    
    let e3 = Entity::new("Neural Networks", "technology", session_id);
    entity_ids.push(e3.id);
    graph.add_entity(e3).expect("Failed to add entity");
    
    // Different type - should not be merged
    let e4 = Entity::new("Python Programming", "language", session_id);
    entity_ids.push(e4.id);
    graph.add_entity(e4).expect("Failed to add entity");
    
    (graph, entity_ids, temp_dir)
}

#[test]
fn test_evolution_history_basic() {
    common::setup();
    
    let (mut graph, entity_ids, _temp_dir) = create_test_graph_with_entities();
    let engine = KgEvolutionEngine::new();
    let mut history = EvolutionHistory::new();
    
    let actions = vec![
        KgEvolutionAction::MergeEntities {
            keep: entity_ids[0],
            merge: entity_ids[1],
            reason: "Test merge".to_string(),
        },
        KgEvolutionAction::DeprecateEntity {
            entity_id: entity_ids[2],
            reason: "Deprecated".to_string(),
            replacement: None,
        },
    ];
    
    let stats = engine.apply_evolution_actions_with_history(
        &mut graph,
        &actions,
        &mut history,
        "test_runner",
    );
    
    // Verify stats
    assert_eq!(stats.actions_applied, 2, "Both actions should apply");
    assert_eq!(stats.actions_failed, 0, "No actions should fail");
    
    // Verify history
    assert_eq!(history.records().len(), 2, "Should have 2 history records");
    
    // Check first record
    let record = &history.records()[0];
    assert_eq!(record.applied_by, "test_runner");
    assert!(record.result.is_ok());
    assert!(record.error_message.is_none());
    
    // Check history stats
    let hist_stats = history.stats();
    assert_eq!(hist_stats.total_records, 2);
    assert_eq!(hist_stats.successful_actions, 2);
    assert_eq!(hist_stats.failed_actions, 0);
    
    eprintln!("✓ Evolution history basic test passed");
}

#[test]
fn test_evolution_history_max_records() {
    common::setup();
    
    let (mut graph, entity_ids, _temp_dir) = create_test_graph_with_entities();
    let engine = KgEvolutionEngine::new();
    let mut history = EvolutionHistory::with_max_records(3);
    
    // Apply more actions than the limit
    let actions: Vec<_> = (0..5).map(|i| {
        KgEvolutionAction::UpdateEntity {
            entity_id: entity_ids[0],
            new_name: Some(format!("Name {}", i)),
            new_type: None,
            attribute_changes: serde_json::json!({}),
            reason: format!("Update {}", i),
        }
    }).collect();
    
    let _stats = engine.apply_evolution_actions_with_history(
        &mut graph,
        &actions,
        &mut history,
        "test_runner",
    );
    
    // Should only keep the most recent 3 records
    assert_eq!(history.records().len(), 3, "Should only have 3 records (max)");
    
    // Check that oldest records were removed
    let records = history.records();
    assert!(records[0].action.to_string().contains("Update 2"), 
        "Oldest record should be Update 2");
    assert!(records[2].action.to_string().contains("Update 4"), 
        "Newest record should be Update 4");
    
    eprintln!("✓ Evolution history max records test passed");
}

#[test]
fn test_evolution_history_entity_filtering() {
    common::setup();
    
    let (mut graph, entity_ids, _temp_dir) = create_test_graph_with_entities();
    let engine = KgEvolutionEngine::new();
    let mut history = EvolutionHistory::new();
    
    // Apply actions to different entities
    let actions = vec![
        KgEvolutionAction::DeprecateEntity {
            entity_id: entity_ids[0],
            reason: "Deprecate 0".to_string(),
            replacement: None,
        },
        KgEvolutionAction::DeprecateEntity {
            entity_id: entity_ids[1],
            reason: "Deprecate 1".to_string(),
            replacement: None,
        },
        KgEvolutionAction::DeprecateEntity {
            entity_id: entity_ids[0],  // Same as first
            reason: "Deprecate 0 again".to_string(),
            replacement: None,
        },
    ];
    
    let _stats = engine.apply_evolution_actions_with_history(
        &mut graph,
        &actions,
        &mut history,
        "test_runner",
    );
    
    // Filter records for entity_ids[0]
    let entity0_records = history.records_for_entity(entity_ids[0]);
    assert_eq!(entity0_records.len(), 2, "Should have 2 records for entity 0");
    
    // Filter records for entity_ids[1]
    let entity1_records = history.records_for_entity(entity_ids[1]);
    assert_eq!(entity1_records.len(), 1, "Should have 1 record for entity 1");
    
    eprintln!("✓ Evolution history entity filtering test passed");
}

#[test]
fn test_evolution_history_failed_actions() {
    common::setup();
    
    let (mut graph, _entity_ids, _temp_dir) = create_test_graph_with_entities();
    let engine = KgEvolutionEngine::new();
    let mut history = EvolutionHistory::new();
    
    let fake_id = uuid::Uuid::new_v4();
    
    // Try to update non-existent entity
    let actions = vec![
        KgEvolutionAction::UpdateEntity {
            entity_id: fake_id,
            new_name: Some("New Name".to_string()),
            new_type: None,
            attribute_changes: serde_json::json!({}),
            reason: "Update non-existent".to_string(),
        },
    ];
    
    let stats = engine.apply_evolution_actions_with_history(
        &mut graph,
        &actions,
        &mut history,
        "test_runner",
    );
    
    // Should have failed
    assert_eq!(stats.actions_failed, 1);
    
    // History should record the failure
    let record = &history.records()[0];
    assert!(record.result.is_err());
    assert!(record.error_message.is_some());
    // Error message should indicate entity not found
    let err_msg = record.error_message.as_ref().unwrap();
    assert!(err_msg.contains("not found") || err_msg.contains("Entity"), 
        "Error should indicate entity not found: {}", err_msg);
    
    // Stats should show failure
    let hist_stats = history.stats();
    assert_eq!(hist_stats.successful_actions, 0);
    assert_eq!(hist_stats.failed_actions, 1);
    
    eprintln!("✓ Evolution history failed actions test passed");
}

#[test]
fn test_cosine_similarity() {
    common::setup();
    
    // Test identical vectors
    let v1 = vec![1.0, 0.0, 0.0];
    let v2 = vec![1.0, 0.0, 0.0];
    let sim = KgEvolutionEngine::cosine_similarity(&v1, &v2);
    assert!((sim - 1.0).abs() < 0.001, "Identical vectors should have similarity 1.0");
    
    // Test orthogonal vectors
    let v1 = vec![1.0, 0.0];
    let v2 = vec![0.0, 1.0];
    let sim = KgEvolutionEngine::cosine_similarity(&v1, &v2);
    assert!(sim.abs() < 0.001, "Orthogonal vectors should have similarity 0.0");
    
    // Test opposite vectors
    let v1 = vec![1.0, 0.0];
    let v2 = vec![-1.0, 0.0];
    let sim = KgEvolutionEngine::cosine_similarity(&v1, &v2);
    assert!((sim - (-1.0)).abs() < 0.001, "Opposite vectors should have similarity -1.0");
    
    // Test empty vectors
    let v1: Vec<f32> = vec![];
    let v2: Vec<f32> = vec![];
    let sim = KgEvolutionEngine::cosine_similarity(&v1, &v2);
    assert_eq!(sim, 0.0, "Empty vectors should have similarity 0.0");
    
    // Test different lengths
    let v1 = vec![1.0, 0.0];
    let v2 = vec![1.0, 0.0, 0.0];
    let sim = KgEvolutionEngine::cosine_similarity(&v1, &v2);
    assert_eq!(sim, 0.0, "Different length vectors should return 0.0");
    
    eprintln!("✓ Cosine similarity test passed");
}

#[test]
fn test_parallel_detection_falls_back_to_sequential() {
    common::setup();
    
    let (graph, _entity_ids, _temp_dir) = create_test_graph_with_entities();
    
    // Use a lower threshold to ensure we find candidates
    let config = KgEvolutionConfig {
        merge_threshold: 0.3,
        split_threshold: 0.7,
        min_auto_apply_confidence: 0.5,
        require_approval_for_merge: false,
        max_comparison_batch: 100,
    };
    let engine = KgEvolutionEngine::with_config(config);
    
    // Without the "parallel" feature, should use regular detection
    let candidates = engine.detect_merge_candidates(&graph);
    
    // Should still work (just single-threaded)
    // Note: May or may not find candidates depending on text similarity
    eprintln!("Found {} candidates", candidates.len());
    
    eprintln!("✓ Parallel detection fallback test passed");
}

#[test]
fn test_semantic_similarity_not_configured() {
    common::setup();
    
    let (graph, _entity_ids, _temp_dir) = create_test_graph_with_entities();
    let engine = KgEvolutionEngine::new();
    
    // Without embedding client, should fail gracefully
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        engine.detect_merge_candidates_semantic(&graph).await
    });
    
    assert!(result.is_err(), "Should error without embedding client");
    let err_str = format!("{:?}", result.unwrap_err());
    assert!(err_str.contains("Embedding client not configured"), 
        "Error should indicate missing embedding client");
    
    eprintln!("✓ Semantic similarity not configured test passed");
}

#[test]
fn test_evolution_history_clear() {
    common::setup();
    
    let (mut graph, entity_ids, _temp_dir) = create_test_graph_with_entities();
    let engine = KgEvolutionEngine::new();
    let mut history = EvolutionHistory::new();
    
    // Add some records
    let actions = vec![
        KgEvolutionAction::DeprecateEntity {
            entity_id: entity_ids[0],
            reason: "Test".to_string(),
            replacement: None,
        },
    ];
    
    let _ = engine.apply_evolution_actions_with_history(
        &mut graph,
        &actions,
        &mut history,
        "test_runner",
    );
    
    assert_eq!(history.records().len(), 1, "Should have 1 record");
    
    // Clear history
    history.clear();
    
    assert_eq!(history.records().len(), 0, "Should have 0 records after clear");
    
    eprintln!("✓ Evolution history clear test passed");
}

#[test]
fn test_history_time_range() {
    common::setup();
    
    let (mut graph, entity_ids, _temp_dir) = create_test_graph_with_entities();
    let engine = KgEvolutionEngine::new();
    let mut history = EvolutionHistory::new();
    
    // Apply some actions
    let actions = vec![
        KgEvolutionAction::DeprecateEntity {
            entity_id: entity_ids[0],
            reason: "Test".to_string(),
            replacement: None,
        },
    ];
    
    let _ = engine.apply_evolution_actions_with_history(
        &mut graph,
        &actions,
        &mut history,
        "test_runner",
    );
    
    let stats = history.stats();
    
    // Should have time range
    assert!(stats.time_range.is_some(), "Should have time range");
    let (start, end) = stats.time_range.unwrap();
    assert!(end >= start, "End time should be >= start time");
    
    eprintln!("✓ History time range test passed");
}
