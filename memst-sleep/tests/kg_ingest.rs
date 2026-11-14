//! Extraction results are written into the graph, deduplicated, linked to
//! their source, and embedded.

mod common;

use common::MockLlmProvider;
use memst_core::graph::conformance::KeywordEmbedder;
use memst_core::graph::{EntitySearch, KnowledgeGraph};
use memst_core::types::{EntityQuery, RelationshipQuery};
use memst_sleep::kg_extract::KgExtractionService;
use memst_sleep::kg_ingest::{GraphIngestor, IngestReport, PARTICIPATED_IN, RUN_ATTRIBUTE};
use std::sync::Arc;
use uuid::Uuid;

const FIRST: &str = r#"{
  "entities": [
    {"name": "Alice", "type": "person", "attributes": {"role": "engineer"}, "confidence": 0.9, "temporal_relevance": "long_term"},
    {"name": "Rust", "type": "language", "attributes": {}, "confidence": 0.95, "temporal_relevance": "permanent"}
  ],
  "relationships": [
    {"subject": "Alice", "predicate": "uses", "object": "Rust", "confidence": 0.7, "temporal_type": "long_term"},
    {"subject": "Alice", "predicate": "knows", "object": "Nobody Mentioned", "confidence": 0.9, "temporal_type": "short_term"}
  ],
  "events": [
    {"name": "RustConf 2026", "type": "conference", "participants": ["Alice"], "timestamp": "2026-09-01T09:00:00Z", "attributes": {"city": "Paris"}, "confidence": 0.8}
  ]
}"#;

// A later conversation: Rust again (different case), Alice again, a new entity.
const SECOND: &str = r#"```json
{
  "entities": [
    {"name": "rust", "entity_type": "language", "attributes": {"paradigm": "systems"}, "confidence": 0.9, "temporal_relevance": "permanent"},
    {"name": "Tokio", "entity_type": "library", "attributes": {}, "confidence": 0.9, "temporal_relevance": "long_term"}
  ],
  "relationships": [
    {"subject": "Alice", "predicate": "uses", "object": "Rust", "confidence": 0.95, "temporal_type": "long_term"},
    {"subject": "Tokio", "predicate": "written_in", "object": "rust", "confidence": 0.9, "temporal_type": "permanent"}
  ],
  "events": []
}
```"#;

fn graph() -> (KnowledgeGraph, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    (KnowledgeGraph::new(dir.path()).unwrap(), dir)
}

fn ingestor(responses: &[&str]) -> GraphIngestor {
    let llm = MockLlmProvider::new(responses.iter().map(|r| r.to_string()).collect());
    GraphIngestor::new(KgExtractionService::new(Arc::new(llm)))
}

#[tokio::test]
async fn extraction_lands_in_the_graph_searchable() {
    let (mut graph, _dir) = graph();
    let embedder = KeywordEmbedder::new("test-model");
    let (first_session, second_session) = (Uuid::new_v4(), Uuid::new_v4());
    let (first_message, second_message) = (Uuid::new_v4(), Uuid::new_v4());

    let report = ingestor(&[FIRST])
        .ingest_messages(
            &mut graph,
            first_session,
            &[(first_message, "Alice, an engineer, uses Rust and went to RustConf.".to_string())],
            4000,
            Some(&embedder),
        )
        .await
        .unwrap();
    assert_eq!(
        report,
        IngestReport {
            entities_created: 3,
            entities_reused: 0,
            relationships_created: 2,
            relationships_reinforced: 0,
            relationships_superseded: 0,
            relationships_skipped: 1,
            entities_embedded: 3,
        }
    );

    // Provenance: session, message, run; temporal relevance sets the decay.
    let alice = graph.find_entities(EntityQuery::new().with_type("person")).unwrap().remove(0);
    assert_eq!(alice.session_id, first_session);
    assert_eq!(alice.source_message_id, Some(first_message));
    assert!(alice.attributes[RUN_ATTRIBUTE].as_str().unwrap().starts_with("run-"));
    assert_eq!(alice.attributes["role"], "engineer");
    assert!(!alice.search_text().contains("run-"), "the run id is not searchable text");
    let rust = graph.find_entities(EntityQuery::new().with_type("language")).unwrap().remove(0);
    assert!(rust.is_stable, "a permanent fact does not decay");
    assert_eq!(alice.half_life_days, 90.0);

    // The event is an entity, linked from its participant.
    let event = graph.find_entities(EntityQuery::new().with_type("conference")).unwrap().remove(0);
    assert_eq!(event.attributes["city"], "Paris");
    assert!(event.attributes["timestamp"].as_str().unwrap().starts_with("2026-09-01"));
    let attended = graph
        .find_relationships(RelationshipQuery::new().with_predicate(PARTICIPATED_IN))
        .unwrap();
    assert_eq!(attended.len(), 1);
    assert_eq!((attended[0].1.id, attended[0].2.id), (alice.id, event.id));

    // No manual step: the entities are already searchable, by words and by meaning.
    let by_text = graph.search_entities(&EntitySearch::text("alice")).unwrap();
    assert_eq!(by_text.hits[0].entity.id, alice.id);
    let by_meaning = EntitySearch { embedding: Some(vec![1.0, 0.05, 0.05]), limit: 1, ..Default::default() };
    assert_eq!(graph.search_entities(&by_meaning).unwrap().hits[0].entity.id, rust.id);

    // A second conversation, in another session.
    let report = ingestor(&[SECOND])
        .ingest_messages(
            &mut graph,
            second_session,
            &[(second_message, "Tokio is written in Rust; Alice still uses Rust.".to_string())],
            4000,
            Some(&embedder),
        )
        .await
        .unwrap();
    assert_eq!(
        report,
        IngestReport {
            entities_created: 1,
            entities_reused: 1,
            relationships_created: 1,
            relationships_reinforced: 1,
            relationships_superseded: 0,
            relationships_skipped: 0,
            // Tokio is new; Rust gained an attribute, so its text changed.
            entities_embedded: 2,
        }
    );

    // Rust is still one entity, now known to both sessions.
    let languages = graph.find_entities(EntityQuery::new().with_type("language")).unwrap();
    assert_eq!(languages.len(), 1);
    assert_eq!(languages[0].id, rust.id);
    assert_eq!(languages[0].session_id, first_session);
    assert_eq!(languages[0].seen_in, vec![second_session]);
    assert_eq!(languages[0].attributes["paradigm"], "systems");

    // "Alice uses Rust" was reinforced, not duplicated; Alice was found
    // although the second extraction did not list her as an entity.
    let uses = graph.find_relationships(RelationshipQuery::new().with_predicate("uses")).unwrap();
    assert_eq!(uses.len(), 1);
    assert_eq!(uses[0].0.confidence, 0.95);
    assert_eq!(uses[0].0.session_id, first_session, "the original provenance is kept");
    let written_in = graph.find_relationships(RelationshipQuery::new().with_predicate("written_in")).unwrap();
    assert_eq!(written_in[0].0.source_message_id, Some(second_message));
    assert_eq!(written_in[0].2.id, rust.id);

    // Everything is embedded and current.
    assert!(graph.entities_needing_embedding("test-model").unwrap().is_empty());
    assert_eq!(graph.all_entities().unwrap().len(), 4);
}

#[tokio::test]
async fn long_conversations_are_extracted_in_batches() {
    let (mut graph, _dir) = graph();
    let messages: Vec<(Uuid, String)> = (0..3).map(|i| (Uuid::new_v4(), format!("message {i} {}", "x".repeat(40)))).collect();
    let llm = Arc::new(MockLlmProvider::new(vec![FIRST.to_string(); 3]));
    let ingestor = GraphIngestor::new(KgExtractionService::new(llm.clone()));

    // 60 characters per batch: one message each.
    let report = ingestor.ingest_messages(&mut graph, Uuid::new_v4(), &messages, 60, None).await.unwrap();
    assert_eq!(llm.call_count(), 3);
    // The same extraction three times: created once, then reused and reinforced.
    assert_eq!((report.entities_created, report.entities_reused), (3, 6));
    assert_eq!((report.relationships_created, report.relationships_reinforced), (2, 4));
    assert_eq!(report.entities_embedded, 0, "no embedder was given");
}

#[tokio::test]
async fn a_bad_llm_answer_is_an_error_and_writes_nothing() {
    let (mut graph, _dir) = graph();
    let result = ingestor(&["I could not find anything to extract."])
        .ingest_messages(&mut graph, Uuid::new_v4(), &[(Uuid::new_v4(), "hello".to_string())], 4000, None)
        .await;
    assert!(result.is_err());
    assert!(graph.all_entities().unwrap().is_empty());
}

const IN_PARIS: &str = r#"{"entities": [
    {"name": "Alice", "type": "person", "confidence": 0.9},
    {"name": "Paris", "type": "city", "confidence": 0.9},
    {"name": "Rust", "type": "language", "confidence": 0.9}],
  "relationships": [
    {"subject": "Alice", "predicate": "lives in", "object": "Paris", "confidence": 0.8},
    {"subject": "Alice", "predicate": "uses", "object": "Rust", "confidence": 0.8}], "events": []}"#;

const IN_LYON: &str = r#"{"entities": [
    {"name": "Alice", "type": "person", "confidence": 0.9},
    {"name": "Lyon", "type": "city", "confidence": 0.9},
    {"name": "Python", "type": "language", "confidence": 0.9}],
  "relationships": [
    {"subject": "Alice", "predicate": "lives_in", "object": "Lyon", "confidence": 0.9},
    {"subject": "Alice", "predicate": "uses", "object": "Python", "confidence": 0.8}], "events": []}"#;

#[tokio::test]
async fn a_later_conversation_updates_a_single_valued_fact() {
    let (mut graph, _dir) = graph();
    let session = Uuid::new_v4();
    let message = |text: &str| vec![(Uuid::new_v4(), text.to_string())];

    ingestor(&[IN_PARIS])
        .ingest_messages(&mut graph, session, &message("I live in Paris and use Rust."), 4000, None)
        .await
        .unwrap();
    let report = ingestor(&[IN_LYON])
        .ingest_messages(&mut graph, session, &message("I moved to Lyon. I also use Python now."), 4000, None)
        .await
        .unwrap();
    assert_eq!(report.relationships_superseded, 1);
    assert_eq!(report.relationships_created, 2);

    // Where she lives has one answer; what she uses has two.
    let alice = graph.find_entities(EntityQuery::new().with_type("person")).unwrap().remove(0);
    let objects = |graph: &KnowledgeGraph, query: RelationshipQuery| -> Vec<String> {
        let mut names: Vec<String> = graph.find_relationships(query).unwrap().into_iter().map(|t| t.2.name).collect();
        names.sort();
        names
    };
    let lives = RelationshipQuery::new().with_subject(alice.id);
    assert_eq!(objects(&graph, lives.clone().with_predicate("lives_in")), vec!["Lyon"]);
    assert_eq!(objects(&graph, RelationshipQuery::new().with_predicate("uses")), vec!["Python", "Rust"]);

    // The old answer is history, not gone. ("lives in" and "lives_in" are
    // stored as the same predicate.)
    let all = graph.find_relationships(RelationshipQuery::new().with_subject(alice.id).with_superseded()).unwrap();
    let paris = all.iter().find(|t| t.2.name == "Paris").expect("still on record");
    assert!(!paris.0.is_current());
    assert!(paris.0.superseded_by.is_some());
}
