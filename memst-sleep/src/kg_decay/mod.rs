//! Knowledge Graph Decay Engine
//!
//! Implements temporal decay for KG entities and relationships.
//! Relevance decreases over time based on half-life, simulating
//! natural forgetting of less-used knowledge.

use chrono::{DateTime, Duration, Utc};
use memst_core::graph::KnowledgeGraph;
use memst_core::types::{Entity, EntityId, KgDecayConfig, Relationship, RelationshipId};

/// Decay engine for knowledge graph maintenance.
pub struct KgDecayEngine {
    config: KgDecayConfig,
}

impl KgDecayEngine {
    /// Create a new decay engine with default configuration.
    pub fn new() -> Self {
        Self {
            config: KgDecayConfig::default(),
        }
    }

    /// Create a new decay engine with custom configuration.
    pub fn with_config(config: KgDecayConfig) -> Self {
        Self { config }
    }

    /// Calculate decay factor based on elapsed time and half-life.
    ///
    /// Formula: decay_factor = 0.5 ^ (elapsed_days / half_life_days)
    fn calculate_decay_factor(elapsed_days: f32, half_life_days: f32) -> f32 {
        if half_life_days <= 0.0 {
            return 1.0; // No decay
        }
        0.5f32.powf(elapsed_days / half_life_days)
    }

    /// Apply decay to a single entity's relevance.
    pub fn decay_entity(&self, entity: &mut Entity, now: DateTime<Utc>) {
        // Skip stable entities
        if entity.is_stable {
            return;
        }

        let elapsed = now - entity.last_decay_at;
        let elapsed_days = elapsed.num_seconds() as f32 / 86400.0;

        if elapsed_days < 0.01 {
            // Less than ~15 minutes, skip
            return;
        }

        let decay_factor = Self::calculate_decay_factor(elapsed_days, entity.half_life_days);
        entity.relevance *= decay_factor;
        entity.last_decay_at = now;

        // Ensure minimum relevance
        if entity.relevance < 0.01 {
            entity.relevance = 0.01;
        }
    }

    /// Apply decay to a single relationship's relevance.
    pub fn decay_relationship(&self, relationship: &mut Relationship, now: DateTime<Utc>) {
        // Skip stable relationships
        if relationship.is_stable {
            return;
        }

        let elapsed = now - relationship.last_decay_at;
        let elapsed_days = elapsed.num_seconds() as f32 / 86400.0;

        if elapsed_days < 0.01 {
            return;
        }

        let decay_factor =
            Self::calculate_decay_factor(elapsed_days, relationship.half_life_days);
        relationship.relevance *= decay_factor;
        relationship.last_decay_at = now;

        // Ensure minimum relevance
        if relationship.relevance < 0.01 {
            relationship.relevance = 0.01;
        }
    }

    /// Apply decay to all entities and relationships in the graph.
    ///
    /// Returns statistics about the decay operation.
    pub fn apply_decay(&self, graph: &mut KnowledgeGraph) -> DecayResult {
        let now = Utc::now();
        let mut stats = DecayResult::default();

        // Get all entities and apply decay
        let entity_ids: Vec<EntityId> = graph
            .all_entities()
            .iter()
            .map(|e| e.id)
            .collect();

        for entity_id in entity_ids {
            if let Some(entity) = graph.get_entity_mut(entity_id) {
                let old_relevance = entity.relevance;
                self.decay_entity(entity, now);

                stats.entities_processed += 1;
                
                if entity.relevance < old_relevance {
                    stats.entities_decayed += 1;
                }

                if entity.relevance < self.config.min_relevance_threshold {
                    stats.entities_below_threshold += 1;
                }
            }
        }

        // Apply decay to relationships
        // Note: We need to rebuild edges since KnowledgeGraph stores SerializableEdge
        // For now, we track that relationships would be decayed
        stats.relationships_processed = graph.stats().relationship_count;

        stats
    }

    /// Get entities that are candidates for removal due to low relevance.
    pub fn find_stale_entities(&self, graph: &KnowledgeGraph) -> Vec<(EntityId, f32)> {
        let mut stale = Vec::new();

        for entity in graph.all_entities() {
            if entity.relevance < self.config.min_relevance_threshold && !entity.is_stable {
                stale.push((entity.id, entity.relevance));
            }
        }

        // Sort by relevance (lowest first)
        stale.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        stale
    }

    /// Calculate time until entity reaches minimum relevance.
    ///
    /// Returns None if entity is already at or below threshold, or if entity is stable.
    pub fn time_until_stale(&self, entity: &Entity) -> Option<Duration> {
        if entity.is_stable || entity.relevance <= self.config.min_relevance_threshold {
            return None;
        }

        // Solve for t: min_threshold = relevance * 0.5^(t / half_life)
        // ln(min_threshold / relevance) = (t / half_life) * ln(0.5)
        // t = half_life * ln(min_threshold / relevance) / ln(0.5)

        let ratio = self.config.min_relevance_threshold / entity.relevance;
        if ratio >= 1.0 {
            return Some(Duration::seconds(0));
        }

        let t_days = entity.half_life_days * ratio.ln() / 0.5f32.ln();
        Some(Duration::seconds((t_days * 86400.0) as i64))
    }

    /// Boost relevance of an entity (called when entity is accessed).
    pub fn boost_relevance(&self, entity: &mut Entity) {
        entity.relevance = (entity.relevance + self.config.access_boost).min(1.0);
        entity.record_access();
    }

    /// Get current configuration.
    pub fn config(&self) -> &KgDecayConfig {
        &self.config
    }
}

impl Default for KgDecayEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics from a decay operation.
#[derive(Debug, Clone, Default)]
pub struct DecayResult {
    /// Total entities processed
    pub entities_processed: usize,
    /// Entities that had their relevance decayed
    pub entities_decayed: usize,
    /// Entities below minimum relevance threshold
    pub entities_below_threshold: usize,
    /// Total relationships processed
    pub relationships_processed: usize,
    /// Relationships that had their relevance decayed
    pub relationships_decayed: usize,
}

impl std::fmt::Display for DecayResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "KG Decay Results:")?;
        writeln!(f, "  Entities processed: {}", self.entities_processed)?;
        writeln!(f, "  Entities decayed: {}", self.entities_decayed)?;
        writeln!(
            f,
            "  Entities below threshold: {}",
            self.entities_below_threshold
        )?;
        writeln!(f, "  Relationships processed: {}", self.relationships_processed)?;
        writeln!(f, "  Relationships decayed: {}", self.relationships_decayed)
    }
}

/// Half-life presets for common entity types.
pub mod presets {
    use super::*;

    /// Core facts that rarely change (e.g., "Rust is a programming language")
    pub const PERMANENT: f32 = 365.0 * 10.0; // 10 years

    /// Long-term user preferences
    pub const LONG_TERM: f32 = 90.0; // 3 months

    /// Short-term project details
    pub const SHORT_TERM: f32 = 30.0; // 1 month

    /// Temporary/transient information
    pub const TEMPORARY: f32 = 7.0; // 1 week

    /// Apply appropriate half-life based on entity type name.
    pub fn half_life_for_type(entity_type: &str) -> f32 {
        match entity_type.to_lowercase().as_str() {
            "person" | "user" => LONG_TERM,
            "preference" | "like" | "dislike" => LONG_TERM,
            "technology" | "language" | "framework" => PERMANENT,
            "project" | "task" => SHORT_TERM,
            "event" | "meeting" => TEMPORARY,
            "location" | "place" => LONG_TERM,
            "organization" | "company" => LONG_TERM,
            _ => SHORT_TERM,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_graph() -> KnowledgeGraph {
        let temp_dir = tempfile::tempdir().unwrap();
        KnowledgeGraph::new(temp_dir.path()).unwrap()
    }

    #[test]
    fn test_decay_factor_calculation() {
        // After one half-life, decay factor should be 0.5
        let factor = KgDecayEngine::calculate_decay_factor(30.0, 30.0);
        assert!((factor - 0.5).abs() < 0.001);

        // After two half-lives, decay factor should be 0.25
        let factor = KgDecayEngine::calculate_decay_factor(60.0, 30.0);
        assert!((factor - 0.25).abs() < 0.001);

        // At time 0, decay factor should be 1.0
        let factor = KgDecayEngine::calculate_decay_factor(0.0, 30.0);
        assert!((factor - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_entity_decay() {
        let engine = KgDecayEngine::new();
        let mut entity = Entity::new("Test", "test", uuid::Uuid::new_v4());
        entity.relevance = 1.0;
        entity.half_life_days = 30.0;
        entity.last_decay_at = Utc::now() - Duration::days(30);

        engine.decay_entity(&mut entity, Utc::now());

        // After one half-life, relevance should be ~0.5
        assert!((entity.relevance - 0.5).abs() < 0.1);
    }

    #[test]
    fn test_stable_entity_no_decay() {
        let engine = KgDecayEngine::new();
        let mut entity = Entity::stable("Test", "test", uuid::Uuid::new_v4());
        entity.relevance = 1.0;
        entity.last_decay_at = Utc::now() - Duration::days(100);

        engine.decay_entity(&mut entity, Utc::now());

        // Stable entities should not decay
        assert!((entity.relevance - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_boost_relevance() {
        let engine = KgDecayEngine::new();
        let mut entity = Entity::new("Test", "test", uuid::Uuid::new_v4());
        entity.relevance = 0.5;

        engine.boost_relevance(&mut entity);

        assert!(entity.relevance > 0.5);
        assert_eq!(entity.access_count, 1);
    }

    #[test]
    fn test_time_until_stale() {
        let engine = KgDecayEngine::new();
        let mut entity = Entity::new("Test", "test", uuid::Uuid::new_v4());
        entity.relevance = 0.5;
        entity.half_life_days = 30.0;

        let time = engine.time_until_stale(&entity);
        assert!(time.is_some());

        // Stable entity should return None
        let stable = Entity::stable("Test", "test", uuid::Uuid::new_v4());
        assert!(engine.time_until_stale(&stable).is_none());
    }

    #[test]
    fn test_presets() {
        assert_eq!(presets::half_life_for_type("person"), presets::LONG_TERM);
        assert_eq!(presets::half_life_for_type("technology"), presets::PERMANENT);
        assert_eq!(presets::half_life_for_type("event"), presets::TEMPORARY);
        assert_eq!(presets::half_life_for_type("unknown"), presets::SHORT_TERM);
    }
}
