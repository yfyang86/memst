# Knowledge Graph Evolution

This document describes the MemSt Knowledge Graph (KG) evolution system, which maintains graph quality through automated entity merging, splitting, deprecation, and relationship management.

## Overview

The KG evolution system (`memst-sleep/src/kg_evolve`) runs during sleep cycles to:

1. **Detect duplicate entities** and propose merges
2. **Split ambiguous entities** into more specific ones
3. **Deprecate stale entities** that are no longer relevant
4. **Manage relationships** (add, remove, update)

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    KgEvolutionEngine                            │
├─────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │   Detect     │  │   Propose    │  │       Apply          │  │
│  │  Candidates  │→ │   Actions    │→ │     Actions          │  │
│  └──────────────┘  └──────────────┘  └──────────────────────┘  │
│         │                 │                    │               │
│         ▼                 ▼                    ▼               │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │                 KgEvolutionAction                         │ │
│  │  • MergeEntities    • SplitEntity    • UpdateEntity       │ │
│  │  • AddRelationship  • RemoveRelationship                  │ │
│  │  • UpdateRelationship  • DeprecateEntity                  │ │
│  └──────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## Evolution Actions

### 1. MergeEntities

Merges two entities that represent the same real-world concept.

**Algorithm:**
```rust
// 1. Compare all entity pairs using similarity metrics
let similarity = calculate_similarity(e1, e2);

// 2. If similarity >= threshold, propose merge
if similarity >= config.merge_threshold {
    candidates.push((e1.id, e2.id, similarity));
}

// 3. Apply merge
fn apply_merge(graph, keep_id, merge_id) {
    // Merge attributes from merge into keep
    // Combine access counts
    // Migrate all relationships from merge to keep
    // Deprecate merge entity
}
```

**Similarity Calculation:**
- **Name similarity** (50% weight): Jaccard coefficient on word sets
  ```
  Jaccard(A, B) = |A ∩ B| / |A ∪ B|
  ```
- **Type match** (20% weight): Binary check if types are equal
- **Attribute overlap** (30% weight): Ratio of matching attribute values

**Configuration:**
```rust
KgEvolutionConfig {
    merge_threshold: 0.85,      // Minimum similarity for merge
    max_comparison_batch: 100,  // Limit pairs to compare
    require_approval_for_merge: false,
}
```

### 2. SplitEntity

Splits an entity that represents multiple concepts into separate entities.

**Algorithm:**
```rust
fn apply_split(graph, original_id, new_entities) {
    // 1. Validate original exists
    // 2. Create all new entities
    // 3. Migrate relationships to first new entity
    // 4. Deprecate original entity
}
```

**Use Cases:**
- "Python" (ambiguous between snake and programming language)
- "Mercury" (planet vs. element vs. car brand)
- Entities with conflicting attributes across sources

### 3. DeprecateEntity

Marks an entity as deprecated when it's no longer relevant.

**Triggers:**
- Very low relevance score (< 0.05)
- Entity merged into another
- Entity split into others
- Manual deprecation

**Implementation:**
```rust
entity.is_deprecated = true;
entity.deprecation_reason = Some(reason);
```

### 4. AddRelationship

Creates a new relationship between two entities.

**Validation:**
- Both subject and object entities must exist
- Duplicate detection prevents identical relationships

### 5. RemoveRelationship

Removes a stale or incorrect relationship.

**Triggers:**
- Relationship confidence drops below threshold
- Manual removal
- Entity deprecation (handled by migrate_relationships)

### 6. UpdateRelationship

Modifies relationship properties (confidence, relevance).

**Algorithm:**
```rust
fn update_relationship(graph, rel_id, confidence) {
    if let Some(edge) = find_edge(rel_id) {
        edge.confidence = new_confidence;
        save_graph();
    }
}
```

### 7. UpdateEntity

Updates entity name, type, or attributes.

**Use Cases:**
- Correcting entity names
- Adding new attributes
- Changing entity type

## Relationship Migration

When entities are merged, all relationships from the deprecated entity must be transferred to the kept entity.

**Algorithm:**
```rust
fn migrate_relationships(from_id, to_id) -> usize {
    let mut migrated = 0;
    
    for edge in &mut graph.edges {
        let mut changed = false;
        
        // Update subject
        if edge.subject_id == from_id {
            edge.subject_id = to_id;
            changed = true;
        }
        
        // Update object
        if edge.object_id == from_id {
            edge.object_id = to_id;
            changed = true;
        }
        
        if changed {
            migrated += 1;
        }
    }
    
    // Remove duplicates that may have been created
    remove_duplicate_relationships();
    
    migrated
}
```

## Evolution Cycle

A typical evolution cycle:

```rust
pub fn evolve(&self, graph: &KnowledgeGraph) -> Vec<KgEvolutionAction> {
    let mut actions = Vec::new();
    
    // 1. Detect merge candidates
    let candidates = self.detect_merge_candidates(graph);
    for (e1, e2, _) in candidates {
        if let Some(action) = self.propose_merge(graph, e1, e2) {
            actions.push(action);
        }
    }
    
    // 2. Detect stale entities for deprecation
    let stale = self.detect_stale_entities(graph, memories);
    actions.extend(stale);
    
    actions
}
```

## Error Handling

All evolution operations use the `KgError` enum:

```rust
pub enum KgError {
    EntityNotFound(EntityId),
    RelationshipNotFound(RelationshipId),
    GraphError(String),
    InvalidAction(String),
    NotImplemented(String),
    // ...
}
```

**Error Types:**
- `EntityNotFound`: Attempted operation on non-existent entity
- `RelationshipNotFound`: Update/remove on non-existent relationship
- `InvalidAction`: Invalid parameters (e.g., split with no new entities)
- `GraphError`: Underlying graph storage failure

## Testing

### Integration Tests

```rust
// Test entity merging with relationship migration
#[test]
fn test_evolution_merge_with_relationships() {
    // 1. Create entities with relationship
    // 2. Apply merge action
    // 3. Verify relationship migrated to kept entity
}

// Test all evolution actions
#[test]
fn test_evolution_add_relationship() { /* ... */ }
#[test]
fn test_evolution_update_relationship() { /* ... */ }
#[test]
fn test_evolution_split_entity() { /* ... */ }
```

### Statistics

Track evolution effectiveness:

```rust
pub struct EvolutionStats {
    pub entities_examined: usize,
    pub merge_candidates: usize,
    pub deprecated_entities: usize,
    pub actions_applied: usize,
    pub actions_failed: usize,
}
```

## Future Enhancements

### LLM-Based Semantic Similarity

Replace or augment Jaccard similarity with LLM embeddings:

```rust
async fn semantic_similarity(&self, e1: &Entity, e2: &Entity) -> f32 {
    let embedding1 = self.llm.embed(&e1.name).await;
    let embedding2 = self.llm.embed(&e2.name).await;
    cosine_similarity(embedding1, embedding2)
}
```

### Parallel Processing

Process large graphs in parallel:

```rust
fn detect_merge_candidates_par(&self, graph: &KnowledgeGraph) -> Vec<Candidate> {
    graph.all_entities()
        .par_chunks(100)
        .flat_map(|chunk| self.compare_chunk(chunk))
        .collect()
}
```

### Evolution History

Track all evolution actions for auditability:

```rust
pub struct EvolutionRecord {
    pub timestamp: DateTime<Utc>,
    pub action: KgEvolutionAction,
    pub applied_by: String,  // "auto" or user_id
    pub result: Result<bool, KgError>,
}
```

## Configuration Reference

```rust
pub struct KgEvolutionConfig {
    /// Minimum similarity for merge proposal (0.0-1.0)
    pub merge_threshold: f32,
    
    /// Minimum confidence for split proposal
    pub split_threshold: f32,
    
    /// Minimum confidence for auto-applying actions
    pub min_auto_apply_confidence: f32,
    
    /// Require manual approval for merges
    pub require_approval_for_merge: bool,
    
    /// Maximum entities to compare (prevents O(n²))
    pub max_comparison_batch: usize,
}
```

## References

- Implementation: `memst-sleep/src/kg_evolve/mod.rs`
- Error types: `memst-sleep/src/error.rs`
- Core types: `memst-core/src/types/mod.rs`
- Graph operations: `memst-core/src/graph.rs`
- Integration tests: `memst-sleep/tests/kg_evolution_integration_tests.rs`
