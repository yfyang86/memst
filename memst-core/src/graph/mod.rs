//! Knowledge graph of entities and relationships.
//!
//! [`KnowledgeGraph`] is the API the rest of MemSt uses. Storage is delegated
//! to a [`GraphBackend`]:
//!
//! | Backend | Crate | Use |
//! |---------|-------|-----|
//! | [`JsonGraphBackend`] | `memst-core` | one `graph.json`, zero dependencies |
//! | `HelixGraphBackend` | `memst-graph-helix` | embedded HelixDB (graph + vector + BM25) |
//!
//! Every operation returns owned values and a `Result`, because a database
//! backend can fail on any call and cannot lend references into its storage.

mod backend;
pub mod conformance;
mod json;
mod search;

pub use backend::{entity_matches, relationship_matches, rrf_fuse, Direction, GraphBackend};
pub use search::{
    relationship_text, EntityHit, EntitySearch, EntitySearchResult, RelationshipSearchResult, SearchMethod,
};
pub use json::JsonGraphBackend;

use crate::error::{Error, Result};
use crate::types::{
    Entity, EntityId, EntityQuery, Relationship, RelationshipId, RelationshipQuery,
};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Embedder id recorded for vectors supplied through
/// [`KnowledgeGraph::set_entity_embedding`].
pub const MANUAL_EMBEDDER: &str = "manual";

/// What an embedding of `entity` by `embedder_id` is computed from: the
/// model, and a fingerprint of the entity's text. Stored next to the vector;
/// when it no longer matches, the embedding is out of date.
pub fn embedding_source(entity: &Entity, embedder_id: &str) -> String {
    let fingerprint = blake3::hash(entity.search_text().as_bytes()).to_hex();
    format!("{embedder_id}#{}", &fingerprint[..16])
}

/// A relationship together with its subject and object entities.
pub type Triple = (Relationship, Entity, Entity);

/// Knowledge graph over a pluggable storage backend.
pub struct KnowledgeGraph {
    backend: Box<dyn GraphBackend>,
}

impl KnowledgeGraph {
    /// Open (or create) a graph stored as `<base_path>/graph.json`.
    pub fn new(base_path: &Path) -> Result<Self> {
        Ok(Self::with_backend(Box::new(JsonGraphBackend::open(base_path)?)))
    }

    /// Wrap an already-opened backend.
    pub fn with_backend(backend: Box<dyn GraphBackend>) -> Self {
        Self { backend }
    }

    /// Identifier of the storage backend in use (`"json"`, `"helix"`, ...).
    pub fn backend_kind(&self) -> &'static str {
        self.backend.kind()
    }

    /// Direct access to the backend, for backend-specific features.
    pub fn backend(&self) -> &dyn GraphBackend {
        self.backend.as_ref()
    }

    /// Make all writes durable.
    pub fn flush(&mut self) -> Result<()> {
        self.backend.flush()
    }

    // ===== Entities =====

    /// Add an entity to the graph (replaces an entity with the same id).
    pub fn add_entity(&mut self, entity: Entity) -> Result<Entity> {
        self.backend.put_entity(&entity)?;
        Ok(entity)
    }

    /// Add an entity, unless the graph already has one with the same name
    /// (ignoring case) and type: then that entity is kept, marked as also
    /// seen in the new entity's session, and given any attributes it lacked.
    ///
    /// Use this when writing what a conversation mentions, so that the same
    /// thing met in several sessions is one node. Returns the stored entity
    /// (whose id may differ from the argument's) and whether it was created.
    pub fn resolve_or_add_entity(&mut self, entity: Entity) -> Result<(Entity, bool)> {
        let Some(mut existing) = self
            .backend
            .find_entity_by_name(&entity.name, &entity.entity_type)?
        else {
            self.backend.put_entity(&entity)?;
            return Ok((entity, true));
        };

        let mut changed = existing.mark_seen_in(entity.session_id);
        if let (Some(kept), Some(incoming)) = (
            existing.attributes.as_object_mut(),
            entity.attributes.as_object(),
        ) {
            for (key, value) in incoming {
                if !kept.contains_key(key) {
                    kept.insert(key.clone(), value.clone());
                    changed = true;
                }
            }
        }
        if changed {
            self.backend.put_entity(&existing)?;
        }
        Ok((existing, false))
    }

    /// Add many entities in one write.
    pub fn add_entities(&mut self, entities: &[Entity]) -> Result<()> {
        self.backend.put_entities(entities)
    }

    /// Get an entity by ID.
    pub fn get_entity(&self, entity_id: EntityId) -> Result<Option<Entity>> {
        self.backend.get_entity(entity_id)
    }

    /// Read-modify-write one entity. Returns `false` if it does not exist.
    ///
    /// This replaces the old `get_entity_mut`, whose in-place edits were
    /// never written back to disk.
    pub fn update_entity<F>(&mut self, entity_id: EntityId, f: F) -> Result<bool>
    where
        F: FnOnce(&mut Entity),
    {
        let Some(mut entity) = self.backend.get_entity(entity_id)? else {
            return Ok(false);
        };
        f(&mut entity);
        self.backend.put_entity(&entity)?;
        Ok(true)
    }

    /// Record an access to an entity (bumps access count and relevance).
    pub fn access_entity(&mut self, entity_id: EntityId) -> Result<bool> {
        self.update_entity(entity_id, Entity::record_access)
    }

    /// Find entities by query, most important first.
    pub fn find_entities(&self, query: EntityQuery) -> Result<Vec<Entity>> {
        let mut results = self.backend.scan_entities(&query)?;
        results.sort_by(|a, b| b.importance().total_cmp(&a.importance()));
        if query.limit > 0 {
            results.truncate(query.limit);
        }
        Ok(results)
    }

    /// Attach an embedding you computed yourself to an entity, for semantic
    /// search. Returns `false` if the entity does not exist.
    ///
    /// Prefer [`embed_pending`](Self::embed_pending), which also keeps
    /// embeddings up to date when entities change.
    pub fn set_entity_embedding(&mut self, entity_id: EntityId, embedding: &[f32]) -> Result<bool> {
        if embedding.is_empty() {
            return Err(Error::InvalidOperation("embedding is empty".to_string()));
        }
        let Some(entity) = self.backend.get_entity(entity_id)? else {
            return Ok(false);
        };
        let source = embedding_source(&entity, MANUAL_EMBEDDER);
        self.backend.set_entity_embedding(entity_id, embedding, &source)
    }

    /// Ids of entities that have an embedding.
    pub fn embedded_entity_ids(&self) -> Result<HashSet<EntityId>> {
        Ok(self
            .backend
            .entity_embedding_sources()?
            .into_iter()
            .map(|(id, _)| id)
            .collect())
    }

    /// Entities that `embedder_id` has not embedded in their current form:
    /// never embedded, embedded by another model, or changed since (renamed,
    /// retyped, attributes added). Deprecated entities are skipped.
    pub fn entities_needing_embedding(&self, embedder_id: &str) -> Result<Vec<Entity>> {
        let stored: HashMap<EntityId, String> =
            self.backend.entity_embedding_sources()?.into_iter().collect();
        Ok(self
            .all_entities()?
            .into_iter()
            .filter(|e| !e.is_deprecated)
            .filter(|e| stored.get(&e.id) != Some(&embedding_source(e, embedder_id)))
            .collect())
    }

    /// Embed every entity that needs it (see
    /// [`entities_needing_embedding`](Self::entities_needing_embedding)) and
    /// store the vectors. Call this after writing to the graph; it does
    /// nothing when everything is current. Returns how many were embedded.
    pub fn embed_pending(&mut self, embedder: &dyn crate::embed::Embedder) -> Result<usize> {
        const BATCH: usize = 32;
        let embedder_id = embedder.id();
        let pending = self.entities_needing_embedding(&embedder_id)?;
        for batch in pending.chunks(BATCH) {
            let texts: Vec<String> = batch.iter().map(Entity::search_text).collect();
            let vectors = embedder.embed(&texts)?;
            if vectors.len() != batch.len() {
                return Err(Error::InvalidOperation(format!(
                    "embedder {embedder_id} returned {} vectors for {} texts",
                    vectors.len(),
                    batch.len()
                )));
            }
            for (entity, vector) in batch.iter().zip(&vectors) {
                let source = embedding_source(entity, &embedder_id);
                self.backend.set_entity_embedding(entity.id, vector, &source)?;
            }
        }
        Ok(pending.len())
    }

    /// Search entities by keywords, by embedding, or both.
    ///
    /// The backend's own indexes are tried first (HelixDB: BM25 over name,
    /// type and attributes, plus vector search, fused by reciprocal rank). If
    /// the backend has none, fails, or finds nothing, MemSt's own matching is
    /// used instead: case-insensitive substring matching on names ranked by
    /// importance, plus brute-force cosine similarity over stored embeddings.
    /// [`EntitySearchResult::method`] says which path answered.
    pub fn search_entities(&self, query: &EntitySearch) -> Result<EntitySearchResult> {
        search::run(self.backend.as_ref(), query)
    }

    /// Search relationships by keywords, matched against
    /// "subject predicate object" (for example "alice uses rust").
    ///
    /// Like [`search_entities`](Self::search_entities): the backend's index
    /// first (HelixDB: BM25), then MemSt's own word matching when the backend
    /// has none, fails, or finds nothing. `limit` 0 means 10.
    pub fn search_relationships(&self, text: &str, limit: usize) -> Result<RelationshipSearchResult> {
        let (ranked, method, fallback_reason) = search::run_relationships(self.backend.as_ref(), text, limit)?;
        let mut hits = Vec::with_capacity(ranked.len());
        for (id, score) in ranked {
            if let Some(relationship) = self.backend.get_relationship(id)? {
                if let Some(triple) = self.join_endpoints(vec![relationship])?.pop() {
                    hits.push((triple, score));
                }
            }
        }
        Ok(RelationshipSearchResult { hits, method, fallback_reason })
    }

    /// Delete an entity and all its relationships.
    pub fn delete_entity(&mut self, entity_id: EntityId) -> Result<bool> {
        self.backend.delete_entity(entity_id)
    }

    /// Get all entities.
    pub fn all_entities(&self) -> Result<Vec<Entity>> {
        self.backend.scan_entities(&EntityQuery::default())
    }

    // ===== Relationships =====

    /// Add a relationship. Both endpoints must already exist.
    pub fn add_relationship(&mut self, relationship: Relationship) -> Result<Relationship> {
        self.backend.put_relationship(&relationship)?;
        Ok(relationship)
    }

    /// Outgoing relationships of an entity, with both endpoints.
    pub fn get_relationships(&self, entity_id: EntityId) -> Result<Vec<Triple>> {
        self.relationships_of(entity_id, Direction::Outgoing)
    }

    /// Relationships of an entity in the given direction, with both endpoints.
    pub fn relationships_of(&self, entity_id: EntityId, direction: Direction) -> Result<Vec<Triple>> {
        let relationships = self.backend.expand(&[entity_id], direction)?;
        self.join_endpoints(relationships)
    }

    /// Find relationships by query.
    pub fn find_relationships(&self, query: RelationshipQuery) -> Result<Vec<Triple>> {
        let relationships = self.backend.scan_relationships(&query)?;
        self.join_endpoints(relationships)
    }

    /// Get a relationship by ID, with both endpoints.
    pub fn get_relationship(&self, relationship_id: RelationshipId) -> Result<Option<Triple>> {
        let Some(relationship) = self.backend.get_relationship(relationship_id)? else {
            return Ok(None);
        };
        Ok(self.join_endpoints(vec![relationship])?.pop())
    }

    /// Delete a relationship.
    pub fn delete_relationship(&mut self, relationship_id: RelationshipId) -> Result<bool> {
        self.backend.delete_relationship(relationship_id)
    }

    /// Update a relationship's confidence.
    pub fn update_relationship(
        &mut self,
        relationship_id: RelationshipId,
        confidence: Option<f32>,
    ) -> Result<bool> {
        self.update_relationship_with(relationship_id, |r| {
            if let Some(confidence) = confidence {
                r.confidence = confidence;
            }
        })
    }

    /// Read-modify-write one relationship. Returns `false` if it does not exist.
    ///
    /// The closure must not change `subject_id` / `object_id`; use
    /// [`migrate_relationships`](Self::migrate_relationships) to re-point edges.
    pub fn update_relationship_with<F>(&mut self, relationship_id: RelationshipId, f: F) -> Result<bool>
    where
        F: FnOnce(&mut Relationship),
    {
        let Some(mut relationship) = self.backend.get_relationship(relationship_id)? else {
            return Ok(false);
        };
        let endpoints = (relationship.subject_id, relationship.object_id);
        f(&mut relationship);
        if (relationship.subject_id, relationship.object_id) != endpoints {
            return Err(Error::InvalidOperation(
                "relationship endpoints cannot be changed in place".to_string(),
            ));
        }
        self.backend.put_relationship(&relationship)?;
        Ok(true)
    }

    /// Get all relationships, with both endpoints.
    pub fn all_relationships(&self) -> Result<Vec<Triple>> {
        self.find_relationships(RelationshipQuery::default())
    }

    /// Re-point every relationship of `from_entity` at `to_entity`.
    ///
    /// Used when merging entities. A relationship that would duplicate one
    /// `to_entity` already has (same subject, predicate and object) is dropped
    /// instead. Decay state and provenance of migrated edges are preserved.
    ///
    /// Returns the number of relationships migrated.
    pub fn migrate_relationships(&mut self, from_entity: EntityId, to_entity: EntityId) -> Result<usize> {
        if self.backend.get_entity(to_entity)?.is_none() {
            return Err(Error::InvalidOperation(format!(
                "Migration target entity not found: {to_entity}"
            )));
        }
        if from_entity == to_entity {
            return Ok(0);
        }

        let moving = self.backend.expand(&[from_entity], Direction::Both)?;
        let mut existing: HashSet<(EntityId, String, EntityId)> = self
            .backend
            .expand(&[to_entity], Direction::Both)?
            .into_iter()
            .filter(|r| r.subject_id != from_entity && r.object_id != from_entity)
            .map(|r| (r.subject_id, r.predicate, r.object_id))
            .collect();

        let mut migrated = 0;
        for old in moving {
            self.backend.delete_relationship(old.id)?;
            let mut new = old;
            if new.subject_id == from_entity {
                new.subject_id = to_entity;
            }
            if new.object_id == from_entity {
                new.object_id = to_entity;
            }
            if !existing.insert((new.subject_id, new.predicate.clone(), new.object_id)) {
                continue;
            }
            new.id = RelationshipId::new_v4();
            self.backend.put_relationship(&new)?;
            migrated += 1;
        }
        Ok(migrated)
    }

    // ===== Traversal =====

    /// Entities reachable from `start` within `max_hops`, with their hop
    /// distance, nearest first. `start` itself is not included.
    ///
    /// This is breadth-first with one backend round trip per hop, so a
    /// database backend expands a whole frontier in a single query.
    pub fn neighborhood(
        &self,
        start: EntityId,
        max_hops: usize,
        direction: Direction,
    ) -> Result<Vec<(Entity, usize)>> {
        let mut distance: HashMap<EntityId, usize> = HashMap::from([(start, 0)]);
        let mut order: Vec<EntityId> = Vec::new();
        let mut frontier = vec![start];

        for hop in 1..=max_hops {
            if frontier.is_empty() {
                break;
            }
            let in_frontier: HashSet<EntityId> = frontier.iter().copied().collect();
            let mut next = Vec::new();
            for relationship in self.backend.expand(&frontier, direction)? {
                // An edge reaches the far endpoint only when followed in an
                // allowed direction from a node that is in the frontier.
                let forward = in_frontier.contains(&relationship.subject_id)
                    && direction != Direction::Incoming;
                let backward = in_frontier.contains(&relationship.object_id)
                    && direction != Direction::Outgoing;
                for (allowed, candidate) in [
                    (forward, relationship.object_id),
                    (backward, relationship.subject_id),
                ] {
                    if allowed && !distance.contains_key(&candidate) {
                        distance.insert(candidate, hop);
                        order.push(candidate);
                        next.push(candidate);
                    }
                }
            }
            frontier = next;
        }

        Ok(self
            .backend
            .get_entities(&order)?
            .into_iter()
            .map(|entity| {
                let hops = distance[&entity.id];
                (entity, hops)
            })
            .collect())
    }

    // ===== Statistics and decay helpers =====

    /// Get graph statistics.
    pub fn stats(&self) -> Result<GraphStats> {
        let mut entity_types: HashMap<String, usize> = HashMap::new();
        for entity in self.all_entities()? {
            *entity_types.entry(entity.entity_type).or_insert(0) += 1;
        }
        let mut predicates: HashMap<String, usize> = HashMap::new();
        for relationship in self.backend.scan_relationships(&RelationshipQuery::default())? {
            *predicates.entry(relationship.predicate).or_insert(0) += 1;
        }
        Ok(GraphStats {
            entity_count: self.backend.entity_count()?,
            relationship_count: self.backend.relationship_count()?,
            entity_types,
            predicates,
        })
    }

    /// Non-deprecated entities sorted by relevance (descending).
    pub fn entities_by_relevance(&self) -> Result<Vec<Entity>> {
        let mut entities: Vec<Entity> = self
            .all_entities()?
            .into_iter()
            .filter(|e| !e.is_deprecated)
            .collect();
        entities.sort_by(|a, b| b.relevance.total_cmp(&a.relevance));
        Ok(entities)
    }

    /// Non-deprecated entities below a relevance threshold.
    pub fn stale_entities(&self, threshold: f32) -> Result<Vec<Entity>> {
        Ok(self
            .all_entities()?
            .into_iter()
            .filter(|e| !e.is_deprecated && e.relevance < threshold)
            .collect())
    }

    /// Deprecated entities.
    pub fn deprecated_entities(&self) -> Result<Vec<Entity>> {
        Ok(self
            .all_entities()?
            .into_iter()
            .filter(|e| e.is_deprecated)
            .collect())
    }

    /// Count active entities (not deprecated).
    pub fn active_entity_count(&self) -> Result<usize> {
        Ok(self.all_entities()?.iter().filter(|e| !e.is_deprecated).count())
    }

    /// Apply a function to every entity and write them back in one batch.
    pub fn update_all_entities<F>(&mut self, mut f: F) -> Result<()>
    where
        F: FnMut(&mut Entity),
    {
        let mut entities = self.all_entities()?;
        entities.iter_mut().for_each(&mut f);
        self.backend.put_entities(&entities)
    }

    /// Apply a function to every relationship and write them back.
    pub fn update_all_relationships<F>(&mut self, mut f: F) -> Result<usize>
    where
        F: FnMut(&mut Relationship),
    {
        let mut relationships = self.backend.scan_relationships(&RelationshipQuery::default())?;
        for relationship in &mut relationships {
            f(relationship);
            self.backend.put_relationship(relationship)?;
        }
        Ok(relationships.len())
    }

    /// Average relevance of all entities (0.0 for an empty graph).
    pub fn average_relevance(&self) -> Result<f32> {
        let entities = self.all_entities()?;
        if entities.is_empty() {
            return Ok(0.0);
        }
        let total: f32 = entities.iter().map(|e| e.relevance).sum();
        Ok(total / entities.len() as f32)
    }

    /// Copy every entity and relationship into another graph (for example
    /// from the JSON backend into HelixDB). Returns `(entities, relationships)`.
    pub fn copy_into(&self, target: &mut KnowledgeGraph) -> Result<(usize, usize)> {
        let entities = self.all_entities()?;
        target.backend.put_entities(&entities)?;
        let sources: HashMap<EntityId, String> =
            self.backend.entity_embedding_sources()?.into_iter().collect();
        for (id, embedding) in self.backend.entity_embeddings()? {
            let source = sources.get(&id).map(String::as_str).unwrap_or_default();
            target.backend.set_entity_embedding(id, &embedding, source)?;
        }
        let relationships = self.backend.scan_relationships(&RelationshipQuery::default())?;
        for relationship in &relationships {
            target.backend.put_relationship(relationship)?;
        }
        target.backend.flush()?;
        Ok((entities.len(), relationships.len()))
    }

    /// Attach subject and object entities; relationships whose endpoints are
    /// gone are skipped.
    fn join_endpoints(&self, relationships: Vec<Relationship>) -> Result<Vec<Triple>> {
        let mut ids: Vec<EntityId> = relationships
            .iter()
            .flat_map(|r| [r.subject_id, r.object_id])
            .collect();
        ids.sort_unstable();
        ids.dedup();
        let entities: HashMap<EntityId, Entity> = self
            .backend
            .get_entities(&ids)?
            .into_iter()
            .map(|e| (e.id, e))
            .collect();
        Ok(relationships
            .into_iter()
            .filter_map(|r| {
                let subject = entities.get(&r.subject_id)?.clone();
                let object = entities.get(&r.object_id)?.clone();
                Some((r, subject, object))
            })
            .collect())
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

    #[test]
    fn json_backend_conforms() {
        let mut dirs = Vec::new();
        conformance::run_all(&mut || {
            let dir = TempDir::new().unwrap();
            let graph = KnowledgeGraph::new(dir.path()).unwrap();
            dirs.push(dir);
            graph
        });
    }

    #[test]
    fn json_backend_persists() {
        let dir = TempDir::new().unwrap();
        conformance::persistence(&mut || KnowledgeGraph::new(dir.path()).unwrap());
    }

    /// `graph.json` files written by MemSt 1.0 (with `node_index`, without
    /// edge decay fields) must keep loading.
    #[test]
    fn loads_v1_graph_file() {
        let dir = TempDir::new().unwrap();
        let (a, b, rel, session) = (
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
        );
        let entity = |id, name: &str| {
            serde_json::json!({
                "id": id, "name": name, "entity_type": "test", "attributes": {},
                "session_id": session, "source_message_id": null, "confidence": 1.0,
                "created_at": "2026-03-07T00:00:00Z", "access_count": 3
            })
        };
        let file = serde_json::json!({
            "entities": [entity(a, "A"), entity(b, "B")],
            "edges": [{
                "subject_id": a, "predicate": "knows", "object_id": b,
                "relationship_id": rel, "confidence": 0.5, "session_id": session,
                "source_message_id": null, "created_at": "2026-03-07T00:00:00Z"
            }],
            "node_index": [[a, 0], [b, 1]]
        });
        std::fs::write(dir.path().join("graph.json"), file.to_string()).unwrap();

        let graph = KnowledgeGraph::new(dir.path()).unwrap();
        assert_eq!(graph.backend_kind(), "json");
        assert_eq!(graph.get_entity(a).unwrap().unwrap().access_count, 3);
        let (stored, _, object) = graph.get_relationship(rel).unwrap().unwrap();
        assert_eq!(stored.relevance, 1.0);
        assert_eq!(object.name, "B");
    }
}
