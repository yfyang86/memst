//! The HelixDB backend must pass the same suite as the JSON backend.

use memst_core::graph::{conformance, EntitySearch, EntitySearchResult, KnowledgeGraph, SearchMethod};
use memst_core::types::{Entity, EntityQuery, Relationship};
use memst_graph_helix::{migrate_json_to_helix, open, BackendChoice, HelixGraphBackend};
use tempfile::TempDir;
use uuid::Uuid;

fn helix(dir: &std::path::Path) -> KnowledgeGraph {
    open(dir, BackendChoice::Helix).unwrap()
}

#[test]
fn helix_backend_conforms() {
    let mut dirs = Vec::new();
    conformance::run_all(&mut || {
        let dir = TempDir::new().unwrap();
        let graph = helix(dir.path());
        assert_eq!(graph.backend_kind(), "helix");
        dirs.push(dir);
        graph
    });
}

#[test]
fn helix_backend_persists_across_reopen() {
    let dir = TempDir::new().unwrap();
    conformance::persistence(&mut || helix(dir.path()));
}

#[test]
fn auto_prefers_existing_data_and_defaults_to_helix() {
    let fresh = TempDir::new().unwrap();
    assert_eq!(open(fresh.path(), BackendChoice::Auto).unwrap().backend_kind(), "helix");

    let legacy = TempDir::new().unwrap();
    KnowledgeGraph::new(legacy.path())
        .unwrap()
        .add_entity(Entity::new("old", "test", Uuid::new_v4()))
        .unwrap();
    assert_eq!(open(legacy.path(), BackendChoice::Auto).unwrap().backend_kind(), "json");
}

#[test]
fn migrates_a_json_graph() {
    let dir = TempDir::new().unwrap();
    let session = Uuid::new_v4();
    let (a, b) = (
        Entity::new("Alice", "person", session),
        Entity::new("Rust", "technology", session),
    );
    let rel = Relationship::new(a.id, "uses", b.id, session).with_confidence(0.9);
    {
        let mut json = KnowledgeGraph::new(dir.path()).unwrap();
        json.add_entities(&[a.clone(), b.clone()]).unwrap();
        json.add_relationship(rel.clone()).unwrap();
        json.set_entity_embedding(a.id, &[0.5, 0.5]).unwrap();
    }

    assert_eq!(migrate_json_to_helix(dir.path()).unwrap(), (2, 1));
    assert!(HelixGraphBackend::exists(dir.path()));
    assert!(dir.path().join("graph.json").exists(), "json is kept as a backup");

    let graph = open(dir.path(), BackendChoice::Auto).unwrap();
    assert_eq!(graph.backend_kind(), "helix");
    assert_eq!(graph.embedded_entity_ids().unwrap().len(), 1, "embeddings migrate too");
    let (stored, subject, object) = graph.get_relationship(rel.id).unwrap().unwrap();
    assert_eq!(stored.confidence, 0.9);
    assert_eq!((subject.name.as_str(), object.name.as_str()), ("Alice", "Rust"));

    // Migrating twice must not duplicate anything.
    drop(graph);
    assert_eq!(migrate_json_to_helix(dir.path()).unwrap(), (2, 1));
    let graph = open(dir.path(), BackendChoice::Auto).unwrap();
    let stats = graph.stats().unwrap();
    assert_eq!((stats.entity_count, stats.relationship_count), (2, 1));
}

/// Bulk writes and scans cross the backend's internal page size.
#[test]
fn pages_through_large_graphs() {
    let dir = TempDir::new().unwrap();
    let mut graph = helix(dir.path());
    let session = Uuid::new_v4();
    let entities: Vec<Entity> = (0..4_500)
        .map(|i| Entity::new(format!("entity-{i}"), if i % 2 == 0 { "even" } else { "odd" }, session))
        .collect();
    graph.add_entities(&entities).unwrap();

    assert_eq!(graph.all_entities().unwrap().len(), 4_500);
    assert_eq!(graph.find_entities(EntityQuery::new().with_type("odd")).unwrap().len(), 2_250);
    assert_eq!(graph.stats().unwrap().entity_count, 4_500);

    // Rewriting the same batch updates in place.
    graph.update_all_entities(|e| e.relevance = 0.5).unwrap();
    assert_eq!(graph.all_entities().unwrap().len(), 4_500);
    assert!((graph.average_relevance().unwrap() - 0.5).abs() < 1e-6);
}

/// The backend must be usable (and droppable) from inside a Tokio runtime.
#[test]
fn works_inside_an_async_runtime() {
    let dir = TempDir::new().unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
    runtime.block_on(async {
        let mut graph = helix(dir.path());
        let entity = graph.add_entity(Entity::new("async", "test", Uuid::new_v4())).unwrap();
        assert!(graph.get_entity(entity.id).unwrap().is_some());
        drop(graph);
    });
}

/// Index builds are asynchronous: poll until HelixDB itself answers.
fn search_native(graph: &KnowledgeGraph, query: &EntitySearch) -> EntitySearchResult {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        let result = graph.search_entities(query).unwrap();
        if result.method == SearchMethod::Native {
            return result;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "native search never became available: {:?}",
            result.fallback_reason
        );
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

fn names(result: &EntitySearchResult) -> Vec<&str> {
    result.hits.iter().map(|h| h.entity.name.as_str()).collect()
}

#[test]
fn searches_with_helix_bm25_and_vectors() {
    let dir = TempDir::new().unwrap();
    let mut graph = helix(dir.path());
    let session = Uuid::new_v4();
    let rust = Entity::new("Rust", "language", session)
        .with_attributes(serde_json::json!({"paradigm": "systems programming", "tool": "cargo"}));
    let python = Entity::new("Python", "language", session)
        .with_attributes(serde_json::json!({"paradigm": "scripting"}));
    let alice = Entity::new("Alice Smith", "person", session);
    graph.add_entities(&[rust.clone(), python.clone(), alice.clone()]).unwrap();

    // BM25 covers attributes and type, which substring-on-name cannot reach.
    let by_attribute = search_native(&graph, &EntitySearch::text("systems programming"));
    assert_eq!(names(&by_attribute), vec!["Rust"]);
    assert!(by_attribute.fallback_reason.is_none());
    let by_type = search_native(&graph, &EntitySearch::text("language"));
    assert_eq!(by_type.hits.len(), 2);
    // Multi-word queries are ORed and ranked: the entity matching more terms wins.
    let ranked = search_native(&graph, &EntitySearch::text("cargo language"));
    assert_eq!(names(&ranked)[0], "Rust");
    assert!(ranked.hits[0].score > ranked.hits[1].score);

    // An updated entity is searchable under its new text, not its old.
    graph.update_entity(alice.id, |e| e.name = "Alice Jones".to_string()).unwrap();
    assert_eq!(names(&search_native(&graph, &EntitySearch::text("jones"))), vec!["Alice Jones"]);
    assert_eq!(
        graph.search_entities(&EntitySearch::text("smith")).unwrap().hits.len(),
        0
    );

    // Vector search.
    graph.set_entity_embedding(rust.id, &[1.0, 0.0, 0.0]).unwrap();
    graph.set_entity_embedding(python.id, &[0.8, 0.6, 0.0]).unwrap();
    graph.set_entity_embedding(alice.id, &[0.0, 0.0, 1.0]).unwrap();
    let semantic = EntitySearch { embedding: Some(vec![0.1, 0.0, 0.9]), limit: 1, ..Default::default() };
    assert_eq!(names(&search_native(&graph, &semantic)), vec!["Alice Jones"]);

    // Hybrid: one request, both indexes, fused.
    let hybrid = EntitySearch::text("language").with_embedding(vec![0.8, 0.6, 0.0]);
    assert_eq!(names(&search_native(&graph, &hybrid))[0], "Python");

    // Embeddings survive an entity update and a reopen.
    graph.access_entity(python.id).unwrap();
    drop(graph);
    let graph = helix(dir.path());
    assert_eq!(graph.embedded_entity_ids().unwrap().len(), 3);
    assert_eq!(names(&search_native(&graph, &hybrid))[0], "Python");
}

#[test]
fn falls_back_when_helix_search_cannot_answer() {
    let dir = TempDir::new().unwrap();
    let mut graph = helix(dir.path());
    let session = Uuid::new_v4();
    let rust = graph.add_entity(Entity::new("Rustacean", "person", session)).unwrap();
    search_native(&graph, &EntitySearch::text("rustacean"));

    // BM25 matches whole terms only, so a partial word finds nothing in
    // HelixDB; MemSt's substring matching then answers.
    let partial = graph.search_entities(&EntitySearch::text("rust")).unwrap();
    assert_eq!(partial.method, SearchMethod::Fallback);
    assert_eq!(names(&partial), vec!["Rustacean"]);
    assert!(partial.fallback_reason.unwrap().contains("found nothing"));

    // No embeddings were ever stored, so there is no vector index: the
    // native request fails, and the text half is still answered.
    let hybrid = EntitySearch::text("rustacean").with_embedding(vec![1.0, 0.0]);
    let result = graph.search_entities(&hybrid).unwrap();
    assert_eq!(result.method, SearchMethod::Fallback);
    assert_eq!(names(&result), vec!["Rustacean"]);
    assert!(result.fallback_reason.unwrap().contains("search failed"));

    // A query vector of the wrong dimension fails in HelixDB; the fallback
    // skips embeddings it cannot compare and still answers from the text.
    graph.set_entity_embedding(rust.id, &[1.0, 0.0, 0.0]).unwrap();
    let wrong = EntitySearch::text("rustacean").with_embedding(vec![1.0, 0.0]);
    search_native(&graph, &EntitySearch { embedding: Some(vec![1.0, 0.0, 0.0]), ..Default::default() });
    let result = graph.search_entities(&wrong).unwrap();
    assert_eq!(result.method, SearchMethod::Fallback);
    assert_eq!(names(&result), vec!["Rustacean"]);
}

#[test]
fn json_backend_always_uses_the_fallback() {
    let dir = TempDir::new().unwrap();
    let mut graph = open(dir.path(), BackendChoice::Json).unwrap();
    graph.add_entity(Entity::new("Rust", "language", Uuid::new_v4())).unwrap();
    let result = graph.search_entities(&EntitySearch::text("rust")).unwrap();
    assert_eq!(result.method, SearchMethod::Fallback);
    assert!(result.fallback_reason.unwrap().contains("no native search"));
}

#[test]
fn searches_relationships_with_helix_bm25() {
    let dir = TempDir::new().unwrap();
    let mut graph = helix(dir.path());
    let session = Uuid::new_v4();
    let alice = graph.add_entity(Entity::new("Alice", "person", session)).unwrap();
    let bob = graph.add_entity(Entity::new("Bob", "person", session)).unwrap();
    let rust = graph.add_entity(Entity::new("Rust", "language", session)).unwrap();
    let uses = graph.add_relationship(Relationship::new(alice.id, "uses", rust.id, session)).unwrap();
    let manages = graph.add_relationship(Relationship::new(bob.id, "manages", alice.id, session)).unwrap();

    // Wait for the edge index, then HelixDB itself must answer.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    let result = loop {
        let result = graph.search_relationships("alice uses", 0).unwrap();
        if result.method == SearchMethod::Native {
            break result;
        }
        assert!(std::time::Instant::now() < deadline, "{:?}", result.fallback_reason);
        std::thread::sleep(std::time::Duration::from_millis(100));
    };
    let ids: Vec<Uuid> = result.hits.iter().map(|(t, _)| t.0.id).collect();
    assert_eq!(ids, vec![uses.id, manages.id], "ranked by how much of the query matches");
    assert!(result.hits[0].1 > result.hits[1].1);

    // Updating an edge keeps it searchable; deleting removes it.
    graph.update_relationship(uses.id, Some(0.2)).unwrap();
    assert_eq!(graph.search_relationships("rust", 0).unwrap().hits.len(), 1);
    graph.delete_relationship(uses.id).unwrap();
    let after = graph.search_relationships("rust", 0).unwrap();
    assert!(after.hits.is_empty());

    // Another owner's edges are not searchable from here.
    drop(graph);
    let other = memst_graph_helix::HelixGraphBackend::open_with(
        dir.path(),
        memst_graph_helix::HelixGraphOptions::for_owner("someone-else"),
    )
    .unwrap();
    let other = KnowledgeGraph::with_backend(Box::new(other));
    assert!(other.search_relationships("manages", 0).unwrap().hits.is_empty());
}
