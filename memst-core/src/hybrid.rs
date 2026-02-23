//! Hybrid Search Module
//!
//! Combines keyword-based full-text search with semantic vector search
//! using Reciprocal Rank Fusion (RRF) for result fusion.

use crate::error::Result;
use crate::search::{SearchQuery, SearchResult};
use crate::vector::{SemanticSearch, VectorSearchResult};
use std::collections::HashMap;
use std::sync::Arc;

/// Fusion strategy for combining results
#[derive(Debug, Clone, Copy)]
pub enum FusionStrategy {
    /// Reciprocal Rank Fusion
    Rrf,
    /// Weighted sum of scores
    Weighted,
    /// Round-robin interleave
    Interleave,
}

/// Configuration for hybrid search
#[derive(Debug, Clone)]
pub struct HybridSearchConfig {
    /// Weight for keyword search (0.0-1.0)
    pub keyword_weight: f32,
    /// Weight for semantic search (0.0-1.0)
    pub semantic_weight: f32,
    /// Fusion strategy
    pub fusion_strategy: FusionStrategy,
    /// RRF k parameter (constant for RRF)
    pub rrf_k: u32,
    /// Maximum results to return
    pub max_results: usize,
    /// Minimum score threshold
    pub min_score: f32,
}

impl Default for HybridSearchConfig {
    fn default() -> Self {
        Self {
            keyword_weight: 0.5,
            semantic_weight: 0.5,
            fusion_strategy: FusionStrategy::Rrf,
            rrf_k: 60,
            max_results: 20,
            min_score: 0.1,
        }
    }
}

/// Hybrid search result combining keyword and semantic relevance
#[derive(Debug, Clone)]
pub struct HybridSearchResult {
    /// Unique document ID
    pub id: String,
    /// Session ID
    pub session_id: String,
    /// Document type
    pub doc_type: String,
    /// Content snippet
    pub content: String,
    /// Keyword search score (BM25)
    pub keyword_score: f32,
    /// Semantic search score (cosine similarity)
    pub semantic_score: f32,
    /// Combined fusion score
    pub fusion_score: f32,
    /// Result source (keyword, semantic, or both)
    pub sources: Vec<ResultSource>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Indicates which search contributed to the result
pub enum ResultSource {
    /// From keyword search
    Keyword,
    /// From semantic search
    Semantic,
    /// From both searches
    Both,
}

/// Hybrid Search Engine
///
/// Combines keyword-based search (native or Tantivy) with semantic search (HNSW)
/// using configurable fusion strategies.
#[derive(Debug)]
pub struct HybridSearchEngine<K: KeywordSearcher> {
    /// Keyword search backend
    keyword_searcher: Arc<K>,
    /// Semantic search service
    semantic_search: Option<SemanticSearch>,
    /// Configuration
    config: HybridSearchConfig,
}

/// Keyword search backend abstraction used by the hybrid engine.
pub trait KeywordSearcher: Send + Sync {
    /// Perform a keyword search using the provided query.
    fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>>;
}

impl<K: KeywordSearcher> HybridSearchEngine<K> {
    /// Create a new hybrid search engine
    pub fn new(
        keyword_searcher: Arc<K>,
        semantic_search: Option<SemanticSearch>,
        config: Option<HybridSearchConfig>,
    ) -> Self {
        Self {
            keyword_searcher,
            semantic_search,
            config: config.unwrap_or_default(),
        }
    }

    /// Perform hybrid search
    pub async fn search(
        &self,
        query: &str,
        session_filter: Option<&str>,
    ) -> Result<Vec<HybridSearchResult>> {
        let mut keyword_results = Vec::new();
        let mut semantic_results = Vec::new();

        // Convert session filter string to Uuid if provided
        let session_uuid = session_filter.and_then(|s| s.parse::<uuid::Uuid>().ok());

        // Keyword search (sync)
        if let Ok(results) = self.keyword_searcher.search(&SearchQuery {
            terms: query.split_whitespace().map(|s| s.to_string()).collect(),
            session_id: session_uuid,
            doc_type: None,
            limit: self.config.max_results,
            min_score: 0.0,
        }) {
            keyword_results = results;
        }

        // Semantic search (async)
        if let Some(ref sem_search) = self.semantic_search {
            if let Ok(results) = sem_search
                .search(query, session_filter, None, self.config.max_results)
                .await
            {
                semantic_results = results;
            }
        }

        // Fuse results
        let fused = self.fuse_results(&keyword_results, &semantic_results);

        // Apply filters and limits
        let filtered: Vec<_> = fused
            .into_iter()
            .filter(|r| r.fusion_score >= self.config.min_score)
            .take(self.config.max_results)
            .collect();

        Ok(filtered)
    }

    /// Fuse keyword and semantic results
    fn fuse_results(
        &self,
        keyword_results: &[SearchResult],
        semantic_results: &[VectorSearchResult],
    ) -> Vec<HybridSearchResult> {
        match self.config.fusion_strategy {
            FusionStrategy::Rrf => self.fuse_rrf(keyword_results, semantic_results),
            FusionStrategy::Weighted => self.fuse_weighted(keyword_results, semantic_results),
            FusionStrategy::Interleave => self.fuse_interleave(keyword_results, semantic_results),
        }
    }

    /// Reciprocal Rank Fusion (RRF)
    ///
    /// RRF_score(d) = sum(1.0 / (k + rank(d)))
    fn fuse_rrf(
        &self,
        keyword_results: &[SearchResult],
        semantic_results: &[VectorSearchResult],
    ) -> Vec<HybridSearchResult> {
        // Create maps of document ID to rank
        let keyword_ranks: HashMap<String, usize> = keyword_results
            .iter()
            .enumerate()
            .map(|(i, r)| (r.id.clone(), i + 1))
            .collect();

        let semantic_ranks: HashMap<String, usize> = semantic_results
            .iter()
            .enumerate()
            .map(|(i, r)| (r.id.clone(), i + 1))
            .collect();

        // Get all unique document IDs
        let all_ids: std::collections::HashSet<String> = keyword_results
            .iter()
            .map(|r| r.id.clone())
            .chain(semantic_results.iter().map(|r| r.id.clone()))
            .collect();

        // Calculate RRF scores
        let mut results: Vec<HybridSearchResult> = all_ids
            .into_iter()
            .map(|id| {
                let k_rank = keyword_ranks
                    .get(&id)
                    .map(|r| *r as u32)
                    .unwrap_or(u32::MAX);
                let s_rank = semantic_ranks
                    .get(&id)
                    .map(|r| *r as u32)
                    .unwrap_or(u32::MAX);

                let k_score = if k_rank != u32::MAX {
                    1.0 / (self.config.rrf_k as f32 + k_rank as f32)
                } else {
                    0.0
                };

                let s_score = if s_rank != u32::MAX {
                    1.0 / (self.config.rrf_k as f32 + s_rank as f32)
                } else {
                    0.0
                };

                let fusion_score =
                    self.config.keyword_weight * k_score + self.config.semantic_weight * s_score;

                let sources = match (k_rank != u32::MAX, s_rank != u32::MAX) {
                    (true, true) => vec![ResultSource::Both],
                    (true, false) => vec![ResultSource::Keyword],
                    (false, true) => vec![ResultSource::Semantic],
                    _ => vec![],
                };

                HybridSearchResult {
                    id: id.clone(),
                    session_id: String::new(),
                    doc_type: String::new(),
                    content: String::new(),
                    keyword_score: k_score,
                    semantic_score: s_score,
                    fusion_score,
                    sources,
                }
            })
            .collect();

        // Sort by fusion score (handle NaN by treating them as less than any real number)
        results.sort_by(|a, b| {
            b.fusion_score
                .partial_cmp(&a.fusion_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Add content from original results
        for result in &mut results {
            if let Some(kr) = keyword_results.iter().find(|r| r.id == result.id) {
                result.session_id = kr.session_id.to_string();
                result.doc_type = kr.doc_type.clone();
                result.content = kr.snippet.clone();
            }
            if let Some(sr) = semantic_results.iter().find(|r| r.id == result.id) {
                result.session_id = sr.document.session_id.clone();
                result.doc_type = sr.document.doc_type.clone();
                result.content = sr.document.content.clone();
            }
        }

        results
    }

    /// Weighted score fusion
    fn fuse_weighted(
        &self,
        keyword_results: &[SearchResult],
        semantic_results: &[VectorSearchResult],
    ) -> Vec<HybridSearchResult> {
        // Normalize keyword scores to [0, 1]
        let max_k_score = keyword_results
            .first()
            .map(|r| r.score)
            .unwrap_or(1.0)
            .max(0.001);

        // Normalize semantic scores to [0, 1]
        let max_s_score = semantic_results
            .first()
            .map(|r| r.score)
            .unwrap_or(1.0)
            .max(0.001);

        // Create maps for quick lookup
        let keyword_map: HashMap<String, (f32, &SearchResult)> = keyword_results
            .iter()
            .map(|r| (r.id.clone(), (r.score / max_k_score, r)))
            .collect();

        let semantic_map: HashMap<String, (f32, &VectorSearchResult)> = semantic_results
            .iter()
            .map(|r| (r.id.clone(), (r.score / max_s_score, r)))
            .collect();

        // Get all unique IDs
        let all_ids: std::collections::HashSet<String> = keyword_map
            .keys()
            .chain(semantic_map.keys())
            .cloned()
            .collect();

        // Calculate weighted scores
        // Handle case where one result set might be empty
        let mut results: Vec<HybridSearchResult> = all_ids
            .into_iter()
            .filter_map(|id| {
                let k_data = keyword_map.get(&id);
                let s_data = semantic_map.get(&id);

                // At least one must exist since id came from the union
                let (k_score, k_orig) = k_data.map(|(s, r)| (*s, Some(*r))).unwrap_or((0.0, None));
                let (s_score, s_orig) = s_data.map(|(s, r)| (*s, Some(r))).unwrap_or((0.0, None));

                // Get session/content from whichever source we have
                let (session_id, doc_type, content) = match (k_orig, s_orig) {
                    (Some(k), _) => (
                        k.session_id.to_string(),
                        k.doc_type.clone(),
                        k.snippet.clone(),
                    ),
                    (None, Some(s)) => (
                        s.document.session_id.clone(),
                        s.document.doc_type.clone(),
                        s.document.content.clone(),
                    ),
                    (None, None) => return None, // This shouldn't happen
                };

                let fusion_score =
                    self.config.keyword_weight * k_score + self.config.semantic_weight * s_score;

                let sources = match (k_score > 0.0, s_score > 0.0) {
                    (true, true) => vec![ResultSource::Both],
                    (true, false) => vec![ResultSource::Keyword],
                    (false, true) => vec![ResultSource::Semantic],
                    _ => vec![],
                };

                Some(HybridSearchResult {
                    id: id.clone(),
                    session_id,
                    doc_type,
                    content,
                    keyword_score: k_score,
                    semantic_score: s_score,
                    fusion_score,
                    sources,
                })
            })
            .collect();

        results.sort_by(|a, b| {
            b.fusion_score
                .partial_cmp(&a.fusion_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results
    }

    /// Interleave results round-robin
    fn fuse_interleave(
        &self,
        keyword_results: &[SearchResult],
        semantic_results: &[VectorSearchResult],
    ) -> Vec<HybridSearchResult> {
        let mut results = Vec::new();
        let mut k_idx = 0;
        let mut s_idx = 0;
        let mut k_from_keyword = true;

        while (k_idx < keyword_results.len() || s_idx < semantic_results.len())
            && results.len() < self.config.max_results
        {
            if k_from_keyword && k_idx < keyword_results.len() {
                let r = &keyword_results[k_idx];
                results.push(HybridSearchResult {
                    id: r.id.clone(),
                    session_id: r.session_id.to_string(),
                    doc_type: r.doc_type.clone(),
                    content: r.snippet.clone(),
                    keyword_score: r.score,
                    semantic_score: 0.0,
                    fusion_score: r.score,
                    sources: vec![ResultSource::Keyword],
                });
                k_idx += 1;
            } else if s_idx < semantic_results.len() {
                let r = &semantic_results[s_idx];
                results.push(HybridSearchResult {
                    id: r.id.clone(),
                    session_id: r.document.session_id.clone(),
                    doc_type: r.document.doc_type.clone(),
                    content: r.document.content.clone(),
                    keyword_score: 0.0,
                    semantic_score: r.score,
                    fusion_score: r.score,
                    sources: vec![ResultSource::Semantic],
                });
                s_idx += 1;
            }
            k_from_keyword = !k_from_keyword;
        }

        // Deduplicate by ID
        let mut seen = std::collections::HashSet::new();
        results.retain(|r| {
            if seen.contains(&r.id) {
                false
            } else {
                seen.insert(r.id.clone());
                true
            }
        });

        results
    }
}

/// Query Router - analyzes queries and routes to optimal search strategy
#[derive(Debug, Clone)]
pub struct QueryRouter {
    /// Configuration
    pub config: RouterConfig,
}

#[derive(Debug, Clone)]
/// Configuration options for query routing.
pub struct RouterConfig {
    /// Minimum query length for semantic search
    pub min_semantic_length: usize,
    /// Keywords that suggest keyword search
    pub exact_keywords: Vec<String>,
    /// Enable automatic routing
    pub auto_route: bool,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            min_semantic_length: 3,
            exact_keywords: vec!["exact".to_string(), "quote".to_string(), "\"".to_string()],
            auto_route: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Search strategy recommendation
pub enum SearchStrategy {
    /// Keyword-only search
    Keyword,
    /// Semantic-only search
    Semantic,
    /// Hybrid search (both)
    Hybrid,
}

impl QueryRouter {
    /// Create a new query router
    pub fn new(config: Option<RouterConfig>) -> Self {
        Self {
            config: config.unwrap_or_default(),
        }
    }

    /// Analyze query and recommend search strategy
    pub fn analyze_query(&self, query: &str) -> SearchStrategy {
        if !self.config.auto_route {
            return SearchStrategy::Hybrid;
        }

        // Check for exact match keywords
        for keyword in &self.config.exact_keywords {
            if query.contains(keyword) {
                return SearchStrategy::Keyword;
            }
        }

        // Short queries benefit more from keyword search
        let word_count = query.split_whitespace().count();
        if word_count < self.config.min_semantic_length {
            return SearchStrategy::Keyword;
        }

        // Longer, natural language queries benefit from semantic search
        if word_count > 5 {
            return SearchStrategy::Semantic;
        }

        SearchStrategy::Hybrid
    }

    /// Get recommendation explanation
    pub fn explain_recommendation(&self, query: &str, strategy: SearchStrategy) -> String {
        let word_count = query.split_whitespace().count();

        match strategy {
            SearchStrategy::Keyword => {
                if self.config.exact_keywords.iter().any(|k| query.contains(k)) {
                    format!(
                        "Keyword search recommended: query contains exact-match indicators ({} words)",
                        word_count
                    )
                } else {
                    format!(
                        "Keyword search recommended: short query ({} words) benefits from exact matching",
                        word_count
                    )
                }
            }
            SearchStrategy::Semantic => format!(
                "Semantic search recommended: natural language query ({} words) benefits from conceptual matching",
                word_count
            ),
            SearchStrategy::Hybrid => format!(
                "Hybrid search recommended: balanced query ({} words) benefits from both exact and conceptual matching",
                word_count
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::SearchResult;
    use crate::vector::VectorSearchResult;
    use uuid;

    /// Helper to create a test HybridSearchEngine with a mock keyword searcher
    struct MockKeywordSearcher {
        results: Vec<SearchResult>,
    }

    impl MockKeywordSearcher {
        fn new(results: Vec<SearchResult>) -> Self {
            Self { results }
        }
    }

    impl KeywordSearcher for MockKeywordSearcher {
        fn search(&self, _query: &crate::search::SearchQuery) -> Result<Vec<SearchResult>> {
            Ok(self.results.clone())
        }
    }

    fn create_test_engine<K: KeywordSearcher + 'static>(
        keyword_searcher: Arc<K>,
        config: Option<HybridSearchConfig>,
    ) -> HybridSearchEngine<K> {
        HybridSearchEngine::new(keyword_searcher, None, config)
    }

    #[test]
    fn test_rrf_fusion() {
        let config = HybridSearchConfig::default();
        let engine = create_test_engine(Arc::new(MockKeywordSearcher::new(vec![])), Some(config));

        let session_uuid = uuid::Uuid::new_v4();
        let keyword_results = vec![
            SearchResult {
                id: "doc1".to_string(),
                session_id: session_uuid,
                doc_type: "message".to_string(),
                score: 10.0,
                snippet: "content 1".to_string(),
                timestamp: chrono::Utc::now(),
            },
            SearchResult {
                id: "doc2".to_string(),
                session_id: session_uuid,
                doc_type: "message".to_string(),
                score: 5.0,
                snippet: "content 2".to_string(),
                timestamp: chrono::Utc::now(),
            },
        ];

        let semantic_results = vec![
            VectorSearchResult {
                id: "doc3".to_string(),
                score: 0.9,
                document: crate::vector::DocumentInfo {
                    id: "doc3".to_string(),
                    session_id: "s1".to_string(),
                    doc_type: "message".to_string(),
                    content: "content 3".to_string(),
                    timestamp: chrono::Utc::now(),
                },
            },
            VectorSearchResult {
                id: "doc1".to_string(),
                score: 0.8,
                document: crate::vector::DocumentInfo {
                    id: "doc1".to_string(),
                    session_id: "s1".to_string(),
                    doc_type: "message".to_string(),
                    content: "content 1".to_string(),
                    timestamp: chrono::Utc::now(),
                },
            },
        ];

        let results = engine.fuse_rrf(&keyword_results, &semantic_results);

        // doc1 appears in both, should have high score
        let doc1 = results.iter().find(|r| r.id == "doc1").unwrap();
        assert!(doc1.sources.contains(&ResultSource::Both));
        assert!(doc1.fusion_score > 0.0);

        // doc2 only in keyword
        let doc2 = results.iter().find(|r| r.id == "doc2").unwrap();
        assert!(doc2.sources.contains(&ResultSource::Keyword));

        // doc3 only in semantic
        let doc3 = results.iter().find(|r| r.id == "doc3").unwrap();
        assert!(doc3.sources.contains(&ResultSource::Semantic));
    }

    #[test]
    fn test_query_router() {
        let router = QueryRouter::new(None);

        // Short query -> Keyword
        assert_eq!(router.analyze_query("hello"), SearchStrategy::Keyword);

        // Long query -> Semantic
        assert_eq!(
            router.analyze_query("What are the key principles of effective software architecture?"),
            SearchStrategy::Semantic
        );

        // Medium query -> Hybrid
        assert_eq!(
            router.analyze_query("How to implement Rust async"),
            SearchStrategy::Hybrid
        );

        // Query with quotes -> Keyword
        assert_eq!(
            router.analyze_query("search for \"exact phrase\""),
            SearchStrategy::Keyword
        );
    }

    #[test]
    fn test_router_explanation() {
        let router = QueryRouter::new(None);

        let explanation = router.explain_recommendation("hello", SearchStrategy::Keyword);
        assert!(explanation.contains("short query"));

        let explanation = router.explain_recommendation(
            "What are the key principles of effective software architecture?",
            SearchStrategy::Semantic,
        );
        assert!(explanation.contains("natural language"));
    }
}
