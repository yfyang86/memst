//! Rust-native Full-Text Search using Inverted Index.
//!
//! This module provides a high-performance full-text search using a custom
//! inverted index implementation that works seamlessly with our binary storage
//! format (bincode + zstd compression).

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

use super::{SearchQuery, SearchResult};

/// A posting entry in the inverted index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Posting {
    /// Document ID
    pub doc_id: String,
    /// Session ID for filtering
    pub session_id: Uuid,
    /// Document type (message/memory)
    pub doc_type: String,
    /// Term frequency in this document
    pub tf: u32,
    /// Term positions in the document
    pub positions: Vec<u32>,
    /// Timestamp for ordering
    pub timestamp: i64,
    /// Snippet preview
    pub snippet: String,
}

/// Inverted index entry mapping a term to its postings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvertedIndexEntry {
    /// Document frequency (how many docs contain this term)
    pub df: u32,
    /// Postings list
    pub postings: Vec<Posting>,
}

/// The document store - stores original content for retrieval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedDocument {
    /// Document ID
    pub id: String,
    /// Session ID
    pub session_id: Uuid,
    /// Document type
    pub doc_type: String,
    /// Original content
    pub content: String,
    /// Tokenized content for snippet generation
    pub tokens: Vec<String>,
    /// Timestamp
    pub timestamp: i64,
}

/// Configuration for the search index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeSearchConfig {
    /// BM25 k1 parameter (term frequency saturation)
    pub k1: f32,
    /// BM25 b parameter (length normalization)
    pub b: f32,
    /// Minimum term length to index
    pub min_term_length: usize,
    /// Maximum results per query
    pub max_results: usize,
    /// Snippet length
    pub snippet_length: usize,
}

impl Default for NativeSearchConfig {
    fn default() -> Self {
        Self {
            k1: 1.2,
            b: 0.75,
            min_term_length: 2,
            max_results: 100,
            snippet_length: 100,
        }
    }
}

/// Rust-native Full-Text Search Index using Inverted Index + BM25.
#[derive(Debug)]
pub struct NativeSearchIndex {
    /// Index directory
    path: PathBuf,
    /// Configuration
    config: NativeSearchConfig,
    /// Inverted index: term -> (df, postings)
    inverted_index: Arc<RwLock<HashMap<String, InvertedIndexEntry>>>,
    /// Document store: doc_id -> document
    document_store: Arc<RwLock<HashMap<String, IndexedDocument>>>,
    /// Total number of documents
    num_docs: Arc<RwLock<u32>>,
    /// Average document length (in tokens)
    avg_doc_len: Arc<RwLock<f32>>,
    /// Set of all document IDs per session
    session_docs: Arc<RwLock<HashMap<Uuid, HashSet<String>>>>,
    /// Documents added since the index was last written or journalled.
    pending: std::sync::Mutex<Vec<Journaled>>,
    /// How many documents the journal file holds.
    journaled: std::sync::atomic::AtomicUsize,
    /// Set while the journal is being read back, so that what it holds is
    /// not journalled a second time.
    replaying: std::sync::atomic::AtomicBool,
}

/// A document as the journal keeps it: what was passed to `add_message` or
/// `add_memory`.
///
/// Writing the whole index for every document made each one cost more than
/// the last. A commit now appends the new documents to `journal.jsonl`,
/// and the index proper is rewritten once the journal holds
/// [`JOURNAL_LIMIT`] of them, and whenever something is deleted (the
/// journal holds text, and a deletion has to remove it from there too).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Journaled {
    memory: bool,
    session_id: Uuid,
    id: Uuid,
    content: String,
    tags: Vec<String>,
    timestamp: chrono::DateTime<chrono::Utc>,
}

/// The journal is folded into the index when it holds this many documents.
const JOURNAL_LIMIT: usize = 200;

impl NativeSearchIndex {
    fn journal_path(&self) -> PathBuf {
        self.path.join("journal.jsonl")
    }

    /// Note a document for the journal, unless it is being read from it.
    fn note_added(&self, memory: bool, session_id: Uuid, id: Uuid, content: &str, tags: &[String], timestamp: chrono::DateTime<chrono::Utc>) {
        if !self.replaying.load(std::sync::atomic::Ordering::SeqCst) {
            self.pending.lock().unwrap().push(Journaled { memory, session_id, id, content: content.to_string(), tags: tags.to_vec(), timestamp });
        }
    }

    /// Read the journal back into the index in memory.
    fn replay_journal(&self) -> Result<()> {
        use super::SearchIndexBackend;
        let Ok(text) = std::fs::read_to_string(self.journal_path()) else { return Ok(()) };
        self.replaying.store(true, std::sync::atomic::Ordering::SeqCst);
        let mut count = 0;
        for line in text.lines() {
            // A line cut short by a crash is the last one; skip it.
            let Ok(entry) = serde_json::from_str::<Journaled>(line) else { continue };
            count += 1;
            // The index may already hold it, if a crash came between
            // writing the index and removing the journal.
            if self.document_store.read().unwrap().contains_key(&entry.id.to_string()) {
                continue;
            }
            let added = if entry.memory {
                self.add_memory(entry.session_id, entry.id, &entry.content, &entry.tags, 1.0, entry.timestamp)
            } else {
                self.add_message(entry.session_id, entry.id, &entry.content, "", entry.timestamp)
            };
            if let Err(error) = added {
                self.replaying.store(false, std::sync::atomic::Ordering::SeqCst);
                return Err(error);
            }
        }
        self.replaying.store(false, std::sync::atomic::Ordering::SeqCst);
        self.journaled.store(count, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    /// Create a new search index.
    pub fn new(path: &PathBuf) -> Result<Self> {
        let index_path = path.join("search_index");
        let postings_path = index_path.join("postings.bincode");
        let docs_path = index_path.join("documents.bincode");
        let meta_path = index_path.join("meta.bincode");

        // Load existing index or create new
        let (inverted_index, document_store, num_docs, avg_doc_len, session_docs, config) =
            if postings_path.exists() && docs_path.exists() {
                Self::load_from_disk(&postings_path, &docs_path, &meta_path)?
            } else {
                (
                    HashMap::new(),
                    HashMap::new(),
                    0,
                    0.0,
                    HashMap::new(),
                    NativeSearchConfig::default(),
                )
            };

        let index = Self {
            path: index_path,
            config,
            inverted_index: Arc::new(RwLock::new(inverted_index)),
            document_store: Arc::new(RwLock::new(document_store)),
            num_docs: Arc::new(RwLock::new(num_docs)),
            avg_doc_len: Arc::new(RwLock::new(avg_doc_len)),
            session_docs: Arc::new(RwLock::new(session_docs)),
            pending: std::sync::Mutex::new(Vec::new()),
            journaled: std::sync::atomic::AtomicUsize::new(0),
            replaying: std::sync::atomic::AtomicBool::new(false),
        };
        index.replay_journal()?;
        Ok(index)
    }

    /// Open an existing search index.
    pub fn open(path: &PathBuf) -> Result<Self> {
        Self::new(path)
    }

    /// Tokenize and normalize text.
    fn tokenize(&self, text: &str) -> Vec<String> {
        text.to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>()
            .split_whitespace()
            .filter(|term| term.len() >= self.config.min_term_length)
            .map(|s| s.to_string())
            .collect()
    }

    /// Create a snippet from content.
    fn create_snippet(&self, content: &str, max_len: usize) -> String {
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

    /// Persist index to disk using bincode + zstd.
    fn persist_to_disk(&self) -> Result<()> {
        use std::fs::{create_dir_all, File};
        use std::io::Write;

        create_dir_all(&self.path)?;

        let postings_path = self.path.join("postings.bincode");
        let docs_path = self.path.join("documents.bincode");
        let meta_path = self.path.join("meta.bincode");

        // Serialize with compression (level 1 for speed)
        let inverted_data = bincode::serialize(&*self.inverted_index.read().unwrap())?;
        let compressed_inverted = zstd::encode_all(std::io::Cursor::new(inverted_data), 1)?;
        let mut file = File::create(postings_path)?;
        file.write_all(&compressed_inverted)?;

        let docs_data = bincode::serialize(&*self.document_store.read().unwrap())?;
        let compressed_docs = zstd::encode_all(std::io::Cursor::new(docs_data), 1)?;
        let mut file = File::create(docs_path)?;
        file.write_all(&compressed_docs)?;

        // Serialize metadata
        let meta = NativeSearchIndexMeta {
            num_docs: *self.num_docs.read().unwrap(),
            avg_doc_len: *self.avg_doc_len.read().unwrap(),
            config: self.config.clone(),
        };
        let meta_data = bincode::serialize(&meta)?;
        let mut file = File::create(meta_path)?;
        file.write_all(&meta_data)?;

        // Everything in memory is now in the index proper.
        self.pending.lock().unwrap().clear();
        let _ = std::fs::remove_file(self.journal_path());
        self.journaled.store(0, std::sync::atomic::Ordering::SeqCst);

        Ok(())
    }

    /// Load index from disk.
    fn load_from_disk(
        postings_path: &PathBuf,
        docs_path: &PathBuf,
        meta_path: &PathBuf,
    ) -> Result<(
        HashMap<String, InvertedIndexEntry>,
        HashMap<String, IndexedDocument>,
        u32,
        f32,
        HashMap<Uuid, HashSet<String>>,
        NativeSearchConfig,
    )> {
        use std::fs::File;
        use std::io::Read;

        // Load metadata
        let mut meta_file = File::open(meta_path)?;
        let mut meta_data = Vec::new();
        meta_file.read_to_end(&mut meta_data)?;
        let meta: NativeSearchIndexMeta = bincode::deserialize(&meta_data)?;

        // Load postings
        let mut postings_file = File::open(postings_path)?;
        let mut compressed_postings = Vec::new();
        postings_file.read_to_end(&mut compressed_postings)?;
        let postings_data = zstd::decode_all(std::io::Cursor::new(compressed_postings))?;
        let inverted_index: HashMap<String, InvertedIndexEntry> =
            bincode::deserialize(&postings_data)?;

        // Load documents
        let mut docs_file = File::open(docs_path)?;
        let mut compressed_docs = Vec::new();
        docs_file.read_to_end(&mut compressed_docs)?;
        let docs_data = zstd::decode_all(std::io::Cursor::new(compressed_docs))?;
        let document_store: HashMap<String, IndexedDocument> = bincode::deserialize(&docs_data)?;

        // Build session docs index
        let mut session_docs: HashMap<Uuid, HashSet<String>> = HashMap::new();
        for doc_id in document_store.keys() {
            if let Some(doc) = document_store.get(doc_id) {
                session_docs
                    .entry(doc.session_id)
                    .or_insert_with(HashSet::new)
                    .insert(doc_id.clone());
            }
        }

        Ok((
            inverted_index,
            document_store,
            meta.num_docs,
            meta.avg_doc_len,
            session_docs,
            meta.config,
        ))
    }
}

/// Metadata for the native search index.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct NativeSearchIndexMeta {
    num_docs: u32,
    avg_doc_len: f32,
    config: NativeSearchConfig,
}

impl super::SearchIndexBackend for NativeSearchIndex {
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
        self.note_added(false, session_id, message_id, content, &[], timestamp);
        let doc_id = message_id.to_string();
        let tokens = self.tokenize(content);
        let timestamp_secs = timestamp.timestamp();

        // Create snippet
        let snippet = self.create_snippet(content, self.config.snippet_length);

        // Create indexed document
        let doc = IndexedDocument {
            id: doc_id.clone(),
            session_id,
            doc_type: "message".to_string(),
            content: content.to_string(),
            tokens: tokens.clone(),
            timestamp: timestamp_secs,
        };

        // Build postings with positions
        let mut term_positions: HashMap<String, Vec<(u32, u32)>> = HashMap::new();
        for (pos, token) in tokens.iter().enumerate() {
            term_positions
                .entry(token.clone())
                .or_insert_with(Vec::new)
                .push((pos as u32, 0)); // 0 = content field
        }

        // Update inverted index
        let mut inverted = self.inverted_index.write().unwrap();
        let mut docs = self.document_store.write().unwrap();
        let mut sessions = self.session_docs.write().unwrap();

        // Store document
        docs.insert(doc_id.clone(), doc);

        // Update session documents
        sessions
            .entry(session_id)
            .or_insert_with(HashSet::new)
            .insert(doc_id.clone());

        // Update postings
        for (term, positions) in &term_positions {
            let entry = inverted
                .entry(term.clone())
                .or_insert_with(|| InvertedIndexEntry {
                    df: 0,
                    postings: Vec::new(),
                });

            let tf = positions.len() as u32;
            let posting = Posting {
                doc_id: doc_id.clone(),
                session_id,
                doc_type: "message".to_string(),
                tf,
                positions: positions.iter().map(|p| p.0).collect(),
                timestamp: timestamp_secs,
                snippet: snippet.clone(),
            };

            entry.postings.push(posting);
            entry.df += 1;
        }

        // Update stats
        let num_docs = {
            let mut num_docs = self.num_docs.write().unwrap();
            *num_docs += 1;
            *num_docs
        };

        {
            let mut avg_len = self.avg_doc_len.write().unwrap();
            let total_tokens: u32 = tokens.len() as u32;
            *avg_len = (*avg_len * (num_docs - 1) as f32 + total_tokens as f32) / num_docs as f32;
        }

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
        self.note_added(true, session_id, memory_id, content, tags, timestamp);
        let doc_id = memory_id.to_string();
        let tags_text = tags.join(" ");
        let full_text = format!("{} {}", content, tags_text);
        let tokens = self.tokenize(&full_text);
        let timestamp_secs = timestamp.timestamp();

        // Create snippet
        let snippet = self.create_snippet(content, self.config.snippet_length);

        // Create indexed document
        let doc = IndexedDocument {
            id: doc_id.clone(),
            session_id,
            doc_type: "memory".to_string(),
            content: content.to_string(),
            tokens: tokens.clone(),
            timestamp: timestamp_secs,
        };

        // Build postings with positions
        let mut term_positions: HashMap<String, Vec<(u32, u32)>> = HashMap::new();
        for (pos, token) in tokens.iter().enumerate() {
            term_positions
                .entry(token.clone())
                .or_insert_with(Vec::new)
                .push((pos as u32, 0));
        }

        // Update inverted index
        let mut inverted = self.inverted_index.write().unwrap();
        let mut docs = self.document_store.write().unwrap();
        let mut sessions = self.session_docs.write().unwrap();

        // Store document
        docs.insert(doc_id.clone(), doc);

        // Update session documents
        sessions
            .entry(session_id)
            .or_insert_with(HashSet::new)
            .insert(doc_id.clone());

        // Update postings
        for (term, positions) in &term_positions {
            let entry = inverted
                .entry(term.clone())
                .or_insert_with(|| InvertedIndexEntry {
                    df: 0,
                    postings: Vec::new(),
                });

            let tf = positions.len() as u32;
            let posting = Posting {
                doc_id: doc_id.clone(),
                session_id,
                doc_type: "memory".to_string(),
                tf,
                positions: positions.iter().map(|p| p.0).collect(),
                timestamp: timestamp_secs,
                snippet: snippet.clone(),
            };

            entry.postings.push(posting);
            entry.df += 1;
        }

        // Update stats
        let num_docs = {
            let mut num_docs = self.num_docs.write().unwrap();
            *num_docs += 1;
            *num_docs
        };

        {
            let mut avg_len = self.avg_doc_len.write().unwrap();
            let doc_len = tokens.len() as f32;
            *avg_len = (*avg_len * (num_docs - 1) as f32 + doc_len) / num_docs as f32;
        }

        Ok(())
    }

    /// Search the index using BM25 scoring.
    fn search(&self, query: SearchQuery) -> Result<Vec<SearchResult>> {
        if query.terms.is_empty() {
            return Ok(Vec::new());
        }

        let num_docs = *self.num_docs.read().unwrap() as f32;
        if num_docs == 0.0 {
            return Ok(Vec::new());
        }

        let avg_doc_len = *self.avg_doc_len.read().unwrap();
        let k1 = self.config.k1;
        let b = self.config.b;

        // Collect candidate documents
        let mut doc_scores: HashMap<String, f32> = HashMap::new();
        let inverted = self.inverted_index.read().unwrap();

        // Normalize query terms to lowercase for lookup
        let normalized_terms: Vec<String> = query.terms.iter().map(|t| t.to_lowercase()).collect();

        // Calculate IDF for query terms using standard BM25 formula
        // IDF = ln((N - n + 0.5) / (n + 0.5) + 1)
        let mut term_idf: HashMap<String, f32> = HashMap::new();
        for term in &normalized_terms {
            if let Some(entry) = inverted.get(term) {
                let df = entry.df as f32;
                let idf = ((num_docs - df + 0.5) / (df + 0.5) + 1.0).ln();
                term_idf.insert(term.clone(), idf.max(0.0));
            }
        }

        // Score documents
        for term in &normalized_terms {
            if let Some(entry) = inverted.get(term) {
                let idf = term_idf.get(term).copied().unwrap_or(0.0);

                for posting in &entry.postings {
                    // Apply session filter
                    if let Some(ref session_id) = query.session_id {
                        if posting.session_id != *session_id {
                            continue;
                        }
                    }

                    // Apply doc_type filter
                    if let Some(ref doc_type) = query.doc_type {
                        if posting.doc_type != *doc_type {
                            continue;
                        }
                    }

                    // BM25 scoring
                    let tf = posting.tf as f32;
                    let doc_len = posting.positions.len() as f32;
                    let numerator = idf * tf * (k1 + 1.0);
                    let denominator = tf + k1 * (1.0 - b + b * doc_len / avg_doc_len);

                    let score = if denominator > 0.0 {
                        numerator / denominator
                    } else {
                        0.0
                    };

                    *doc_scores.entry(posting.doc_id.clone()).or_insert(0.0) += score;
                }
            }
        }

        // Filter and sort results
        let mut results: Vec<SearchResult> = doc_scores
            .into_iter()
            .filter(|(_, score)| *score >= query.min_score)
            .map(|(doc_id, score)| {
                let docs = self.document_store.read().unwrap();
                if let Some(doc) = docs.get(&doc_id) {
                    SearchResult {
                        id: doc_id.clone(),
                        session_id: doc.session_id,
                        doc_type: doc.doc_type.clone(),
                        score: score.max(0.0).min(100.0), // Normalize score
                        snippet: self.create_snippet(&doc.content, self.config.snippet_length),
                        timestamp: chrono::DateTime::from_timestamp(doc.timestamp, 0)
                            .unwrap_or_else(chrono::Utc::now),
                    }
                } else {
                    // Fallback for deleted docs
                    SearchResult {
                        id: doc_id,
                        session_id: Uuid::nil(),
                        doc_type: "unknown".to_string(),
                        score,
                        snippet: String::new(),
                        timestamp: chrono::Utc::now(),
                    }
                }
            })
            .filter(|r| r.session_id != Uuid::nil())
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Limit results
        let limit = query.limit.max(1).min(self.config.max_results);
        results.truncate(limit);

        Ok(results)
    }

    /// Delete all documents for a session.
    fn delete_session(&self, session_id: Uuid) -> Result<()> {
        let mut inverted = self.inverted_index.write().unwrap();
        let mut docs = self.document_store.write().unwrap();
        let mut sessions = self.session_docs.write().unwrap();
        let mut num_docs = self.num_docs.write().unwrap();

        // Get document IDs for this session
        let doc_ids = if let Some(ids) = sessions.get(&session_id) {
            ids.clone()
        } else {
            return Ok(());
        };

        // Remove from document store
        for doc_id in &doc_ids {
            docs.remove(doc_id);
        }

        // Remove from inverted index
        for entry in inverted.values_mut() {
            entry.postings.retain(|p| p.session_id != session_id);
            entry.df = entry.postings.len() as u32;
        }

        // Remove empty entries
        inverted.retain(|_, entry| !entry.postings.is_empty());

        // Update session docs
        sessions.remove(&session_id);

        // Update count
        *num_docs -= doc_ids.len() as u32;

        // Recalculate average document length
        let new_num_docs = *num_docs;
        {
            let mut avg_len = self.avg_doc_len.write().unwrap();
            if new_num_docs > 0 {
                let total_len: u32 = docs.values().map(|d| d.tokens.len() as u32).sum();
                *avg_len = total_len as f32 / new_num_docs as f32;
            } else {
                *avg_len = 0.0;
            }
        }

        drop(inverted);
        drop(docs);
        drop(sessions);
        drop(num_docs);
        // The journal may hold the session's text: write the index without
        // it and drop the journal.
        self.persist_to_disk()
    }

    /// Delete one document by its id, and write the index to disk without it.
    fn delete_document(&self, doc_id: Uuid) -> Result<()> {
        let doc_id = doc_id.to_string();
        {
            let mut inverted = self.inverted_index.write().unwrap();
            let mut docs = self.document_store.write().unwrap();
            let mut sessions = self.session_docs.write().unwrap();
            let mut num_docs = self.num_docs.write().unwrap();
            if docs.remove(&doc_id).is_none() {
                return Ok(());
            }
            for entry in inverted.values_mut() {
                entry.postings.retain(|p| p.doc_id != doc_id);
                entry.df = entry.postings.len() as u32;
            }
            inverted.retain(|_, entry| !entry.postings.is_empty());
            for ids in sessions.values_mut() {
                ids.remove(&doc_id);
            }
            *num_docs = num_docs.saturating_sub(1);
            let mut avg_len = self.avg_doc_len.write().unwrap();
            *avg_len = if *num_docs > 0 { docs.values().map(|d| d.tokens.len() as u32).sum::<u32>() as f32 / *num_docs as f32 } else { 0.0 };
        }
        self.persist_to_disk()
    }

    /// Commit pending changes (persists to disk).
    fn commit(&self) -> Result<()> {
        use std::io::Write;
        use std::sync::atomic::Ordering;
        let waiting = self.pending.lock().unwrap().len();
        if waiting == 0 {
            return Ok(());
        }
        if self.journaled.load(Ordering::SeqCst) + waiting >= JOURNAL_LIMIT {
            return self.persist_to_disk();
        }
        std::fs::create_dir_all(&self.path)?;
        let mut journal = std::fs::OpenOptions::new().create(true).append(true).open(self.journal_path())?;
        let mut pending = self.pending.lock().unwrap();
        for entry in pending.iter() {
            writeln!(journal, "{}", serde_json::to_string(entry)?)?;
        }
        self.journaled.fetch_add(pending.len(), Ordering::SeqCst);
        pending.clear();
        Ok(())
    }

    /// Get the number of indexed documents.
    fn count(&self) -> u64 {
        *self.num_docs.read().unwrap() as u64
    }

    /// Get the number of indexed terms.
    fn term_count(&self) -> usize {
        self.inverted_index.read().unwrap().len()
    }
}

impl Drop for NativeSearchIndex {
    fn drop(&mut self) {
        // What was added and not yet committed is not lost. This used to
        // write the whole index, on every drop, including after a search
        // that had changed nothing.
        let _ = super::SearchIndexBackend::commit(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::SearchIndexBackend;
    use chrono::Utc;

    #[test]
    fn documents_are_journalled_and_the_journal_is_folded_in() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().to_path_buf();
        let session = Uuid::new_v4();
        let mut ids = Vec::new();
        // One document to a handle, as the session store does it.
        for i in 0..(JOURNAL_LIMIT + 30) {
            let index = NativeSearchIndex::new(&path).unwrap();
            let id = Uuid::new_v4();
            index.add_message(session, id, &format!("remark {i} about the harbour"), "user", Utc::now()).unwrap();
            index.commit().unwrap();
            ids.push(id);
            if i == 2 {
                // Early on there is a journal and no index proper yet.
                assert!(path.join("search_index/journal.jsonl").exists());
                assert!(!path.join("search_index/postings.bincode").exists());
            }
        }
        // The journal was folded in once, and holds what came after.
        assert!(path.join("search_index/postings.bincode").exists());
        let lines = std::fs::read_to_string(path.join("search_index/journal.jsonl")).unwrap().lines().count();
        assert_eq!(lines, 30);

        // A new handle sees every document, from the index and the journal.
        let index = NativeSearchIndex::new(&path).unwrap();
        assert_eq!(index.count(), (JOURNAL_LIMIT + 30) as u64);
        assert_eq!(index.search(crate::search::SearchQuery::new().with_term("harbour").with_limit(100)).unwrap().len(), 100);

        // Deleting a document that is only in the journal removes its text
        // from the journal file too.
        let last = *ids.last().unwrap();
        index.delete_document(last).unwrap();
        assert!(!path.join("search_index/journal.jsonl").exists());
        let again = NativeSearchIndex::new(&path).unwrap();
        assert_eq!(again.count(), (JOURNAL_LIMIT + 29) as u64);

        // A journal line cut short by a crash is skipped, not fatal.
        again.add_message(session, Uuid::new_v4(), "one more about the harbour", "user", Utc::now()).unwrap();
        again.commit().unwrap();
        let journal = path.join("search_index/journal.jsonl");
        std::fs::write(&journal, std::fs::read_to_string(&journal).unwrap() + "{\"memory\":false,\"sess").unwrap();
        assert_eq!(NativeSearchIndex::new(&path).unwrap().count(), (JOURNAL_LIMIT + 30) as u64);
    }

    fn create_test_index(dir: &tempfile::TempDir) -> NativeSearchIndex {
        let path = dir.path().join("search");
        NativeSearchIndex::new(&path).unwrap()
    }

    #[test]
    fn test_search_index_new_and_open() {
        let dir = tempfile::TempDir::new().unwrap();
        let index = create_test_index(&dir);
        assert_eq!(index.count(), 0);
        assert_eq!(index.term_count(), 0);
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
        index.commit().unwrap();

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
    fn test_persistence() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("search");

        {
            let index = NativeSearchIndex::new(&path).unwrap();
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
            index.commit().unwrap();
        }

        // Reopen and verify
        let index = NativeSearchIndex::open(&path).unwrap();
        assert_eq!(index.count(), 1);

        let results = index
            .search(SearchQuery::new().with_term("Persistent"))
            .unwrap();
        assert_eq!(results.len(), 1);
    }
}
