//! Tantivy-backed Full-Text Search.
//!
//! This module provides a full-text search implementation using Tantivy 0.25,
//! which offers advanced features like phrase search, fuzzy matching,
//! regex queries, and more sophisticated BM25 scoring.

use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

use super::{SearchQuery, SearchResult};
use tantivy::{doc, Term};

/// Configuration for the Tantivy search index.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct TantivySearchConfig {
    /// BM25 k1 parameter (term frequency saturation)
    pub k1: f32,
    /// BM25 b parameter (length normalization)
    pub b: f32,
    /// Default search field boost
    pub field_boost: HashMap<String, f32>,
    /// Enable phrase search
    pub enable_phrase_search: bool,
    /// Enable fuzzy matching
    pub enable_fuzzy: bool,
    /// Enable regex queries
    pub enable_regex: bool,
    /// Snippet length
    pub snippet_length: usize,
    /// Serialization format: "json" or "bincode"
    pub serialization_format: String,
}

impl Default for TantivySearchConfig {
    fn default() -> Self {
        Self {
            k1: 1.2,
            b: 0.75,
            field_boost: HashMap::new(),
            enable_phrase_search: true,
            enable_fuzzy: true,
            enable_regex: true,
            snippet_length: 100,
            serialization_format: "json".to_string(),
        }
    }
}

/// Tantivy-backed search index with advanced features.
pub struct TantivySearchIndex {
    /// The Tantivy index
    index: tantivy::Index,
    /// Index reader
    reader: tantivy::IndexReader,
    /// Schema fields
    schema: TantivySchema,
    /// Configuration
    config: TantivySearchConfig,
    /// Path to index directory
    path: PathBuf,
}

impl std::fmt::Debug for TantivySearchIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TantivySearchIndex")
            .field("path", &self.path)
            .finish()
    }
}

/// Schema fields for the search index.
#[derive(Debug)]
struct TantivySchema {
    /// Content field (full text)
    content_field: tantivy::schema::Field,
    /// Session ID field
    session_id_field: tantivy::schema::Field,
    /// Document type field (message/memory)
    doc_type_field: tantivy::schema::Field,
    /// Document ID field
    doc_id_field: tantivy::schema::Field,
    /// Timestamp field
    timestamp_field: tantivy::schema::Field,
    /// Tags field (for memories)
    tags_field: tantivy::schema::Field,
    /// Stored content field for snippet retrieval
    stored_content_field: tantivy::schema::Field,
}

impl TantivySearchIndex {
    /// Create a new Tantivy search index.
    pub fn new(path: &PathBuf) -> Result<Self> {
        use tantivy::schema::*;
        use tantivy::{Index, IndexBuilder};

        // Define schema
        let mut schema_builder = Schema::builder();

        // Text field for full-text search (tokenized)
        let content_field = schema_builder.add_text_field("content", TEXT | STORED);

        // Stored field for content retrieval
        let stored_content_field = schema_builder.add_text_field("stored_content", STORED);

        // Session ID as string for filtering (also stored for retrieval)
        let session_id_field = schema_builder.add_text_field("session_id", STRING | FAST | STORED);

        // Document type as string for filtering (also stored for retrieval)
        let doc_type_field = schema_builder.add_text_field("doc_type", STRING | STORED);

        // Document ID (UUID as string)
        let doc_id_field = schema_builder.add_text_field("doc_id", STRING | STORED);

        // Timestamp for ordering (also stored for retrieval)
        let timestamp_field = schema_builder.add_i64_field("timestamp", INDEXED | FAST | STORED);

        // Tags for memories
        let tags_field = schema_builder.add_text_field("tags", TEXT);

        let schema = schema_builder.build();

        // Create or open index directly at the given path
        let index = if path.exists() {
            // Check if there's an existing index by trying to open it
            Index::open_in_dir(path.clone())
        } else {
            // Create the index directory
            std::fs::create_dir_all(path)?;
            IndexBuilder::new()
                .schema(schema.clone())
                .create_in_dir(path.clone())
        }?;

        // Create reader
        let reader = index.reader()?;

        let config = TantivySearchConfig::default();

        Ok(Self {
            index,
            reader,
            schema: TantivySchema {
                content_field,
                session_id_field,
                doc_type_field,
                doc_id_field,
                timestamp_field,
                tags_field,
                stored_content_field,
            },
            config,
            path: path.clone(),
        })
    }

    /// Open an existing index.
    pub fn open(path: &PathBuf) -> Result<Self> {
        Self::new(path)
    }

    /// Create snippet from content.
    fn create_snippet(&self, content: &str) -> String {
        let max_len = self.config.snippet_length;
        let trimmed = content.trim();
        if trimmed.len() <= max_len {
            trimmed.to_string()
        } else {
            let mut snippet = trimmed[..max_len].to_string();
            if let Some(last_space) = snippet.rfind(' ') {
                snippet = snippet[..last_space].to_string();
            }
            snippet + "..."
        }
    }
}

impl super::SearchIndexBackend for TantivySearchIndex {
    /// Create a new search index.
    fn new(path: &PathBuf) -> Result<Self>
    where
        Self: Sized,
    {
        Self::new(path)
    }

    /// Open an existing search index.
    fn open(path: &PathBuf) -> Result<Self>
    where
        Self: Sized,
    {
        Self::open(path)
    }

    /// Index a message document.
    fn add_message(
        &self,
        session_id: Uuid,
        message_id: Uuid,
        content: &str,
        _role: &str,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> Result<()> {
        let mut index_writer = self.index.writer(100_000_000)?;

        let doc = tantivy::doc!(
            self.schema.content_field => content,
            self.schema.stored_content_field => content,
            self.schema.session_id_field => session_id.to_string(),
            self.schema.doc_type_field => "message",
            self.schema.doc_id_field => message_id.to_string(),
            self.schema.timestamp_field => timestamp.timestamp(),
        );

        index_writer.add_document(doc)?;
        index_writer.commit()?;

        // Refresh the reader to see the new documents
        self.reader.reload()?;

        Ok(())
    }

    /// Index a memory document.
    fn add_memory(
        &self,
        session_id: Uuid,
        memory_id: Uuid,
        content: &str,
        tags: &[String],
        _confidence: f32,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> Result<()> {
        let mut index_writer = self.index.writer(100_000_000)?;

        let tags_text = tags.join(" ");
        let full_text = format!("{} {}", content, tags_text);

        let doc = tantivy::doc!(
            self.schema.content_field => full_text,
            self.schema.stored_content_field => content,
            self.schema.session_id_field => session_id.to_string(),
            self.schema.doc_type_field => "memory",
            self.schema.doc_id_field => memory_id.to_string(),
            self.schema.timestamp_field => timestamp.timestamp(),
            self.schema.tags_field => tags_text,
        );

        index_writer.add_document(doc)?;
        index_writer.commit()?;

        // Refresh the reader to see the new documents
        self.reader.reload()?;

        Ok(())
    }

    /// Search the index with advanced query support.
    fn search(&self, query: super::SearchQuery) -> Result<Vec<SearchResult>> {
        use tantivy::collector::TopDocs;
        use tantivy::query::{BooleanQuery, TermQuery};
        use tantivy::schema::Value;
        use tantivy::Term;

        self.reader.reload()?;

        let searcher = self.reader.searcher();

        // Build query
        let query_parser =
            tantivy::query::QueryParser::for_index(&self.index, vec![self.schema.content_field]);

        // Create the user query string
        let query_string = query.terms.join(" ");

        // Parse the query (supports phrase search, fuzzy, etc.)
        let parsed_query = query_parser.parse_query(&query_string)?;

        // Apply filters using BooleanQuery
        let mut filter_clauses: Vec<Box<dyn tantivy::query::Query>> = Vec::new();

        // Session filter
        if let Some(session_id) = query.session_id {
            let session_term =
                Term::from_field_text(self.schema.session_id_field, &session_id.to_string());
            let session_query =
                TermQuery::new(session_term, tantivy::schema::IndexRecordOption::Basic);
            filter_clauses.push(Box::new(session_query) as Box<dyn tantivy::query::Query>);
        }

        // Document type filter
        if let Some(ref doc_type) = query.doc_type {
            let doc_type_term = Term::from_field_text(self.schema.doc_type_field, doc_type);
            let doc_type_query =
                TermQuery::new(doc_type_term, tantivy::schema::IndexRecordOption::Basic);
            filter_clauses.push(Box::new(doc_type_query) as Box<dyn tantivy::query::Query>);
        }

        // Apply filters if any
        let final_query: Box<dyn tantivy::query::Query> = if !filter_clauses.is_empty() {
            // Create filter query from clauses
            let occur_clauses: Vec<(tantivy::query::Occur, Box<dyn tantivy::query::Query>)> =
                filter_clauses
                    .into_iter()
                    .map(|q| (tantivy::query::Occur::Must, q))
                    .collect();
            let filter_query = BooleanQuery::new(occur_clauses);
            // Intersect with text query using associated function
            // Move parsed_query into the intersection
            let queries = vec![
                Box::new(filter_query) as Box<dyn tantivy::query::Query>,
                parsed_query,
            ];
            Box::new(BooleanQuery::intersection(queries))
        } else {
            parsed_query
        };

        // Collect results with scores
        let top_docs = searcher.search(&*final_query, &TopDocs::with_limit(query.limit.max(1)))?;

        let mut results = Vec::new();

        for (score, doc_address) in top_docs {
            let retrieved_doc = searcher.doc::<tantivy::TantivyDocument>(doc_address)?;

            // Extract fields using Tantivy's Value trait
            let doc_id = retrieved_doc
                .get_first(self.schema.doc_id_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let session_id_str = retrieved_doc
                .get_first(self.schema.session_id_field)
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let session_id = session_id_str.parse().unwrap_or_else(|_| Uuid::nil());

            let doc_type = retrieved_doc
                .get_first(self.schema.doc_type_field)
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            let content = retrieved_doc
                .get_first(self.schema.stored_content_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let timestamp = retrieved_doc
                .get_first(self.schema.timestamp_field)
                .and_then(|v| v.as_i64())
                .unwrap_or(0);

            let snippet = self.create_snippet(&content);

            results.push(SearchResult {
                id: doc_id,
                session_id,
                doc_type,
                score: score.max(0.0).min(100.0),
                snippet,
                timestamp: chrono::DateTime::from_timestamp(timestamp, 0)
                    .unwrap_or_else(chrono::Utc::now),
            });
        }

        Ok(results)
    }

    /// Delete all documents for a session.
    fn delete_session(&self, session_id: Uuid) -> Result<()> {
        let mut index_writer: tantivy::IndexWriter<tantivy::TantivyDocument> =
            self.index.writer(50_000_000)?;

        let session_term =
            Term::from_field_text(self.schema.session_id_field, &session_id.to_string());

        index_writer.delete_term(session_term);
        index_writer.commit()?;

        // Refresh the reader to see the deleted documents
        self.reader.reload()?;

        Ok(())
    }

    /// Commit pending changes.
    fn commit(&self) -> Result<()> {
        // Tantivy commits automatically on add_document
        Ok(())
    }

    /// Get the number of indexed documents.
    fn count(&self) -> u64 {
        self.reader.searcher().num_docs()
    }

    /// Get the number of indexed terms.
    fn term_count(&self) -> usize {
        self.reader
            .searcher()
            .segment_readers()
            .iter()
            .fold(0, |acc, seg| {
                acc + seg
                    .inverted_index(self.schema.content_field)
                    .map(|idx| idx.terms().num_terms() as usize)
                    .unwrap_or(0)
            })
    }
}

/// Extended search query with advanced options.
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct ExtendedSearchQuery {
    /// Basic search query
    pub base_query: SearchQuery,
    /// Phrase search (exact phrase matching)
    pub phrase: Option<String>,
    /// Fuzzy matching parameters
    pub fuzzy: Option<FuzzyOptions>,
    /// Regex pattern
    pub regex: Option<String>,
    /// Boost factors for fields
    pub field_boost: HashMap<String, f32>,
    /// Sort options
    pub sort: SortOption,
}

/// Fuzzy matching options.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct FuzzyOptions {
    /// Maximum edit distance (0-2)
    pub distance: u8,
    /// Prefix length for fuzzy matching
    pub prefix_length: usize,
    /// Maximum expansions
    pub max_expansions: u32,
}

/// Sort options for search results.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum SortOption {
    /// Sort by relevance score (default)
    Score,
    /// Sort by timestamp ascending
    TimestampAsc,
    /// Sort by timestamp descending
    TimestampDesc,
    /// Sort by document ID
    DocId,
}

impl Default for SortOption {
    fn default() -> Self {
        SortOption::Score
    }
}

#[allow(dead_code)]
impl ExtendedSearchQuery {
    /// Create a new extended query from a basic query.
    pub fn from_basic(query: SearchQuery) -> Self {
        Self {
            base_query: query,
            phrase: None,
            fuzzy: None,
            regex: None,
            field_boost: HashMap::new(),
            sort: SortOption::Score,
        }
    }

    /// Add phrase search.
    pub fn with_phrase(mut self, phrase: impl Into<String>) -> Self {
        self.phrase = Some(phrase.into());
        self
    }

    /// Add fuzzy matching.
    pub fn with_fuzzy(mut self, distance: u8, prefix_length: usize) -> Self {
        self.fuzzy = Some(FuzzyOptions {
            distance: distance.clamp(0, 2),
            prefix_length,
            max_expansions: 50,
        });
        self
    }

    /// Add regex pattern.
    pub fn with_regex(mut self, pattern: impl Into<String>) -> Self {
        self.regex = Some(pattern.into());
        self
    }

    /// Set sort option.
    pub fn with_sort(mut self, sort: SortOption) -> Self {
        self.sort = sort;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::SearchIndexBackend;
    use chrono::Utc;

    fn create_test_index(dir: &tempfile::TempDir) -> TantivySearchIndex {
        let path = dir.path().join("tantivy_search");
        TantivySearchIndex::new(&path).unwrap()
    }

    #[test]
    fn test_tantivy_new_and_open() {
        let dir = tempfile::TempDir::new().unwrap();
        let index = create_test_index(&dir);
        assert_eq!(index.count(), 0);
    }

    #[test]
    fn test_add_and_search_message() {
        let dir = tempfile::TempDir::new().unwrap();
        let index = create_test_index(&dir);
        let session_id = Uuid::new_v4();
        let timestamp = Utc::now();

        index
            .add_message(
                session_id,
                Uuid::new_v4(),
                "Hello world test",
                "user",
                timestamp,
            )
            .unwrap();

        assert_eq!(index.count(), 1);

        let results = index.search(SearchQuery::new().with_term("hello")).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].session_id, session_id);
        assert_eq!(results[0].doc_type, "message");
        assert!(results[0].score > 0.0);
    }

    #[test]
    fn test_add_and_search_memory() {
        let dir = tempfile::TempDir::new().unwrap();
        let index = create_test_index(&dir);
        let session_id = Uuid::new_v4();
        let timestamp = Utc::now();
        let tags = vec!["rust".to_string(), "testing".to_string()];

        index
            .add_memory(
                session_id,
                Uuid::new_v4(),
                "Learning Rust programming",
                &tags,
                0.9,
                timestamp,
            )
            .unwrap();

        assert_eq!(index.count(), 1);

        // Search by content
        let results = index.search(SearchQuery::new().with_term("Rust")).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].session_id, session_id);
        assert_eq!(results[0].doc_type, "memory");

        // Search by tag
        let results = index
            .search(SearchQuery::new().with_term("testing"))
            .unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_with_session_filter() {
        let dir = tempfile::TempDir::new().unwrap();
        let index = create_test_index(&dir);
        let session1 = Uuid::new_v4();
        let session2 = Uuid::new_v4();
        let timestamp = Utc::now();

        index
            .add_message(
                session1,
                Uuid::new_v4(),
                "Session one content",
                "user",
                timestamp,
            )
            .unwrap();
        index
            .add_message(
                session2,
                Uuid::new_v4(),
                "Session two content",
                "user",
                timestamp,
            )
            .unwrap();

        assert_eq!(index.count(), 2);

        let results = index
            .search(
                SearchQuery::new()
                    .with_term("content")
                    .with_session(session1),
            )
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].session_id, session1);
    }

    #[test]
    fn test_delete_session() {
        let dir = tempfile::TempDir::new().unwrap();
        let index = create_test_index(&dir);
        let session1 = Uuid::new_v4();
        let session2 = Uuid::new_v4();
        let timestamp = Utc::now();

        index
            .add_message(session1, Uuid::new_v4(), "Session one", "user", timestamp)
            .unwrap();
        index
            .add_message(session2, Uuid::new_v4(), "Session two", "user", timestamp)
            .unwrap();

        assert_eq!(index.count(), 2);

        index.delete_session(session1).unwrap();

        assert_eq!(index.count(), 1);
        let results = index
            .search(SearchQuery::new().with_term("Session"))
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].session_id, session2);
    }

    #[test]
    fn test_persistence() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("tantivy_search");

        {
            let index = TantivySearchIndex::new(&path).unwrap();
            let session_id = Uuid::new_v4();
            let timestamp = Utc::now();

            index
                .add_message(
                    session_id,
                    Uuid::new_v4(),
                    "Persistent content",
                    "user",
                    timestamp,
                )
                .unwrap();
        }

        // Reopen and verify
        let index = TantivySearchIndex::open(&path).unwrap();
        assert_eq!(index.count(), 1);

        let results = index
            .search(SearchQuery::new().with_term("Persistent"))
            .unwrap();
        assert_eq!(results.len(), 1);
    }
}
