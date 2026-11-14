//! Storage abstraction for the knowledge graph.
//!
//! [`KnowledgeGraph`](super::KnowledgeGraph) owns the *semantics* of the graph
//! (importance ordering, relationship migration, decay helpers). A
//! [`GraphBackend`] owns *storage and filtering*. Two implementations exist:
//!
//! - [`JsonGraphBackend`](super::JsonGraphBackend) — a single `graph.json`
//!   file held in memory. No dependencies, good for tests and small sessions.
//! - `memst_graph_helix::HelixGraphBackend` — an embedded
//!   [HelixDB](https://github.com/yfyang86/helix-db) database (graph + vector +
//!   BM25 in one transactional engine). Lives in its own crate so that the
//!   engine is only compiled when asked for.
//!
//! All values cross this boundary by value: a database-backed store cannot
//! hand out `&Entity` / `&mut Entity` into its pages, so mutation is always
//! "read, change, [`put_entity`](GraphBackend::put_entity)".

use crate::error::Result;
use crate::types::{Entity, EntityId, EntityQuery, Relationship, RelationshipId, RelationshipQuery};

/// Which edges to follow from an entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Edges where the entity is the subject.
    Outgoing,
    /// Edges where the entity is the object.
    Incoming,
    /// Both of the above.
    Both,
}

/// Storage backend for entities (nodes) and relationships (edges).
///
/// Contract shared by every implementation (and checked by
/// [`conformance`](super::conformance)):
///
/// - `put_*` is an upsert keyed on the value's `id`.
/// - `put_relationship` fails if either endpoint does not exist.
/// - `delete_entity` also removes every relationship touching the entity.
/// - `scan_*` apply the query's filters; `scan_relationships` also applies
///   `limit` (0 = unlimited). `scan_entities` ignores `limit` because the
///   caller ranks by importance before truncating.
/// - Scans return items in a stable order (insertion / creation order).
pub trait GraphBackend: Send {
    /// Short identifier for diagnostics, e.g. `"json"` or `"helix"`.
    fn kind(&self) -> &'static str;

    /// Insert or replace an entity.
    fn put_entity(&mut self, entity: &Entity) -> Result<()>;

    /// Insert or replace many entities. Backends should make this one write.
    fn put_entities(&mut self, entities: &[Entity]) -> Result<()> {
        entities.iter().try_for_each(|e| self.put_entity(e))
    }

    /// Fetch one entity.
    fn get_entity(&self, id: EntityId) -> Result<Option<Entity>>;

    /// Fetch several entities; missing ids are skipped.
    fn get_entities(&self, ids: &[EntityId]) -> Result<Vec<Entity>> {
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(entity) = self.get_entity(*id)? {
                out.push(entity);
            }
        }
        Ok(out)
    }

    /// Delete an entity and its relationships. Returns whether it existed.
    fn delete_entity(&mut self, id: EntityId) -> Result<bool>;

    /// Entities matching the query's type / name / confidence filters.
    fn scan_entities(&self, query: &EntityQuery) -> Result<Vec<Entity>>;

    /// Insert or replace a relationship.
    fn put_relationship(&mut self, relationship: &Relationship) -> Result<()>;

    /// Fetch one relationship.
    fn get_relationship(&self, id: RelationshipId) -> Result<Option<Relationship>>;

    /// Delete a relationship. Returns whether it existed.
    fn delete_relationship(&mut self, id: RelationshipId) -> Result<bool>;

    /// Relationships matching the query's filters, truncated to its limit.
    fn scan_relationships(&self, query: &RelationshipQuery) -> Result<Vec<Relationship>>;

    /// Relationships touching any entity in `frontier` in the given direction.
    ///
    /// This is the traversal primitive: one call per hop of a k-hop walk.
    fn expand(&self, frontier: &[EntityId], direction: Direction) -> Result<Vec<Relationship>>;

    /// Number of stored entities (including deprecated ones).
    fn entity_count(&self) -> Result<usize>;

    /// Number of stored relationships.
    fn relationship_count(&self) -> Result<usize>;

    /// Make everything written so far durable.
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Shared filter used by backends that scan in memory.
pub fn entity_matches(entity: &Entity, query: &EntityQuery) -> bool {
    if let Some(entity_type) = &query.entity_type {
        if entity.entity_type != *entity_type {
            return false;
        }
    }
    if let Some(needle) = &query.name_contains {
        if !entity.name.to_lowercase().contains(&needle.to_lowercase()) {
            return false;
        }
    }
    if let Some(min) = query.min_confidence {
        if entity.confidence < min {
            return false;
        }
    }
    true
}

/// Shared filter used by backends that scan in memory.
pub fn relationship_matches(relationship: &Relationship, query: &RelationshipQuery) -> bool {
    if let Some(subject) = query.subject_id {
        if relationship.subject_id != subject {
            return false;
        }
    }
    if let Some(object) = query.object_id {
        if relationship.object_id != object {
            return false;
        }
    }
    if let Some(predicate) = &query.predicate {
        if relationship.predicate != *predicate {
            return false;
        }
    }
    if let Some(min) = query.min_confidence {
        if relationship.confidence < min {
            return false;
        }
    }
    true
}
