//! Full-Text Search Module
//!
//! This module provides full-text search capabilities for MemSt with
//! pluggable backends. The default backend is a Rust-native inverted index,
//! while an optional Tantivy backend provides advanced features.
//!
//! # Backends
//!
//! - **Native Backend** (`native-backend` feature): A custom Rust inverted index
//!   with BM25 scoring. Lightweight with no external dependencies.
//!
//! - **Tantivy Backend** (`tantivy-backend` feature): Integration with Tantivy
//!   search engine providing phrase search, fuzzy matching, regex queries,
//!   and more sophisticated scoring.
//!
//! # Usage
//!
//! ```rust,no_run
//! use anyhow::Result;
//! use memst_core::search::{SearchIndex, SearchIndexBackend, SearchQuery};
//! use std::path::PathBuf;
//! use uuid::Uuid;
//!
//! # fn main() -> Result<()> {
//! // Create a search index on disk.
//! let path: PathBuf = std::env::temp_dir().join(format!("memst-search-doctest-{}", Uuid::new_v4()));
//! std::fs::create_dir_all(&path)?;
//! let index = SearchIndex::new(&path)?;
//!
//! // Add one message document.
//! let session_id = Uuid::new_v4();
//! let message_id = Uuid::new_v4();
//! let timestamp = chrono::Utc::now();
//! index.add_message(session_id, message_id, "Hello world", "user", timestamp)?;
//!
//! // Search
//! let results = index.search(SearchQuery::new().with_term("hello"))?;
//! assert!(!results.is_empty());
//! # Ok(())
//! # }
//! ```

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

// Re-export backends
#[cfg(feature = "native-backend")]
pub use native::NativeSearchIndex;

#[cfg(feature = "tantivy-backend")]
pub use tantivy::TantivySearchIndex;

#[cfg(feature = "native-backend")]
mod native;

#[cfg(feature = "tantivy-backend")]
mod tantivy;

/// Search result with BM25 relevance score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Document ID
    pub id: String,
    /// Session ID
    pub session_id: Uuid,
    /// Document type (message, memory)
    pub doc_type: String,
    /// Relevance score
    pub score: f32,
    /// Snippet/preview of the content
    pub snippet: String,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Search query options.
#[derive(Debug, Clone, Default)]
pub struct SearchQuery {
    /// Search terms (will be tokenized by backend)
    pub terms: Vec<String>,
    /// Session ID filter (None = all sessions)
    pub session_id: Option<Uuid>,
    /// Document type filter (message, memory)
    pub doc_type: Option<String>,
    /// Maximum results
    pub limit: usize,
    /// Minimum score threshold (0.0 - 1.0)
    pub min_score: f32,
}

impl SearchQuery {
    /// Create a new query.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add search term.
    pub fn with_term(mut self, term: impl Into<String>) -> Self {
        self.terms.push(term.into());
        self
    }

    /// Filter by session.
    pub fn with_session(mut self, session_id: Uuid) -> Self {
        self.session_id = Some(session_id);
        self
    }

    /// Filter by document type.
    pub fn with_doc_type(mut self, doc_type: impl Into<String>) -> Self {
        self.doc_type = Some(doc_type.into());
        self
    }

    /// Set result limit.
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Set minimum score threshold.
    pub fn with_min_score(mut self, min_score: f32) -> Self {
        self.min_score = min_score;
        self
    }

    /// Set search terms (replaces existing).
    pub fn with_terms(mut self, terms: Vec<String>) -> Self {
        self.terms = terms;
        self
    }
}

/// Backend-agnostic search index trait.
/// All search backends implement this trait.
pub trait SearchIndexBackend: Send + Sync {
    /// Create a new search index.
    fn new(path: &PathBuf) -> Result<Self>
    where
        Self: Sized;

    /// Open an existing search index.
    fn open(path: &PathBuf) -> Result<Self>
    where
        Self: Sized;

    /// Index a message document.
    fn add_message(
        &self,
        session_id: Uuid,
        message_id: Uuid,
        content: &str,
        role: &str,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> Result<()>;

    /// Index a memory document.
    fn add_memory(
        &self,
        session_id: Uuid,
        memory_id: Uuid,
        content: &str,
        tags: &[String],
        confidence: f32,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> Result<()>;

    /// Search the index.
    fn search(&self, query: SearchQuery) -> Result<Vec<SearchResult>>;

    /// Delete all documents for a session.
    fn delete_session(&self, session_id: Uuid) -> Result<()>;

    /// Commit pending changes.
    fn commit(&self) -> Result<()>;

    /// Get the number of indexed documents.
    fn count(&self) -> u64;

    /// Get the number of indexed terms.
    fn term_count(&self) -> usize;
}

/// Unified search index that can use different backends.
#[derive(Debug)]
pub enum SearchIndex {
    #[cfg(feature = "native-backend")]
    /// Native in-memory search backend.
    Native(NativeSearchIndex),
    #[cfg(feature = "tantivy-backend")]
    /// Tantivy full-text search backend.
    Tantivy(TantivySearchIndex),
}

impl SearchIndex {
    /// Create a new native backend index.
    #[cfg(feature = "native-backend")]
    pub fn native_new(path: &PathBuf) -> Result<Self> {
        NativeSearchIndex::new(path).map(SearchIndex::Native)
    }

    /// Create a new Tantivy backend index.
    #[cfg(feature = "tantivy-backend")]
    pub fn tantivy_new(path: &PathBuf) -> Result<Self> {
        TantivySearchIndex::new(path).map(SearchIndex::Tantivy)
    }

    /// Create a new index with the best available backend.
    pub fn new(path: &PathBuf) -> Result<Self>
    where
        Self: Sized,
    {
        #[cfg(all(feature = "tantivy-backend", feature = "native-backend"))]
        {
            // Prefer Tantivy when both are available
            return Self::tantivy_new(path);
        }

        #[cfg(all(feature = "tantivy-backend", not(feature = "native-backend")))]
        {
            return Self::tantivy_new(path);
        }

        #[cfg(all(feature = "native-backend", not(feature = "tantivy-backend")))]
        {
            return Self::native_new(path);
        }

        #[cfg(not(any(feature = "native-backend", feature = "tantivy-backend")))]
        {
            bail!(
                "No search backend enabled. Enable 'native-backend' or 'tantivy-backend' feature."
            );
        }
    }

    /// Open an existing index.
    pub fn open(path: &PathBuf) -> Result<Self>
    where
        Self: Sized,
    {
        #[cfg(all(feature = "tantivy-backend", feature = "native-backend"))]
        {
            return Self::tantivy_new(path);
        }

        #[cfg(all(feature = "tantivy-backend", not(feature = "native-backend")))]
        {
            return Self::tantivy_new(path);
        }

        #[cfg(all(feature = "native-backend", not(feature = "tantivy-backend")))]
        {
            return Self::native_new(path);
        }

        #[cfg(not(any(feature = "native-backend", feature = "tantivy-backend")))]
        {
            bail!(
                "No search backend enabled. Enable 'native-backend' or 'tantivy-backend' feature."
            );
        }
    }

    /// Get the underlying backend type.
    pub fn backend_type(&self) -> &'static str {
        match self {
            #[cfg(feature = "native-backend")]
            SearchIndex::Native(_) => "native",
            #[cfg(feature = "tantivy-backend")]
            SearchIndex::Tantivy(_) => "tantivy",
        }
    }
}

impl SearchIndexBackend for SearchIndex {
    fn new(path: &PathBuf) -> Result<Self>
    where
        Self: Sized,
    {
        Self::new(path)
    }

    fn open(path: &PathBuf) -> Result<Self>
    where
        Self: Sized,
    {
        Self::open(path)
    }

    fn add_message(
        &self,
        session_id: Uuid,
        message_id: Uuid,
        content: &str,
        role: &str,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> Result<()> {
        match self {
            #[cfg(feature = "native-backend")]
            SearchIndex::Native(idx) => {
                idx.add_message(session_id, message_id, content, role, timestamp)
            }
            #[cfg(feature = "tantivy-backend")]
            SearchIndex::Tantivy(idx) => {
                idx.add_message(session_id, message_id, content, role, timestamp)
            }
        }
    }

    fn add_memory(
        &self,
        session_id: Uuid,
        memory_id: Uuid,
        content: &str,
        tags: &[String],
        confidence: f32,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> Result<()> {
        match self {
            #[cfg(feature = "native-backend")]
            SearchIndex::Native(idx) => {
                idx.add_memory(session_id, memory_id, content, tags, confidence, timestamp)
            }
            #[cfg(feature = "tantivy-backend")]
            SearchIndex::Tantivy(idx) => {
                idx.add_memory(session_id, memory_id, content, tags, confidence, timestamp)
            }
        }
    }

    fn search(&self, query: SearchQuery) -> Result<Vec<SearchResult>> {
        match self {
            #[cfg(feature = "native-backend")]
            SearchIndex::Native(idx) => idx.search(query),
            #[cfg(feature = "tantivy-backend")]
            SearchIndex::Tantivy(idx) => idx.search(query),
        }
    }

    fn delete_session(&self, session_id: Uuid) -> Result<()> {
        match self {
            #[cfg(feature = "native-backend")]
            SearchIndex::Native(idx) => idx.delete_session(session_id),
            #[cfg(feature = "tantivy-backend")]
            SearchIndex::Tantivy(idx) => idx.delete_session(session_id),
        }
    }

    fn commit(&self) -> Result<()> {
        match self {
            #[cfg(feature = "native-backend")]
            SearchIndex::Native(idx) => idx.commit(),
            #[cfg(feature = "tantivy-backend")]
            SearchIndex::Tantivy(idx) => idx.commit(),
        }
    }

    fn count(&self) -> u64 {
        match self {
            #[cfg(feature = "native-backend")]
            SearchIndex::Native(idx) => idx.count(),
            #[cfg(feature = "tantivy-backend")]
            SearchIndex::Tantivy(idx) => idx.count(),
        }
    }

    fn term_count(&self) -> usize {
        match self {
            #[cfg(feature = "native-backend")]
            SearchIndex::Native(idx) => idx.term_count(),
            #[cfg(feature = "tantivy-backend")]
            SearchIndex::Tantivy(idx) => idx.term_count(),
        }
    }
}

/// Search engine type for configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchEngine {
    /// Native Rust inverted index
    Native,
    /// Tantivy search engine
    Tantivy,
}

impl Default for SearchEngine {
    fn default() -> Self {
        #[cfg(feature = "tantivy-backend")]
        {
            SearchEngine::Tantivy
        }
        #[cfg(not(feature = "tantivy-backend"))]
        {
            SearchEngine::Native
        }
    }
}

/// Builder for creating search indexes with specific backends.
#[derive(Debug)]
pub struct SearchIndexBuilder {
    engine: SearchEngine,
    path: Option<PathBuf>,
}

impl SearchIndexBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            engine: SearchEngine::default(),
            path: None,
        }
    }

    /// Set the search engine type.
    pub fn engine(mut self, engine: SearchEngine) -> Self {
        self.engine = engine;
        self
    }

    /// Set the index path.
    pub fn path(mut self, path: PathBuf) -> Self {
        self.path = Some(path);
        self
    }

    /// Build the search index.
    pub fn build(self) -> Result<SearchIndex> {
        let path = self.path.expect("Path must be set");

        match self.engine {
            SearchEngine::Native => {
                #[cfg(feature = "native-backend")]
                {
                    SearchIndex::native_new(&path)
                }
                #[cfg(not(feature = "native-backend"))]
                {
                    anyhow::bail!("Native backend not enabled. Enable 'native-backend' feature.");
                }
            }
            SearchEngine::Tantivy => {
                #[cfg(feature = "tantivy-backend")]
                {
                    SearchIndex::tantivy_new(&path)
                }
                #[cfg(not(feature = "tantivy-backend"))]
                {
                    anyhow::bail!("Tantivy backend not enabled. Enable 'tantivy-backend' feature.");
                }
            }
        }
    }
}

impl Default for SearchIndexBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_search_query_builder() {
        let query = SearchQuery::new()
            .with_term("hello")
            .with_term("world")
            .with_session(Uuid::new_v4())
            .with_doc_type("message")
            .with_limit(10)
            .with_min_score(0.5);

        assert_eq!(query.terms.len(), 2);
        assert!(query.session_id.is_some());
        assert_eq!(query.doc_type, Some("message".to_string()));
        assert_eq!(query.limit, 10);
        assert_eq!(query.min_score, 0.5);
    }

    #[test]
    fn test_search_index_builder() {
        #[cfg(any(feature = "native-backend", feature = "tantivy-backend"))]
        {
            let builder = SearchIndexBuilder::new().path(PathBuf::from("/tmp/test_index"));

            // Should not fail to build
            let _index = builder.build();
        }
    }

    #[test]
    fn test_search_engine_default() {
        let engine = SearchEngine::default();

        #[cfg(feature = "tantivy-backend")]
        assert_eq!(engine, SearchEngine::Tantivy);

        #[cfg(not(feature = "tantivy-backend"))]
        assert_eq!(engine, SearchEngine::Native);
    }
}
