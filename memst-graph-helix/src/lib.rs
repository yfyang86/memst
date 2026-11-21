//! HelixDB backend for the MemSt knowledge graph.
//!
//! [`HelixGraphBackend`] implements [`memst_core::graph::GraphBackend`] on an
//! **embedded** [HelixDB](https://github.com/yfyang86/helix-db) database: the
//! engine runs inside the MemSt process and stores its data in a directory,
//! so there is no server to deploy.
//!
//! ```no_run
//! use memst_core::types::Entity;
//! # fn main() -> memst_core::error::Result<()> {
//! let mut graph = memst_graph_helix::open(
//!     std::path::Path::new("./memst-store/sessions/<id>"),
//!     memst_graph_helix::BackendChoice::Helix,
//! )?;
//! graph.add_entity(Entity::new("Rust", "technology", uuid::Uuid::new_v4()))?;
//! # Ok(())
//! # }
//! ```
//!
//! # Data model
//!
//! | MemSt | HelixDB |
//! |-------|---------|
//! | `Entity` | node with label `Entity`; every field is a typed property |
//! | `Entity::id` | property `uid`, covered by a unique equality index |
//! | `Entity::attributes` | property `attributes`, a JSON string (HelixDB stores scalars and flat lists only) |
//! | `Relationship` | edge of type `REL` from subject to object |
//! | `Relationship::predicate` | edge property `predicate` (free text, so it is data, never part of a query) |
//!
//! Queries are parameterised Cypher run through HelixDB's embedded Cypher
//! engine; filters and one-hop expansions execute inside the database.
//!
//! # Concurrency
//!
//! HelixDB allows **one writer per database directory**. Opening the same
//! directory twice for writing fences the first handle.
//!
//! The [`GraphBackend`] trait is synchronous while HelixDB is async, so the
//! backend owns a small Tokio runtime and blocks the calling thread until each
//! request finishes. Calling it from inside another async runtime is allowed
//! but blocks that runtime's worker thread; wrap calls in `spawn_blocking`
//! there.

// Proving that HelixDB's query futures are `Send` walks deeply nested types;
// newer compilers (1.100) exceed the default limit of 128 doing so.
#![recursion_limit = "512"]

mod codec;
mod store;

pub use store::{
    migrate_store, open_store, unmigrated_sessions, GraphLocation, MigrationReport, StoreGraphOptions,
    MIGRATED_MARKER,
};

use helix_db::dsl::prelude::{g, read_batch, write_batch, IndexSpec, QueryRequest, VectorDistanceMetric};
use helix_db::{Client, HelixDbSource, HelixError};
use memst_core::error::{Error, Result};
use memst_core::graph::{rrf_fuse, Direction, GraphBackend, JsonGraphBackend, KnowledgeGraph};
use memst_core::types::{Entity, EntityId, EntityQuery, Relationship, RelationshipId, RelationshipQuery};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};
use std::future::Future;
use std::path::{Path, PathBuf};

/// Directory (inside a graph directory) that holds the HelixDB database.
pub const HELIX_DIR: &str = "graph.helix";
/// Lock file (next to [`HELIX_DIR`]) held by the process writing the graph.
pub const LOCK_FILE: &str = "graph.helix.lock";

const NODE_LABEL: &str = "Entity";
const EDGE_TYPE: &str = "REL";
/// Property holding an entity's owner; also the tenant key of the search
/// indexes.
const OWNER_PROPERTY: &str = "owner";
/// Property covered by the BM25 index (see `Entity::search_text`).
const TEXT_PROPERTY: &str = "search_text";
/// Longest time to wait for a freshly requested index to activate.
const INDEX_WAIT: std::time::Duration = std::time::Duration::from_secs(10);
/// Property covered by the vector index.
const VECTOR_PROPERTY: &str = "embedding";
/// Rows per request when reading or writing in bulk. Keeps every request
/// far below HelixDB's 16 MiB response and 200,000-value parameter limits.
const PAGE: usize = 2_000;

/// Owner used when none is given.
pub const DEFAULT_OWNER: &str = "default";

/// Options for [`HelixGraphBackend::open_with`].
#[derive(Debug, Clone)]
pub struct HelixGraphOptions {
    /// Let the embedded engine send HelixDB's anonymous usage telemetry.
    ///
    /// **Off by default**: a memory store should not phone home. When this is
    /// `false` and neither `HELIX_TELEMETRY_LEVEL` nor `DO_NOT_TRACK` is set
    /// in the environment, the backend sets `HELIX_TELEMETRY_LEVEL=off` for
    /// this process before opening the database.
    pub telemetry: bool,

    /// Whose graph this handle sees.
    ///
    /// One database holds every owner's graph. Each entity is stamped with
    /// its owner, every read and write is restricted to this owner, and the
    /// search indexes are partitioned by it, so a handle can never read or
    /// change another owner's data. `None` turns the restriction off; it
    /// exists only to read databases written before owners were recorded
    /// (see [`migrate_store`]) and has no native search.
    pub owner: Option<String>,

    /// Open without taking the writer lock. Every write fails; reads see
    /// what the writer has made durable.
    pub read_only: bool,
}

impl Default for HelixGraphOptions {
    fn default() -> Self {
        Self {
            telemetry: false,
            owner: Some(DEFAULT_OWNER.to_string()),
            read_only: false,
        }
    }
}

impl HelixGraphOptions {
    /// Options for the given owner.
    pub fn for_owner(owner: impl Into<String>) -> Self {
        Self {
            owner: Some(owner.into()),
            ..Self::default()
        }
    }

    /// The same options, read-only.
    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }
}

/// Knowledge graph stored in an embedded HelixDB database.
pub struct HelixGraphBackend {
    // `client` is declared (and therefore dropped) before the runtime it
    // was opened on; `Drop` closes it explicitly first.
    client: Client,
    runtime: Option<tokio::runtime::Runtime>,
    root: PathBuf,
    /// Dimension the vector index is known to exist for (set on first use).
    vector_dimension: std::sync::Mutex<Option<usize>>,
    owner: Option<String>,
    /// Held for the lifetime of a writer; `None` for a read-only handle.
    writer_lock: Option<std::fs::File>,
}

impl HelixGraphBackend {
    /// Open (or create) the database at `<dir>/graph.helix`.
    pub fn open(dir: &Path) -> Result<Self> {
        Self::open_with(dir, HelixGraphOptions::default())
    }

    /// Open (or create) the database at `<dir>/graph.helix` with options.
    pub fn open_with(dir: &Path, options: HelixGraphOptions) -> Result<Self> {
        if !options.telemetry
            && std::env::var_os("HELIX_TELEMETRY_LEVEL").is_none()
            && std::env::var_os("DO_NOT_TRACK").is_none()
        {
            std::env::set_var("HELIX_TELEMETRY_LEVEL", "off");
        }

        if options.owner.as_deref() == Some("") {
            return Err(Error::InvalidOperation("graph owner must not be empty".to_string()));
        }
        let root = dir.join(HELIX_DIR);
        if options.read_only && !root.is_dir() {
            return Err(Error::InvalidOperation(format!(
                "no HelixDB graph to read in {}",
                dir.display()
            )));
        }
        std::fs::create_dir_all(&root)?;
        let writer_lock = if options.read_only {
            None
        } else {
            Some(acquire_writer_lock(dir)?)
        };

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("memst-helix")
            .enable_all()
            .build()?;

        let source = HelixDbSource::Disk {
            root: root.clone(),
            database: "kg".to_string(),
        };
        let client = if options.read_only {
            block_on(&runtime, Client::open_reader(source))
        } else {
            block_on(&runtime, Client::open(source))
        }
        .map_err(helix_error)?;

        let backend = Self {
            client,
            runtime: Some(runtime),
            root,
            vector_dimension: std::sync::Mutex::new(None),
            owner: options.owner,
            writer_lock,
        };
        if backend.writer_lock.is_some() {
            backend.ensure_indexes()?;
        }
        Ok(backend)
    }

    /// Whether a HelixDB graph already exists in `dir`.
    pub fn exists(dir: &Path) -> bool {
        dir.join(HELIX_DIR).is_dir()
    }

    /// The owner this handle is restricted to, if any.
    pub fn owner(&self) -> Option<&str> {
        self.owner.as_deref()
    }

    /// Whether this handle was opened read-only.
    pub fn is_read_only(&self) -> bool {
        self.writer_lock.is_none()
    }

    fn writable(&self) -> Result<()> {
        if self.is_read_only() {
            return Err(Error::InvalidOperation(
                "the graph is open read-only".to_string(),
            ));
        }
        Ok(())
    }

    /// Value that selects this owner's partition of a search index.
    fn tenant(&self) -> Result<helix_db::dsl::prelude::PropertyValue> {
        self.owner
            .clone()
            .map(helix_db::dsl::prelude::PropertyValue::String)
            .ok_or_else(|| Error::GraphBackend("helix: an unscoped handle has no native search".to_string()))
    }

    /// Filesystem location of the database.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The embedded HelixDB client, for features this backend does not wrap
    /// (vector search, BM25, shortest paths). Drive it with [`Self::block_on`].
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Run a HelixDB future to completion on the backend's runtime.
    pub fn block_on<F>(&self, future: F) -> F::Output
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        block_on(self.runtime.as_ref().expect("runtime lives until drop"), future)
    }

    /// Idempotent. `uid` gets a unique index: it makes point lookups and
    /// `UNWIND`-driven joins index probes, and is what turns a racing double
    /// insert into an error instead of a duplicate node.
    fn ensure_indexes(&self) -> Result<()> {
        for spec in [
            IndexSpec::node_unique_equality(NODE_LABEL, "uid"),
            IndexSpec::node_equality(NODE_LABEL, "entity_type"),
            IndexSpec::node_equality(NODE_LABEL, OWNER_PROPERTY),
            IndexSpec::node_equality(NODE_LABEL, "resolve_key"),
        ] {
            self.create_index(spec)?;
        }
        let Ok(tenant) = self.tenant() else {
            return Ok(());
        };
        // BM25 keyword search, partitioned by owner. The vector index is
        // created on the first `set_entity_embedding`, when the dimension is
        // known.
        self.create_index(IndexSpec::node_text(NODE_LABEL, TEXT_PROPERTY, Some(OWNER_PROPERTY)))?;
        self.create_index(IndexSpec::edge_text(EDGE_TYPE, TEXT_PROPERTY, Some(OWNER_PROPERTY)))?;
        self.wait_until_searchable(move || {
            let probe = g()
                .text_search_nodes(NODE_LABEL, TEXT_PROPERTY, "memst", 1, Some(tenant.clone()))
                .value_map(Some(vec!["uid"]));
            QueryRequest::read(read_batch().var_as("probe", probe).returning(["probe"]))
        });
        Ok(())
    }

    /// Wait, for a bounded time, until a search index answers.
    ///
    /// HelixDB builds indexes in the background. A short-lived process (one
    /// CLI command) would otherwise exit before the build activates and every
    /// search would fall back. On an existing database this is one cheap
    /// query. If the index is still not ready after the limit (a large
    /// backfill), give up quietly: searches fall back until it is.
    fn wait_until_searchable(&self, probe: impl Fn() -> QueryRequest) {
        let deadline = std::time::Instant::now() + INDEX_WAIT;
        loop {
            let request = probe();
            let client = self.client.clone();
            let outcome = self.block_on(async move { client.query::<Value>(request).send().await });
            let building = matches!(&outcome, Err(error) if error.error_code() == Some("index_not_found"));
            if !building || std::time::Instant::now() >= deadline {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
    }

    /// Request an index. Creation is accepted immediately and the index is
    /// built in the background; until it activates, searching it fails with
    /// `index_not_found`, which `KnowledgeGraph` answers with its fallback.
    fn create_index(&self, spec: IndexSpec) -> Result<()> {
        let request = QueryRequest::write(
            write_batch()
                .var_as("index", g().create_index_if_not_exists(spec))
                .returning(["index"]),
        );
        let client = self.client.clone();
        self.block_on(async move { client.query::<Value>(request).send().await })
            .map(|_| ())
            .map_err(helix_error)
    }

    fn ensure_vector_index(&self, dimension: usize) -> Result<()> {
        let tenant = self.tenant()?;
        let mut known = self.vector_dimension.lock().expect("vector dimension lock");
        if *known == Some(dimension) {
            return Ok(());
        }
        let Some(non_zero) = std::num::NonZeroUsize::new(dimension) else {
            return Err(Error::InvalidOperation("embedding is empty".to_string()));
        };
        // If an index of another dimension already exists this is a no-op,
        // and the write that follows fails with `invalid_vector_dimension`.
        self.create_index(IndexSpec::node_vector(
            NODE_LABEL,
            VECTOR_PROPERTY,
            non_zero,
            VectorDistanceMetric::Cosine,
            Some(OWNER_PROPERTY),
        ))?;
        *known = Some(dimension);
        drop(known);
        self.wait_until_searchable(move || {
            let mut unit = vec![0.0; dimension];
            unit[0] = 1.0;
            let probe = g()
                .vector_search_nodes(NODE_LABEL, VECTOR_PROPERTY, unit, 1, Some(tenant.clone()))
                .value_map(Some(vec!["uid"]));
            QueryRequest::read(read_batch().var_as("probe", probe).returning(["probe"]))
        });
        Ok(())
    }

    /// Run one Cypher statement and return its rows.
    fn cypher(&self, query: &'static str, parameters: Value) -> Result<Vec<Vec<Value>>> {
        self.cypher_owned(query.to_string(), parameters)
    }

    fn cypher_owned(&self, query: String, parameters: Value) -> Result<Vec<Vec<Value>>> {
        let Value::Object(parameters) = parameters else {
            unreachable!("cypher parameters are always built with json!({{...}})");
        };
        let mut parameters: BTreeMap<String, Value> = parameters.into_iter().collect();
        // Statements mark where the owner restriction goes: `%O` inside a
        // property map that has other entries, `%N` on a node without one.
        // The owner is always a parameter, never spliced into the text.
        let query = match &self.owner {
            Some(owner) => {
                let query = query.replace("%O", "owner: $owner, ").replace("%N", " {owner: $owner}");
                if query.contains("$owner") {
                    parameters.insert("owner".to_string(), json!(owner));
                }
                query
            }
            None => query.replace("%O", "").replace("%N", ""),
        };
        let client = self.client.clone();
        self.block_on(async move { client.cypher(&query, parameters, None).await })
            .map(|response| response.rows)
            .map_err(helix_error)
    }

    /// First column of the first row as a count.
    fn count(&self, query: &'static str, parameters: Value) -> Result<usize> {
        let rows = self.cypher(query, parameters)?;
        Ok(rows
            .first()
            .and_then(|row| row.first())
            .and_then(codec::as_u64)
            .unwrap_or(0) as usize)
    }

    fn existing_entity_uids(&self, uids: &[String]) -> Result<HashSet<String>> {
        let rows = self.cypher(
            "UNWIND $uids AS k MATCH (e:Entity {%Ouid: k}) RETURN e.uid AS uid",
            json!({ "uids": uids }),
        )?;
        Ok(rows
            .into_iter()
            .filter_map(|mut row| match row.pop() {
                Some(Value::String(uid)) => Some(uid),
                _ => None,
            })
            .collect())
    }

    fn edges_from(&self, query: &'static str, uids: &[String]) -> Result<Vec<Relationship>> {
        let mut out = Vec::new();
        for chunk in uids.chunks(PAGE) {
            for row in self.cypher(query, json!({ "uids": chunk }))? {
                out.push(codec::relationship_from_row(row)?);
            }
        }
        Ok(out)
    }
}

impl GraphBackend for HelixGraphBackend {
    fn kind(&self) -> &'static str {
        "helix"
    }

    fn put_entity(&mut self, entity: &Entity) -> Result<()> {
        self.put_entities(std::slice::from_ref(entity))
    }

    fn put_entities(&mut self, entities: &[Entity]) -> Result<()> {
        self.writable()?;
        for chunk in entities.chunks(PAGE) {
            // Later duplicates of an id within one batch win, as with a map.
            let mut rows: BTreeMap<String, Value> = BTreeMap::new();
            for entity in chunk {
                let uid = entity.id.to_string();
                let mut props = codec::entity_props(entity);
                if let Some(owner) = &self.owner {
                    props[OWNER_PROPERTY] = json!(owner);
                }
                rows.insert(uid.clone(), json!({ "uid": uid, "props": props }));
            }
            let uids: Vec<String> = rows.keys().cloned().collect();
            let existing = self.existing_entity_uids(&uids)?;
            let (update, create): (Vec<Value>, Vec<Value>) = {
                let (update, create): (Vec<_>, Vec<_>) =
                    rows.into_iter().partition(|(uid, _)| existing.contains(uid));
                let values = |rows: Vec<(String, Value)>| rows.into_iter().map(|(_, row)| row).collect();
                (values(update), values(create))
            };

            if !update.is_empty() {
                // `WITH row.uid AS k` matters: the planner probes the uid
                // index for a bare variable (`{uid: k}`) but falls back to a
                // label scan per row for a field of an UNWIND element
                // (`{uid: row.uid}`), which is quadratic. Measured on 1,000
                // rows: 0.8 s with the WITH, >30 s (deadline) without.
                // `+=` rather than `=`: replacing every property would also
                // drop the entity's embedding, which is not part of `props`.
                // Cleared optional fields still disappear, because `props`
                // carries them as explicit nulls.
                self.cypher(
                    "UNWIND $rows AS row WITH row.uid AS k, row.props AS props MATCH (e:Entity {%Ouid: k}) SET e += props RETURN count(e) AS n",
                    json!({ "rows": update }),
                )?;
            }
            if !create.is_empty() {
                self.cypher(
                    "UNWIND $rows AS row CREATE (e:Entity {uid: row.uid}) SET e += row.props RETURN count(e) AS n",
                    json!({ "rows": create }),
                )?;
            }
        }
        Ok(())
    }

    fn get_entity(&self, id: EntityId) -> Result<Option<Entity>> {
        Ok(self.get_entities(&[id])?.pop())
    }

    fn get_entities(&self, ids: &[EntityId]) -> Result<Vec<Entity>> {
        let uids: Vec<String> = ids.iter().map(ToString::to_string).collect();
        let mut out = Vec::with_capacity(ids.len());
        for chunk in uids.chunks(PAGE) {
            let rows = self.cypher(
                "UNWIND $uids AS k MATCH (e:Entity {%Ouid: k}) RETURN properties(e) AS e",
                json!({ "uids": chunk }),
            )?;
            for row in rows {
                out.push(codec::entity_from_row(row)?);
            }
        }
        Ok(out)
    }

    fn delete_entity(&mut self, id: EntityId) -> Result<bool> {
        self.writable()?;
        let deleted = self.count(
            "MATCH (e:Entity {%Ouid: $uid}) DETACH DELETE e RETURN count(e) AS n",
            json!({ "uid": id.to_string() }),
        )?;
        Ok(deleted > 0)
    }

    fn scan_entities(&self, query: &EntityQuery) -> Result<Vec<Entity>> {
        let mut conditions = Vec::new();
        let mut parameters = serde_json::Map::new();
        if let Some(entity_type) = &query.entity_type {
            conditions.push("e.entity_type = $entity_type");
            parameters.insert("entity_type".into(), json!(entity_type));
        }
        if let Some(needle) = &query.name_contains {
            conditions.push("e.name_lc CONTAINS $needle");
            parameters.insert("needle".into(), json!(needle.to_lowercase()));
        }
        if let Some(min) = query.min_confidence {
            conditions.push("e.confidence >= $min_confidence");
            parameters.insert("min_confidence".into(), codec::float(min));
        }
        if let Some(session) = query.session_id {
            conditions.push("$session IN e.sessions");
            parameters.insert("session".into(), json!(session.to_string()));
        }
        let filter = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {} ", conditions.join(" AND "))
        };
        // Only fixed fragments are interpolated; every value is a parameter.
        let statement = format!(
            "MATCH (e:Entity%N) {filter}RETURN properties(e) AS e ORDER BY e.created_at, e.uid SKIP $skip LIMIT $limit"
        );

        let mut out = Vec::new();
        loop {
            parameters.insert("skip".into(), json!(out.len()));
            parameters.insert("limit".into(), json!(PAGE));
            let rows = self.cypher_owned(statement.clone(), Value::Object(parameters.clone()))?;
            let page = rows.len();
            for row in rows {
                out.push(codec::entity_from_row(row)?);
            }
            if page < PAGE {
                return Ok(out);
            }
        }
    }

    fn find_entity_by_name(&self, name: &str, entity_type: &str) -> Result<Option<Entity>> {
        let rows = self.cypher(
            "MATCH (e:Entity {%Oresolve_key: $key}) WHERE e.is_deprecated = false RETURN properties(e) AS e",
            json!({ "key": codec::resolve_key(name, entity_type) }),
        )?;
        let mut best: Option<Entity> = None;
        for row in rows {
            let entity = codec::entity_from_row(row)?;
            if best.as_ref().map_or(true, |b| entity.importance() > b.importance()) {
                best = Some(entity);
            }
        }
        Ok(best)
    }

    fn put_relationship(&mut self, relationship: &Relationship) -> Result<()> {
        self.writable()?;
        let uid = relationship.id.to_string();
        let (subject, object) = (
            relationship.subject_id.to_string(),
            relationship.object_id.to_string(),
        );
        // The edge is indexed under the names of its endpoints as they are
        // now; renaming an entity later does not re-index its edges.
        let names: std::collections::HashMap<EntityId, Entity> = self
            .get_entities(&[relationship.subject_id, relationship.object_id])?
            .into_iter()
            .map(|e| (e.id, e))
            .collect();
        let search_text = match (names.get(&relationship.subject_id), names.get(&relationship.object_id)) {
            (Some(subject_entity), Some(object_entity)) => {
                memst_core::graph::relationship_text(subject_entity, &relationship.predicate, object_entity)
            }
            _ => relationship.predicate.clone(),
        };
        let mut props = codec::relationship_props(relationship, &search_text);
        if let Some(owner) = &self.owner {
            props[OWNER_PROPERTY] = json!(owner);
        }

        // An edge cannot be re-pointed, so an existing edge is updated in
        // place only when its endpoints are unchanged.
        if let Some(existing) = self.get_relationship(relationship.id)? {
            if existing.subject_id == relationship.subject_id
                && existing.object_id == relationship.object_id
            {
                self.cypher(
                    "MATCH (:Entity {%Ouid: $s})-[r:REL]->(:Entity {uid: $o}) WHERE r.uid = $uid SET r = $props RETURN count(r) AS n",
                    json!({ "s": subject, "o": object, "uid": uid, "props": props }),
                )?;
                return Ok(());
            }
            self.delete_relationship(relationship.id)?;
        }

        let created = self.count(
            "MATCH (a:Entity {%Ouid: $s}), (b:Entity {%Ouid: $o}) CREATE (a)-[r:REL]->(b) SET r += $props RETURN count(r) AS n",
            json!({ "s": subject, "o": object, "props": props }),
        )?;
        if created > 0 {
            return Ok(());
        }
        // Nothing matched: say which endpoint is missing.
        let present = self.existing_entity_uids(&[subject.clone(), object.clone()])?;
        let (role, missing) = if present.contains(&subject) {
            ("Object", object)
        } else {
            ("Subject", subject)
        };
        Err(Error::InvalidOperation(format!("{role} entity not found: {missing}")))
    }

    fn get_relationship(&self, id: RelationshipId) -> Result<Option<Relationship>> {
        self.cypher(
            "MATCH (:Entity%N)-[r:REL]->(:Entity) WHERE r.uid = $uid RETURN properties(r) AS r",
            json!({ "uid": id.to_string() }),
        )?
        .pop()
        .map(codec::relationship_from_row)
        .transpose()
    }

    fn get_relationships(&self, ids: &[RelationshipId]) -> Result<Vec<Relationship>> {
        let uids: Vec<String> = ids.iter().map(ToString::to_string).collect();
        let mut out = Vec::with_capacity(ids.len());
        for chunk in uids.chunks(PAGE) {
            let rows = self.cypher(
                "MATCH (:Entity%N)-[r:REL]->(:Entity) WHERE r.uid IN $uids RETURN properties(r) AS r",
                json!({ "uids": chunk }),
            )?;
            for row in rows {
                out.push(codec::relationship_from_row(row)?);
            }
        }
        Ok(out)
    }

    fn delete_relationship(&mut self, id: RelationshipId) -> Result<bool> {
        self.writable()?;
        let deleted = self.count(
            "MATCH (:Entity%N)-[r:REL]->(:Entity) WHERE r.uid = $uid DELETE r RETURN count(r) AS n",
            json!({ "uid": id.to_string() }),
        )?;
        Ok(deleted > 0)
    }

    fn scan_relationships(&self, query: &RelationshipQuery) -> Result<Vec<Relationship>> {
        let mut parameters = serde_json::Map::new();
        // Endpoint filters go into the pattern so they use the uid index.
        let subject = match query.subject_id {
            Some(id) => {
                parameters.insert("s".into(), json!(id.to_string()));
                "(:Entity {%Ouid: $s})"
            }
            None => "(:Entity%N)",
        };
        let object = match query.object_id {
            Some(id) => {
                parameters.insert("o".into(), json!(id.to_string()));
                "(:Entity {uid: $o})"
            }
            None => "(:Entity)",
        };
        let mut conditions = Vec::new();
        if let Some(predicate) = &query.predicate {
            conditions.push("r.predicate = $predicate");
            parameters.insert("predicate".into(), json!(predicate));
        }
        if let Some(min) = query.min_confidence {
            conditions.push("r.confidence >= $min_confidence");
            parameters.insert("min_confidence".into(), codec::float(min));
        }
        if let Some(session) = query.session_id {
            conditions.push("r.session_id = $session");
            parameters.insert("session".into(), json!(session.to_string()));
        }
        let filter = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {} ", conditions.join(" AND "))
        };
        let statement = format!(
            "MATCH {subject}-[r:{EDGE_TYPE}]->{object} {filter}RETURN properties(r) AS r ORDER BY r.created_at, r.uid SKIP $skip LIMIT $limit"
        );

        let wanted = if query.limit > 0 { query.limit } else { usize::MAX };
        let mut out = Vec::new();
        loop {
            let page_size = PAGE.min(wanted - out.len());
            parameters.insert("skip".into(), json!(out.len()));
            parameters.insert("limit".into(), json!(page_size));
            let rows = self.cypher_owned(statement.clone(), Value::Object(parameters.clone()))?;
            let page = rows.len();
            for row in rows {
                out.push(codec::relationship_from_row(row)?);
            }
            if page < page_size || out.len() >= wanted {
                return Ok(out);
            }
        }
    }

    fn expand(&self, frontier: &[EntityId], direction: Direction) -> Result<Vec<Relationship>> {
        const OUT: &str =
            "UNWIND $uids AS k MATCH (:Entity {%Ouid: k})-[r:REL]->(:Entity) RETURN properties(r) AS r";
        const IN: &str =
            "UNWIND $uids AS k MATCH (:Entity {%Ouid: k})<-[r:REL]-(:Entity) RETURN properties(r) AS r";

        let mut uids: Vec<String> = frontier.iter().map(ToString::to_string).collect();
        uids.sort_unstable();
        uids.dedup();

        let mut out = Vec::new();
        if direction != Direction::Incoming {
            out.extend(self.edges_from(OUT, &uids)?);
        }
        if direction != Direction::Outgoing {
            out.extend(self.edges_from(IN, &uids)?);
        }
        // An edge whose two ends are both in the frontier is found twice.
        let mut seen = HashSet::new();
        out.retain(|r| seen.insert(r.id));
        Ok(out)
    }

    fn set_entity_embedding(&mut self, id: EntityId, embedding: &[f32], source: &str) -> Result<bool> {
        self.writable()?;
        self.ensure_vector_index(embedding.len())?;
        let updated = self.count(
            "MATCH (e:Entity {%Ouid: $uid}) SET e += {embedding: $embedding, embedding_source: $source} RETURN count(e) AS n",
            json!({ "uid": id.to_string(), "embedding": embedding, "source": source }),
        )?;
        Ok(updated > 0)
    }

    fn entity_embedding_sources(&self) -> Result<Vec<(EntityId, String)>> {
        let mut out = Vec::new();
        loop {
            let rows = self.cypher(
                "MATCH (e:Entity%N) WHERE e.embedding IS NOT NULL RETURN e.uid AS uid, e.embedding_source AS source ORDER BY uid SKIP $skip LIMIT $limit",
                json!({ "skip": out.len(), "limit": PAGE }),
            )?;
            let page = rows.len();
            for row in rows {
                let [Value::String(uid), source] = row.as_slice() else {
                    return Err(Error::Corruption("helix embedding source row".to_string()));
                };
                let id = uuid::Uuid::parse_str(uid)
                    .map_err(|e| Error::Corruption(format!("helix entity uid: {e}")))?;
                out.push((id, source.as_str().unwrap_or_default().to_string()));
            }
            if page < PAGE {
                return Ok(out);
            }
        }
    }

    fn entity_embeddings(&self) -> Result<Vec<(EntityId, Vec<f32>)>> {
        let mut out = Vec::new();
        loop {
            let rows = self.cypher(
                "MATCH (e:Entity%N) WHERE e.embedding IS NOT NULL RETURN e.uid AS uid, e.embedding AS embedding ORDER BY uid SKIP $skip LIMIT $limit",
                json!({ "skip": out.len(), "limit": PAGE }),
            )?;
            let page = rows.len();
            for row in rows {
                out.push(codec::embedding_from_row(row)?);
            }
            if page < PAGE {
                return Ok(out);
            }
        }
    }

    /// BM25 over `search_text` and/or vector search over `embedding`, both
    /// inside HelixDB and in one request. With both, the two rankings are
    /// fused by reciprocal rank.
    fn search_entities(
        &self,
        text: Option<&str>,
        embedding: Option<&[f32]>,
        limit: usize,
    ) -> Result<Option<Vec<(EntityId, f32)>>> {
        let tenant = self.tenant()?;
        // Fusion needs more than `limit` candidates from each side.
        let k = if text.is_some() && embedding.is_some() { limit * 3 } else { limit };
        let mut batch = read_batch();
        let mut returning = Vec::new();
        if let Some(text) = text {
            batch = batch.var_as(
                "keyword",
                g().text_search_nodes(NODE_LABEL, TEXT_PROPERTY, text, k, Some(tenant.clone()))
                    .value_map(Some(vec!["uid", "$score"])),
            );
            returning.push("keyword");
        }
        if let Some(embedding) = embedding {
            batch = batch.var_as(
                "semantic",
                g().vector_search_nodes(NODE_LABEL, VECTOR_PROPERTY, embedding.to_vec(), k, Some(tenant))
                    .value_map(Some(vec!["uid", "$distance"])),
            );
            returning.push("semantic");
        }
        let request = QueryRequest::read(batch.returning(returning));
        let client = self.client.clone();
        let response = self
            .block_on(async move { client.query::<Value>(request).send().await })
            .map_err(helix_error)?;

        // BM25 score: higher is better. Cosine distance: lower is better,
        // reported as similarity (1 - distance).
        let keyword = codec::search_hits(&response, "keyword", "$score")?;
        let semantic: Vec<(EntityId, f32)> = codec::search_hits(&response, "semantic", "$distance")?
            .into_iter()
            .map(|(id, distance)| (id, 1.0 - distance))
            .collect();

        let mut ranked = match (text.is_some(), embedding.is_some()) {
            (true, true) => rrf_fuse(&[
                keyword.into_iter().map(|(id, _)| id).collect(),
                semantic.into_iter().map(|(id, _)| id).collect(),
            ]),
            (true, false) => keyword,
            _ => semantic,
        };
        ranked.truncate(limit);
        Ok(Some(ranked))
    }

    /// BM25 over each edge's "subject predicate object" text.
    fn search_relationships(&self, text: &str, limit: usize) -> Result<Option<Vec<(RelationshipId, f32)>>> {
        let tenant = self.tenant()?;
        let hits = g()
            .text_search_edges(EDGE_TYPE, TEXT_PROPERTY, text, limit, Some(tenant))
            .value_map(Some(vec!["uid", "$score"]));
        let request = QueryRequest::read(read_batch().var_as("keyword", hits).returning(["keyword"]));
        let client = self.client.clone();
        let response = self
            .block_on(async move { client.query::<Value>(request).send().await })
            .map_err(helix_error)?;
        Ok(Some(codec::search_hits(&response, "keyword", "$score")?))
    }

    fn entity_count(&self) -> Result<usize> {
        self.count("MATCH (e:Entity%N) RETURN count(e) AS n", json!({}))
    }

    fn relationship_count(&self) -> Result<usize> {
        self.count("MATCH (:Entity%N)-[r:REL]->(:Entity) RETURN count(r) AS n", json!({}))
    }
}

impl Drop for HelixGraphBackend {
    fn drop(&mut self) {
        let Some(runtime) = self.runtime.take() else {
            return;
        };
        // Close flushes and releases the writer lease, so the directory can
        // be reopened immediately. Errors cannot be reported from `drop`.
        let client = self.client.clone();
        let _ = block_on(&runtime, async move { client.close().await });
        // `shutdown_background` (unlike dropping the runtime) is legal even
        // when this value is dropped inside another async runtime.
        runtime.shutdown_background();
    }
}

/// Which storage backend [`open`] should use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackendChoice {
    /// Use whatever the directory already contains; HelixDB for a new graph.
    #[default]
    Auto,
    /// `graph.json` (the MemSt 1.0 format).
    Json,
    /// Embedded HelixDB.
    Helix,
}

impl std::str::FromStr for BackendChoice {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, String> {
        match value.to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "json" => Ok(Self::Json),
            "helix" | "helixdb" | "helix-db" => Ok(Self::Helix),
            other => Err(format!("unknown graph backend '{other}' (expected auto, json or helix)")),
        }
    }
}

/// Open the knowledge graph stored in `dir`.
///
/// With [`BackendChoice::Auto`] an existing HelixDB graph wins, then an
/// existing `graph.json` (so MemSt 1.0 stores keep working untouched), and a
/// directory with neither gets a new HelixDB graph.
pub fn open(dir: &Path, choice: BackendChoice) -> Result<KnowledgeGraph> {
    let use_helix = match choice {
        BackendChoice::Helix => true,
        BackendChoice::Json => false,
        BackendChoice::Auto => HelixGraphBackend::exists(dir) || !JsonGraphBackend::exists(dir),
    };
    if use_helix {
        Ok(KnowledgeGraph::with_backend(Box::new(HelixGraphBackend::open(dir)?)))
    } else {
        KnowledgeGraph::new(dir)
    }
}

/// Copy a `graph.json` graph in `dir` into a HelixDB graph in the same
/// directory. The JSON file is left in place as a backup; because an existing
/// HelixDB graph takes precedence, [`open`] uses the new graph from then on.
///
/// Returns `(entities, relationships)` copied.
pub fn migrate_json_to_helix(dir: &Path) -> Result<(usize, usize)> {
    if !JsonGraphBackend::exists(dir) {
        return Err(Error::InvalidOperation(format!(
            "no graph.json to migrate in {}",
            dir.display()
        )));
    }
    let source = KnowledgeGraph::new(dir)?;
    let mut target = open(dir, BackendChoice::Helix)?;
    source.copy_into(&mut target)
}

/// Take the exclusive writer lock for the graph in `dir`.
///
/// HelixDB itself lets a second writer in and fences the *first*, whose
/// writes then start failing mid-flight. MemSt wants the opposite: the
/// process already writing keeps going, and the newcomer is told plainly.
fn acquire_writer_lock(dir: &Path) -> Result<std::fs::File> {
    use fs2::FileExt;
    use std::io::{Read, Seek, Write};

    let path = dir.join(LOCK_FILE);
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)?;
    if file.try_lock_exclusive().is_err() {
        let mut holder = String::new();
        let _ = file.read_to_string(&mut holder);
        let holder = holder.trim();
        return Err(Error::LockError(format!(
            "the graph in {} is open for writing by another process{}; open it read-only, or try again when that process has finished",
            dir.display(),
            if holder.is_empty() { String::new() } else { format!(" (pid {holder})") }
        )));
    }
    file.set_len(0)?;
    file.rewind()?;
    write!(file, "{}", std::process::id())?;
    file.flush()?;
    Ok(file)
}

fn helix_error(error: HelixError) -> Error {
    Error::GraphBackend(format!("helix: {error}"))
}

/// Block the current thread on a future running on `runtime`.
///
/// `Runtime::block_on` panics when called from inside an async runtime, so
/// the future is spawned and its result awaited over a channel instead.
fn block_on<F>(runtime: &tokio::runtime::Runtime, future: F) -> F::Output
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    runtime.spawn(async move {
        let _ = sender.send(future.await);
    });
    receiver
        .recv()
        .expect("helix runtime dropped a request without answering")
}
