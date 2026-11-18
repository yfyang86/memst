//! Python access to the store's knowledge graph.

use memst_core::graph::{Direction, EntitySearch, KnowledgeGraph, SearchMethod};
use memst_core::types::{Entity, EntityQuery, KgExtractionResult, Relationship, RelationshipQuery};
use memst_graph_helix::{open_store, GraphLocation, StoreGraphOptions};
use memst_sleep::kg_extract::KgExtractionService;
use memst_sleep::kg_ingest::{ingest_and_embed, GraphIngestor, IngestReport, Provenance};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Mutex;
use uuid::Uuid;

fn to_py_err(error: impl std::fmt::Display) -> PyErr {
    PyErr::new::<PyRuntimeError, _>(error.to_string())
}

fn parse_uuid(text: &str, what: &str) -> PyResult<Uuid> {
    Uuid::parse_str(text).map_err(|e| PyErr::new::<PyValueError, _>(format!("invalid {what} '{text}': {e}")))
}

/// Convert JSON into plain Python objects (dict, list, str, int, float, bool, None).
fn json_to_py(py: Python<'_>, value: &Value) -> PyResult<PyObject> {
    Ok(match value {
        Value::Null => py.None(),
        Value::Bool(b) => b.into_py(py),
        Value::Number(n) => match (n.as_i64(), n.as_f64()) {
            (Some(i), _) => i.into_py(py),
            (None, Some(f)) => f.into_py(py),
            (None, None) => py.None(),
        },
        Value::String(s) => s.into_py(py),
        Value::Array(items) => {
            let list = PyList::empty_bound(py);
            for item in items {
                list.append(json_to_py(py, item)?)?;
            }
            list.into_py(py)
        }
        Value::Object(map) => {
            let dict = PyDict::new_bound(py);
            for (key, item) in map {
                dict.set_item(key, json_to_py(py, item)?)?;
            }
            dict.into_py(py)
        }
    })
}

fn entity_json(entity: &Entity) -> Value {
    json!({
        "id": entity.id.to_string(),
        "name": entity.name,
        "entity_type": entity.entity_type,
        "attributes": entity.attributes,
        "session_id": entity.session_id.to_string(),
        "seen_in": entity.seen_in.iter().map(Uuid::to_string).collect::<Vec<_>>(),
        "source_message_id": entity.source_message_id.map(|id| id.to_string()),
        "confidence": entity.confidence,
        "relevance": entity.relevance,
        "access_count": entity.access_count,
        "is_deprecated": entity.is_deprecated,
        "created_at": entity.created_at.to_rfc3339(),
    })
}

fn triple_json((relationship, subject, object): &(Relationship, Entity, Entity)) -> Value {
    json!({
        "id": relationship.id.to_string(),
        "predicate": relationship.predicate,
        "subject_id": subject.id.to_string(),
        "subject": subject.name,
        "object_id": object.id.to_string(),
        "object": object.name,
        "confidence": relationship.confidence,
        "relevance": relationship.relevance,
        "session_id": relationship.session_id.to_string(),
        "source_message_id": relationship.source_message_id.map(|id| id.to_string()),
        "created_at": relationship.created_at.to_rfc3339(),
        "valid_from": relationship.valid_from.map(|t| t.to_rfc3339()),
        "valid_to": relationship.valid_to.map(|t| t.to_rfc3339()),
        "valid_precision": relationship.valid_precision.map(|p| p.as_str()),
        "stated_at": relationship.stated_at.map(|t| t.to_rfc3339()),
    })
}

fn report_json(report: &IngestReport) -> Value {
    json!({
        "entities_created": report.entities_created,
        "entities_reused": report.entities_reused,
        "relationships_created": report.relationships_created,
        "relationships_reinforced": report.relationships_reinforced,
        "relationships_superseded": report.relationships_superseded,
        "relationships_skipped": report.relationships_skipped,
        "entities_embedded": report.entities_embedded,
        "llm_calls": report.llm_calls,
        "llm_failures": report.llm_failures,
        "llm_input_chars": report.llm_input_chars,
    })
}

/// The knowledge graph of a MemSt store.
///
/// One graph is shared by every session of the store. Each owner sees only
/// their own entities. Only one process can hold the graph open for writing;
/// open with ``read_only=True`` to read alongside a writer.
#[pyclass]
pub struct Graph {
    graph: Mutex<KnowledgeGraph>,
    location: GraphLocation,
    owner: String,
}

impl Graph {
    /// Run `f` on the graph. Does not need the GIL.
    fn locked<T, E: std::fmt::Display>(
        &self,
        f: impl FnOnce(&mut KnowledgeGraph) -> Result<T, E>,
    ) -> Result<T, String> {
        let mut graph = self.graph.lock().map_err(|_| "graph lock poisoned".to_string())?;
        f(&mut graph).map_err(|e| e.to_string())
    }

    fn with<T>(&self, f: impl FnOnce(&mut KnowledgeGraph) -> memst_core::error::Result<T>) -> PyResult<T> {
        self.locked(f).map_err(to_py_err)
    }
}

#[pymethods]
impl Graph {
    /// Open the graph of the store at ``store_path``.
    ///
    /// ``session_id`` matters only for stores from older versions: a session
    /// that still has its own, unmigrated graph opens that one.
    /// ``backend`` is ``"auto"``, ``"helix"`` or ``"json"``.
    #[staticmethod]
    #[pyo3(signature = (store_path, session_id=None, owner="default", backend="auto", read_only=false))]
    fn open(
        py: Python<'_>,
        store_path: String,
        session_id: Option<String>,
        owner: &str,
        backend: &str,
        read_only: bool,
    ) -> PyResult<Self> {
        let store_dir = PathBuf::from(store_path);
        let session_dir = match &session_id {
            Some(id) => store_dir.join("sessions").join(parse_uuid(id, "session id")?.to_string()),
            None => store_dir.join("sessions").join("_no_session"),
        };
        let options = StoreGraphOptions {
            choice: backend.parse().map_err(PyErr::new::<PyValueError, _>)?,
            owner: owner.to_string(),
            read_only,
        };
        let (graph, location) = py
            .allow_threads(|| open_store(&store_dir, &session_dir, &options))
            .map_err(to_py_err)?;
        Ok(Self {
            graph: Mutex::new(graph),
            location,
            owner: owner.to_string(),
        })
    }

    /// ``"helix"`` or ``"json"``.
    #[getter]
    fn backend(&self) -> PyResult<&'static str> {
        self.with(|g| Ok(g.backend_kind()))
    }

    /// Whether this is the store's shared graph (``True``) or a session's
    /// own graph that has not been migrated (``False``).
    #[getter]
    fn is_shared(&self) -> bool {
        self.location == GraphLocation::Store
    }

    /// The owner this handle is restricted to.
    #[getter]
    fn owner(&self) -> &str {
        &self.owner
    }

    /// Counts of entities and relationships, and per type and predicate.
    fn stats(&self, py: Python<'_>) -> PyResult<PyObject> {
        let stats = self.with(|g| g.stats())?;
        json_to_py(
            py,
            &json!({
                "entity_count": stats.entity_count,
                "relationship_count": stats.relationship_count,
                "entity_types": stats.entity_types,
                "predicates": stats.predicates,
            }),
        )
    }

    /// Add an entity mentioned in ``session_id``.
    ///
    /// With ``resolve=True`` (the default) an existing entity with the same
    /// name and type is reused and linked to the session. Returns the stored
    /// entity, with ``"created"`` saying whether it is new.
    #[pyo3(signature = (session_id, name, entity_type, attributes=None, resolve=true))]
    fn add_entity(
        &self,
        py: Python<'_>,
        session_id: &str,
        name: &str,
        entity_type: &str,
        attributes: Option<&str>,
        resolve: bool,
    ) -> PyResult<PyObject> {
        let mut entity = Entity::new(name, entity_type, parse_uuid(session_id, "session id")?);
        if let Some(text) = attributes {
            let parsed: Value = serde_json::from_str(text)
                .map_err(|e| PyErr::new::<PyValueError, _>(format!("attributes is not JSON: {e}")))?;
            entity = entity.with_attributes(parsed);
        }
        let (stored, created) = self.with(|g| {
            if resolve {
                g.resolve_or_add_entity(entity)
            } else {
                Ok((g.add_entity(entity)?, true))
            }
        })?;
        let mut out = entity_json(&stored);
        out["created"] = json!(created);
        json_to_py(py, &out)
    }

    /// Add a relationship between two existing entities.
    #[pyo3(signature = (session_id, subject_id, predicate, object_id, confidence=1.0))]
    fn add_relationship(
        &self,
        py: Python<'_>,
        session_id: &str,
        subject_id: &str,
        predicate: &str,
        object_id: &str,
        confidence: f32,
    ) -> PyResult<PyObject> {
        let relationship = Relationship::new(
            parse_uuid(subject_id, "subject id")?,
            predicate,
            parse_uuid(object_id, "object id")?,
            parse_uuid(session_id, "session id")?,
        )
        .with_confidence(confidence);
        let id = relationship.id;
        let triple = self.with(|g| {
            g.add_relationship(relationship)?;
            g.get_relationship(id)
        })?;
        match triple {
            Some(triple) => json_to_py(py, &triple_json(&triple)),
            None => Err(to_py_err("relationship was not stored")),
        }
    }

    /// Entities, most important first. ``limit=0`` means no limit.
    #[pyo3(signature = (session_id=None, entity_type=None, limit=0, include_deprecated=false))]
    fn entities(
        &self,
        py: Python<'_>,
        session_id: Option<&str>,
        entity_type: Option<String>,
        limit: usize,
        include_deprecated: bool,
    ) -> PyResult<PyObject> {
        let mut query = EntityQuery::new().with_limit(limit);
        if let Some(id) = session_id {
            query = query.with_session(parse_uuid(id, "session id")?);
        }
        if let Some(entity_type) = entity_type {
            query = query.with_type(entity_type);
        }
        let entities = self.with(|g| g.find_entities(query))?;
        let rows: Vec<Value> = entities
            .iter()
            .filter(|e| include_deprecated || !e.is_deprecated)
            .map(entity_json)
            .collect();
        json_to_py(py, &Value::Array(rows))
    }

    /// Relationships, each with the names of its endpoints.
    #[pyo3(signature = (session_id=None, entity_id=None))]
    fn relationships(&self, py: Python<'_>, session_id: Option<&str>, entity_id: Option<&str>) -> PyResult<PyObject> {
        let triples = match entity_id {
            Some(id) => {
                let id = parse_uuid(id, "entity id")?;
                self.with(|g| g.relationships_of(id, Direction::Both))?
            }
            None => {
                let mut query = RelationshipQuery::new();
                if let Some(id) = session_id {
                    query = query.with_session(parse_uuid(id, "session id")?);
                }
                self.with(|g| g.find_relationships(query))?
            }
        };
        json_to_py(py, &Value::Array(triples.iter().map(triple_json).collect()))
    }

    /// Search entities by keywords and, with ``embedding``, by meaning.
    ///
    /// Returns ``{"method": "native" | "fallback", "fallback_reason": str | None,
    /// "hits": [{..entity.., "score": float}]}``.
    #[pyo3(signature = (text=None, limit=10, session_id=None, embedding=None, include_deprecated=false))]
    fn search(
        &self,
        py: Python<'_>,
        text: Option<String>,
        limit: usize,
        session_id: Option<&str>,
        embedding: Option<Vec<f32>>,
        include_deprecated: bool,
    ) -> PyResult<PyObject> {
        let search = EntitySearch {
            text,
            embedding,
            limit,
            include_deprecated,
            session_id: session_id.map(|id| parse_uuid(id, "session id")).transpose()?,
        };
        let result = py
            .allow_threads(|| self.locked(|graph| graph.search_entities(&search)))
            .map_err(to_py_err)?;
        let hits: Vec<Value> = result
            .hits
            .iter()
            .map(|hit| {
                let mut row = entity_json(&hit.entity);
                row["score"] = json!(hit.score);
                row
            })
            .collect();
        json_to_py(
            py,
            &json!({
                "method": match result.method {
                    SearchMethod::Native => "native",
                    SearchMethod::Fallback => "fallback",
                },
                "fallback_reason": result.fallback_reason,
                "hits": hits,
            }),
        )
    }

    /// Search relationships by keywords, matched against "subject predicate
    /// object". Superseded facts are left out.
    ///
    /// Returns ``{"method": "native" | "fallback", "fallback_reason": str | None,
    /// "hits": [{..relationship.., "score": float}]}``.
    #[pyo3(signature = (text, limit=10))]
    fn search_relationships(&self, py: Python<'_>, text: &str, limit: usize) -> PyResult<PyObject> {
        let result = py
            .allow_threads(|| self.locked(|graph| graph.search_relationships(text, limit)))
            .map_err(to_py_err)?;
        let hits: Vec<Value> = result
            .hits
            .iter()
            .map(|(triple, score)| {
                let mut row = triple_json(triple);
                row["score"] = json!(score);
                row
            })
            .collect();
        json_to_py(
            py,
            &json!({
                "method": match result.method {
                    SearchMethod::Native => "native",
                    SearchMethod::Fallback => "fallback",
                },
                "fallback_reason": result.fallback_reason,
                "hits": hits,
            }),
        )
    }

    /// Entities within ``hops`` of an entity, nearest first, each with
    /// ``"hops"``. ``direction`` is ``"out"``, ``"in"`` or ``"both"``.
    #[pyo3(signature = (entity_id, hops=2, direction="both"))]
    fn neighbors(&self, py: Python<'_>, entity_id: &str, hops: usize, direction: &str) -> PyResult<PyObject> {
        let direction = match direction {
            "out" => Direction::Outgoing,
            "in" => Direction::Incoming,
            "both" => Direction::Both,
            other => {
                return Err(PyErr::new::<PyValueError, _>(format!(
                    "unknown direction '{other}' (expected out, in or both)"
                )))
            }
        };
        let id = parse_uuid(entity_id, "entity id")?;
        let hits = self.with(|g| g.neighborhood(id, hops, direction))?;
        let rows: Vec<Value> = hits
            .iter()
            .map(|(entity, distance)| {
                let mut row = entity_json(entity);
                row["hops"] = json!(distance);
                row
            })
            .collect();
        json_to_py(py, &Value::Array(rows))
    }

    /// Write an extraction result into the graph.
    ///
    /// ``extraction`` is JSON with ``entities`` (``name``, ``entity_type``,
    /// ``attributes``, ``confidence``), ``relationships`` (``subject``,
    /// ``predicate``, ``object``, ``confidence``) and ``events``. Known
    /// entities are reused, repeated relationships reinforced. With an
    /// ``embedder`` other than ``"none"`` the new entities are embedded.
    /// Returns a report of what changed.
    #[pyo3(signature = (session_id, extraction, message_id=None, embedder="none"))]
    fn ingest_extraction(
        &self,
        py: Python<'_>,
        session_id: &str,
        extraction: &str,
        message_id: Option<&str>,
        embedder: &str,
    ) -> PyResult<PyObject> {
        let extraction: KgExtractionResult = serde_json::from_str(extraction)
            .map_err(|e| PyErr::new::<PyValueError, _>(format!("extraction is not valid: {e}")))?;
        let mut provenance = Provenance::new(parse_uuid(session_id, "session id")?);
        if let Some(id) = message_id {
            provenance = provenance.with_message(parse_uuid(id, "message id")?);
        }
        let embedder = memst_embed::select_embedder(embedder, |_| {}).map_err(to_py_err)?;
        let report = py
            .allow_threads(|| {
                self.locked(|graph| ingest_and_embed(graph, &extraction, &provenance, embedder.as_deref()))
            })
            .map_err(to_py_err)?;
        json_to_py(py, &report_json(&report))
    }

    /// Extract entities and relationships from messages with an LLM and write
    /// them into the graph.
    ///
    /// ``messages`` is a list of ``(message_id, text)``. The LLM is an
    /// OpenAI-compatible endpoint. Returns a report of what changed.
    ///
    /// ``max_tokens`` is the model's output budget per call; reasoning models
    /// spend part of it thinking. ``llm_extra`` is a JSON object of
    /// provider-specific request fields, for example
    /// ``'{"thinking": {"type": "disabled"}}'``. Batches the model fails on
    /// are skipped and counted in ``llm_failures``.
    ///
    /// ``timestamps``, if given, has one entry per message: when it was sent,
    /// as an RFC 3339 string, or ``None``. Facts are then dated from the
    /// message that states them.
    #[pyo3(signature = (session_id, messages, llm_api_url, llm_model, llm_api_key=None, embedder="auto", batch_chars=6000, max_tokens=16000, llm_extra=None, timestamps=None))]
    #[allow(clippy::too_many_arguments)]
    fn ingest_messages(
        &self,
        py: Python<'_>,
        session_id: &str,
        messages: Vec<(String, String)>,
        llm_api_url: &str,
        llm_model: &str,
        llm_api_key: Option<String>,
        embedder: &str,
        batch_chars: usize,
        max_tokens: u32,
        llm_extra: Option<&str>,
        timestamps: Option<Vec<Option<String>>>,
    ) -> PyResult<PyObject> {
        if timestamps.as_ref().is_some_and(|t| t.len() != messages.len()) {
            return Err(PyErr::new::<PyValueError, _>("timestamps must have one entry per message"));
        }
        use memst_core::llm::loader::{LlmLoader, LlmProviderConfig};

        let mut extraction_config = memst_sleep::kg_extract::KgExtractionConfig {
            max_tokens,
            ..Default::default()
        };
        if let Some(text) = llm_extra {
            let Value::Object(extra) = serde_json::from_str::<Value>(text)
                .map_err(|e| PyErr::new::<PyValueError, _>(format!("llm_extra is not JSON: {e}")))?
            else {
                return Err(PyErr::new::<PyValueError, _>("llm_extra must be a JSON object"));
            };
            extraction_config.request_extra = extra;
        }

        let session = parse_uuid(session_id, "session id")?;
        let messages: Vec<memst_sleep::kg_ingest::SourceMessage> = messages
            .into_iter()
            .enumerate()
            .map(|(index, (id, text))| {
                let timestamp = match timestamps.as_ref().and_then(|t| t[index].as_deref()) {
                    Some(text) => Some(
                        chrono::DateTime::parse_from_rfc3339(text)
                            .map(|t| t.with_timezone(&chrono::Utc))
                            .map_err(|e| PyErr::new::<PyValueError, _>(format!("invalid timestamp '{text}': {e}")))?,
                    ),
                    None => None,
                };
                Ok(memst_sleep::kg_ingest::SourceMessage { id: parse_uuid(&id, "message id")?, text, timestamp })
            })
            .collect::<PyResult<_>>()?;
        let mut config = LlmProviderConfig::openai(llm_api_url, llm_model);
        if let Some(key) = llm_api_key.filter(|key| !key.is_empty()) {
            config = config.with_api_key(key);
        }
        let llm = LlmLoader::load(config).map_err(to_py_err)?;
        let embedder = memst_embed::select_embedder(embedder, |_| {}).map_err(to_py_err)?;
        let ingestor = GraphIngestor::new(KgExtractionService::with_config(llm, extraction_config));

        let (report, errors) = py
            .allow_threads(|| -> Result<(IngestReport, Vec<String>), String> {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| e.to_string())?;
                let report = self.locked(|graph| {
                    runtime.block_on(ingestor.ingest_dated_messages(
                        graph,
                        session,
                        &messages,
                        batch_chars,
                        embedder.as_deref(),
                    ))
                })?;
                Ok((report, ingestor.take_errors()))
            })
            .map_err(to_py_err)?;
        let mut out = report_json(&report);
        out["llm_errors"] = json!(errors);
        json_to_py(py, &out)
    }

    /// Embed entities that are new or have changed. Returns how many.
    #[pyo3(signature = (embedder="auto"))]
    fn embed_pending(&self, py: Python<'_>, embedder: &str) -> PyResult<usize> {
        let Some(embedder) = memst_embed::select_embedder(embedder, |_| {}).map_err(to_py_err)? else {
            return Ok(0);
        };
        py.allow_threads(|| self.locked(|graph| graph.embed_pending(embedder.as_ref())))
            .map_err(to_py_err)
    }

    /// Embed a text with the selected embedder, for use as a search query.
    #[staticmethod]
    #[pyo3(signature = (text, embedder="auto"))]
    fn embed_text(py: Python<'_>, text: String, embedder: &str) -> PyResult<Vec<f32>> {
        let Some(embedder) = memst_embed::select_embedder(embedder, |_| {}).map_err(to_py_err)? else {
            return Err(PyErr::new::<PyValueError, _>("embedder 'none' cannot embed"));
        };
        py.allow_threads(|| embedder.embed(&[text]))
            .map_err(to_py_err)?
            .pop()
            .ok_or_else(|| to_py_err("the embedder returned nothing"))
    }
}

/// A text embedding model, kept loaded.
///
/// ``kind`` is ``"auto"``, ``"local"`` (BGE-M3 on the CPU) or ``"http"``
/// (the configured embeddings endpoint).
#[pyclass]
pub struct Embedder {
    inner: Box<dyn memst_core::embed::Embedder>,
}

#[pymethods]
impl Embedder {
    #[new]
    #[pyo3(signature = (kind="auto"))]
    fn new(py: Python<'_>, kind: &str) -> PyResult<Self> {
        let chosen = py
            .allow_threads(|| memst_embed::select_embedder(kind, |_| {}))
            .map_err(to_py_err)?;
        match chosen {
            Some(inner) => Ok(Self { inner }),
            None => Err(PyErr::new::<PyValueError, _>("embedder 'none' cannot embed")),
        }
    }

    /// Identifies the model, for example ``"local:bge-m3"``.
    #[getter]
    fn id(&self) -> String {
        self.inner.id()
    }

    /// Embed each text; one vector per text, in order.
    fn embed(&self, py: Python<'_>, texts: Vec<String>) -> PyResult<Vec<Vec<f32>>> {
        py.allow_threads(|| self.inner.embed(&texts)).map_err(to_py_err)
    }
}
