//! The HelixDB backend must pass the same suite as the JSON backend.

use memst_core::graph::{conformance, KnowledgeGraph};
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
    }

    assert_eq!(migrate_json_to_helix(dir.path()).unwrap(), (2, 1));
    assert!(HelixGraphBackend::exists(dir.path()));
    assert!(dir.path().join("graph.json").exists(), "json is kept as a backup");

    let graph = open(dir.path(), BackendChoice::Auto).unwrap();
    assert_eq!(graph.backend_kind(), "helix");
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
