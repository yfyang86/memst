//! Performance Benchmarks for KG Evolution Engine
//!
//! Tests performance characteristics of evolution operations.
//! Run with: cargo test -p memst-sleep --test kg_evolution_benchmarks -- --nocapture

use memst_core::graph::KnowledgeGraph;
use memst_core::types::{Entity, KgEvolutionAction, KgEvolutionConfig};
use memst_sleep::kg_evolve::KgEvolutionEngine;
use std::time::{Duration, Instant};

mod common;

/// Helper to measure execution time
fn measure<T>(f: impl FnOnce() -> T) -> (T, Duration) {
    let start = Instant::now();
    let result = f();
    let elapsed = start.elapsed();
    (result, elapsed)
}

fn create_graph_with_n_entities(n: usize) -> (KnowledgeGraph, Vec<uuid::Uuid>, tempfile::TempDir) {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let mut graph = KnowledgeGraph::new(temp_dir.path()).expect("Failed to create graph");
    let session_id = uuid::Uuid::new_v4();
    
    let mut entity_ids = Vec::with_capacity(n);
    
    for i in 0..n {
        // Create entities with similar names to trigger similarity detection
        let name = if i % 2 == 0 {
            format!("Technology {}", i)
        } else {
            format!("Tech {}", i)  // Similar naming for merge detection
        };
        
        let entity = Entity::new(&name, "technology", session_id);
        entity_ids.push(entity.id);
        graph.add_entity(entity).expect("Failed to add entity");
    }
    
    (graph, entity_ids, temp_dir)
}

fn create_graph_with_relationships(
    n_entities: usize,
    n_relationships: usize,
) -> (KnowledgeGraph, Vec<uuid::Uuid>, tempfile::TempDir) {
    use memst_core::types::Relationship;
    
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let mut graph = KnowledgeGraph::new(temp_dir.path()).expect("Failed to create graph");
    let session_id = uuid::Uuid::new_v4();
    
    let mut entity_ids = Vec::with_capacity(n_entities);
    
    // Create entities
    for i in 0..n_entities {
        let entity = Entity::new(&format!("Entity {}", i), "test", session_id);
        entity_ids.push(entity.id);
        graph.add_entity(entity).expect("Failed to add entity");
    }
    
    // Create relationships (chain: 0->1->2->3...)
    for i in 0..n_relationships.min(n_entities - 1) {
        let rel = Relationship::new(
            entity_ids[i],
            "connected_to",
            entity_ids[i + 1],
            session_id,
        );
        let _ = graph.add_relationship(rel);
    }
    
    (graph, entity_ids, temp_dir)
}

#[test]
fn benchmark_detect_merge_candidates_small() {
    common::setup();
    
    let (graph, _ids, _temp) = create_graph_with_n_entities(10);
    let engine = KgEvolutionEngine::new();
    
    let (candidates, elapsed) = measure(|| engine.detect_merge_candidates(&graph));
    
    eprintln!("\n=== Benchmark: detect_merge_candidates (10 entities) ===");
    eprintln!("  Time: {:?}", elapsed);
    eprintln!("  Candidates found: {}", candidates.len());
    eprintln!("  Time per entity pair: {:?}", elapsed / 100.max(candidates.len() as u32));
    
    // Should complete quickly for small graphs
    assert!(elapsed < Duration::from_millis(100), "Should complete in < 100ms");
}

#[test]
fn benchmark_detect_merge_candidates_medium() {
    common::setup();
    
    let (graph, _ids, _temp) = create_graph_with_n_entities(100);
    let engine = KgEvolutionEngine::new();
    
    let (candidates, elapsed) = measure(|| engine.detect_merge_candidates(&graph));
    
    eprintln!("\n=== Benchmark: detect_merge_candidates (100 entities) ===");
    eprintln!("  Time: {:?}", elapsed);
    eprintln!("  Candidates found: {}", candidates.len());
    eprintln!("  Time per entity: {:?}", elapsed / 100);
    
    // O(n²) with n=100 = 10,000 comparisons
    assert!(elapsed < Duration::from_secs(1), "Should complete in < 1s");
}

#[test]
fn benchmark_detect_merge_candidates_large() {
    common::setup();
    
    let (graph, _ids, _temp) = create_graph_with_n_entities(500);
    let engine = KgEvolutionEngine::new();
    
    let (candidates, elapsed) = measure(|| engine.detect_merge_candidates(&graph));
    
    eprintln!("\n=== Benchmark: detect_merge_candidates (500 entities) ===");
    eprintln!("  Time: {:?}", elapsed);
    eprintln!("  Candidates found: {}", candidates.len());
    eprintln!("  Entities examined: {}", graph.all_entities().len());
    eprintln!("  Time per entity: {:?}", elapsed / 500);
    
    // Note: Limited by max_comparison_batch (default 100)
    // So this should still be fairly fast
}

#[test]
fn benchmark_apply_merge_with_many_relationships() {
    common::setup();
    
    let (mut graph, ids, _temp) = create_graph_with_relationships(100, 99);
    let engine = KgEvolutionEngine::new();
    
    let action = KgEvolutionAction::MergeEntities {
        keep: ids[0],
        merge: ids[1],
        reason: "Benchmark merge".to_string(),
    };
    
    let (result, elapsed) = measure(|| engine.apply_action(&mut graph, &action));
    
    eprintln!("\n=== Benchmark: apply_merge (100 entities, 99 relationships) ===");
    eprintln!("  Time: {:?}", elapsed);
    eprintln!("  Success: {}", result.is_ok());
    
    assert!(result.is_ok());
    assert!(elapsed < Duration::from_millis(100), "Merge should be fast");
}

#[test]
fn benchmark_full_evolution_cycle() {
    common::setup();
    
    let (mut graph, _ids, _temp) = create_graph_with_n_entities(50);
    let engine = KgEvolutionEngine::new();
    
    // Detect
    let (actions, elapsed_detect) = measure(|| engine.evolve(&graph));
    eprintln!("\n=== Benchmark: full_evolution_cycle (50 entities) ===");
    eprintln!("  Detect phase: {:?}", elapsed_detect);
    eprintln!("  Actions proposed: {}", actions.len());
    
    // Apply
    let (stats, elapsed_apply) = measure(|| engine.apply_evolution_actions(&mut graph, &actions));
    eprintln!("  Apply phase: {:?}", elapsed_apply);
    eprintln!("  Actions applied: {}", stats.actions_applied);
    eprintln!("  Actions failed: {}", stats.actions_failed);
    
    let total = elapsed_detect + elapsed_apply;
    eprintln!("  Total time: {:?}", total);
    
    assert!(total < Duration::from_secs(2), "Full cycle should complete in < 2s");
}

#[test]
fn benchmark_split_entity_performance() {
    common::setup();
    
    let (mut graph, ids, _temp) = create_graph_with_n_entities(50);
    let engine = KgEvolutionEngine::new();
    let session_id = uuid::Uuid::new_v4();
    
    // Create entity to split into
    let new_entity = Entity::new("New Split Entity", "technology", session_id);
    
    let action = KgEvolutionAction::SplitEntity {
        original: ids[0],
        new_entities: vec![new_entity],
        reason: "Benchmark split".to_string(),
    };
    
    let (result, elapsed) = measure(|| engine.apply_action(&mut graph, &action));
    
    eprintln!("\n=== Benchmark: apply_split (50 entity graph) ===");
    eprintln!("  Time: {:?}", elapsed);
    eprintln!("  Success: {}", result.is_ok());
    
    assert!(result.is_ok());
    assert!(elapsed < Duration::from_millis(50), "Split should be very fast");
}

#[test]
fn benchmark_relationship_operations() {
    common::setup();
    
    let (mut graph, ids, _temp) = create_graph_with_relationships(50, 49);
    let engine = KgEvolutionEngine::new();
    let session_id = uuid::Uuid::new_v4();
    use memst_core::types::Relationship;
    
    // Benchmark add relationship
    let new_rel = Relationship::new(
        ids[0],
        "benchmark_rel",
        ids[10],
        session_id,
    );
    let new_rel_id = new_rel.id;
    let add_action = KgEvolutionAction::AddRelationship {
        relationship: new_rel,
    };
    
    let (result, elapsed) = measure(|| engine.apply_action(&mut graph, &add_action));
    eprintln!("\n=== Benchmark: relationship operations ===");
    eprintln!("  Add relationship: {:?}", elapsed);
    assert!(result.is_ok());
    
    // Benchmark update
    let update_action = KgEvolutionAction::UpdateRelationship {
        relationship_id: new_rel_id,
        confidence: Some(0.99),
        relevance: None,
        reason: "Benchmark".to_string(),
    };
    
    let (result, elapsed) = measure(|| engine.apply_action(&mut graph, &update_action));
    eprintln!("  Update relationship: {:?}", elapsed);
    assert!(result.is_ok());
    
    // Benchmark remove
    let remove_action = KgEvolutionAction::RemoveRelationship {
        relationship_id: new_rel_id,
        reason: "Benchmark".to_string(),
    };
    
    let (result, elapsed) = measure(|| engine.apply_action(&mut graph, &remove_action));
    eprintln!("  Remove relationship: {:?}", elapsed);
    assert!(result.is_ok());
}

#[test]
fn benchmark_multiple_merges_sequential() {
    common::setup();
    
    let (mut graph, ids, _temp) = create_graph_with_n_entities(20);
    let engine = KgEvolutionEngine::new();
    
    // Perform multiple merges in sequence
    let mut merge_actions = vec![];
    for i in 0..5 {
        merge_actions.push(KgEvolutionAction::MergeEntities {
            keep: ids[i * 2],
            merge: ids[i * 2 + 1],
            reason: format!("Batch merge {}", i),
        });
    }
    
    let start = Instant::now();
    let stats = engine.apply_evolution_actions(&mut graph, &merge_actions);
    let elapsed = start.elapsed();
    
    eprintln!("\n=== Benchmark: 5 sequential merges ===");
    eprintln!("  Total time: {:?}", elapsed);
    eprintln!("  Average per merge: {:?}", elapsed / 5);
    eprintln!("  Applied: {}, Failed: {}", stats.actions_applied, stats.actions_failed);
    
    assert_eq!(stats.actions_applied, 5, "All merges should succeed");
}

#[test]
fn benchmark_config_impact() {
    common::setup();
    
    let (graph, _ids, _temp) = create_graph_with_n_entities(100);
    
    // Test with strict config
    let strict_config = KgEvolutionConfig {
        merge_threshold: 0.9,
        split_threshold: 0.95,
        min_auto_apply_confidence: 0.95,
        require_approval_for_merge: true,
        max_comparison_batch: 100,
    };
    let strict_engine = KgEvolutionEngine::with_config(strict_config);
    
    let (candidates_strict, elapsed_strict) = measure(|| strict_engine.detect_merge_candidates(&graph));
    
    // Test with lenient config
    let lenient_config = KgEvolutionConfig {
        merge_threshold: 0.3,
        split_threshold: 0.5,
        min_auto_apply_confidence: 0.3,
        require_approval_for_merge: false,
        max_comparison_batch: 100,
    };
    let lenient_engine = KgEvolutionEngine::with_config(lenient_config);
    
    let (candidates_lenient, elapsed_lenient) = measure(|| lenient_engine.detect_merge_candidates(&graph));
    
    eprintln!("\n=== Benchmark: config impact ===");
    eprintln!("  Strict config (threshold 0.9):");
    eprintln!("    Time: {:?}, Candidates: {}", elapsed_strict, candidates_strict.len());
    eprintln!("  Lenient config (threshold 0.3):");
    eprintln!("    Time: {:?}, Candidates: {}", elapsed_lenient, candidates_lenient.len());
    
    // Config shouldn't significantly impact performance
    let ratio = elapsed_strict.as_nanos() as f64 / elapsed_lenient.as_nanos() as f64;
    eprintln!("  Performance ratio: {:.2}x", ratio);
    assert!(ratio < 2.0, "Config shouldn't change performance by >2x");
}
