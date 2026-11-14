//! Backend conformance suite.
//!
//! Every [`GraphBackend`](super::GraphBackend) must behave identically through
//! [`KnowledgeGraph`]. Backend crates call [`run_all`] from their own tests
//! with a factory for fresh, empty graphs, and [`persistence`] with a factory
//! that reopens the same store. The functions panic on the first violation.

use super::{Direction, EntitySearch, KnowledgeGraph};
use crate::types::{Entity, EntityQuery, Relationship, RelationshipQuery};
use uuid::Uuid;

/// Run every check that needs only an empty graph.
pub fn run_all(fresh: &mut dyn FnMut() -> KnowledgeGraph) {
    entity_roundtrip(&mut fresh());
    entity_queries(&mut fresh());
    relationship_roundtrip(&mut fresh());
    relationship_requires_endpoints(&mut fresh());
    delete_entity_detaches(&mut fresh());
    updates_are_persisted(&mut fresh());
    migrate_relationships(&mut fresh());
    neighborhood(&mut fresh());
    stats_and_decay_helpers(&mut fresh());
    search(&mut fresh());
    sessions_share_entities(&mut fresh());
    embeddings_follow_entities(&mut fresh());
    relationship_search(&mut fresh());
}

/// `reopen` must return the same store each time; the previous handle is
/// dropped before the next call.
pub fn persistence(reopen: &mut dyn FnMut() -> KnowledgeGraph) {
    let session = Uuid::new_v4();
    let a = Entity::new("Persistent A", "test", session);
    let b = Entity::new("Persistent B", "test", session);
    let rel = Relationship::new(a.id, "linked_to", b.id, session).with_confidence(0.4);
    {
        let mut graph = reopen();
        graph.add_entity(a.clone()).unwrap();
        graph.add_entity(b.clone()).unwrap();
        graph.add_relationship(rel.clone()).unwrap();
        graph.flush().unwrap();
    }
    let graph = reopen();
    assert_eq!(graph.all_entities().unwrap().len(), 2);
    assert_eq!(graph.get_entity(a.id).unwrap().unwrap().name, "Persistent A");
    let (stored, subject, object) = graph.get_relationship(rel.id).unwrap().unwrap();
    assert_eq!(stored.id, rel.id);
    assert_eq!(stored.confidence, 0.4);
    assert_eq!((subject.id, object.id), (a.id, b.id));
}

fn entity_roundtrip(graph: &mut KnowledgeGraph) {
    let message = Uuid::new_v4();
    let mut entity = Entity::new("Rust \"lang\" – 语言", "technology", Uuid::new_v4())
        .with_attributes(serde_json::json!({"paradigm": ["systems", "functional"], "year": 2010}))
        .with_source(message)
        .with_confidence(0.75);
    entity.half_life_days = 12.5;
    entity.deprecate("superseded");

    graph.add_entity(entity.clone()).unwrap();
    let stored = graph.get_entity(entity.id).unwrap().expect("entity stored");

    assert_eq!(stored.id, entity.id);
    assert_eq!(stored.name, entity.name);
    assert_eq!(stored.entity_type, entity.entity_type);
    assert_eq!(stored.attributes, entity.attributes);
    assert_eq!(stored.session_id, entity.session_id);
    assert_eq!(stored.source_message_id, Some(message));
    assert_eq!(stored.confidence, 0.75);
    assert_eq!(stored.half_life_days, 12.5);
    assert_eq!(stored.created_at, entity.created_at);
    assert_eq!(stored.last_decay_at, entity.last_decay_at);
    assert!(stored.is_deprecated);
    assert_eq!(stored.deprecation_reason.as_deref(), Some("superseded"));

    assert!(graph.get_entity(Uuid::new_v4()).unwrap().is_none());

    // put is an upsert, not a duplicate insert
    graph.add_entity(entity.clone()).unwrap();
    assert_eq!(graph.all_entities().unwrap().len(), 1);

    // clearing an optional field must stick
    graph
        .update_entity(entity.id, |e| {
            e.is_deprecated = false;
            e.deprecation_reason = None;
        })
        .unwrap();
    let cleared = graph.get_entity(entity.id).unwrap().unwrap();
    assert!(!cleared.is_deprecated);
    assert_eq!(cleared.deprecation_reason, None);
}

fn entity_queries(graph: &mut KnowledgeGraph) {
    let session = Uuid::new_v4();
    let mut popular = Entity::new("Rust Programming", "technology", session).with_confidence(0.6);
    popular.access_count = 20;
    graph.add_entity(popular.clone()).unwrap();
    graph
        .add_entity(Entity::new("Rust the metal", "material", session).with_confidence(0.9))
        .unwrap();
    graph
        .add_entity(Entity::new("Python", "technology", session).with_confidence(0.3))
        .unwrap();

    let by_type = graph.find_entities(EntityQuery::new().with_type("technology")).unwrap();
    assert_eq!(by_type.len(), 2);

    let by_name = graph.find_entities(EntityQuery::new().with_name_contains("RUST")).unwrap();
    assert_eq!(by_name.len(), 2);
    assert_eq!(by_name[0].id, popular.id, "results are ranked by importance");

    let confident = graph.find_entities(EntityQuery::new().with_min_confidence(0.5)).unwrap();
    assert_eq!(confident.len(), 2);

    let limited = graph.find_entities(EntityQuery::new().with_limit(1)).unwrap();
    assert_eq!(limited.len(), 1);
    assert_eq!(limited[0].id, popular.id);

    let combined = graph
        .find_entities(EntityQuery::new().with_type("technology").with_name_contains("rust"))
        .unwrap();
    assert_eq!(combined.len(), 1);
}

fn relationship_roundtrip(graph: &mut KnowledgeGraph) {
    let session = Uuid::new_v4();
    let a = graph.add_entity(Entity::new("A", "test", session)).unwrap();
    let b = graph.add_entity(Entity::new("B", "test", session)).unwrap();
    let c = graph.add_entity(Entity::new("C", "test", session)).unwrap();

    let message = Uuid::new_v4();
    let mut knows = Relationship::new(a.id, "knows", b.id, session)
        .with_confidence(0.8)
        .with_source(message);
    knows.relevance = 0.5;
    knows.half_life_days = 7.0;
    knows.is_stable = true;
    graph.add_relationship(knows.clone()).unwrap();
    // predicates are free text, not identifiers
    let odd = Relationship::new(a.id, "works at / 就职于", c.id, session).with_confidence(0.2);
    graph.add_relationship(odd.clone()).unwrap();
    graph
        .add_relationship(Relationship::new(b.id, "knows", c.id, session))
        .unwrap();

    let (stored, subject, object) = graph.get_relationship(knows.id).unwrap().expect("stored");
    assert_eq!(stored.id, knows.id);
    assert_eq!(stored.predicate, "knows");
    assert_eq!(stored.confidence, 0.8);
    assert_eq!(stored.source_message_id, Some(message));
    assert_eq!(stored.relevance, 0.5);
    assert_eq!(stored.half_life_days, 7.0);
    assert!(stored.is_stable);
    assert_eq!(stored.created_at, knows.created_at);
    assert_eq!((subject.id, object.id), (a.id, b.id));

    let outgoing = graph.get_relationships(a.id).unwrap();
    assert_eq!(outgoing.len(), 2);
    assert!(outgoing.iter().all(|(r, s, _)| r.subject_id == a.id && s.id == a.id));
    assert_eq!(graph.relationships_of(c.id, Direction::Incoming).unwrap().len(), 2);
    assert_eq!(graph.relationships_of(b.id, Direction::Both).unwrap().len(), 2);

    let by_predicate = graph
        .find_relationships(RelationshipQuery::new().with_predicate("knows"))
        .unwrap();
    assert_eq!(by_predicate.len(), 2);
    assert!(by_predicate.iter().any(|(r, _, _)| r.id == knows.id), "ids survive queries");

    let by_odd = graph
        .find_relationships(RelationshipQuery::new().with_predicate("works at / 就职于"))
        .unwrap();
    assert_eq!(by_odd.len(), 1);
    assert_eq!(by_odd[0].0.id, odd.id);

    let by_object = graph.find_relationships(RelationshipQuery::new().with_object(c.id)).unwrap();
    assert_eq!(by_object.len(), 2);
    let by_both = graph
        .find_relationships(RelationshipQuery::new().with_subject(a.id).with_object(c.id))
        .unwrap();
    assert_eq!(by_both.len(), 1);
    let confident = graph
        .find_relationships(RelationshipQuery::new().with_min_confidence(0.5))
        .unwrap();
    assert_eq!(confident.len(), 2);
    let limited = graph.find_relationships(RelationshipQuery::new().with_limit(1)).unwrap();
    assert_eq!(limited.len(), 1);
    assert_eq!(graph.all_relationships().unwrap().len(), 3);

    assert!(graph.update_relationship(knows.id, Some(0.1)).unwrap());
    assert_eq!(graph.get_relationship(knows.id).unwrap().unwrap().0.confidence, 0.1);
    assert!(!graph.update_relationship(Uuid::new_v4(), Some(0.1)).unwrap());
    assert_eq!(graph.all_relationships().unwrap().len(), 3, "update is not an insert");

    assert!(graph.delete_relationship(knows.id).unwrap());
    assert!(!graph.delete_relationship(knows.id).unwrap());
    assert!(graph.get_relationship(knows.id).unwrap().is_none());
    assert_eq!(graph.all_relationships().unwrap().len(), 2);
}

fn relationship_requires_endpoints(graph: &mut KnowledgeGraph) {
    let session = Uuid::new_v4();
    let a = graph.add_entity(Entity::new("A", "test", session)).unwrap();
    let missing = Uuid::new_v4();
    assert!(graph
        .add_relationship(Relationship::new(a.id, "knows", missing, session))
        .is_err());
    assert!(graph
        .add_relationship(Relationship::new(missing, "knows", a.id, session))
        .is_err());
    assert_eq!(graph.all_relationships().unwrap().len(), 0);
}

fn delete_entity_detaches(graph: &mut KnowledgeGraph) {
    let session = Uuid::new_v4();
    let a = graph.add_entity(Entity::new("A", "test", session)).unwrap();
    let b = graph.add_entity(Entity::new("B", "test", session)).unwrap();
    let c = graph.add_entity(Entity::new("C", "test", session)).unwrap();
    graph.add_relationship(Relationship::new(a.id, "r", b.id, session)).unwrap();
    graph.add_relationship(Relationship::new(b.id, "r", c.id, session)).unwrap();
    graph.add_relationship(Relationship::new(a.id, "r", c.id, session)).unwrap();

    assert!(graph.delete_entity(b.id).unwrap());
    assert!(!graph.delete_entity(b.id).unwrap());
    assert!(graph.get_entity(b.id).unwrap().is_none());
    let remaining = graph.all_relationships().unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!((remaining[0].0.subject_id, remaining[0].0.object_id), (a.id, c.id));
    let stats = graph.stats().unwrap();
    assert_eq!((stats.entity_count, stats.relationship_count), (2, 1));
}

fn updates_are_persisted(graph: &mut KnowledgeGraph) {
    let entity = graph.add_entity(Entity::new("Test", "test", Uuid::new_v4())).unwrap();
    assert!(graph.access_entity(entity.id).unwrap());
    assert!(graph.access_entity(entity.id).unwrap());
    assert!(!graph.access_entity(Uuid::new_v4()).unwrap());
    assert_eq!(graph.get_entity(entity.id).unwrap().unwrap().access_count, 2);

    graph.update_all_entities(|e| e.relevance = 0.25).unwrap();
    assert_eq!(graph.get_entity(entity.id).unwrap().unwrap().relevance, 0.25);
    assert_eq!(graph.all_entities().unwrap().len(), 1);
}

fn migrate_relationships(graph: &mut KnowledgeGraph) {
    let session = Uuid::new_v4();
    let keep = graph.add_entity(Entity::new("Rust", "technology", session)).unwrap();
    let dup = graph.add_entity(Entity::new("rust-lang", "technology", session)).unwrap();
    let user = graph.add_entity(Entity::new("User", "person", session)).unwrap();
    let cargo = graph.add_entity(Entity::new("Cargo", "tool", session)).unwrap();

    // user -uses-> dup (duplicate of user -uses-> keep), dup -has-> cargo
    graph.add_relationship(Relationship::new(user.id, "uses", keep.id, session)).unwrap();
    graph.add_relationship(Relationship::new(user.id, "uses", dup.id, session)).unwrap();
    let mut has = Relationship::new(dup.id, "has", cargo.id, session).with_confidence(0.7);
    has.relevance = 0.3;
    has.is_stable = true;
    graph.add_relationship(has).unwrap();

    assert!(graph.migrate_relationships(dup.id, Uuid::new_v4()).is_err());
    let migrated = graph.migrate_relationships(dup.id, keep.id).unwrap();
    assert_eq!(migrated, 1, "the duplicate 'uses' edge is dropped, 'has' moves");

    assert!(graph.relationships_of(dup.id, Direction::Both).unwrap().is_empty());
    let all = graph.all_relationships().unwrap();
    assert_eq!(all.len(), 2);
    let (moved, subject, object) = all.iter().find(|(r, _, _)| r.predicate == "has").unwrap();
    assert_eq!((subject.id, object.id), (keep.id, cargo.id));
    assert_eq!(moved.confidence, 0.7);
    assert_eq!(moved.relevance, 0.3, "decay state survives migration");
    assert!(moved.is_stable);
}

fn neighborhood(graph: &mut KnowledgeGraph) {
    // a -> b -> c -> d, plus x -> a
    let session = Uuid::new_v4();
    let [a, b, c, d, x] = ["a", "b", "c", "d", "x"]
        .map(|name| graph.add_entity(Entity::new(name, "node", session)).unwrap());
    for (s, o) in [(&a, &b), (&b, &c), (&c, &d), (&x, &a)] {
        graph.add_relationship(Relationship::new(s.id, "next", o.id, session)).unwrap();
    }

    let names = |hits: Vec<(Entity, usize)>| {
        let mut out: Vec<(String, usize)> = hits.into_iter().map(|(e, h)| (e.name, h)).collect();
        out.sort();
        out
    };
    let pair = |name: &str, hops: usize| (name.to_string(), hops);

    assert_eq!(
        names(graph.neighborhood(a.id, 2, Direction::Outgoing).unwrap()),
        vec![pair("b", 1), pair("c", 2)]
    );
    assert_eq!(
        names(graph.neighborhood(a.id, 1, Direction::Incoming).unwrap()),
        vec![pair("x", 1)]
    );
    assert_eq!(
        names(graph.neighborhood(b.id, 2, Direction::Both).unwrap()),
        vec![pair("a", 1), pair("c", 1), pair("d", 2), pair("x", 2)]
    );
    assert!(graph.neighborhood(d.id, 3, Direction::Outgoing).unwrap().is_empty());
    assert!(graph.neighborhood(a.id, 0, Direction::Both).unwrap().is_empty());
}

fn stats_and_decay_helpers(graph: &mut KnowledgeGraph) {
    assert_eq!(graph.average_relevance().unwrap(), 0.0);
    let session = Uuid::new_v4();
    let mut fresh = Entity::new("fresh", "t1", session);
    fresh.relevance = 0.9;
    let mut stale = Entity::new("stale", "t1", session);
    stale.relevance = 0.1;
    let mut gone = Entity::new("gone", "t2", session);
    gone.relevance = 0.05;
    gone.deprecate("old");
    graph.add_entities(&[fresh.clone(), stale.clone(), gone.clone()]).unwrap();
    graph.add_relationship(Relationship::new(fresh.id, "p", stale.id, session)).unwrap();

    let stats = graph.stats().unwrap();
    assert_eq!((stats.entity_count, stats.relationship_count), (3, 1));
    assert_eq!(stats.entity_types.get("t1"), Some(&2));
    assert_eq!(stats.predicates.get("p"), Some(&1));

    let ranked = graph.entities_by_relevance().unwrap();
    assert_eq!(ranked.iter().map(|e| e.id).collect::<Vec<_>>(), vec![fresh.id, stale.id]);
    let stale_ids: Vec<_> = graph.stale_entities(0.5).unwrap().into_iter().map(|e| e.id).collect();
    assert_eq!(stale_ids, vec![stale.id]);
    assert_eq!(graph.deprecated_entities().unwrap().len(), 1);
    assert_eq!(graph.active_entity_count().unwrap(), 2);
    assert!((graph.average_relevance().unwrap() - 0.35).abs() < 1e-6);
    assert_eq!(graph.update_all_relationships(|r| r.relevance = 0.5).unwrap(), 1);
    assert_eq!(graph.all_relationships().unwrap()[0].0.relevance, 0.5);
}

/// Search must give the same answers whichever path serves it (a backend's
/// own indexes, or the fallback while they are unavailable).
fn search(graph: &mut KnowledgeGraph) {
    let session = Uuid::new_v4();
    let rust = graph.add_entity(Entity::new("Rust Programming Language", "technology", session)).unwrap();
    let python = graph.add_entity(Entity::new("Python", "technology", session)).unwrap();
    let alice = graph.add_entity(Entity::new("Alice Smith", "person", session)).unwrap();
    let mut old = Entity::new("Rust 2015 edition", "technology", session);
    old.deprecate("superseded");
    graph.add_entity(old.clone()).unwrap();

    fn ids(graph: &KnowledgeGraph, query: EntitySearch) -> Vec<Uuid> {
        graph.search_entities(&query).unwrap().hits.into_iter().map(|h| h.entity.id).collect()
    }

    assert_eq!(ids(graph, EntitySearch::text("rust")), vec![rust.id], "deprecated entities are hidden");
    assert_eq!(ids(graph, EntitySearch::text("alice")), vec![alice.id]);
    assert!(ids(graph, EntitySearch::text("zzz-no-such-thing")).is_empty());
    let mut with_deprecated = EntitySearch::text("rust");
    with_deprecated.include_deprecated = true;
    assert_eq!(ids(graph, with_deprecated).len(), 2);
    assert_eq!(ids(graph, EntitySearch::text("rust").with_limit(1)).len(), 1);
    assert!(graph.search_entities(&EntitySearch::default()).is_err());
    assert!(graph.search_entities(&EntitySearch::text("   ")).is_err());

    // Embeddings: stored, survive an entity update, and drive vector search.
    assert!(graph.set_entity_embedding(rust.id, &[1.0, 0.0, 0.0]).unwrap());
    assert!(graph.set_entity_embedding(python.id, &[0.8, 0.6, 0.0]).unwrap());
    assert!(graph.set_entity_embedding(alice.id, &[0.0, 0.0, 1.0]).unwrap());
    assert!(!graph.set_entity_embedding(Uuid::new_v4(), &[1.0, 0.0, 0.0]).unwrap());
    assert!(graph.set_entity_embedding(rust.id, &[]).is_err());
    graph.access_entity(rust.id).unwrap();
    assert_eq!(graph.embedded_entity_ids().unwrap().len(), 3);

    let semantic = EntitySearch { embedding: Some(vec![0.9, 0.1, 0.0]), limit: 2, ..Default::default() };
    assert_eq!(ids(graph, semantic), vec![rust.id, python.id]);

    // Hybrid: the entity that matches both the words and the vector wins.
    let hybrid = EntitySearch::text("python").with_embedding(vec![0.8, 0.6, 0.0]);
    assert_eq!(ids(graph, hybrid)[0], python.id);

    graph.delete_entity(alice.id).unwrap();
    assert_eq!(graph.embedded_entity_ids().unwrap().len(), 2);
}

/// One graph serves several sessions: the same thing met twice is one entity,
/// and queries can still be narrowed to a session.
fn sessions_share_entities(graph: &mut KnowledgeGraph) {
    let (first, second) = (Uuid::new_v4(), Uuid::new_v4());

    let (rust, created) = graph
        .resolve_or_add_entity(
            Entity::new("Rust", "language", first).with_attributes(serde_json::json!({"since": 2015})),
        )
        .unwrap();
    assert!(created);
    let (alice, _) = graph.resolve_or_add_entity(Entity::new("Alice", "person", first)).unwrap();

    // Second session meets "rust" again, with one new and one conflicting attribute.
    let again = Entity::new("RUST", "language", second)
        .with_attributes(serde_json::json!({"since": 1999, "paradigm": "systems"}));
    let (resolved, created) = graph.resolve_or_add_entity(again).unwrap();
    assert!(!created);
    assert_eq!(resolved.id, rust.id, "same name and type resolve to the same entity");
    assert_eq!(resolved.session_id, first, "the creating session is kept");
    assert_eq!(resolved.seen_in, vec![second]);
    assert_eq!(resolved.attributes["since"], 2015, "existing attributes win");
    assert_eq!(resolved.attributes["paradigm"], "systems", "missing attributes are added");
    // Same name, different type: a different thing.
    let (metal, created) = graph.resolve_or_add_entity(Entity::new("Rust", "material", second)).unwrap();
    assert!(created);
    assert_ne!(metal.id, rust.id);
    // Meeting it a third time from the same session changes nothing.
    graph.resolve_or_add_entity(Entity::new("rust", "language", second)).unwrap();
    assert_eq!(graph.get_entity(rust.id).unwrap().unwrap().seen_in, vec![second]);
    assert_eq!(graph.all_entities().unwrap().len(), 3);

    // A deprecated entity is not resolved to.
    graph.update_entity(alice.id, |e| e.deprecate("left")).unwrap();
    let (new_alice, created) = graph.resolve_or_add_entity(Entity::new("Alice", "person", second)).unwrap();
    assert!(created);
    assert_ne!(new_alice.id, alice.id);

    // Session filters.
    let in_first: Vec<Uuid> = sorted_ids(graph.find_entities(EntityQuery::new().with_session(first)).unwrap());
    assert_eq!(in_first, sorted(vec![rust.id, alice.id]));
    let in_second = sorted_ids(graph.find_entities(EntityQuery::new().with_session(second)).unwrap());
    assert_eq!(in_second, sorted(vec![rust.id, metal.id, new_alice.id]));
    assert!(graph.find_entities(EntityQuery::new().with_session(Uuid::new_v4())).unwrap().is_empty());

    graph.add_relationship(Relationship::new(alice.id, "uses", rust.id, first)).unwrap();
    graph.add_relationship(Relationship::new(new_alice.id, "uses", rust.id, second)).unwrap();
    let by_session = graph.find_relationships(RelationshipQuery::new().with_session(second)).unwrap();
    assert_eq!(by_session.len(), 1);
    assert_eq!(by_session[0].1.id, new_alice.id);

    let mut scoped = EntitySearch::text("rust");
    scoped.session_id = Some(first);
    let hits = graph.search_entities(&scoped).unwrap().hits;
    assert_eq!(hits.iter().map(|h| h.entity.id).collect::<Vec<_>>(), vec![rust.id]);
}

fn sorted(mut ids: Vec<Uuid>) -> Vec<Uuid> {
    ids.sort();
    ids
}

fn sorted_ids(entities: Vec<Entity>) -> Vec<Uuid> {
    sorted(entities.into_iter().map(|e| e.id).collect())
}

/// A deterministic embedder: counts calls, and maps a text to a 3-d vector
/// from which words it contains.
pub struct KeywordEmbedder {
    id: &'static str,
    calls: std::sync::atomic::AtomicUsize,
}

impl KeywordEmbedder {
    /// An embedder that identifies itself as `id`.
    pub fn new(id: &'static str) -> Self {
        Self { id, calls: std::sync::atomic::AtomicUsize::new(0) }
    }

    /// How many texts have been embedded so far.
    pub fn embedded(&self) -> usize {
        self.calls.load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl crate::embed::Embedder for KeywordEmbedder {
    fn id(&self) -> String {
        self.id.to_string()
    }

    fn embed(&self, texts: &[String]) -> crate::error::Result<Vec<Vec<f32>>> {
        self.calls.fetch_add(texts.len(), std::sync::atomic::Ordering::SeqCst);
        Ok(texts
            .iter()
            .map(|text| {
                let text = text.to_lowercase();
                let has = |words: &[&str]| if words.iter().any(|w| text.contains(w)) { 1.0 } else { 0.05 };
                vec![has(&["rust", "python", "language"]), has(&["alice", "person"]), has(&["paris", "city"])]
            })
            .collect())
    }
}

/// `embed_pending` embeds what is new or changed, and nothing else.
fn embeddings_follow_entities(graph: &mut KnowledgeGraph) {
    let session = Uuid::new_v4();
    let embedder = KeywordEmbedder::new("test-model");
    let rust = graph.add_entity(Entity::new("Rust", "language", session)).unwrap();
    let alice = graph.add_entity(Entity::new("Alice", "person", session)).unwrap();
    let mut gone = Entity::new("Old", "thing", session);
    gone.deprecate("gone");
    graph.add_entity(gone).unwrap();

    assert_eq!(graph.entities_needing_embedding("test-model").unwrap().len(), 2);
    assert_eq!(graph.embed_pending(&embedder).unwrap(), 2, "deprecated entities are skipped");
    assert_eq!(graph.embed_pending(&embedder).unwrap(), 0, "nothing to do the second time");
    assert_eq!(embedder.embedded(), 2);

    // Bookkeeping changes do not invalidate an embedding; text changes do.
    graph.access_entity(rust.id).unwrap();
    assert_eq!(graph.embed_pending(&embedder).unwrap(), 0);
    graph.update_entity(alice.id, |e| e.attributes = serde_json::json!({"city": "Paris"})).unwrap();
    let stale: Vec<Uuid> = graph
        .entities_needing_embedding("test-model")
        .unwrap()
        .into_iter()
        .map(|e| e.id)
        .collect();
    assert_eq!(stale, vec![alice.id]);
    assert_eq!(graph.embed_pending(&embedder).unwrap(), 1);

    // The new vector is the one that is searched.
    let paris = EntitySearch { embedding: Some(vec![0.05, 0.05, 1.0]), limit: 1, ..Default::default() };
    assert_eq!(graph.search_entities(&paris).unwrap().hits[0].entity.id, alice.id);

    // A different model re-embeds everything; a vector set by hand counts as
    // not embedded by any model.
    let other = KeywordEmbedder::new("other-model");
    assert_eq!(graph.embed_pending(&other).unwrap(), 2);
    graph.set_entity_embedding(rust.id, &[1.0, 0.0, 0.0]).unwrap();
    assert_eq!(graph.embed_pending(&other).unwrap(), 1);
}

/// Relationships are found by the words of "subject predicate object".
fn relationship_search(graph: &mut KnowledgeGraph) {
    let session = Uuid::new_v4();
    let alice = graph.add_entity(Entity::new("Alice", "person", session)).unwrap();
    let bob = graph.add_entity(Entity::new("Bob", "person", session)).unwrap();
    let rust = graph.add_entity(Entity::new("Rust", "language", session)).unwrap();
    let uses = graph.add_relationship(Relationship::new(alice.id, "uses", rust.id, session)).unwrap();
    let manages = graph.add_relationship(Relationship::new(bob.id, "manages", alice.id, session)).unwrap();

    let ids = |graph: &KnowledgeGraph, text: &str| -> Vec<Uuid> {
        graph.search_relationships(text, 0).unwrap().hits.into_iter().map(|(t, _)| t.0.id).collect()
    };
    assert_eq!(ids(graph, "manages"), vec![manages.id]);
    assert_eq!(ids(graph, "rust"), vec![uses.id]);
    // Both mention Alice; the one matching more of the query comes first.
    assert_eq!(ids(graph, "alice uses"), vec![uses.id, manages.id]);
    assert!(ids(graph, "zzz-nothing").is_empty());
    assert!(graph.search_relationships("  ", 0).is_err());
    assert_eq!(graph.search_relationships("alice", 1).unwrap().hits.len(), 1);

    let hit = &graph.search_relationships("manages", 0).unwrap().hits[0];
    assert_eq!((hit.0 .1.id, hit.0 .2.id), (bob.id, alice.id), "hits carry both endpoints");
    assert!(hit.1 > 0.0);

    graph.delete_relationship(manages.id).unwrap();
    assert!(ids(graph, "manages").is_empty());
}
