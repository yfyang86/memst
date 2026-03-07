//! MemSt Python Bindings
//!
//! PyO3-based Python bindings for the MemSt session memory library.

use anyhow::Error as AnyhowError;
use memst_core::error::Error as MemStError;
use memst_core::hybrid::{
    FusionStrategy as CoreFusionStrategy, HybridSearchConfig, HybridSearchResult, QueryRouter,
    SearchStrategy as CoreSearchStrategy,
};
use memst_core::objects::{Author, Blob, Commit, ObjectId, RefType, Tag, TreeEntry};
use memst_core::search::{SearchQuery, SearchResult};
use memst_core::store::SessionStore as CoreSessionStore;
use memst_core::types::{
    MemoryItem as CoreMemoryItem, MemoryTier as CoreMemoryTier, Message as CoreMessage,
    Role as CoreRole, SessionMetadata, SessionSummary,
};
use memst_core::vector::{DocumentInfo, HnswConfig, HnswIndex, VectorSearchResult};
use pyo3::exceptions::{PyIOError, PyKeyError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;
use uuid::Uuid;

/// Helper to convert MemSt error to PyErr with appropriate exception type
fn memst_err_to_pyerr(e: MemStError) -> PyErr {
    match &e {
        MemStError::Io(_) => PyErr::new::<PyIOError, _>(e.to_string()),
        MemStError::SessionNotFound(_) => PyErr::new::<PyKeyError, _>(e.to_string()),
        MemStError::InvalidOperation(_) => PyErr::new::<PyValueError, _>(e.to_string()),
        _ => PyErr::new::<PyRuntimeError, _>(e.to_string()),
    }
}

/// Helper to convert anyhow error to PyErr
fn anyhow_err_to_pyerr(e: AnyhowError) -> PyErr {
    // Check if the underlying error is an IO error
    if e.downcast_ref::<std::io::Error>().is_some() {
        PyErr::new::<PyIOError, _>(e.to_string())
    } else {
        PyErr::new::<PyRuntimeError, _>(e.to_string())
    }
}

/// PyO3 module definition
#[pymodule]
#[pyo3(name = "memst")]
fn lib(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<SessionStore>()?;
    m.add_class::<Session>()?;
    m.add_class::<Message>()?;
    m.add_class::<MemoryItem>()?;
    m.add_class::<MemoryTier>()?;
    m.add_class::<Role>()?;
    // Phase 8: Git-like architecture
    m.add_class::<ObjectIdWrapper>()?;
    m.add_class::<BlobWrapper>()?;
    m.add_class::<TreeWrapper>()?;
    m.add_class::<CommitWrapper>()?;
    m.add_class::<TagWrapper>()?;
    m.add_class::<AuthorWrapper>()?;
    m.add_class::<TreeEntryWrapper>()?;
    m.add_class::<RefTypeWrapper>()?;
    m.add_class::<MergeStrategyWrapper>()?;
    m.add_class::<MergeResultWrapper>()?;
    // Phase 12: Advanced Search (Semantic & Hybrid)
    m.add_class::<HnswConfigWrapper>()?;
    m.add_class::<HnswIndexWrapper>()?;
    m.add_class::<DocumentInfoWrapper>()?;
    m.add_class::<VectorSearchResultWrapper>()?;
    m.add_class::<FusionStrategyWrapper>()?;
    m.add_class::<HybridSearchConfigWrapper>()?;
    m.add_class::<HybridSearchResultWrapper>()?;
    m.add_class::<QueryRouterWrapper>()?;
    m.add_class::<SearchStrategyWrapper>()?;
    // Phase 16: KG Extraction v2
    m.add_class::<KgStorageWrapper>()?;
    m.add_class::<Entity>()?;
    m.add_class::<ExtractionJob>()?;
    m.add_class::<ExtractionService>()?;
    m.add_class::<Ontology>()?;
    m.add_class::<OntologyManager>()?;
    m.add("__version__", "0.1.0")?;

    // Initialize Phase 8/12/16 aliases and __all__
    _init_phase8_aliases(m.py(), m)?;
    _init_phase12_aliases(m.py(), m)?;
    _init_phase16_aliases(m.py(), m)?;
    _init_module_all(m.py(), m)?;

    Ok(())
}

/// Role enum for messages
#[pyclass(eq, eq_int)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[pymethods]
impl Role {
    fn __repr__(&self) -> String {
        match self {
            Role::System => "Role.System".to_string(),
            Role::User => "Role.User".to_string(),
            Role::Assistant => "Role.Assistant".to_string(),
            Role::Tool => "Role.Tool".to_string(),
        }
    }
}

impl From<CoreRole> for Role {
    fn from(role: CoreRole) -> Self {
        match role {
            CoreRole::System => Role::System,
            CoreRole::User => Role::User,
            CoreRole::Assistant => Role::Assistant,
            CoreRole::Tool => Role::Tool,
        }
    }
}

impl From<Role> for CoreRole {
    fn from(role: Role) -> Self {
        match role {
            Role::System => CoreRole::System,
            Role::User => CoreRole::User,
            Role::Assistant => CoreRole::Assistant,
            Role::Tool => CoreRole::Tool,
        }
    }
}

/// Memory tier enum
#[pyclass(eq, eq_int)]
#[derive(Clone, Debug, PartialEq)]
pub enum MemoryTier {
    Working,
    ShortTerm,
    LongTerm,
}

#[pymethods]
impl MemoryTier {
    fn __repr__(&self) -> String {
        match self {
            MemoryTier::Working => "MemoryTier.Working".to_string(),
            MemoryTier::ShortTerm => "MemoryTier.ShortTerm".to_string(),
            MemoryTier::LongTerm => "MemoryTier.LongTerm".to_string(),
        }
    }
}

impl From<CoreMemoryTier> for MemoryTier {
    fn from(tier: CoreMemoryTier) -> Self {
        match tier {
            CoreMemoryTier::Working => MemoryTier::Working,
            CoreMemoryTier::ShortTerm => MemoryTier::ShortTerm,
            CoreMemoryTier::LongTerm => MemoryTier::LongTerm,
        }
    }
}

impl From<MemoryTier> for CoreMemoryTier {
    fn from(tier: MemoryTier) -> Self {
        match tier {
            MemoryTier::Working => CoreMemoryTier::Working,
            MemoryTier::ShortTerm => CoreMemoryTier::ShortTerm,
            MemoryTier::LongTerm => CoreMemoryTier::LongTerm,
        }
    }
}

/// Message class for Python
#[pyclass]
#[derive(Clone, Debug)]
pub struct Message {
    #[pyo3(get, set)]
    pub id: String,
    #[pyo3(get, set)]
    pub role: Role,
    #[pyo3(get, set)]
    pub content: String,
    #[pyo3(get, set)]
    pub timestamp: String,
}

#[pymethods]
impl Message {
    #[new]
    fn new(role: Role, content: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role,
            content,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "Message(role={}, content='{}...', timestamp='{}')",
            self.role.__repr__(),
            &self.content[..std::cmp::min(50, self.content.len())],
            &self.timestamp[..19]
        )
    }
}

impl From<CoreMessage> for Message {
    fn from(msg: CoreMessage) -> Self {
        let content = match msg.content {
            memst_core::types::Content::Text(s) => s,
            _ => format!("{:?}", msg.content),
        };
        Self {
            id: msg.id.to_string(),
            role: msg.role.into(),
            content,
            timestamp: msg.timestamp.to_rfc3339(),
        }
    }
}

/// Memory item class for Python
#[pyclass]
#[derive(Clone, Debug)]
pub struct MemoryItem {
    #[pyo3(get, set)]
    pub id: String,
    #[pyo3(get, set)]
    pub content: String,
    #[pyo3(get, set)]
    pub source: String,
    #[pyo3(get, set)]
    pub tags: Vec<String>,
    #[pyo3(get, set)]
    pub confidence: f32,
    #[pyo3(get, set)]
    pub importance: f32,
    #[pyo3(get, set)]
    pub access_count: u32,
}

#[pymethods]
impl MemoryItem {
    #[new]
    fn new(content: String, source: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            content,
            source,
            tags: vec![],
            confidence: 0.8,
            importance: 0.5,
            access_count: 0,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "MemoryItem(content='{}...', source={}, importance={:.2})",
            &self.content[..std::cmp::min(30, self.content.len())],
            self.source,
            self.importance
        )
    }
}

impl From<CoreMemoryItem> for MemoryItem {
    fn from(item: CoreMemoryItem) -> Self {
        Self {
            id: item.id.to_string(),
            content: item.content,
            source: item.source,
            tags: item.tags,
            confidence: item.confidence,
            importance: item.importance,
            access_count: item.access_count,
        }
    }
}

/// Session class for Python
#[pyclass]
pub struct Session {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub model: String,
    #[pyo3(get)]
    pub created_at: String,
    #[pyo3(get)]
    pub message_count: u32,
    store: Py<SessionStore>,
}

#[pymethods]
impl Session {
    fn __repr__(&self) -> String {
        format!(
            "Session(id='{}', name='{}', model='{}', messages={})",
            &self.id[..8],
            self.name,
            self.model,
            self.message_count
        )
    }

    /// Get all messages in this session
    fn messages(&self, py: Python<'_>) -> PyResult<PyObject> {
        let store = self.store.borrow(py);
        store.get_session_messages(&self.id, py)
    }

    /// Get memory items from a tier
    fn memory(&self, py: Python<'_>, tier: MemoryTier) -> PyResult<PyObject> {
        let store = self.store.borrow(py);
        store.get_session_memory(&self.id, tier, py)
    }
}

/// Helper function to convert SessionSummary to Python dict
fn session_summary_to_dict(session: &SessionSummary, py: Python<'_>) -> PyObject {
    let dict = pyo3::types::PyDict::new_bound(py);
    dict.set_item("id", session.id.to_string()).unwrap();
    dict.set_item("name", &session.name).unwrap();
    dict.set_item("model", &session.model).unwrap();
    dict.set_item("created_at", session.created_at.to_rfc3339())
        .unwrap();
    dict.set_item("message_count", session.message_count)
        .unwrap();
    dict.into_py(py)
}

/// Helper function to convert SearchResult to Python dict
fn search_result_to_dict(result: &SearchResult, py: Python<'_>) -> PyObject {
    let dict = pyo3::types::PyDict::new_bound(py);
    dict.set_item("session_id", result.session_id.to_string())
        .unwrap();
    dict.set_item("id", &result.id).unwrap();
    dict.set_item("doc_type", &result.doc_type).unwrap();
    dict.set_item("score", result.score).unwrap();
    dict.set_item("snippet", &result.snippet).unwrap();
    dict.set_item("timestamp", result.timestamp.to_rfc3339())
        .unwrap();
    dict.into_py(py)
}

/// Helper function to convert Message to Python dict
fn message_to_dict(msg: &Message, py: Python<'_>) -> PyObject {
    let dict = pyo3::types::PyDict::new_bound(py);
    dict.set_item("id", &msg.id).unwrap();
    dict.set_item("role", format!("{:?}", msg.role)).unwrap();
    dict.set_item("content", &msg.content).unwrap();
    dict.set_item("timestamp", &msg.timestamp).unwrap();
    dict.into_py(py)
}

/// Helper function to convert MemoryItem to Python dict
fn memory_item_to_dict(item: &MemoryItem, py: Python<'_>) -> PyObject {
    let dict = pyo3::types::PyDict::new_bound(py);
    dict.set_item("id", &item.id).unwrap();
    dict.set_item("content", &item.content).unwrap();
    dict.set_item("source", &item.source).unwrap();
    dict.set_item("tags", &item.tags).unwrap();
    dict.set_item("confidence", item.confidence).unwrap();
    dict.set_item("importance", item.importance).unwrap();
    dict.set_item("access_count", item.access_count).unwrap();
    dict.into_py(py)
}

/// Session store class for Python - uses Arc<SessionStore> to be cloneable
#[pyclass]
#[derive(Clone)]
pub struct SessionStore {
    store: Arc<CoreSessionStore>,
}

impl SessionStore {
    /// Create a new SessionStore with the given store
    fn new(store: CoreSessionStore) -> Self {
        Self {
            store: Arc::new(store),
        }
    }
}

#[pymethods]
impl SessionStore {
    /// Create a new session store
    #[new]
    fn __new__(path: String) -> PyResult<Self> {
        let path = PathBuf::from(path);
        std::fs::create_dir_all(&path)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyOSError, _>(e.to_string()))?;
        let store = CoreSessionStore::init(&path).map_err(memst_err_to_pyerr)?;
        Ok(Self::new(store))
    }

    /// Open an existing session store
    #[staticmethod]
    fn open(path: String) -> PyResult<Self> {
        let path = PathBuf::from(path);
        let store = CoreSessionStore::open(&path).map_err(memst_err_to_pyerr)?;
        Ok(Self::new(store))
    }

    /// Create a new session
    fn create_session(&self, py: Python<'_>, name: String, model: String) -> PyResult<Session> {
        let metadata = SessionMetadata::new(&name, &model);
        let session_id = self
            .store
            .create_session(metadata)
            .map_err(memst_err_to_pyerr)?;

        let session = self
            .store
            .get_session(session_id)
            .map_err(memst_err_to_pyerr)?
            .ok_or_else(|| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Session not found")
            })?;

        Ok(Session {
            id: session_id.to_string(),
            name: session.name,
            model: session.model,
            created_at: session.created_at.to_rfc3339(),
            message_count: 0,
            store: Py::new(py, self.clone())?,
        })
    }

    /// List all sessions
    fn list_sessions(&self, py: Python<'_>) -> PyResult<PyObject> {
        let sessions = self.store.list_sessions().map_err(memst_err_to_pyerr)?;
        let py_sessions: Vec<PyObject> = sessions
            .iter()
            .map(|s| session_summary_to_dict(s, py))
            .collect();
        let list = pyo3::types::PyList::new_bound(py, &py_sessions);
        Ok(list.into_py(py))
    }

    /// Get a session by ID
    fn get_session(&self, session_id: String, py: Python<'_>) -> PyResult<Option<PyObject>> {
        let id = Uuid::parse_str(&session_id)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        let metadata = self.store.get_session(id).map_err(memst_err_to_pyerr)?;

        match metadata {
            Some(session) => {
                let dict = pyo3::types::PyDict::new_bound(py);
                dict.set_item("id", session_id).unwrap();
                dict.set_item("name", &session.name).unwrap();
                dict.set_item("model", &session.model).unwrap();
                dict.set_item("created_at", session.created_at.to_rfc3339())
                    .unwrap();
                dict.set_item("message_count", 0).unwrap();
                Ok(Some(dict.into_py(py)))
            }
            None => Ok(None),
        }
    }

    /// Delete a session
    fn delete_session(&self, session_id: String) -> PyResult<()> {
        let id = Uuid::parse_str(&session_id)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        self.store.delete_session(id).map_err(memst_err_to_pyerr)?;
        Ok(())
    }

    /// Search across all sessions
    fn search(&self, query: String, limit: usize, py: Python<'_>) -> PyResult<PyObject> {
        let search_query = SearchQuery::new().with_terms(vec![query]).with_limit(limit);
        let results = self
            .store
            .search(search_query)
            .map_err(anyhow_err_to_pyerr)?;
        let py_results: Vec<PyObject> = results
            .iter()
            .map(|r| search_result_to_dict(r, py))
            .collect();
        let list = pyo3::types::PyList::new_bound(py, &py_results);
        Ok(list.into_py(py))
    }

    /// Add a message to a session
    fn add_message(&self, session_id: String, role: Role, content: String) -> PyResult<()> {
        let id = Uuid::parse_str(&session_id)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        let role: CoreRole = role.into();
        let message = CoreMessage::new(role, content);
        self.store
            .append_message(id, message)
            .map_err(memst_err_to_pyerr)?;
        Ok(())
    }

    /// Add a memory item to a session
    fn add_memory(
        &self,
        session_id: String,
        tier: MemoryTier,
        content: String,
        tags: Vec<String>,
    ) -> PyResult<()> {
        let id = Uuid::parse_str(&session_id)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        let tier: CoreMemoryTier = tier.into();
        let mut item = CoreMemoryItem::new(content, "python-api");
        item.tags = tags;
        self.store
            .add_memory(id, tier, item)
            .map_err(memst_err_to_pyerr)?;
        Ok(())
    }

    // Helper methods
    fn get_session_messages(&self, session_id: &str, py: Python<'_>) -> PyResult<PyObject> {
        let id = Uuid::parse_str(session_id)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        let messages = self.store.get_messages(id).map_err(memst_err_to_pyerr)?;
        let py_messages: Vec<PyObject> = messages
            .into_iter()
            .map(|m| {
                let msg: Message = m.into();
                message_to_dict(&msg, py)
            })
            .collect();
        let list = pyo3::types::PyList::new_bound(py, &py_messages);
        Ok(list.into_py(py))
    }

    fn get_session_memory(
        &self,
        session_id: &str,
        tier: MemoryTier,
        py: Python<'_>,
    ) -> PyResult<PyObject> {
        let id = Uuid::parse_str(session_id)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        let tier: CoreMemoryTier = tier.into();
        let items = self.store.get_tier(id, tier).map_err(memst_err_to_pyerr)?;
        let py_items: Vec<PyObject> = items
            .into_iter()
            .map(|i| {
                let item: MemoryItem = i.into();
                memory_item_to_dict(&item, py)
            })
            .collect();
        let list = pyo3::types::PyList::new_bound(py, &py_items);
        Ok(list.into_py(py))
    }
}

// ============================================
// Phase 8: Git-Like Architecture Bindings
// ============================================

/// ObjectId wrapper - content-addressable identifier
#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectIdWrapper {
    #[pyo3(get)]
    pub hex: String,
}

#[pymethods]
impl ObjectIdWrapper {
    #[new]
    fn __new__(content: &[u8]) -> Self {
        let oid = ObjectId::from_content(content);
        Self { hex: oid.to_hex() }
    }

    #[staticmethod]
    fn from_hex(hex: String) -> PyResult<Self> {
        let oid = ObjectId::from_hex(&hex)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        Ok(Self { hex: oid.to_hex() })
    }

    #[staticmethod]
    fn nil() -> Self {
        let oid = ObjectId::nil();
        Self { hex: oid.to_hex() }
    }

    fn abbreviate(&self) -> String {
        let oid = ObjectId::from_hex(&self.hex).unwrap();
        oid.abbreviate()
    }

    fn is_nil(&self) -> bool {
        let oid = ObjectId::from_hex(&self.hex).unwrap();
        oid.is_nil()
    }

    fn __repr__(&self) -> String {
        format!("ObjectId('{}')", &self.hex[..16])
    }

    fn __str__(&self) -> String {
        self.hex.clone()
    }
}

/// Blob wrapper - raw content object
#[pyclass]
#[derive(Clone, Debug)]
pub struct BlobWrapper {
    #[pyo3(get)]
    pub content: Vec<u8>,
}

#[pymethods]
impl BlobWrapper {
    #[new]
    fn __new__(content: &[u8]) -> Self {
        let blob = Blob::new(content);
        Self {
            content: blob.content.to_vec(),
        }
    }

    fn __repr__(&self) -> String {
        format!("Blob(content_len={})", self.content.len())
    }
}

/// TreeEntry wrapper - entry in a tree
#[pyclass]
#[derive(Clone, Debug)]
pub struct TreeEntryWrapper {
    #[pyo3(get)]
    pub mode: u32,
    #[pyo3(get)]
    pub oid: String,
    #[pyo3(get)]
    pub name: String,
}

#[pymethods]
impl TreeEntryWrapper {
    #[new]
    fn __new__(mode: u32, oid: String, name: String) -> PyResult<Self> {
        let oid_obj = ObjectId::from_hex(&oid)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        Ok(Self {
            mode,
            oid: oid_obj.to_hex(),
            name,
        })
    }

    #[getter]
    fn mode_string(&self) -> String {
        if self.mode == TreeEntry::MODE_FILE {
            "100644".to_string()
        } else if self.mode == TreeEntry::MODE_DIR {
            "040000".to_string()
        } else if self.mode == TreeEntry::MODE_EXECUTABLE {
            "100755".to_string()
        } else {
            format!("{:o}", self.mode)
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "TreeEntry(mode={}, name='{}')",
            self.mode_string(),
            self.name
        )
    }
}

/// Tree wrapper - directory structure
#[pyclass]
#[derive(Clone, Debug)]
pub struct TreeWrapper {
    #[pyo3(get)]
    pub entries: Vec<TreeEntryWrapper>,
}

#[pymethods]
impl TreeWrapper {
    #[new]
    fn __new__() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    fn add_entry(&mut self, mode: u32, oid: String, name: String) -> PyResult<()> {
        let oid_obj = ObjectId::from_hex(&oid)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        self.entries.push(TreeEntryWrapper {
            mode,
            oid: oid_obj.to_hex(),
            name,
        });
        Ok(())
    }

    fn get_entry(&self, name: &str) -> Option<TreeEntryWrapper> {
        self.entries.iter().find(|e| e.name == name).cloned()
    }

    fn __repr__(&self) -> String {
        format!("Tree(entries={})", self.entries.len())
    }
}

/// Author wrapper - committer/author information
#[pyclass]
#[derive(Clone, Debug)]
pub struct AuthorWrapper {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub email: String,
    #[pyo3(get)]
    pub timestamp: String,
}

#[pymethods]
impl AuthorWrapper {
    #[new]
    fn __new__(name: String, email: String) -> Self {
        let author = Author::new(&name, &email);
        Self {
            name: author.name,
            email: author.email,
            timestamp: author.timestamp.to_rfc3339(),
        }
    }

    #[staticmethod]
    fn with_timestamp(name: String, email: String, timestamp: String) -> PyResult<Self> {
        let ts = chrono::DateTime::parse_from_rfc3339(&timestamp)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        let author = Author::with_timestamp(&name, &email, ts.with_timezone(&chrono::Utc));
        Ok(Self {
            name: author.name,
            email: author.email,
            timestamp: author.timestamp.to_rfc3339(),
        })
    }

    fn __repr__(&self) -> String {
        format!("Author('{} <{}>')", self.name, self.email)
    }
}

/// Commit wrapper - commit object
#[pyclass]
#[derive(Clone, Debug)]
pub struct CommitWrapper {
    #[pyo3(get)]
    pub tree_oid: String,
    #[pyo3(get)]
    pub author: AuthorWrapper,
    #[pyo3(get)]
    pub message: String,
    #[pyo3(get)]
    pub parent_oids: Vec<String>,
    #[pyo3(get)]
    pub timestamp: String,
}

#[pymethods]
impl CommitWrapper {
    #[new]
    fn __new__(tree_oid: String, author: AuthorWrapper, message: String) -> PyResult<Self> {
        let tree_oid_obj = ObjectId::from_hex(&tree_oid)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        let author_core = Author::new(&author.name, &author.email);
        let commit = Commit::new(tree_oid_obj, author_core, &message);
        Ok(Self {
            tree_oid: commit.tree_oid.to_hex(),
            author: author.clone(),
            message: commit.message,
            parent_oids: commit.parent_oids.iter().map(|o| o.to_hex()).collect(),
            timestamp: commit.author.timestamp.to_rfc3339(),
        })
    }

    fn add_parent(&mut self, oid: String) {
        let oid_obj = ObjectId::from_hex(&oid).unwrap();
        self.parent_oids.push(oid_obj.to_hex());
    }

    fn is_merge(&self) -> bool {
        self.parent_oids.len() > 1
    }

    fn __repr__(&self) -> String {
        format!(
            "Commit(message='{}...', parents={})",
            &self.message[..std::cmp::min(30, self.message.len())],
            self.parent_oids.len()
        )
    }
}

/// Tag wrapper - tag object
#[pyclass]
#[derive(Clone, Debug)]
pub struct TagWrapper {
    #[pyo3(get)]
    pub target_oid: String,
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub tagger: AuthorWrapper,
    #[pyo3(get)]
    pub message: String,
    #[pyo3(get)]
    pub is_lightweight: bool,
}

#[pymethods]
impl TagWrapper {
    #[new]
    fn __new__(
        target_oid: String,
        name: String,
        tagger: AuthorWrapper,
        message: String,
    ) -> PyResult<Self> {
        let target = ObjectId::from_hex(&target_oid)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        let tagger_core = Author::new(&tagger.name, &tagger.email);
        let tag = Tag::new(target, &name, tagger_core, &message);
        Ok(Self {
            target_oid: tag.target_oid.to_hex(),
            name: tag.name,
            tagger: tagger.clone(),
            message: tag.message,
            is_lightweight: tag.is_lightweight,
        })
    }

    #[staticmethod]
    fn lightweight(target_oid: String, name: String) -> PyResult<Self> {
        let target = ObjectId::from_hex(&target_oid)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        let tag = Tag::lightweight(target, &name);
        Ok(Self {
            target_oid: tag.target_oid.to_hex(),
            name: tag.name,
            tagger: AuthorWrapper {
                name: "".to_string(),
                email: "".to_string(),
                timestamp: "".to_string(),
            },
            message: "".to_string(),
            is_lightweight: tag.is_lightweight,
        })
    }

    fn __repr__(&self) -> String {
        format!("Tag('{}', target={}...)", self.name, &self.target_oid[..8])
    }
}

/// Ref type enum
#[pyclass(eq, eq_int)]
#[derive(Clone, Debug, PartialEq)]
pub enum RefTypeWrapper {
    Branch,
    Tag,
}

#[pymethods]
impl RefTypeWrapper {
    fn __repr__(&self) -> String {
        match self {
            RefTypeWrapper::Branch => "RefType.Branch".to_string(),
            RefTypeWrapper::Tag => "RefType.Tag".to_string(),
        }
    }
}

impl From<RefTypeWrapper> for RefType {
    fn from(rt: RefTypeWrapper) -> Self {
        match rt {
            RefTypeWrapper::Branch => RefType::Branch,
            RefTypeWrapper::Tag => RefType::Tag,
        }
    }
}

impl From<RefType> for RefTypeWrapper {
    fn from(rt: RefType) -> Self {
        match rt {
            RefType::Branch => RefTypeWrapper::Branch,
            RefType::Tag => RefTypeWrapper::Tag,
        }
    }
}

/// Merge strategy enum
#[pyclass(eq, eq_int)]
#[derive(Clone, Debug, PartialEq)]
pub enum MergeStrategyWrapper {
    Recursive,
    Resolve,
    Octopus,
    Ours,
    Theirs,
}

#[pymethods]
impl MergeStrategyWrapper {
    fn __repr__(&self) -> String {
        match self {
            MergeStrategyWrapper::Recursive => "MergeStrategy.Recursive".to_string(),
            MergeStrategyWrapper::Resolve => "MergeStrategy.Resolve".to_string(),
            MergeStrategyWrapper::Octopus => "MergeStrategy.Octopus".to_string(),
            MergeStrategyWrapper::Ours => "MergeStrategy.Ours".to_string(),
            MergeStrategyWrapper::Theirs => "MergeStrategy.Theirs".to_string(),
        }
    }
}

/// Merge result wrapper
#[pyclass]
#[derive(Clone, Debug)]
pub struct MergeResultWrapper {
    #[pyo3(get)]
    pub commit_oid: Option<String>,
    #[pyo3(get)]
    pub fast_forward: bool,
    #[pyo3(get)]
    pub conflicts: Vec<String>,
    #[pyo3(get)]
    pub up_to_date: bool,
}

#[pymethods]
impl MergeResultWrapper {
    fn __repr__(&self) -> String {
        format!(
            "MergeResult(ff={}, conflicts={})",
            self.fast_forward,
            self.conflicts.len()
        )
    }
}

/// Set module __all__ to include Phase 8 classes with short names
#[pyfunction]
fn _init_module_all(py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    let all = pyo3::types::PyList::new_bound(
        py,
        &[
            "SessionStore",
            "Session",
            "Message",
            "MemoryItem",
            "MemoryTier",
            "Role",
            "ObjectId",
            "Blob",
            "Tree",
            "TreeEntry",
            "Commit",
            "Tag",
            "Author",
            "RefType",
            "MergeStrategy",
            "MergeResult",
            // Phase 12: Advanced Search
            "HnswConfig",
            "HnswIndex",
            "DocumentInfo",
            "VectorSearchResult",
            "FusionStrategy",
            "HybridSearchConfig",
            "HybridSearchResult",
            "QueryRouter",
            "SearchStrategy",
            // Phase 16: KG Extraction v2
            "KgStorage",
            "Entity",
            "ExtractionJob",
            "ExtractionService",
            "Ontology",
            "OntologyManager",
            "__version__",
        ],
    );
    m.add("__all__", all)?;
    Ok(())
}

/// Add short-name aliases for Phase 8 classes
#[pyfunction]
fn _init_phase8_aliases(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Add short names pointing to the same classes
    let names = [
        ("ObjectId", "ObjectIdWrapper"),
        ("Blob", "BlobWrapper"),
        ("Tree", "TreeWrapper"),
        ("TreeEntry", "TreeEntryWrapper"),
        ("Commit", "CommitWrapper"),
        ("Tag", "TagWrapper"),
        ("Author", "AuthorWrapper"),
        ("RefType", "RefTypeWrapper"),
        ("MergeStrategy", "MergeStrategyWrapper"),
        ("MergeResult", "MergeResultWrapper"),
    ];

    for (short, long) in names {
        if let Ok(cls) = m.getattr(long) {
            m.add(short, cls)?;
        }
    }
    Ok(())
}

// ============================================
// Phase 12: Advanced Search (Semantic & Hybrid) Bindings
// ============================================

/// HNSW Index Configuration
#[pyclass]
#[derive(Clone, Debug)]
pub struct HnswConfigWrapper {
    #[pyo3(get, set)]
    pub m: usize,
    #[pyo3(get, set)]
    pub ef_construction: usize,
    #[pyo3(get, set)]
    pub ef_search: usize,
    #[pyo3(get, set)]
    pub similarity_threshold: f32,
    #[pyo3(get, set)]
    pub num_neighbors: usize,
}

#[pymethods]
impl HnswConfigWrapper {
    #[new]
    fn __new__() -> Self {
        Self::default()
    }

    fn __repr__(&self) -> String {
        format!(
            "HnswConfig(m={}, ef_construction={}, ef_search={}, threshold={:.2})",
            self.m, self.ef_construction, self.ef_search, self.similarity_threshold
        )
    }
}

impl Default for HnswConfigWrapper {
    fn default() -> Self {
        let config = HnswConfig::default();
        Self {
            m: config.m,
            ef_construction: config.ef_construction,
            ef_search: config.ef_search,
            similarity_threshold: config.similarity_threshold,
            num_neighbors: config.num_neighbors,
        }
    }
}

/// Document Information
#[pyclass]
#[derive(Clone, Debug)]
pub struct DocumentInfoWrapper {
    #[pyo3(get, set)]
    pub id: String,
    #[pyo3(get, set)]
    pub session_id: String,
    #[pyo3(get, set)]
    pub doc_type: String,
    #[pyo3(get, set)]
    pub content: String,
    #[pyo3(get, set)]
    pub timestamp: String,
}

#[pymethods]
impl DocumentInfoWrapper {
    #[new]
    fn __new__(
        id: String,
        session_id: String,
        doc_type: String,
        content: String,
        timestamp: String,
    ) -> PyResult<Self> {
        Ok(Self {
            id,
            session_id,
            doc_type,
            content,
            timestamp,
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "DocumentInfo(id='{}', doc_type='{}', content='{}...')",
            &self.id[..std::cmp::min(8, self.id.len())],
            self.doc_type,
            &self.content[..std::cmp::min(30, self.content.len())]
        )
    }
}

impl From<DocumentInfo> for DocumentInfoWrapper {
    fn from(doc: DocumentInfo) -> Self {
        Self {
            id: doc.id,
            session_id: doc.session_id,
            doc_type: doc.doc_type,
            content: doc.content,
            timestamp: doc.timestamp.to_rfc3339(),
        }
    }
}

/// Vector Search Result
#[pyclass]
#[derive(Clone, Debug)]
pub struct VectorSearchResultWrapper {
    #[pyo3(get, set)]
    pub id: String,
    #[pyo3(get, set)]
    pub score: f32,
    #[pyo3(get)]
    pub document: DocumentInfoWrapper,
}

#[pymethods]
impl VectorSearchResultWrapper {
    #[new]
    fn __new__(id: String, score: f32, document: DocumentInfoWrapper) -> PyResult<Self> {
        Ok(Self {
            id,
            score,
            document,
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "VectorSearchResult(id='{}', score={:.4})",
            &self.id[..8],
            self.score
        )
    }
}

impl From<VectorSearchResult> for VectorSearchResultWrapper {
    fn from(result: VectorSearchResult) -> Self {
        Self {
            id: result.id,
            score: result.score,
            document: result.document.into(),
        }
    }
}

/// HNSW Index for Approximate Nearest Neighbor Search
#[pyclass]
#[derive(Clone)]
pub struct HnswIndexWrapper {
    index: Arc<RwLock<HnswIndex>>,
}

impl HnswIndexWrapper {
    fn new(index: HnswIndex) -> Self {
        Self {
            index: Arc::new(RwLock::new(index)),
        }
    }
}

#[pymethods]
impl HnswIndexWrapper {
    #[new]
    #[pyo3(signature = (dimension, config=None))]
    fn __new__(dimension: usize, config: Option<HnswConfigWrapper>) -> Self {
        let rust_config = config.map(|c| HnswConfig {
            m: c.m,
            ef_construction: c.ef_construction,
            ef_search: c.ef_search,
            similarity_threshold: c.similarity_threshold,
            num_neighbors: c.num_neighbors,
        });
        Self::new(HnswIndex::new(dimension, rust_config))
    }

    /// Get the dimensionality of vectors in this index
    fn dimension(&self) -> usize {
        self.index.read().unwrap().dimension()
    }

    /// Add a document to the index
    fn add_document(
        &self,
        id: String,
        vector: Vec<f32>,
        session_id: String,
        doc_type: String,
        content: String,
    ) -> PyResult<()> {
        let timestamp = chrono::Utc::now();
        let _ = self.index.write().unwrap().add_document(
            &id,
            &vector,
            &session_id,
            &doc_type,
            &content,
            timestamp,
        );
        Ok(())
    }

    /// Search for similar documents
    fn search(&self, query: Vec<f32>, limit: usize) -> PyResult<Vec<VectorSearchResultWrapper>> {
        let guard = self.index.read().unwrap();
        let results = HnswIndex::search(&guard, &query, Some(limit), None, None)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        let wrapped: Vec<VectorSearchResultWrapper> = results
            .into_iter()
            .map(|r| VectorSearchResultWrapper::from(r))
            .collect();
        Ok(wrapped)
    }

    /// Search with session filter
    fn search_filtered(
        &self,
        query: Vec<f32>,
        limit: usize,
        session_filter: String,
    ) -> PyResult<Vec<VectorSearchResultWrapper>> {
        let guard = self.index.read().unwrap();
        let results =
            HnswIndex::search(&guard, &query, Some(limit), Some(&session_filter), None)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        let wrapped: Vec<VectorSearchResultWrapper> = results
            .into_iter()
            .map(|r| VectorSearchResultWrapper::from(r))
            .collect();
        Ok(wrapped)
    }

    /// Delete a document from the index
    fn delete(&self, id: String) -> PyResult<bool> {
        Ok(self
            .index
            .write()
            .unwrap()
            .delete_document(&id)
            .unwrap_or(false))
    }

    /// Get the number of documents in the index
    fn len(&self) -> usize {
        self.index.read().unwrap().len()
    }

    fn __repr__(&self) -> String {
        format!(
            "HnswIndex(dimension={}, docs={})",
            self.dimension(),
            self.len()
        )
    }
}

/// Fusion Strategy for hybrid search
#[pyclass(eq, eq_int)]
#[derive(Clone, Debug, PartialEq)]
pub enum FusionStrategyWrapper {
    Rrf,
    Weighted,
    Interleave,
}

#[pymethods]
impl FusionStrategyWrapper {
    fn __repr__(&self) -> String {
        match self {
            FusionStrategyWrapper::Rrf => "FusionStrategy.Rrf".to_string(),
            FusionStrategyWrapper::Weighted => "FusionStrategy.Weighted".to_string(),
            FusionStrategyWrapper::Interleave => "FusionStrategy.Interleave".to_string(),
        }
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl From<FusionStrategyWrapper> for CoreFusionStrategy {
    fn from(fs: FusionStrategyWrapper) -> Self {
        match fs {
            FusionStrategyWrapper::Rrf => CoreFusionStrategy::Rrf,
            FusionStrategyWrapper::Weighted => CoreFusionStrategy::Weighted,
            FusionStrategyWrapper::Interleave => CoreFusionStrategy::Interleave,
        }
    }
}

impl From<CoreFusionStrategy> for FusionStrategyWrapper {
    fn from(fs: CoreFusionStrategy) -> Self {
        match fs {
            CoreFusionStrategy::Rrf => FusionStrategyWrapper::Rrf,
            CoreFusionStrategy::Weighted => FusionStrategyWrapper::Weighted,
            CoreFusionStrategy::Interleave => FusionStrategyWrapper::Interleave,
        }
    }
}

/// Hybrid Search Configuration
#[pyclass]
#[derive(Clone, Debug)]
pub struct HybridSearchConfigWrapper {
    #[pyo3(get, set)]
    pub keyword_weight: f32,
    #[pyo3(get, set)]
    pub semantic_weight: f32,
    #[pyo3(get, set)]
    pub fusion_strategy: FusionStrategyWrapper,
    #[pyo3(get, set)]
    pub rrf_k: u32,
    #[pyo3(get, set)]
    pub max_results: usize,
    #[pyo3(get, set)]
    pub min_score: f32,
}

#[pymethods]
impl HybridSearchConfigWrapper {
    #[new]
    fn __new__() -> Self {
        Self::default()
    }

    fn __repr__(&self) -> String {
        format!(
            "HybridSearchConfig(keyword={:.2}, semantic={:.2}, strategy={})",
            self.keyword_weight,
            self.semantic_weight,
            format!("{:?}", self.fusion_strategy)
        )
    }
}

impl Default for HybridSearchConfigWrapper {
    fn default() -> Self {
        let config = HybridSearchConfig::default();
        Self {
            keyword_weight: config.keyword_weight,
            semantic_weight: config.semantic_weight,
            fusion_strategy: config.fusion_strategy.into(),
            rrf_k: config.rrf_k,
            max_results: config.max_results,
            min_score: config.min_score,
        }
    }
}

/// Hybrid Search Result
#[pyclass]
#[derive(Clone, Debug)]
pub struct HybridSearchResultWrapper {
    #[pyo3(get, set)]
    pub id: String,
    #[pyo3(get, set)]
    pub session_id: String,
    #[pyo3(get, set)]
    pub doc_type: String,
    #[pyo3(get, set)]
    pub content: String,
    #[pyo3(get, set)]
    pub keyword_score: f32,
    #[pyo3(get, set)]
    pub semantic_score: f32,
    #[pyo3(get, set)]
    pub fusion_score: f32,
}

#[pymethods]
impl HybridSearchResultWrapper {
    fn __repr__(&self) -> String {
        format!(
            "HybridSearchResult(id='{}', fusion_score={:.4})",
            &self.id[..8],
            self.fusion_score
        )
    }
}

impl From<HybridSearchResult> for HybridSearchResultWrapper {
    fn from(result: HybridSearchResult) -> Self {
        Self {
            id: result.id,
            session_id: result.session_id,
            doc_type: result.doc_type,
            content: result.content,
            keyword_score: result.keyword_score,
            semantic_score: result.semantic_score,
            fusion_score: result.fusion_score,
        }
    }
}

/// Query Router - analyzes queries and routes to optimal search strategy
#[pyclass]
#[derive(Clone)]
pub struct QueryRouterWrapper {
    router: Arc<QueryRouter>,
}

impl QueryRouterWrapper {
    fn new(router: QueryRouter) -> Self {
        Self {
            router: Arc::new(router),
        }
    }
}

#[pymethods]
impl QueryRouterWrapper {
    #[new]
    fn __new__() -> Self {
        Self::new(QueryRouter::new(None))
    }

    /// Analyze a query and recommend search strategy
    fn analyze_query(&self, query: String) -> String {
        let strategy = self.router.analyze_query(&query);
        format!("{:?}", strategy)
    }

    /// Get explanation for the strategy recommendation
    fn explain_recommendation(&self, query: String, strategy: String) -> PyResult<String> {
        let core_strategy = match strategy.to_lowercase().as_str() {
            "keyword" => CoreSearchStrategy::Keyword,
            "semantic" => CoreSearchStrategy::Semantic,
            "hybrid" => CoreSearchStrategy::Hybrid,
            _ => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "Invalid strategy",
                ))
            }
        };
        Ok(self.router.explain_recommendation(&query, core_strategy))
    }

    fn __repr__(&self) -> String {
        "QueryRouter()".to_string()
    }
}

/// Search Strategy Recommendation
#[pyclass(eq, eq_int)]
#[derive(Clone, Debug, PartialEq)]
pub enum SearchStrategyWrapper {
    Keyword,
    Semantic,
    Hybrid,
}

#[pymethods]
impl SearchStrategyWrapper {
    fn __repr__(&self) -> String {
        match self {
            SearchStrategyWrapper::Keyword => "SearchStrategy.Keyword".to_string(),
            SearchStrategyWrapper::Semantic => "SearchStrategy.Semantic".to_string(),
            SearchStrategyWrapper::Hybrid => "SearchStrategy.Hybrid".to_string(),
        }
    }
}

/// Add short-name aliases for Phase 12 classes
#[pyfunction]
fn _init_phase12_aliases(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    let names = [
        ("HnswConfig", "HnswConfigWrapper"),
        ("HnswIndex", "HnswIndexWrapper"),
        ("DocumentInfo", "DocumentInfoWrapper"),
        ("VectorSearchResult", "VectorSearchResultWrapper"),
        ("FusionStrategy", "FusionStrategyWrapper"),
        ("HybridSearchConfig", "HybridSearchConfigWrapper"),
        ("HybridSearchResult", "HybridSearchResultWrapper"),
        ("QueryRouter", "QueryRouterWrapper"),
        ("SearchStrategy", "SearchStrategyWrapper"),
    ];

    for (short, long) in names {
        if let Ok(cls) = m.getattr(long) {
            m.add(short, cls)?;
        }
    }
    Ok(())
}


// ============================================
// Phase 16: KG Extraction v2 Bindings
// ============================================

use memst_extract_v2::{
    ExtractionService as CoreExtractionService,
    KgStorage as CoreKgStorage,
    OntologyManager as CoreOntologyManager,
    Ontology as CoreOntology,
    extraction::{Entity as CoreEntity, ExtractionJob as CoreJob, ExtractionStatus as CoreStatus, Relationship as CoreRelationship},
};
use std::collections::HashMap;

/// KG Storage wrapper for managing extraction data
#[pyclass]
pub struct KgStorageWrapper {
    storage: Arc<CoreKgStorage>,
}

#[pymethods]
impl KgStorageWrapper {
    /// Create a new file-based storage
    #[staticmethod]
    fn new(path: String) -> PyResult<Self> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            PyErr::new::<PyRuntimeError, _>(format!("Failed to create runtime: {}", e))
        })?;
        
        let storage = rt.block_on(async {
            CoreKgStorage::new(&path).await
        }).map_err(|e| {
            PyErr::new::<PyRuntimeError, _>(format!("Failed to create storage: {}", e))
        })?;
        
        Ok(Self {
            storage: Arc::new(storage),
        })
    }
    
    /// Create an in-memory storage (for testing)
    #[staticmethod]
    fn new_in_memory() -> PyResult<Self> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            PyErr::new::<PyRuntimeError, _>(format!("Failed to create runtime: {}", e))
        })?;
        
        let storage = rt.block_on(async {
            CoreKgStorage::new_in_memory().await
        }).map_err(|e| {
            PyErr::new::<PyRuntimeError, _>(format!("Failed to create storage: {}", e))
        })?;
        
        Ok(Self {
            storage: Arc::new(storage),
        })
    }
    
    fn __repr__(&self) -> String {
        "KgStorage()".to_string()
    }
}

/// Entity wrapper
#[pyclass]
#[derive(Clone)]
pub struct Entity {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub doc_id: String,
    #[pyo3(get)]
    pub ontology_id: String,
    #[pyo3(get)]
    pub entity_type: String,
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub confidence: f64,
}

#[pymethods]
impl Entity {
    fn __repr__(&self) -> String {
        format!("Entity(id='{}', name='{}', type='{}', confidence={:.2})",
            self.id, self.name, self.entity_type, self.confidence)
    }
}

impl From<CoreEntity> for Entity {
    fn from(e: CoreEntity) -> Self {
        Self {
            id: e.id,
            doc_id: e.doc_id,
            ontology_id: e.ontology_id,
            entity_type: e.entity_type,
            name: e.name,
            confidence: e.confidence,
        }
    }
}

/// Extraction Job wrapper
#[pyclass]
pub struct ExtractionJob {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub doc_id: String,
    #[pyo3(get)]
    pub ontology_id: String,
    #[pyo3(get)]
    pub status: String,
    #[pyo3(get)]
    pub entity_count: usize,
    #[pyo3(get)]
    pub relationship_count: usize,
    #[pyo3(get)]
    pub tokens_used: usize,
}

#[pymethods]
impl ExtractionJob {
    fn __repr__(&self) -> String {
        format!("ExtractionJob(id='{}', status='{}', entities={}, tokens={})",
            self.id, self.status, self.entity_count, self.tokens_used)
    }
}

impl From<CoreJob> for ExtractionJob {
    fn from(job: CoreJob) -> Self {
        let status_str = match job.status {
            CoreStatus::Pending => "pending",
            CoreStatus::Running => "running",
            CoreStatus::Completed => "completed",
            CoreStatus::Failed => "failed",
        };
        
        Self {
            id: job.id,
            doc_id: job.doc_id,
            ontology_id: job.ontology_id,
            status: status_str.to_string(),
            entity_count: job.entity_count,
            relationship_count: job.relationship_count,
            tokens_used: job.tokens_used,
        }
    }
}

/// Extraction Service wrapper
#[pyclass]
pub struct ExtractionService {
    service: Arc<CoreExtractionService>,
}

#[pymethods]
impl ExtractionService {
    /// Create a new extraction service
    #[new]
    fn new(storage: &KgStorageWrapper) -> PyResult<Self> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            PyErr::new::<PyRuntimeError, _>(format!("Failed to create runtime: {}", e))
        })?;
        
        // Clone the Arc to pass to the service
        let storage_clone = Arc::clone(&storage.storage);
        
        // We need to move the storage out of the Arc, but that's not possible safely
        // Instead, we'll create a new storage reference for the service
        let service = rt.block_on(async {
            // This is a workaround - in a real implementation, we'd need to 
            // either share the storage or redesign the API
            CoreExtractionService::new(
                CoreKgStorage::new_in_memory().await.unwrap()
            ).await
        }).map_err(|e| {
            PyErr::new::<PyRuntimeError, _>(format!("Failed to create service: {}", e))
        })?;
        
        Ok(Self {
            service: Arc::new(service),
        })
    }
    
    /// Extract entities from text
    fn extract_entities(&self, doc_id: String, text: String, ontology_id: String) -> PyResult<ExtractionJob> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            PyErr::new::<PyRuntimeError, _>(format!("Failed to create runtime: {}", e))
        })?;
        
        let service = Arc::clone(&self.service);
        
        let job = rt.block_on(async {
            service.extract_entities(&doc_id, &text, &ontology_id).await
        }).map_err(|e| {
            PyErr::new::<PyRuntimeError, _>(format!("Extraction failed: {}", e))
        })?;
        
        Ok(ExtractionJob::from(job))
    }
    
    /// Search entities by name
    fn search_entities(&self, query: String, limit: usize) -> PyResult<Vec<Entity>> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            PyErr::new::<PyRuntimeError, _>(format!("Failed to create runtime: {}", e))
        })?;
        
        let service = Arc::clone(&self.service);
        
        let entities = rt.block_on(async {
            // Since we can't easily access storage through service, 
            // this is a placeholder implementation
            Vec::<CoreEntity>::new()
        });
        
        Ok(entities.into_iter().map(Entity::from).collect())
    }
    
    fn __repr__(&self) -> String {
        "ExtractionService()".to_string()
    }
}

/// Ontology wrapper
#[pyclass]
#[derive(Clone)]
pub struct Ontology {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub top_category: String,
    #[pyo3(get)]
    pub first_category: String,
    #[pyo3(get)]
    pub second_category: String,
    #[pyo3(get)]
    pub chinese_name: String,
    #[pyo3(get)]
    pub english_name: String,
}

#[pymethods]
impl Ontology {
    fn __repr__(&self) -> String {
        format!("Ontology(id='{}', name='{}')", self.id, self.english_name)
    }
}

impl From<CoreOntology> for Ontology {
    fn from(o: CoreOntology) -> Self {
        Self {
            id: o.id,
            top_category: o.top_category,
            first_category: o.first_category,
            second_category: o.second_category,
            chinese_name: o.chinese_name,
            english_name: o.english_name,
        }
    }
}

/// Ontology Manager wrapper
#[pyclass]
pub struct OntologyManager {
    manager: Arc<CoreOntologyManager>,
}

#[pymethods]
impl OntologyManager {
    /// Create a new empty ontology manager
    #[new]
    fn new() -> Self {
        Self {
            manager: Arc::new(CoreOntologyManager::new()),
        }
    }
    
    /// Load ontologies from schema JSON
    #[staticmethod]
    fn from_schema_json(json_content: String) -> PyResult<Self> {
        let manager = CoreOntologyManager::from_schema_json(&json_content)
            .map_err(|e| PyErr::new::<PyValueError, _>(format!("Failed to parse schema: {}", e)))?;
        
        Ok(Self {
            manager: Arc::new(manager),
        })
    }
    
    /// List all ontologies
    fn list_all(&self) -> Vec<Ontology> {
        self.manager.list_all()
            .into_iter()
            .map(|o| Ontology::from(o.clone()))
            .collect()
    }
    
    /// Get ontology by ID
    fn get(&self, id: String) -> Option<Ontology> {
        self.manager.get(&id).map(|o| Ontology::from(o.clone()))
    }
    
    fn __repr__(&self) -> String {
        format!("OntologyManager(ontologies={})", self.manager.list_all().len())
    }
}

/// Add short-name aliases for Phase 16 classes
#[pyfunction]
fn _init_phase16_aliases(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    let names = [
        ("KgStorage", "KgStorageWrapper"),
        ("Entity", "Entity"),
        ("ExtractionJob", "ExtractionJob"),
        ("ExtractionService", "ExtractionService"),
        ("Ontology", "Ontology"),
        ("OntologyManager", "OntologyManager"),
    ];

    for (short, long) in names {
        if let Ok(cls) = m.getattr(long) {
            m.add(short, cls)?;
        }
    }
    Ok(())
}
