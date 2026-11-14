//! Entity search: backend indexes first, MemSt's own matching as fallback.

use super::backend::{rrf_fuse, GraphBackend};
use crate::error::{Error, Result};
use crate::types::{Entity, EntityId, EntityQuery};
use std::collections::HashMap;

/// What to search for. Give `text`, `embedding`, or both.
#[derive(Debug, Clone, Default)]
pub struct EntitySearch {
    /// Keyword query.
    pub text: Option<String>,
    /// Query vector; must have the dimension of the stored embeddings.
    pub embedding: Option<Vec<f32>>,
    /// Maximum number of hits (0 means 10).
    pub limit: usize,
    /// Also return deprecated entities.
    pub include_deprecated: bool,
    /// Only entities created in or seen in this session.
    pub session_id: Option<uuid::Uuid>,
}

impl EntitySearch {
    /// Keyword search.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            ..Self::default()
        }
    }

    /// Add a query vector (semantic search; hybrid when text is also set).
    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.embedding = Some(embedding);
        self
    }

    /// Set the maximum number of hits.
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }
}

/// Which code path produced a search result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMethod {
    /// The backend's own indexes (HelixDB BM25 and vector search).
    Native,
    /// MemSt's substring and brute-force cosine matching.
    Fallback,
}

/// One search hit.
#[derive(Debug, Clone)]
pub struct EntityHit {
    /// The matching entity.
    pub entity: Entity,
    /// Relevance; higher is better. Only comparable within one result.
    pub score: f32,
}

/// Search hits, best first, and how they were found.
#[derive(Debug, Clone)]
pub struct EntitySearchResult {
    /// Hits, best first.
    pub hits: Vec<EntityHit>,
    /// The path that answered.
    pub method: SearchMethod,
    /// Why the backend's native search was not used, when `method` is
    /// [`SearchMethod::Fallback`].
    pub fallback_reason: Option<String>,
}

/// Relationship search hits, best first, and how they were found.
#[derive(Debug, Clone)]
pub struct RelationshipSearchResult {
    /// `(relationship, subject, object)` with a relevance score, best first.
    pub hits: Vec<(super::Triple, f32)>,
    /// The path that answered.
    pub method: SearchMethod,
    /// Why the backend's native search was not used, on fallback.
    pub fallback_reason: Option<String>,
}

/// The text a relationship is found by: "subject predicate object".
pub fn relationship_text(subject: &Entity, predicate: &str, object: &Entity) -> String {
    format!("{} {} {}", subject.name, predicate, object.name)
}

pub(super) fn run_relationships(
    backend: &dyn GraphBackend,
    text: &str,
    limit: usize,
) -> Result<(Vec<(crate::types::RelationshipId, f32)>, SearchMethod, Option<String>)> {
    let text = text.trim();
    if text.is_empty() {
        return Err(Error::InvalidOperation("relationship search needs a text query".to_string()));
    }
    let limit = if limit == 0 { 10 } else { limit };
    let reason = match backend.search_relationships(text, limit) {
        Ok(Some(ranked)) if !ranked.is_empty() => return Ok((ranked, SearchMethod::Native, None)),
        Ok(Some(_)) => format!("{} search found nothing", backend.kind()),
        Ok(None) => format!("the {} backend has no native search", backend.kind()),
        Err(error) => format!("{} search failed: {error}", backend.kind()),
    };

    // MemSt's own matching: the fraction of query words found (as
    // substrings) in "subject predicate object", ties broken by confidence
    // and relevance.
    let whole = text.to_lowercase();
    let words: Vec<&str> = whole
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    let relationships = backend.scan_relationships(&crate::types::RelationshipQuery::default())?;
    let mut ids: Vec<EntityId> = relationships.iter().flat_map(|r| [r.subject_id, r.object_id]).collect();
    ids.sort_unstable();
    ids.dedup();
    let entities: HashMap<EntityId, Entity> =
        backend.get_entities(&ids)?.into_iter().map(|e| (e.id, e)).collect();
    let mut ranked: Vec<(crate::types::RelationshipId, f32, f32)> = relationships
        .iter()
        .filter_map(|r| {
            let haystack =
                relationship_text(entities.get(&r.subject_id)?, &r.predicate, entities.get(&r.object_id)?)
                    .to_lowercase();
            let score = if haystack.contains(&whole) {
                1.0
            } else if words.is_empty() {
                0.0
            } else {
                words.iter().filter(|w| haystack.contains(**w)).count() as f32 / words.len() as f32
            };
            (score > 0.0).then_some((r.id, score, r.confidence * r.relevance))
        })
        .collect();
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then(b.2.total_cmp(&a.2)));
    ranked.truncate(limit);
    Ok((
        ranked.into_iter().map(|(id, score, _)| (id, score)).collect(),
        SearchMethod::Fallback,
        Some(reason),
    ))
}

pub(super) fn run(backend: &dyn GraphBackend, query: &EntitySearch) -> Result<EntitySearchResult> {
    let text = query.text.as_deref().map(str::trim).filter(|t| !t.is_empty());
    let embedding = query.embedding.as_deref().filter(|e| !e.is_empty());
    if text.is_none() && embedding.is_none() {
        return Err(Error::InvalidOperation(
            "entity search needs a text query or an embedding".to_string(),
        ));
    }
    let limit = if query.limit == 0 { 10 } else { query.limit };
    // Deprecated entities, and those outside the requested session, are
    // dropped after ranking, so ask for some spare.
    let fetch = match (query.include_deprecated, query.session_id) {
        (true, None) => limit,
        (_, None) => limit * 2,
        (_, Some(_)) => limit * 5,
    };

    let fallback_reason = match backend.search_entities(text, embedding, fetch) {
        Ok(Some(ranked)) => {
            let hits = materialise(backend, ranked, limit, query)?;
            if !hits.is_empty() {
                return Ok(EntitySearchResult {
                    hits,
                    method: SearchMethod::Native,
                    fallback_reason: None,
                });
            }
            format!("{} search found nothing", backend.kind())
        }
        Ok(None) => format!("the {} backend has no native search", backend.kind()),
        Err(error) => format!("{} search failed: {error}", backend.kind()),
    };

    let ranked = fallback(backend, text, embedding)?;
    Ok(EntitySearchResult {
        hits: materialise(backend, ranked, limit, query)?,
        method: SearchMethod::Fallback,
        fallback_reason: Some(fallback_reason),
    })
}

/// Load the ranked entities, keeping rank order.
fn materialise(
    backend: &dyn GraphBackend,
    ranked: Vec<(EntityId, f32)>,
    limit: usize,
    query: &EntitySearch,
) -> Result<Vec<EntityHit>> {
    let ids: Vec<EntityId> = ranked.iter().map(|(id, _)| *id).collect();
    let mut entities: HashMap<EntityId, Entity> = backend
        .get_entities(&ids)?
        .into_iter()
        .map(|e| (e.id, e))
        .collect();
    Ok(ranked
        .into_iter()
        .filter_map(|(id, score)| Some(EntityHit { entity: entities.remove(&id)?, score }))
        .filter(|hit| query.include_deprecated || !hit.entity.is_deprecated)
        .filter(|hit| query.session_id.map_or(true, |s| hit.entity.is_in_session(s)))
        .take(limit)
        .collect())
}

/// MemSt's own matching, used when the backend cannot answer.
fn fallback(
    backend: &dyn GraphBackend,
    text: Option<&str>,
    embedding: Option<&[f32]>,
) -> Result<Vec<(EntityId, f32)>> {
    let by_text = text.map(|t| substring_ranking(backend, t)).transpose()?;
    let by_vector = embedding.map(|e| cosine_ranking(backend, e)).transpose()?;
    Ok(match (by_text, by_vector) {
        (Some(text), Some(vector)) if !text.is_empty() && !vector.is_empty() => rrf_fuse(&[
            text.into_iter().map(|(id, _)| id).collect(),
            vector.into_iter().map(|(id, _)| id).collect(),
        ]),
        (Some(text), vector) if vector.as_ref().map_or(true, Vec::is_empty) => text,
        (_, Some(vector)) => vector,
        (Some(text), None) => text,
        (None, None) => Vec::new(),
    })
}

/// Case-insensitive substring matching on entity names. The score is the
/// fraction of query words found in the name (1.0 when the whole query is a
/// substring); ties are broken by entity importance, as in `find_entities`.
fn substring_ranking(backend: &dyn GraphBackend, text: &str) -> Result<Vec<(EntityId, f32)>> {
    let whole = text.to_lowercase();
    let words: Vec<&str> = whole
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    let mut matches: Vec<(Entity, f32)> = backend
        .scan_entities(&EntityQuery::default())?
        .into_iter()
        .filter_map(|entity| {
            let name = entity.name.to_lowercase();
            let score = if name.contains(&whole) {
                1.0
            } else if words.is_empty() {
                0.0
            } else {
                words.iter().filter(|w| name.contains(**w)).count() as f32 / words.len() as f32
            };
            (score > 0.0).then_some((entity, score))
        })
        .collect();
    matches.sort_by(|(a, a_score), (b, b_score)| {
        b_score
            .total_cmp(a_score)
            .then(b.importance().total_cmp(&a.importance()))
    });
    Ok(matches.into_iter().map(|(e, score)| (e.id, score)).collect())
}

/// Brute-force cosine similarity over every stored embedding. Embeddings of
/// another dimension, and non-positive similarities, are skipped.
fn cosine_ranking(backend: &dyn GraphBackend, query: &[f32]) -> Result<Vec<(EntityId, f32)>> {
    let mut ranked: Vec<(EntityId, f32)> = backend
        .entity_embeddings()?
        .into_iter()
        .filter(|(_, embedding)| embedding.len() == query.len())
        .map(|(id, embedding)| (id, cosine(query, &embedding)))
        .filter(|(_, similarity)| *similarity > 0.0)
        .collect();
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1));
    Ok(ranked)
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm = |v: &[f32]| v.iter().map(|x| x * x).sum::<f32>().sqrt();
    let denominator = norm(a) * norm(b);
    if denominator == 0.0 {
        0.0
    } else {
        dot / denominator
    }
}
