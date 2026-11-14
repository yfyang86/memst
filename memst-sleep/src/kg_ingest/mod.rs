//! From extraction to the knowledge graph.
//!
//! [`KgExtractionService`] turns text into names and
//! triples. This module writes that into a [`KnowledgeGraph`]:
//!
//! - an entity that the graph already knows (same name and type) is reused
//!   and linked to the session, rather than duplicated;
//! - a relationship that is already there is reinforced rather than repeated;
//! - everything written carries its provenance: the session, the message it
//!   came from, and the extraction run;
//! - new and changed entities are embedded, so they are searchable by meaning
//!   straight away.

use crate::error::{KgError, Result};
use crate::kg_extract::KgExtractionService;
use memst_core::embed::Embedder;
use memst_core::graph::KnowledgeGraph;
use memst_core::types::{
    Entity, EntityId, EntityQuery, KgExtractionResult, Relationship, RelationshipQuery, TemporalRelevance,
};
use std::collections::HashMap;
use uuid::Uuid;

/// Attribute recording which extraction run last wrote an entity. The
/// leading underscore keeps it out of the entity's searchable text.
pub const RUN_ATTRIBUTE: &str = "_extraction_run";

/// Predicate linking a participant to an event.
pub const PARTICIPATED_IN: &str = "participated_in";

/// Predicates that hold one value at a time by default: a new object for
/// the same subject replaces the old one instead of standing beside it.
/// Compared after lower-casing and turning spaces and hyphens into `_`.
pub const SINGLE_VALUED_PREDICATES: &[&str] = &[
    "lives_in",
    "resides_in",
    "based_in",
    "works_at",
    "works_for",
    "employed_by",
    "married_to",
    "reports_to",
    "has_role",
    "has_status",
];

/// Decides how extracted relationships are written.
#[derive(Debug, Clone)]
pub struct IngestPolicy {
    /// Predicates that hold one value at a time (see
    /// [`SINGLE_VALUED_PREDICATES`]). For these, a new object supersedes the
    /// subject's current one. Every other predicate may have many objects.
    pub single_valued: Vec<String>,
}

impl Default for IngestPolicy {
    fn default() -> Self {
        Self {
            single_valued: SINGLE_VALUED_PREDICATES.iter().map(|p| p.to_string()).collect(),
        }
    }
}

impl IngestPolicy {
    /// Treat every predicate as many-valued: nothing is ever superseded.
    pub fn never_supersede() -> Self {
        Self { single_valued: Vec::new() }
    }

    fn is_single_valued(&self, predicate: &str) -> bool {
        let normalised = normalise_predicate(predicate);
        self.single_valued.iter().any(|p| *p == normalised)
    }
}

/// The form predicates are stored in: lower case, words joined by `_`.
/// Models write the same relation as "lives in", "Lives_In" or "lives-in";
/// stored as written, those would be three different predicates.
pub fn normalise_predicate(predicate: &str) -> String {
    predicate
        .trim()
        .to_lowercase()
        .split(|c: char| c.is_whitespace() || c == '-' || c == '_')
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

/// Where an extraction came from.
#[derive(Debug, Clone)]
pub struct Provenance {
    /// Session the text belongs to.
    pub session_id: Uuid,
    /// Message the text came from, when it is a single message (or the
    /// first message of a batch).
    pub source_message_id: Option<Uuid>,
    /// Identifies this extraction run.
    pub run_id: String,
}

impl Provenance {
    /// Provenance for a new run over `session_id`.
    pub fn new(session_id: Uuid) -> Self {
        Self {
            session_id,
            source_message_id: None,
            run_id: format!("run-{}", Uuid::new_v4().simple()),
        }
    }

    /// Set the source message.
    pub fn with_message(mut self, message_id: Uuid) -> Self {
        self.source_message_id = Some(message_id);
        self
    }
}

/// What an ingestion changed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IngestReport {
    /// Entities that did not exist before.
    pub entities_created: usize,
    /// Mentions resolved to an entity the graph already had.
    pub entities_reused: usize,
    /// Relationships that did not exist before.
    pub relationships_created: usize,
    /// Relationships seen again: confidence raised, relevance restored.
    pub relationships_reinforced: usize,
    /// Earlier facts replaced by a new value of a single-valued predicate.
    pub relationships_superseded: usize,
    /// Relationships dropped because an endpoint could not be identified.
    pub relationships_skipped: usize,
    /// Entities embedded (or re-embedded) afterwards.
    pub entities_embedded: usize,
}

impl IngestReport {
    fn add(&mut self, other: IngestReport) {
        self.entities_created += other.entities_created;
        self.entities_reused += other.entities_reused;
        self.relationships_created += other.relationships_created;
        self.relationships_reinforced += other.relationships_reinforced;
        self.relationships_superseded += other.relationships_superseded;
        self.relationships_skipped += other.relationships_skipped;
        self.entities_embedded += other.entities_embedded;
    }
}

/// Write an extraction result into the graph with the default
/// [`IngestPolicy`]. Does not embed; see [`ingest_and_embed`].
pub fn ingest_extraction(
    graph: &mut KnowledgeGraph,
    extraction: &KgExtractionResult,
    provenance: &Provenance,
) -> Result<IngestReport> {
    ingest_extraction_with(graph, extraction, provenance, &IngestPolicy::default())
}

/// Write an extraction result into the graph. Does not embed.
pub fn ingest_extraction_with(
    graph: &mut KnowledgeGraph,
    extraction: &KgExtractionResult,
    provenance: &Provenance,
    policy: &IngestPolicy,
) -> Result<IngestReport> {
    let mut report = IngestReport::default();
    // Lower-cased name -> id, for resolving relationship endpoints.
    let mut by_name: HashMap<String, EntityId> = HashMap::new();

    for extracted in &extraction.entities {
        let entity = new_entity(
            &extracted.name,
            &extracted.entity_type,
            extracted.attributes.clone(),
            extracted.confidence,
            &extracted.temporal_relevance,
            provenance,
        );
        let (stored, created) = graph.resolve_or_add_entity(entity)?;
        if created {
            report.entities_created += 1;
        } else {
            report.entities_reused += 1;
        }
        by_name.insert(extracted.name.to_lowercase(), stored.id);
    }

    // Events become entities, with an edge from each participant.
    let mut event_edges = Vec::new();
    for event in &extraction.events {
        let mut attributes = match &event.attributes {
            serde_json::Value::Object(map) => map.clone(),
            _ => serde_json::Map::new(),
        };
        if let Some(timestamp) = event.timestamp {
            attributes.insert("timestamp".to_string(), serde_json::json!(timestamp.to_rfc3339()));
        }
        let entity = new_entity(
            &event.name,
            if event.event_type.is_empty() { "event" } else { &event.event_type },
            serde_json::Value::Object(attributes),
            event.confidence,
            &TemporalRelevance::LongTerm,
            provenance,
        );
        let (stored, created) = graph.resolve_or_add_entity(entity)?;
        if created {
            report.entities_created += 1;
        } else {
            report.entities_reused += 1;
        }
        by_name.insert(event.name.to_lowercase(), stored.id);
        for participant in &event.participants {
            event_edges.push((participant.clone(), event.name.clone(), event.confidence));
        }
    }

    let triples = extraction
        .relationships
        .iter()
        .map(|r| (r.subject.as_str(), r.predicate.as_str(), r.object.as_str(), r.confidence, &r.temporal_type))
        .chain(event_edges.iter().map(|(participant, event, confidence)| {
            (participant.as_str(), PARTICIPATED_IN, event.as_str(), *confidence, &TemporalRelevance::LongTerm)
        }));

    for (subject, predicate, object, confidence, temporal) in triples {
        let predicate = normalise_predicate(predicate);
        let predicate = predicate.as_str();
        let (Some(subject_id), Some(object_id)) = (
            resolve_endpoint(graph, &by_name, subject)?,
            resolve_endpoint(graph, &by_name, object)?,
        ) else {
            report.relationships_skipped += 1;
            continue;
        };

        if policy.is_single_valued(predicate) {
            let mut fact = Relationship::new(subject_id, predicate, object_id, provenance.session_id)
                .with_confidence(confidence);
            fact.source_message_id = provenance.source_message_id;
            fact.half_life_days = temporal.default_half_life_days();
            fact.is_stable = temporal.is_stable();
            let new_id = fact.id;
            let (stored, superseded) = graph.assert_single_valued(fact)?;
            if stored.id == new_id {
                report.relationships_created += 1;
            } else {
                report.relationships_reinforced += 1;
            }
            report.relationships_superseded += superseded.len();
            continue;
        }

        let existing = graph
            .find_relationships(
                RelationshipQuery::new()
                    .with_subject(subject_id)
                    .with_object(object_id)
                    .with_predicate(predicate),
            )?
            .into_iter()
            .next();
        match existing {
            Some((relationship, _, _)) => {
                // Seen again: at least as confident as before, and fresh.
                graph.update_relationship_with(relationship.id, |r| {
                    r.confidence = r.confidence.max(confidence);
                    r.relevance = 1.0;
                    r.last_decay_at = chrono::Utc::now();
                })?;
                report.relationships_reinforced += 1;
            }
            None => {
                let mut relationship =
                    Relationship::new(subject_id, predicate, object_id, provenance.session_id)
                        .with_confidence(confidence);
                relationship.source_message_id = provenance.source_message_id;
                relationship.half_life_days = temporal.default_half_life_days();
                relationship.is_stable = temporal.is_stable();
                graph.add_relationship(relationship)?;
                report.relationships_created += 1;
            }
        }
    }

    Ok(report)
}

/// [`ingest_extraction`], then embed whatever is new or changed.
pub fn ingest_and_embed(
    graph: &mut KnowledgeGraph,
    extraction: &KgExtractionResult,
    provenance: &Provenance,
    embedder: Option<&dyn Embedder>,
) -> Result<IngestReport> {
    let mut report = ingest_extraction(graph, extraction, provenance)?;
    if let Some(embedder) = embedder {
        report.entities_embedded = graph.embed_pending(embedder)?;
    }
    Ok(report)
}

/// Runs extraction over text and writes the result into a graph.
pub struct GraphIngestor {
    extractor: KgExtractionService,
}

impl GraphIngestor {
    /// Ingest using this extraction service.
    pub fn new(extractor: KgExtractionService) -> Self {
        Self { extractor }
    }

    /// Extract from one text and write the result into `graph`.
    pub async fn ingest_text(
        &self,
        graph: &mut KnowledgeGraph,
        text: &str,
        provenance: &Provenance,
        embedder: Option<&dyn Embedder>,
    ) -> Result<IngestReport> {
        let extraction = self
            .extractor
            .extract_from_text(text)
            .await
            .map_err(|error| KgError::llm_error(error.to_string()))?;
        ingest_and_embed(graph, &extraction, provenance, embedder)
    }

    /// Extract from a session's messages, in batches, and write the results
    /// into `graph`. Each message is `(id, text)`. Embedding happens once, at
    /// the end. A batch whose extraction fails stops the run with the error;
    /// what earlier batches wrote stays.
    pub async fn ingest_messages(
        &self,
        graph: &mut KnowledgeGraph,
        session_id: Uuid,
        messages: &[(Uuid, String)],
        batch_chars: usize,
        embedder: Option<&dyn Embedder>,
    ) -> Result<IngestReport> {
        let mut provenance = Provenance::new(session_id);
        let mut report = IngestReport::default();
        for batch in batches(messages, batch_chars.max(1)) {
            provenance.source_message_id = batch.first().map(|(id, _)| *id);
            let text = batch.iter().map(|(_, text)| text.as_str()).collect::<Vec<_>>().join("\n\n");
            let extraction = self
                .extractor
                .extract_from_text(&text)
                .await
                .map_err(|error| KgError::llm_error(error.to_string()))?;
            report.add(ingest_extraction(graph, &extraction, &provenance)?);
        }
        if let Some(embedder) = embedder {
            report.entities_embedded = graph.embed_pending(embedder)?;
        }
        Ok(report)
    }
}

/// Split messages into consecutive groups of at most `max_chars` characters
/// (a single longer message is a group of its own).
fn batches(messages: &[(Uuid, String)], max_chars: usize) -> Vec<&[(Uuid, String)]> {
    let mut out = Vec::new();
    let (mut start, mut size) = (0, 0);
    for (index, (_, text)) in messages.iter().enumerate() {
        if index > start && size + text.len() > max_chars {
            out.push(&messages[start..index]);
            start = index;
            size = 0;
        }
        size += text.len();
    }
    if start < messages.len() {
        out.push(&messages[start..]);
    }
    out
}

fn new_entity(
    name: &str,
    entity_type: &str,
    attributes: serde_json::Value,
    confidence: f32,
    temporal: &TemporalRelevance,
    provenance: &Provenance,
) -> Entity {
    let mut attributes = match attributes {
        serde_json::Value::Object(map) => map,
        _ => serde_json::Map::new(),
    };
    attributes.insert(RUN_ATTRIBUTE.to_string(), serde_json::json!(provenance.run_id));

    let mut entity = Entity::new(name.trim(), entity_type.trim(), provenance.session_id)
        .with_attributes(serde_json::Value::Object(attributes))
        .with_confidence(confidence);
    entity.source_message_id = provenance.source_message_id;
    entity.half_life_days = temporal.default_half_life_days();
    entity.is_stable = temporal.is_stable();
    entity
}

/// The entity a relationship endpoint names: one written in this run, or
/// otherwise the most important non-deprecated entity with exactly that name.
fn resolve_endpoint(
    graph: &KnowledgeGraph,
    by_name: &HashMap<String, EntityId>,
    name: &str,
) -> Result<Option<EntityId>> {
    let wanted = name.trim().to_lowercase();
    if let Some(id) = by_name.get(&wanted) {
        return Ok(Some(*id));
    }
    Ok(graph
        .find_entities(EntityQuery::new().with_name_contains(wanted.clone()))?
        .into_iter()
        .find(|e| !e.is_deprecated && e.name.to_lowercase() == wanted)
        .map(|e| e.id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn predicates_are_normalised() {
        for spelling in ["lives in", "Lives_In", " lives-in ", "LIVES  IN"] {
            assert_eq!(normalise_predicate(spelling), "lives_in");
        }
        assert_eq!(normalise_predicate("就职于"), "就职于");
        assert!(IngestPolicy::default().is_single_valued("Works At"));
        assert!(!IngestPolicy::default().is_single_valued("uses"));
        assert!(!IngestPolicy::never_supersede().is_single_valued("lives_in"));
    }

    #[test]
    fn batches_respect_the_size_limit() {
        let message = |text: &str| (Uuid::new_v4(), text.to_string());
        let messages = vec![message("aaaa"), message("bbbb"), message("cccccccccccc"), message("dd")];
        let sizes: Vec<usize> = batches(&messages, 8).iter().map(|b| b.len()).collect();
        assert_eq!(sizes, vec![2, 1, 1], "an oversized message stands alone");
        assert!(batches(&[], 8).is_empty());
        assert_eq!(batches(&messages, 1000).len(), 1);
    }
}
