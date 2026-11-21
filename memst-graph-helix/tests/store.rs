//! Store-level graph: owners, the writer lock, read-only handles, migration.

use memst_core::error::Error;
use memst_core::graph::{EntitySearch, KnowledgeGraph, SearchMethod};
use memst_core::types::{Entity, EntityQuery, Relationship};
use memst_graph_helix::{
    migrate_store, open_store, unmigrated_sessions, BackendChoice, GraphLocation, HelixGraphBackend,
    HelixGraphOptions, MigrationReport, StoreGraphOptions,
};
use std::path::Path;
use tempfile::TempDir;
use uuid::Uuid;

fn graph_for(dir: &Path, owner: &str) -> KnowledgeGraph {
    let backend = HelixGraphBackend::open_with(dir, HelixGraphOptions::for_owner(owner)).unwrap();
    KnowledgeGraph::with_backend(Box::new(backend))
}

/// Native search needs its index, which is built in the background.
fn native_names(graph: &KnowledgeGraph, text: &str) -> Vec<String> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        let result = graph.search_entities(&EntitySearch::text(text)).unwrap();
        // "found nothing" means HelixDB answered, with no hits; any other
        // fallback reason means the index is not ready yet.
        let answered = result.method == SearchMethod::Native
            || result.fallback_reason.as_deref().is_some_and(|r| r.contains("found nothing"));
        if answered || std::time::Instant::now() > deadline {
            return result.hits.into_iter().map(|h| h.entity.name).collect();
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

#[test]
fn owners_never_see_or_change_each_others_data() {
    let dir = TempDir::new().unwrap();
    let session = Uuid::new_v4();
    let secret = Entity::new("Project Falcon", "project", session);
    let boss = Entity::new("Alice", "person", session);
    let edge = Relationship::new(boss.id, "leads", secret.id, session);
    {
        let mut alice = graph_for(dir.path(), "alice");
        alice.add_entities(&[secret.clone(), boss.clone()]).unwrap();
        alice.add_relationship(edge.clone()).unwrap();
        alice.set_entity_embedding(secret.id, &[1.0, 0.0]).unwrap();
        assert_eq!(native_names(&alice, "falcon"), vec!["Project Falcon"]);
    }

    {
        let mut bob = graph_for(dir.path(), "bob");
        // Reads: nothing of alice's is visible, by id, scan, traversal or search.
        assert!(bob.get_entity(secret.id).unwrap().is_none());
        assert!(bob.all_entities().unwrap().is_empty());
        assert!(bob.all_relationships().unwrap().is_empty());
        assert!(bob.get_relationship(edge.id).unwrap().is_none());
        assert!(bob.get_relationships(boss.id).unwrap().is_empty());
        assert!(bob.find_entities(EntityQuery::new().with_session(session)).unwrap().is_empty());
        assert!(bob.embedded_entity_ids().unwrap().is_empty());
        let stats = bob.stats().unwrap();
        assert_eq!((stats.entity_count, stats.relationship_count), (0, 0));
        assert!(native_names(&bob, "falcon").is_empty());
        let by_vector = EntitySearch { embedding: Some(vec![1.0, 0.0]), ..Default::default() };
        assert!(bob.search_entities(&by_vector).unwrap().hits.is_empty());

        // Writes: bob cannot update, delete, link to or overwrite alice's data.
        assert!(!bob.update_entity(secret.id, |e| e.name = "hijacked".into()).unwrap());
        assert!(!bob.delete_entity(secret.id).unwrap());
        assert!(!bob.delete_relationship(edge.id).unwrap());
        assert!(!bob.set_entity_embedding(secret.id, &[0.0, 1.0]).unwrap());
        let mine = bob.add_entity(Entity::new("Bob", "person", session)).unwrap();
        assert!(bob.add_relationship(Relationship::new(mine.id, "spies_on", secret.id, session)).is_err());
        let mut forged = secret.clone();
        forged.name = "forged".into();
        assert!(bob.add_entity(forged).is_err(), "an id owned by someone else cannot be taken");

        // Same name and type under another owner is bob's own, new entity.
        let (own, created) = bob.resolve_or_add_entity(Entity::new("Alice", "person", session)).unwrap();
        assert!(created);
        assert_ne!(own.id, boss.id);
        assert_eq!(bob.all_entities().unwrap().len(), 2);
    }

    let alice = graph_for(dir.path(), "alice");
    assert_eq!(alice.get_entity(secret.id).unwrap().unwrap().name, "Project Falcon");
    assert_eq!(alice.all_entities().unwrap().len(), 2);
    assert_eq!(alice.all_relationships().unwrap().len(), 1);
    assert_eq!(alice.embedded_entity_ids().unwrap().len(), 1);
    assert_eq!(native_names(&alice, "falcon"), vec!["Project Falcon"]);
    assert!(native_names(&alice, "bob").is_empty());
}

#[test]
fn second_writer_is_refused_and_readers_still_read() {
    let dir = TempDir::new().unwrap();
    let entity = Entity::new("Rust", "language", Uuid::new_v4());
    {
        graph_for(dir.path(), "default").add_entity(entity.clone()).unwrap();
    }

    let mut writer = graph_for(dir.path(), "default");

    let refused = HelixGraphBackend::open(dir.path()).err().expect("second writer must fail");
    let Error::LockError(message) = &refused else {
        panic!("expected a lock error, got {refused:?}");
    };
    assert!(message.contains(&std::process::id().to_string()), "names the holder: {message}");

    // The first writer was not disturbed.
    writer.add_entity(Entity::new("Cargo", "tool", Uuid::new_v4())).unwrap();

    // A read-only handle opens alongside it, reads, and cannot write.
    let reader = HelixGraphBackend::open_with(dir.path(), HelixGraphOptions::default().read_only()).unwrap();
    assert!(reader.is_read_only());
    let mut reader = KnowledgeGraph::with_backend(Box::new(reader));
    assert_eq!(reader.get_entity(entity.id).unwrap().unwrap().name, "Rust");
    let write = reader.add_entity(Entity::new("nope", "x", Uuid::new_v4()));
    assert!(matches!(write, Err(Error::InvalidOperation(m)) if m.contains("read-only")));
    assert!(reader.delete_entity(entity.id).is_err());
    drop(reader);

    // Once the writer is gone, the next one gets in.
    drop(writer);
    assert_eq!(graph_for(dir.path(), "default").all_entities().unwrap().len(), 2);

    // Reading a graph that does not exist is an error, not an empty graph.
    let empty = TempDir::new().unwrap();
    assert!(HelixGraphBackend::open_with(empty.path(), HelixGraphOptions::default().read_only()).is_err());
}

#[test]
fn migrates_per_session_graphs_into_the_store_graph() {
    let store = TempDir::new().unwrap();
    let (first, second) = (Uuid::new_v4(), Uuid::new_v4());
    let dir_of = |session: Uuid| store.path().join("sessions").join(session.to_string());
    let options = StoreGraphOptions::default();

    // Session one: a MemSt 1.0 graph.json.
    let a = Entity::new("Alice", "person", first);
    let rust_one = Entity::new("Rust", "language", first);
    let uses = Relationship::new(a.id, "uses", rust_one.id, first);
    {
        std::fs::create_dir_all(dir_of(first)).unwrap();
        let mut json = KnowledgeGraph::new(&dir_of(first)).unwrap();
        json.add_entities(&[a.clone(), rust_one.clone()]).unwrap();
        json.add_relationship(uses.clone()).unwrap();
        json.set_entity_embedding(a.id, &[0.5, 0.5]).unwrap();
    }
    // Session two: a per-session HelixDB graph from before graphs were shared.
    let rust_two = Entity::new("Rust", "language", second);
    {
        let legacy = HelixGraphOptions { owner: None, ..HelixGraphOptions::default() };
        let backend = HelixGraphBackend::open_with(&dir_of(second), legacy).unwrap();
        KnowledgeGraph::with_backend(Box::new(backend)).add_entity(rust_two.clone()).unwrap();
    }

    // Before migration each session still opens its own graph.
    let (graph, location) = open_store(store.path(), &dir_of(first), &options).unwrap();
    assert_eq!((location, graph.backend_kind()), (GraphLocation::LegacySession, "json"));
    drop(graph);
    let (graph, location) = open_store(store.path(), &dir_of(second), &options).unwrap();
    assert_eq!((location, graph.backend_kind()), (GraphLocation::LegacySession, "helix"));
    assert_eq!(graph.all_entities().unwrap().len(), 1);
    drop(graph);
    assert_eq!(unmigrated_sessions(store.path()).unwrap().len(), 2);

    let report = migrate_store(store.path(), "default").unwrap();
    assert_eq!(report, MigrationReport { sessions: 2, entities: 3, relationships: 1 });
    assert!(unmigrated_sessions(store.path()).unwrap().is_empty());
    assert_eq!(migrate_store(store.path(), "default").unwrap(), MigrationReport::default());
    assert!(dir_of(first).join("graph.json").exists(), "the old graph is kept");

    // Now every session opens the shared graph.
    let (graph, location) = open_store(store.path(), &dir_of(first), &options).unwrap();
    assert_eq!((location, graph.backend_kind()), (GraphLocation::Store, "helix"));
    let stats = graph.stats().unwrap();
    assert_eq!((stats.entity_count, stats.relationship_count), (3, 1));
    let (stored, subject, object) = graph.get_relationship(uses.id).unwrap().unwrap();
    assert_eq!((stored.id, subject.id, object.id), (uses.id, a.id, rust_one.id));
    assert_eq!(graph.embedded_entity_ids().unwrap().len(), 1);
    assert_eq!(graph.find_entities(EntityQuery::new().with_session(first)).unwrap().len(), 2);
    assert_eq!(graph.find_entities(EntityQuery::new().with_session(second)).unwrap().len(), 1);
    // Ids are preserved, so the two "Rust" entities are still two.
    assert!(graph.get_entity(rust_two.id).unwrap().is_some());
    drop(graph);

    // A brand-new session of a store that has a shared graph uses it too.
    let third = dir_of(Uuid::new_v4());
    std::fs::create_dir_all(&third).unwrap();
    let (_, location) = open_store(store.path(), &third, &options).unwrap();
    assert_eq!(location, GraphLocation::Store);

    // Explicit JSON still reaches the session file.
    let json_options = StoreGraphOptions { choice: BackendChoice::Json, ..StoreGraphOptions::default() };
    let (graph, location) = open_store(store.path(), &dir_of(first), &json_options).unwrap();
    assert_eq!((location, graph.backend_kind()), (GraphLocation::LegacySession, "json"));
}

#[test]
fn a_new_store_gets_a_shared_graph() {
    let store = TempDir::new().unwrap();
    let session_dir = store.path().join("sessions").join(Uuid::new_v4().to_string());
    std::fs::create_dir_all(&session_dir).unwrap();
    let (graph, location) = open_store(store.path(), &session_dir, &StoreGraphOptions::default()).unwrap();
    assert_eq!((location, graph.backend_kind()), (GraphLocation::Store, "helix"));
    assert!(HelixGraphBackend::exists(store.path()));
    assert!(!HelixGraphBackend::exists(&session_dir));
}

#[test]
fn forgetting_a_message_removes_its_facts_from_the_shared_graph() {
    let dir = TempDir::new().unwrap();
    let mut graph = graph_for(dir.path(), "default");
    let session = Uuid::new_v4();
    let (m1, m2) = (Uuid::new_v4(), Uuid::new_v4());
    let alice = graph.add_entity(Entity::new("Alice", "person", session).with_source(m1)).unwrap();
    let lyon = graph.add_entity(Entity::new("Lyon", "place", session).with_source(m1)).unwrap();
    let acme = graph.add_entity(Entity::new("Acme", "organisation", session).with_source(m2)).unwrap();
    graph.add_relationship(Relationship::new(alice.id, "lives_in", lyon.id, session).with_source(m1)).unwrap();
    graph.add_relationship(Relationship::new(alice.id, "works_at", acme.id, session).with_source(m2)).unwrap();

    let gone = graph.forget_message(m1).unwrap();
    assert_eq!((gone.relationships, gone.entities, gone.entities_kept), (1, 1, 1));
    drop(graph);

    // It is still gone after the database is opened again.
    let mut graph = graph_for(dir.path(), "default");
    assert!(graph.get_entity(lyon.id).unwrap().is_none());
    assert!(graph.get_entity(alice.id).unwrap().is_some());
    assert_eq!(graph.backend().relationship_count().unwrap(), 1);
    let gone = graph.forget_session(session).unwrap();
    assert_eq!((gone.relationships, gone.entities), (1, 2));
    assert_eq!((graph.backend().entity_count().unwrap(), graph.backend().relationship_count().unwrap()), (0, 0));
}
