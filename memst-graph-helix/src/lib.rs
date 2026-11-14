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

mod codec;

use helix_db::dsl::prelude::{g, write_batch, IndexSpec, QueryRequest};
use helix_db::{Client, HelixDbSource, HelixError};
use memst_core::error::{Error, Result};
use memst_core::graph::{Direction, GraphBackend, JsonGraphBackend, KnowledgeGraph};
use memst_core::types::{Entity, EntityId, EntityQuery, Relationship, RelationshipId, RelationshipQuery};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};
use std::future::Future;
use std::path::{Path, PathBuf};

/// Directory (inside a graph directory) that holds the HelixDB database.
pub const HELIX_DIR: &str = "graph.helix";

const NODE_LABEL: &str = "Entity";
const EDGE_TYPE: &str = "REL";
/// Rows per request when reading or writing in bulk. Keeps every request
/// far below HelixDB's 16 MiB response and 200,000-value parameter limits.
const PAGE: usize = 2_000;

/// Options for [`HelixGraphBackend::open_with`].
#[derive(Debug, Clone, Default)]
pub struct HelixGraphOptions {
    /// Let the embedded engine send HelixDB's anonymous usage telemetry.
    ///
    /// **Off by default**: a memory store should not phone home. When this is
    /// `false` and neither `HELIX_TELEMETRY_LEVEL` nor `DO_NOT_TRACK` is set
    /// in the environment, the backend sets `HELIX_TELEMETRY_LEVEL=off` for
    /// this process before opening the database.
    pub telemetry: bool,
}

/// Knowledge graph stored in an embedded HelixDB database.
pub struct HelixGraphBackend {
    // `client` is declared (and therefore dropped) before the runtime it
    // was opened on; `Drop` closes it explicitly first.
    client: Client,
    runtime: Option<tokio::runtime::Runtime>,
    root: PathBuf,
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

        let root = dir.join(HELIX_DIR);
        std::fs::create_dir_all(&root)?;

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("memst-helix")
            .enable_all()
            .build()?;

        let source = HelixDbSource::Disk {
            root: root.clone(),
            database: "kg".to_string(),
        };
        let client = block_on(&runtime, Client::open(source)).map_err(helix_error)?;

        let backend = Self {
            client,
            runtime: Some(runtime),
            root,
        };
        backend.ensure_indexes()?;
        Ok(backend)
    }

    /// Whether a HelixDB graph already exists in `dir`.
    pub fn exists(dir: &Path) -> bool {
        dir.join(HELIX_DIR).is_dir()
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
        ] {
            let request = QueryRequest::write(
                write_batch()
                    .var_as("index", g().create_index_if_not_exists(spec))
                    .returning(["index"]),
            );
            let client = self.client.clone();
            self.block_on(async move { client.query::<Value>(request).send().await })
                .map_err(helix_error)?;
        }
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
        let parameters: BTreeMap<String, Value> = parameters.into_iter().collect();
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
            "UNWIND $uids AS k MATCH (e:Entity {uid: k}) RETURN e.uid AS uid",
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
        for chunk in entities.chunks(PAGE) {
            // Later duplicates of an id within one batch win, as with a map.
            let mut rows: BTreeMap<String, Value> = BTreeMap::new();
            for entity in chunk {
                let uid = entity.id.to_string();
                rows.insert(uid.clone(), json!({ "uid": uid, "props": codec::entity_props(entity) }));
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
                // `SET e = map` replaces every property, so cleared optional
                // fields really disappear.
                self.cypher(
                    "UNWIND $rows AS row WITH row.uid AS k, row.props AS props MATCH (e:Entity {uid: k}) SET e = props RETURN count(e) AS n",
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
                "UNWIND $uids AS k MATCH (e:Entity {uid: k}) RETURN properties(e) AS e",
                json!({ "uids": chunk }),
            )?;
            for row in rows {
                out.push(codec::entity_from_row(row)?);
            }
        }
        Ok(out)
    }

    fn delete_entity(&mut self, id: EntityId) -> Result<bool> {
        let deleted = self.count(
            "MATCH (e:Entity {uid: $uid}) DETACH DELETE e RETURN count(e) AS n",
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
        let filter = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {} ", conditions.join(" AND "))
        };
        // Only fixed fragments are interpolated; every value is a parameter.
        let statement = format!(
            "MATCH (e:Entity) {filter}RETURN properties(e) AS e ORDER BY e.created_at, e.uid SKIP $skip LIMIT $limit"
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

    fn put_relationship(&mut self, relationship: &Relationship) -> Result<()> {
        let uid = relationship.id.to_string();
        let (subject, object) = (
            relationship.subject_id.to_string(),
            relationship.object_id.to_string(),
        );
        let props = codec::relationship_props(relationship);

        // An edge cannot be re-pointed, so an existing edge is updated in
        // place only when its endpoints are unchanged.
        if let Some(existing) = self.get_relationship(relationship.id)? {
            if existing.subject_id == relationship.subject_id
                && existing.object_id == relationship.object_id
            {
                self.cypher(
                    "MATCH (:Entity {uid: $s})-[r:REL]->(:Entity {uid: $o}) WHERE r.uid = $uid SET r = $props RETURN count(r) AS n",
                    json!({ "s": subject, "o": object, "uid": uid, "props": props }),
                )?;
                return Ok(());
            }
            self.delete_relationship(relationship.id)?;
        }

        let created = self.count(
            "MATCH (a:Entity {uid: $s}), (b:Entity {uid: $o}) CREATE (a)-[r:REL]->(b) SET r += $props RETURN count(r) AS n",
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
            "MATCH (:Entity)-[r:REL]->(:Entity) WHERE r.uid = $uid RETURN properties(r) AS r",
            json!({ "uid": id.to_string() }),
        )?
        .pop()
        .map(codec::relationship_from_row)
        .transpose()
    }

    fn delete_relationship(&mut self, id: RelationshipId) -> Result<bool> {
        let deleted = self.count(
            "MATCH (:Entity)-[r:REL]->(:Entity) WHERE r.uid = $uid DELETE r RETURN count(r) AS n",
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
                "(:Entity {uid: $s})"
            }
            None => "(:Entity)",
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
            "UNWIND $uids AS k MATCH (:Entity {uid: k})-[r:REL]->(:Entity) RETURN properties(r) AS r";
        const IN: &str =
            "UNWIND $uids AS k MATCH (:Entity {uid: k})<-[r:REL]-(:Entity) RETURN properties(r) AS r";

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

    fn entity_count(&self) -> Result<usize> {
        self.count("MATCH (e:Entity) RETURN count(e) AS n", json!({}))
    }

    fn relationship_count(&self) -> Result<usize> {
        self.count("MATCH (:Entity)-[r:REL]->(:Entity) RETURN count(r) AS n", json!({}))
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
