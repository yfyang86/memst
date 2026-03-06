//! Knowledge Graph Evolution Engine
//!
//! Implements intelligent evolution of the knowledge graph including:
//! - Entity merging (detecting duplicates/aliases)
//! - Entity splitting (disambiguating homonyms)
//! - Relationship updates based on memory evolution
//! - Deprecation of stale entities

use crate::error::KgError;
use crate::evolve::MemoryRelation;
use memst_core::graph::KnowledgeGraph;
use memst_core::llm::providers::LlmProvider;
use memst_core::types::{Entity, EntityId, KgEvolutionAction, KgEvolutionConfig, MemoryItem, RelationshipId};
use std::sync::Arc;

/// Result type for evolution operations.
pub type Result<T> = crate::error::Result<T>;

/// Evolution engine for knowledge graph maintenance.
pub struct KgEvolutionEngine {
    config: KgEvolutionConfig,
    llm_client: Option<Arc<dyn LlmProvider>>,
}

impl KgEvolutionEngine {
    /// Create a new evolution engine with default configuration.
    pub fn new() -> Self {
        Self {
            config: KgEvolutionConfig::default(),
            llm_client: None,
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: KgEvolutionConfig) -> Self {
        Self {
            config,
            llm_client: None,
        }
    }

    /// Set LLM client for intelligent entity resolution.
    pub fn with_llm_client(mut self, client: Arc<dyn LlmProvider>) -> Self {
        self.llm_client = Some(client);
        self
    }

    /// Detect potential entity merges based on name similarity.
    pub fn detect_merge_candidates(&self, graph: &KnowledgeGraph) -> Vec<(EntityId, EntityId, f32)> {
        let mut candidates = Vec::new();
        let entities: Vec<&Entity> = graph.all_entities();

        // Compare all pairs (limit to avoid O(n²) explosion)
        let limit = self.config.max_comparison_batch.min(entities.len());
        
        for i in 0..limit {
            for j in (i + 1)..limit {
                let e1 = entities[i];
                let e2 = entities[j];

                // Skip if either is deprecated
                if e1.is_deprecated || e2.is_deprecated {
                    continue;
                }

                // Skip if different types
                if e1.entity_type != e2.entity_type {
                    continue;
                }

                let similarity = self.calculate_entity_similarity(e1, e2);
                
                if similarity >= self.config.merge_threshold {
                    candidates.push((e1.id, e2.id, similarity));
                }
            }
        }

        // Sort by similarity (highest first)
        candidates.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
        candidates
    }

    /// Calculate similarity between two entities.
    fn calculate_entity_similarity(&self, e1: &Entity, e2: &Entity) -> f32 {
        let mut score = 0.0;
        let mut weights = 0.0;

        // Name similarity (Jaccard on words)
        let name_sim = self.text_similarity(&e1.name, &e2.name);
        score += name_sim * 0.5;
        weights += 0.5;

        // Type match
        if e1.entity_type == e2.entity_type {
            score += 0.2;
        }
        weights += 0.2;

        // Attribute overlap (if both have attributes)
        if let (Some(a1), Some(a2)) = (
            e1.attributes.as_object(),
            e2.attributes.as_object(),
        ) {
            let attr_sim = self.attribute_similarity(a1, a2);
            score += attr_sim * 0.3;
            weights += 0.3;
        }

        if weights > 0.0 {
            score / weights
        } else {
            0.0
        }
    }

    /// Calculate text similarity using Jaccard coefficient.
    fn text_similarity(&self, a: &str, b: &str) -> f32 {
        let a_words: std::collections::HashSet<String> = a
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        let b_words: std::collections::HashSet<String> = b
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        if a_words.is_empty() || b_words.is_empty() {
            return 0.0;
        }

        let intersection: std::collections::HashSet<_> =
            a_words.intersection(&b_words).collect();
        let union: std::collections::HashSet<_> = a_words.union(&b_words).collect();

        intersection.len() as f32 / union.len() as f32
    }

    /// Calculate attribute similarity.
    fn attribute_similarity(
        &self,
        a: &serde_json::Map<String, serde_json::Value>,
        b: &serde_json::Map<String, serde_json::Value>,
    ) -> f32 {
        let shared_keys: std::collections::HashSet<_> = a
            .keys()
            .filter(|k| b.contains_key(*k))
            .collect();

        if shared_keys.is_empty() {
            return 0.0;
        }

        let mut matches = 0;
        for key in &shared_keys {
            if a.get(*key) == b.get(*key) {
                matches += 1;
            }
        }

        matches as f32 / shared_keys.len() as f32
    }

    /// Generate merge action for two entities.
    pub fn propose_merge(
        &self,
        graph: &KnowledgeGraph,
        keep_id: EntityId,
        merge_id: EntityId,
    ) -> Option<KgEvolutionAction> {
        let keep = graph.get_entity(keep_id)?;
        let merge = graph.get_entity(merge_id)?;

        // Prefer to keep the entity with higher relevance/access count
        let (final_keep, final_merge) = if keep.effective_importance() >= merge.effective_importance() {
            (keep_id, merge_id)
        } else {
            (merge_id, keep_id)
        };

        Some(KgEvolutionAction::MergeEntities {
            keep: final_keep,
            merge: final_merge,
            reason: format!(
                "Similar names ('{}' and '{}'), same type '{}'",
                keep.name, merge.name, keep.entity_type
            ),
        })
    }

    /// Detect entities that should be deprecated based on memory evolution.
    pub fn detect_stale_entities(
        &self,
        graph: &KnowledgeGraph,
        memory_evolutions: &[(MemoryItem, MemoryRelation, MemoryItem)],
    ) -> Vec<KgEvolutionAction> {
        let mut actions = Vec::new();

        for (_old_memory, relation, _new_memory) in memory_evolutions {
            match relation {
                MemoryRelation::Supersedes => {
                    // Find entities linked to the old memory and mark for review
                    // In a real implementation, we'd track memory-entity links
                }
                MemoryRelation::Contradicts => {
                    // Flag related entities for review
                }
                _ => {}
            }
        }

        // Find entities with very low relevance
        for entity in graph.all_entities() {
            if !entity.is_deprecated && entity.relevance < 0.05 {
                actions.push(KgEvolutionAction::DeprecateEntity {
                    entity_id: entity.id,
                    reason: "Very low relevance due to decay".to_string(),
                    replacement: None,
                });
            }
        }

        actions
    }

    /// Apply an evolution action to the knowledge graph.
    ///
    /// Returns true if the action was successfully applied.
    pub fn apply_action(
        &self,
        graph: &mut KnowledgeGraph,
        action: &KgEvolutionAction,
    ) -> Result<bool> {
        match action {
            KgEvolutionAction::MergeEntities { keep, merge, .. } => {
                self.apply_merge(graph, *keep, *merge)
            }
            KgEvolutionAction::DeprecateEntity {
                entity_id,
                reason,
                replacement,
            } => self.apply_deprecation(graph, *entity_id, reason.clone(), *replacement),
            KgEvolutionAction::UpdateEntity { entity_id, new_name, new_type, attribute_changes, .. } => {
                self.apply_entity_update(graph, *entity_id, new_name.clone(), new_type.clone(), attribute_changes.clone())
            }
            KgEvolutionAction::RemoveRelationship {
                relationship_id,
                reason: _,
            } => {
                graph
                    .delete_relationship(*relationship_id)
                    .map_err(|e| KgError::GraphError(e.to_string()))
            }
            KgEvolutionAction::AddRelationship { relationship } => {
                // Verify both entities exist before adding
                if graph.get_entity(relationship.subject_id).is_none() {
                    return Err(KgError::EntityNotFound(relationship.subject_id));
                }
                if graph.get_entity(relationship.object_id).is_none() {
                    return Err(KgError::EntityNotFound(relationship.object_id));
                }
                graph
                    .add_relationship(relationship.clone())
                    .map_err(|e| KgError::GraphError(e.to_string()))?;
                Ok(true)
            }
            KgEvolutionAction::UpdateRelationship {
                relationship_id,
                confidence,
                relevance,
                reason: _,
            } => self.apply_relationship_update(graph, *relationship_id, *confidence, *relevance),
            KgEvolutionAction::SplitEntity {
                original,
                new_entities,
                reason: _,
            } => self.apply_split(graph, *original, new_entities),
        }
    }

    /// Apply entity merge.
    fn apply_merge(
        &self,
        graph: &mut KnowledgeGraph,
        keep_id: EntityId,
        merge_id: EntityId,
    ) -> Result<bool> {
        // Get the entity to merge
        let merge_entity = graph
            .get_entity(merge_id)
            .ok_or(KgError::EntityNotFound(merge_id))?
            .clone();

        // Get the keep entity
        let keep_entity = graph
            .get_entity_mut(keep_id)
            .ok_or(KgError::EntityNotFound(keep_id))?;

        // Merge attributes
        if let (Some(keep_attrs), Some(merge_attrs)) = (
            keep_entity.attributes.as_object_mut(),
            merge_entity.attributes.as_object(),
        ) {
            for (key, value) in merge_attrs {
                if !keep_attrs.contains_key(key) {
                    keep_attrs.insert(key.clone(), value.clone());
                }
            }
        }

        // Update confidence (boost for merged entity)
        keep_entity.confidence = (keep_entity.confidence + merge_entity.confidence * 0.5).min(1.0);
        
        // Combine access counts
        keep_entity.access_count += merge_entity.access_count;

        // Mark merged entity as deprecated
        // (keep_entity borrow ends here implicitly)
        if let Some(merge_ent) = graph.get_entity_mut(merge_id) {
            merge_ent.deprecate(format!("Merged into {}", keep_id));
            // Note: The deprecation_reason already indicates the replacement
        }

        // Transfer relationships from merge to keep
        match graph.migrate_relationships(merge_id, keep_id) {
            Ok(count) => {
                if count > 0 {
                    eprintln!("Migrated {} relationships from {:?} to {:?}", count, merge_id, keep_id);
                }
            }
            Err(e) => {
                eprintln!("Warning: Failed to migrate relationships: {}", e);
                // Continue with merge even if relationship migration fails
            }
        }

        Ok(true)
    }

    /// Apply entity deprecation.
    fn apply_deprecation(
        &self,
        graph: &mut KnowledgeGraph,
        entity_id: EntityId,
        reason: String,
        replacement: Option<EntityId>,
    ) -> Result<bool> {
        if let Some(entity) = graph.get_entity_mut(entity_id) {
            entity.is_deprecated = true;
            entity.deprecation_reason = Some(reason);
            // Note: replacement tracking would need additional field
            let _ = replacement; // Silence warning for now
            Ok(true)
        } else {
            Err(KgError::EntityNotFound(entity_id))
        }
    }

    /// Run full evolution cycle: detect and propose actions.
    pub fn evolve(&self, graph: &KnowledgeGraph) -> Vec<KgEvolutionAction> {
        let mut actions = Vec::new();

        // Detect merge candidates
        let merge_candidates = self.detect_merge_candidates(graph);
        for (e1, e2, similarity) in merge_candidates {
            if let Some(action) = self.propose_merge(graph, e1, e2) {
                // Only auto-apply if confidence is high enough
                if similarity >= self.config.min_auto_apply_confidence {
                    actions.push(action);
                }
            }
        }

        // Detect stale entities
        let stale_actions = self.detect_stale_entities(graph, &[]);
        actions.extend(stale_actions);

        actions
    }

    /// Apply evolution actions to the knowledge graph.
    /// 
    /// Returns statistics about the applied actions.
    pub fn apply_evolution_actions(
        &self,
        graph: &mut KnowledgeGraph,
        actions: &[KgEvolutionAction],
    ) -> EvolutionStats {
        let mut stats = EvolutionStats::default();

        for action in actions {
            match self.apply_action(graph, action) {
                Ok(true) => stats.actions_applied += 1,
                Ok(false) => { /* No change needed */ }
                Err(_) => stats.actions_failed += 1,
            }
        }

        stats
    }

    /// Apply updates to an entity.
    fn apply_entity_update(
        &self,
        graph: &mut KnowledgeGraph,
        entity_id: EntityId,
        new_name: Option<String>,
        new_type: Option<String>,
        attribute_changes: serde_json::Value,
    ) -> Result<bool> {
        if let Some(entity) = graph.get_entity_mut(entity_id) {
            if let Some(name) = new_name {
                entity.name = name;
            }
            if let Some(entity_type) = new_type {
                entity.entity_type = entity_type;
            }
            // Merge attribute changes
            if let (Some(existing), Some(changes)) = (
                entity.attributes.as_object_mut(),
                attribute_changes.as_object(),
            ) {
                for (key, value) in changes {
                    existing.insert(key.clone(), value.clone());
                }
            }
            Ok(true)
        } else {
            Err(KgError::EntityNotFound(entity_id))
        }
    }

    /// Apply updates to a relationship.
    fn apply_relationship_update(
        &self,
        graph: &mut KnowledgeGraph,
        relationship_id: RelationshipId,
        confidence: Option<f32>,
        _relevance: Option<f32>,
    ) -> Result<bool> {
        // Check if relationship exists
        if graph.get_relationship(relationship_id).is_none() {
            return Err(KgError::RelationshipNotFound(relationship_id));
        }
        
        // Update the relationship
        graph
            .update_relationship(relationship_id, confidence)
            .map_err(|e| KgError::GraphError(e.to_string()))?;
        
        Ok(true)
    }

    /// Split an entity into multiple new entities.
    fn apply_split(
        &self,
        graph: &mut KnowledgeGraph,
        original_id: EntityId,
        new_entities: &[memst_core::types::Entity],
    ) -> Result<bool> {
        // Verify original entity exists
        let original = graph
            .get_entity(original_id)
            .ok_or(KgError::EntityNotFound(original_id))?
            .clone();
        
        if new_entities.is_empty() {
            return Err(KgError::invalid_action("No new entities provided for split"));
        }
        
        // Add all new entities to the graph
        for entity in new_entities {
            // Check for ID conflicts
            if graph.get_entity(entity.id).is_some() {
                return Err(KgError::invalid_action(
                    format!("Entity with ID {:?} already exists", entity.id)
                ));
            }
            let _ = graph.add_entity(entity.clone());
        }
        
        // Get relationships of original entity to potentially redistribute
        let _original_relationships = graph.get_relationships(original_id);
        
        // For now, migrate all relationships to the first new entity
        // A smarter implementation would distribute based on semantic similarity
        if let Some(first_new) = new_entities.first() {
            graph.migrate_relationships(original_id, first_new.id)
                .map_err(|e| KgError::GraphError(e.to_string()))?;
        }
        
        // Deprecate the original entity
        if let Some(entity) = graph.get_entity_mut(original_id) {
            entity.deprecate(format!("Split into {} new entities", new_entities.len()));
        }
        
        // Store original entity info for reference
        let _ = original;
        
        Ok(true)
    }

    /// Get current configuration.
    pub fn config(&self) -> &KgEvolutionConfig {
        &self.config
    }
}

impl Default for KgEvolutionEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics from an evolution run.
#[derive(Debug, Clone, Default)]
pub struct EvolutionStats {
    /// Entities examined
    pub entities_examined: usize,
    /// Merge candidates found
    pub merge_candidates: usize,
    /// Entities marked for deprecation
    pub deprecated_entities: usize,
    /// Actions successfully applied
    pub actions_applied: usize,
    /// Actions that failed
    pub actions_failed: usize,
}

impl std::fmt::Display for EvolutionStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "KG Evolution Statistics:")?;
        writeln!(f, "  Entities examined: {}", self.entities_examined)?;
        writeln!(f, "  Merge candidates: {}", self.merge_candidates)?;
        writeln!(f, "  Deprecated entities: {}", self.deprecated_entities)?;
        writeln!(f, "  Actions applied: {}", self.actions_applied)?;
        writeln!(f, "  Actions failed: {}", self.actions_failed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_graph() -> KnowledgeGraph {
        let temp_dir = tempfile::tempdir().unwrap();
        KnowledgeGraph::new(temp_dir.path()).unwrap()
    }

    #[test]
    fn test_detect_merge_candidates() {
        let engine = KgEvolutionEngine::new();
        let mut graph = create_test_graph();
        let session_id = uuid::Uuid::new_v4();

        // Add similar entities
        graph
            .add_entity(Entity::new("Rust Programming", "technology", session_id))
            .unwrap();
        graph
            .add_entity(Entity::new("Rust Lang", "technology", session_id))
            .unwrap();
        graph
            .add_entity(Entity::new("Python", "technology", session_id))
            .unwrap();

        let candidates = engine.detect_merge_candidates(&graph);

        // Should detect Rust Programming and Rust Lang as similar
        assert!(!candidates.is_empty());
        
        // Check that Python is not similar to Rust entities
        let has_python_merge = candidates.iter().any(|(e1, e2, _)| {
            let ent1 = graph.get_entity(*e1).unwrap();
            let ent2 = graph.get_entity(*e2).unwrap();
            ent1.name == "Python" || ent2.name == "Python"
        });
        assert!(!has_python_merge);
    }

    #[test]
    fn test_text_similarity() {
        let engine = KgEvolutionEngine::new();

        let sim_high = engine.text_similarity("hello world", "hello world test");
        let sim_low = engine.text_similarity("hello world", "completely different");

        assert!(sim_high > 0.5);
        assert!(sim_low < 0.3);
    }

    #[test]
    fn test_propose_merge() {
        let engine = KgEvolutionEngine::new();
        let mut graph = create_test_graph();
        let session_id = uuid::Uuid::new_v4();

        let e1 = graph
            .add_entity(Entity::new("Rust", "technology", session_id))
            .unwrap();
        let e2 = graph
            .add_entity(Entity::new("Rust Lang", "technology", session_id))
            .unwrap();

        let action = engine.propose_merge(&graph, e1.id, e2.id);

        assert!(action.is_some());
        match action.unwrap() {
            KgEvolutionAction::MergeEntities { keep, merge, reason } => {
                assert!(reason.contains("Rust"));
                // The engine should prefer the entity with higher importance
                assert!(keep == e1.id || keep == e2.id);
                assert!(merge == e1.id || merge == e2.id);
                assert_ne!(keep, merge);
            }
            _ => panic!("Expected MergeEntities action"),
        }
    }

    #[test]
    fn test_evolve_cycle() {
        let engine = KgEvolutionEngine::new();
        let mut graph = create_test_graph();
        let session_id = uuid::Uuid::new_v4();

        // Add some test entities
        graph
            .add_entity(Entity::new("Alice", "person", session_id))
            .unwrap();
        graph
            .add_entity(Entity::new("Bob", "person", session_id))
            .unwrap();
        graph
            .add_entity(Entity::new("Alice Smith", "person", session_id))
            .unwrap();

        let actions = engine.evolve(&graph);

        // Should propose some actions
        assert!(!actions.is_empty());
    }
}
