//! Knowledge Graph Storage using petgraph.
//!
//! This module provides graph-based storage for entities and relationships
//! extracted from conversations and memories.

use crate::error::Result;
use crate::types::{
    Entity, EntityId, EntityQuery, Relationship, RelationshipId, RelationshipQuery,
};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Serializable graph data.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SerializableGraph {
    entities: Vec<Entity>,
    edges: Vec<SerializableEdge>,
    node_index: Vec<(EntityId, usize)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SerializableEdge {
    subject_id: EntityId,
    predicate: String,
    object_id: EntityId,
    relationship_id: RelationshipId,
    confidence: f32,
    session_id: Uuid,
    source_message_id: Option<Uuid>,
    created_at: chrono::DateTime<chrono::Utc>,
}

/// Knowledge graph index for a session.
pub struct KnowledgeGraph {
    /// Base path
    #[allow(dead_code)]
    base_path: PathBuf,
    /// Graph file path
    graph_path: PathBuf,
    /// Entities
    entities: Vec<Entity>,
    /// Entity ID to index map
    entity_index: std::collections::HashMap<EntityId, usize>,
    /// Edges
    edges: Vec<SerializableEdge>,
}

impl KnowledgeGraph {
    /// Create or open a knowledge graph.
    pub fn new(base_path: &Path) -> Result<Self> {
        let graph_path = base_path.join("graph.json");

        // Create directory if needed
        fs::create_dir_all(base_path)?;

        // Load existing graph or create new one
        let (entities, edges, entity_index) = if graph_path.exists() {
            Self::load_graph(&graph_path)?
        } else {
            (Vec::new(), Vec::new(), std::collections::HashMap::new())
        };

        Ok(Self {
            base_path: base_path.to_path_buf(),
            graph_path,
            entities,
            entity_index,
            edges,
        })
    }

    /// Load graph from file.
    fn load_graph(
        path: &Path,
    ) -> Result<(
        Vec<Entity>,
        Vec<SerializableEdge>,
        std::collections::HashMap<EntityId, usize>,
    )> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let data: SerializableGraph = serde_json::from_reader(reader)?;

        let mut entity_index = std::collections::HashMap::new();
        for (idx, entity) in data.entities.iter().enumerate() {
            entity_index.insert(entity.id, idx);
        }

        Ok((data.entities, data.edges, entity_index))
    }

    /// Save graph to file.
    fn save(&self) -> Result<()> {
        let data = SerializableGraph {
            entities: self.entities.clone(),
            edges: self.edges.clone(),
            node_index: self
                .entity_index
                .iter()
                .map(|(id, idx)| (*id, *idx))
                .collect(),
        };

        let file = File::create(&self.graph_path)?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, &data)?;
        writer.write_all(b"\n")?;
        Ok(())
    }

    /// Add an entity to the graph.
    pub fn add_entity(&mut self, entity: Entity) -> Result<Entity> {
        self.entities.push(entity.clone());
        self.entity_index.insert(entity.id, self.entities.len() - 1);
        self.save()?;
        Ok(entity)
    }

    /// Get an entity by ID.
    pub fn get_entity(&self, entity_id: EntityId) -> Option<&Entity> {
        self.entity_index
            .get(&entity_id)
            .and_then(|idx| self.entities.get(*idx))
    }

    /// Get a mutable entity by ID.
    pub fn get_entity_mut(&mut self, entity_id: EntityId) -> Option<&mut Entity> {
        self.entity_index
            .get(&entity_id)
            .and_then(|idx| self.entities.get_mut(*idx))
    }

    /// Record an access to an entity.
    pub fn access_entity(&mut self, entity_id: EntityId) -> bool {
        if let Some(entity) = self.get_entity_mut(entity_id) {
            entity.record_access();
            return true;
        }
        false
    }

    /// Add a relationship to the graph.
    pub fn add_relationship(&mut self, relationship: Relationship) -> Result<Relationship> {
        use crate::error::Error;

        // Verify both entities exist
        if !self.entity_index.contains_key(&relationship.subject_id) {
            return Err(Error::InvalidOperation(format!(
                "Subject entity not found: {}",
                relationship.subject_id
            )));
        }
        if !self.entity_index.contains_key(&relationship.object_id) {
            return Err(Error::InvalidOperation(format!(
                "Object entity not found: {}",
                relationship.object_id
            )));
        }

        let edge = SerializableEdge {
            subject_id: relationship.subject_id,
            predicate: relationship.predicate.clone(),
            object_id: relationship.object_id,
            relationship_id: relationship.id,
            confidence: relationship.confidence,
            session_id: relationship.session_id,
            source_message_id: relationship.source_message_id,
            created_at: relationship.created_at,
        };

        self.edges.push(edge);
        self.save()?;
        Ok(relationship)
    }

    /// Get relationships for an entity.
    pub fn get_relationships(&self, entity_id: EntityId) -> Vec<(Relationship, Entity, Entity)> {
        self.edges
            .iter()
            .filter(|e| e.subject_id == entity_id)
            .filter_map(|e| {
                let object_entity = self.get_entity(e.object_id)?.clone();
                let subject_entity = self.get_entity(e.subject_id)?.clone();

                let relationship = Relationship::new(
                    entity_id,
                    e.predicate.clone(),
                    object_entity.id,
                    e.session_id,
                )
                .with_confidence(e.confidence)
                .with_source(e.source_message_id.unwrap_or_else(Uuid::nil));

                Some((relationship, subject_entity, object_entity))
            })
            .collect()
    }

    /// Find entities by query.
    pub fn find_entities(&self, query: EntityQuery) -> Vec<&Entity> {
        let mut results: Vec<&Entity> = self.entities.iter().collect();

        // Filter by type
        if let Some(ref entity_type) = query.entity_type {
            results.retain(|e| e.entity_type == *entity_type);
        }

        // Filter by name contains
        if let Some(ref name_contains) = query.name_contains {
            results.retain(|e| {
                e.name
                    .to_lowercase()
                    .contains(&name_contains.to_lowercase())
            });
        }

        // Filter by confidence
        if let Some(min_conf) = query.min_confidence {
            results.retain(|e| e.confidence >= min_conf);
        }

        // Sort by importance (descending)
        results.sort_by(|a, b| b.importance().partial_cmp(&a.importance()).unwrap());

        // Apply limit
        if query.limit > 0 && results.len() > query.limit {
            results.truncate(query.limit);
        }

        results
    }

    /// Find relationships by query.
    pub fn find_relationships(
        &self,
        query: RelationshipQuery,
    ) -> Vec<(Relationship, Entity, Entity)> {
        self.edges
            .iter()
            .filter(|e| {
                // Filter by subject
                if let Some(ref subj) = query.subject_id {
                    if e.subject_id != *subj {
                        return false;
                    }
                }

                // Filter by object
                if let Some(ref obj) = query.object_id {
                    if e.object_id != *obj {
                        return false;
                    }
                }

                // Filter by predicate
                if let Some(ref pred) = query.predicate {
                    if e.predicate != *pred {
                        return false;
                    }
                }

                // Filter by confidence
                if let Some(min_conf) = query.min_confidence {
                    if e.confidence < min_conf {
                        return false;
                    }
                }

                true
            })
            .filter_map(|e| {
                let source_entity = self.get_entity(e.subject_id)?.clone();
                let target_entity = self.get_entity(e.object_id)?.clone();

                let relationship = Relationship::new(
                    source_entity.id,
                    e.predicate.clone(),
                    target_entity.id,
                    e.session_id,
                )
                .with_confidence(e.confidence)
                .with_source(e.source_message_id.unwrap_or_else(Uuid::nil));

                Some((relationship, source_entity, target_entity))
            })
            .take(if query.limit > 0 {
                query.limit
            } else {
                usize::MAX
            })
            .collect()
    }

    /// Delete an entity and all its relationships.
    pub fn delete_entity(&mut self, entity_id: EntityId) -> Result<bool> {
        if let Some(idx) = self.entity_index.get(&entity_id) {
            // Remove entity
            self.entities.remove(*idx);

            // Rebuild entity index
            self.entity_index.clear();
            for (new_idx, entity) in self.entities.iter().enumerate() {
                self.entity_index.insert(entity.id, new_idx);
            }

            // Remove edges connected to this entity
            self.edges
                .retain(|e| e.subject_id != entity_id && e.object_id != entity_id);

            self.save()?;
            return Ok(true);
        }
        Ok(false)
    }

    /// Delete a relationship.
    pub fn delete_relationship(&mut self, relationship_id: RelationshipId) -> Result<bool> {
        let initial_len = self.edges.len();
        self.edges.retain(|e| e.relationship_id != relationship_id);

        if self.edges.len() < initial_len {
            self.save()?;
            return Ok(true);
        }
        Ok(false)
    }

    /// Get graph statistics.
    pub fn stats(&self) -> GraphStats {
        let entity_types: std::collections::HashMap<String, usize> = self
            .entities
            .iter()
            .map(|e| e.entity_type.clone())
            .fold(std::collections::HashMap::new(), |mut acc, t| {
                *acc.entry(t).or_insert(0) += 1;
                acc
            });

        let predicates: std::collections::HashMap<String, usize> = self
            .edges
            .iter()
            .map(|e| e.predicate.clone())
            .fold(std::collections::HashMap::new(), |mut acc, p| {
                *acc.entry(p).or_insert(0) += 1;
                acc
            });

        GraphStats {
            entity_count: self.entities.len(),
            relationship_count: self.edges.len(),
            entity_types,
            predicates,
        }
    }

    /// Get all entities.
    pub fn all_entities(&self) -> Vec<&Entity> {
        self.entities.iter().collect()
    }

    /// Get all relationships.
    pub fn all_relationships(&self) -> Vec<(Relationship, Entity, Entity)> {
        self.edges
            .iter()
            .filter_map(|e| {
                let source = self.get_entity(e.subject_id)?.clone();
                let target = self.get_entity(e.object_id)?.clone();

                let relationship =
                    Relationship::new(source.id, e.predicate.clone(), target.id, e.session_id)
                        .with_confidence(e.confidence)
                        .with_source(e.source_message_id.unwrap_or_else(Uuid::nil));

                Some((relationship, source, target))
            })
            .collect()
    }
}

/// Graph statistics.
#[derive(Debug)]
pub struct GraphStats {
    /// Total number of entities in the graph.
    pub entity_count: usize,
    /// Total number of relationships (edges) in the graph.
    pub relationship_count: usize,
    /// Count of entities per entity type.
    pub entity_types: std::collections::HashMap<String, usize>,
    /// Count of relationships per predicate.
    pub predicates: std::collections::HashMap<String, usize>,
}

impl std::fmt::Display for GraphStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Knowledge Graph Statistics")?;
        writeln!(f, "  Entities: {}", self.entity_count)?;
        writeln!(f, "  Relationships: {}", self.relationship_count)?;

        if !self.entity_types.is_empty() {
            writeln!(f, "  Entity Types:")?;
            for (t, c) in &self.entity_types {
                writeln!(f, "    {}: {}", t, c)?;
            }
        }

        if !self.predicates.is_empty() {
            writeln!(f, "  Predicates:")?;
            for (p, c) in &self.predicates {
                writeln!(f, "    {}: {}", p, c)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_graph(path: &Path) -> KnowledgeGraph {
        KnowledgeGraph::new(path).unwrap()
    }

    #[test]
    fn test_add_entity() {
        let temp_dir = TempDir::new().unwrap();
        let mut graph = create_test_graph(temp_dir.path());

        let entity = Entity::new("Rust", "technology", Uuid::new_v4());
        let added = graph.add_entity(entity).unwrap();

        assert_eq!(added.name, "Rust");
        assert!(added.id != Uuid::nil());

        let retrieved = graph.get_entity(added.id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Rust");
    }

    #[test]
    fn test_add_relationship() {
        let temp_dir = TempDir::new().unwrap();
        let mut graph = create_test_graph(temp_dir.path());

        let entity1 = Entity::new("User", "person", Uuid::new_v4());
        let entity2 = Entity::new("Rust", "technology", Uuid::new_v4());

        graph.add_entity(entity1.clone()).unwrap();
        graph.add_entity(entity2.clone()).unwrap();

        let relationship = Relationship::new(entity1.id, "uses", entity2.id, Uuid::new_v4());
        let added = graph.add_relationship(relationship).unwrap();

        assert_eq!(added.predicate, "uses");

        let rels = graph.get_relationships(entity1.id);
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].0.predicate, "uses");
    }

    #[test]
    fn test_find_entities_by_type() {
        let temp_dir = TempDir::new().unwrap();
        let mut graph = create_test_graph(temp_dir.path());

        let session_id = Uuid::new_v4();
        graph
            .add_entity(Entity::new("Rust", "technology", session_id))
            .unwrap();
        graph
            .add_entity(Entity::new("GPT-4", "technology", session_id))
            .unwrap();
        graph
            .add_entity(Entity::new("Alice", "person", session_id))
            .unwrap();

        let tech_results = graph.find_entities(EntityQuery::new().with_type("technology"));
        assert_eq!(tech_results.len(), 2);

        let person_results = graph.find_entities(EntityQuery::new().with_type("person"));
        assert_eq!(person_results.len(), 1);
    }

    #[test]
    fn test_find_entities_by_name() {
        let temp_dir = TempDir::new().unwrap();
        let mut graph = create_test_graph(temp_dir.path());

        let session_id = Uuid::new_v4();
        graph
            .add_entity(Entity::new("Rust Programming", "technology", session_id))
            .unwrap();
        graph
            .add_entity(Entity::new("Rust the metal", "material", session_id))
            .unwrap();
        graph
            .add_entity(Entity::new("Python", "technology", session_id))
            .unwrap();

        let results = graph.find_entities(EntityQuery::new().with_name_contains("rust"));
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_delete_entity() {
        let temp_dir = TempDir::new().unwrap();
        let mut graph = create_test_graph(temp_dir.path());

        let entity = Entity::new("Test", "test", Uuid::new_v4());
        let entity_id = entity.id;
        graph.add_entity(entity).unwrap();

        assert!(graph.get_entity(entity_id).is_some());

        let deleted = graph.delete_entity(entity_id).unwrap();
        assert!(deleted);

        assert!(graph.get_entity(entity_id).is_none());
    }

    #[test]
    fn test_graph_stats() {
        let temp_dir = TempDir::new().unwrap();
        let mut graph = create_test_graph(temp_dir.path());

        let session_id = Uuid::new_v4();
        graph
            .add_entity(Entity::new("A", "type1", session_id))
            .unwrap();
        graph
            .add_entity(Entity::new("B", "type1", session_id))
            .unwrap();
        graph
            .add_entity(Entity::new("C", "type2", session_id))
            .unwrap();

        let entity1 = Entity::new("D", "type1", session_id).with_confidence(0.8);
        graph.add_entity(entity1).unwrap();

        let stats = graph.stats();
        assert_eq!(stats.entity_count, 4);
    }

    #[test]
    fn test_access_count() {
        let temp_dir = TempDir::new().unwrap();
        let mut graph = create_test_graph(temp_dir.path());

        let entity = Entity::new("Test", "test", Uuid::new_v4());
        let entity_id = entity.id;
        graph.add_entity(entity).unwrap();

        assert_eq!(graph.get_entity(entity_id).unwrap().access_count, 0);

        graph.access_entity(entity_id);
        graph.access_entity(entity_id);

        assert_eq!(graph.get_entity(entity_id).unwrap().access_count, 2);
    }

    #[test]
    fn test_entity_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let graph = create_test_graph(temp_dir.path());

        let fake_id = Uuid::new_v4();
        assert!(graph.get_entity(fake_id).is_none());
    }

    #[test]
    fn test_relationship_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let mut graph = create_test_graph(temp_dir.path());

        let fake_rel_id = RelationshipId::new_v4();
        let deleted = graph.delete_relationship(fake_rel_id).unwrap();
        assert!(!deleted);
    }

    #[test]
    fn test_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Create and add data
        {
            let mut graph = create_test_graph(path);
            let entity = Entity::new("Persistent", "test", Uuid::new_v4());
            graph.add_entity(entity).unwrap();
        }

        // Reopen and verify
        {
            let graph = create_test_graph(path);
            let all = graph.all_entities();
            assert_eq!(all.len(), 1);
            assert_eq!(all[0].name, "Persistent");
        }
    }

    #[test]
    fn test_find_relationships_by_predicate() {
        let temp_dir = TempDir::new().unwrap();
        let mut graph = create_test_graph(temp_dir.path());

        let session_id = Uuid::new_v4();
        let entity1 = Entity::new("A", "test", session_id);
        let entity2 = Entity::new("B", "test", session_id);
        let entity3 = Entity::new("C", "test", session_id);

        graph.add_entity(entity1.clone()).unwrap();
        graph.add_entity(entity2.clone()).unwrap();
        graph.add_entity(entity3.clone()).unwrap();

        graph
            .add_relationship(Relationship::new(
                entity1.id, "knows", entity2.id, session_id,
            ))
            .unwrap();
        graph
            .add_relationship(Relationship::new(
                entity1.id, "knows", entity3.id, session_id,
            ))
            .unwrap();
        graph
            .add_relationship(Relationship::new(
                entity2.id, "likes", entity3.id, session_id,
            ))
            .unwrap();

        let knows_rels = graph.find_relationships(RelationshipQuery::new().with_predicate("knows"));
        assert_eq!(knows_rels.len(), 2);

        let likes_rels = graph.find_relationships(RelationshipQuery::new().with_predicate("likes"));
        assert_eq!(likes_rels.len(), 1);
    }
}
