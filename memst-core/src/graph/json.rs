//! File-backed graph backend: the whole graph in one `graph.json`.
//!
//! This is the original MemSt graph store. The graph is loaded into memory on
//! open and the file is rewritten (atomically) after every mutation, so it is
//! only appropriate for small per-session graphs. Use the HelixDB backend for
//! anything larger.

use super::backend::{entity_matches, relationship_matches, Direction, GraphBackend};
use crate::error::{Error, Result};
use crate::types::{Entity, EntityId, EntityQuery, Relationship, RelationshipId, RelationshipQuery};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// On-disk layout. Field names are kept from MemSt 1.0 so existing
/// `graph.json` files keep loading.
#[derive(Debug, Serialize, Deserialize)]
struct GraphFile {
    entities: Vec<Entity>,
    edges: Vec<EdgeRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EdgeRecord {
    subject_id: EntityId,
    predicate: String,
    object_id: EntityId,
    relationship_id: RelationshipId,
    confidence: f32,
    session_id: Uuid,
    source_message_id: Option<Uuid>,
    created_at: chrono::DateTime<chrono::Utc>,
    #[serde(default = "default_relevance")]
    relevance: f32,
    #[serde(default = "default_half_life_days")]
    half_life_days: f32,
    #[serde(default)]
    is_stable: bool,
    #[serde(default = "chrono::Utc::now")]
    last_decay_at: chrono::DateTime<chrono::Utc>,
}

fn default_relevance() -> f32 {
    1.0
}
fn default_half_life_days() -> f32 {
    30.0
}

impl From<&Relationship> for EdgeRecord {
    fn from(r: &Relationship) -> Self {
        Self {
            subject_id: r.subject_id,
            predicate: r.predicate.clone(),
            object_id: r.object_id,
            relationship_id: r.id,
            confidence: r.confidence,
            session_id: r.session_id,
            source_message_id: r.source_message_id,
            created_at: r.created_at,
            relevance: r.relevance,
            half_life_days: r.half_life_days,
            is_stable: r.is_stable,
            last_decay_at: r.last_decay_at,
        }
    }
}

impl From<EdgeRecord> for Relationship {
    fn from(e: EdgeRecord) -> Self {
        Self {
            id: e.relationship_id,
            subject_id: e.subject_id,
            predicate: e.predicate,
            object_id: e.object_id,
            confidence: e.confidence,
            session_id: e.session_id,
            source_message_id: e.source_message_id,
            created_at: e.created_at,
            relevance: e.relevance,
            half_life_days: e.half_life_days,
            is_stable: e.is_stable,
            last_decay_at: e.last_decay_at,
        }
    }
}

/// Graph stored as a single JSON file.
pub struct JsonGraphBackend {
    path: PathBuf,
    entities: IndexMap<EntityId, Entity>,
    relationships: IndexMap<RelationshipId, Relationship>,
}

impl JsonGraphBackend {
    /// File name used inside a graph directory.
    pub const FILE_NAME: &'static str = "graph.json";

    /// Open (or create) `<dir>/graph.json`.
    pub fn open(dir: &Path) -> Result<Self> {
        fs::create_dir_all(dir)?;
        let path = dir.join(Self::FILE_NAME);
        let mut backend = Self {
            path,
            entities: IndexMap::new(),
            relationships: IndexMap::new(),
        };
        if backend.path.exists() {
            let file: GraphFile = serde_json::from_reader(BufReader::new(File::open(&backend.path)?))?;
            backend.entities = file.entities.into_iter().map(|e| (e.id, e)).collect();
            backend.relationships = file
                .edges
                .into_iter()
                .map(Relationship::from)
                .map(|r| (r.id, r))
                .collect();
        }
        Ok(backend)
    }

    /// Whether a graph file already exists in `dir`.
    pub fn exists(dir: &Path) -> bool {
        dir.join(Self::FILE_NAME).exists()
    }

    /// Write to a sibling temp file and rename, so a crash never leaves a
    /// truncated graph behind.
    fn save(&self) -> Result<()> {
        let file = GraphFile {
            entities: self.entities.values().cloned().collect(),
            edges: self.relationships.values().map(EdgeRecord::from).collect(),
        };
        let tmp = self.path.with_extension("json.tmp");
        {
            let mut writer = BufWriter::new(File::create(&tmp)?);
            serde_json::to_writer_pretty(&mut writer, &file)?;
            writer.write_all(b"\n")?;
            writer.flush()?;
        }
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

impl GraphBackend for JsonGraphBackend {
    fn kind(&self) -> &'static str {
        "json"
    }

    fn put_entity(&mut self, entity: &Entity) -> Result<()> {
        self.entities.insert(entity.id, entity.clone());
        self.save()
    }

    fn put_entities(&mut self, entities: &[Entity]) -> Result<()> {
        for entity in entities {
            self.entities.insert(entity.id, entity.clone());
        }
        self.save()
    }

    fn get_entity(&self, id: EntityId) -> Result<Option<Entity>> {
        Ok(self.entities.get(&id).cloned())
    }

    fn delete_entity(&mut self, id: EntityId) -> Result<bool> {
        if self.entities.shift_remove(&id).is_none() {
            return Ok(false);
        }
        self.relationships
            .retain(|_, r| r.subject_id != id && r.object_id != id);
        self.save()?;
        Ok(true)
    }

    fn scan_entities(&self, query: &EntityQuery) -> Result<Vec<Entity>> {
        Ok(self
            .entities
            .values()
            .filter(|e| entity_matches(e, query))
            .cloned()
            .collect())
    }

    fn put_relationship(&mut self, relationship: &Relationship) -> Result<()> {
        for (role, id) in [
            ("Subject", relationship.subject_id),
            ("Object", relationship.object_id),
        ] {
            if !self.entities.contains_key(&id) {
                return Err(Error::InvalidOperation(format!(
                    "{role} entity not found: {id}"
                )));
            }
        }
        self.relationships
            .insert(relationship.id, relationship.clone());
        self.save()
    }

    fn get_relationship(&self, id: RelationshipId) -> Result<Option<Relationship>> {
        Ok(self.relationships.get(&id).cloned())
    }

    fn delete_relationship(&mut self, id: RelationshipId) -> Result<bool> {
        if self.relationships.shift_remove(&id).is_none() {
            return Ok(false);
        }
        self.save()?;
        Ok(true)
    }

    fn scan_relationships(&self, query: &RelationshipQuery) -> Result<Vec<Relationship>> {
        let limit = if query.limit > 0 { query.limit } else { usize::MAX };
        Ok(self
            .relationships
            .values()
            .filter(|r| relationship_matches(r, query))
            .take(limit)
            .cloned()
            .collect())
    }

    fn expand(&self, frontier: &[EntityId], direction: Direction) -> Result<Vec<Relationship>> {
        let frontier: std::collections::HashSet<&EntityId> = frontier.iter().collect();
        Ok(self
            .relationships
            .values()
            .filter(|r| match direction {
                Direction::Outgoing => frontier.contains(&r.subject_id),
                Direction::Incoming => frontier.contains(&r.object_id),
                Direction::Both => {
                    frontier.contains(&r.subject_id) || frontier.contains(&r.object_id)
                }
            })
            .cloned()
            .collect())
    }

    fn entity_count(&self) -> Result<usize> {
        Ok(self.entities.len())
    }

    fn relationship_count(&self) -> Result<usize> {
        Ok(self.relationships.len())
    }
}
