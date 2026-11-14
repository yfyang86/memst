"""The store's knowledge graph from Python."""

import json
import uuid

import pytest

import memst


@pytest.fixture
def store(tmp_path):
    path = tmp_path / "store"
    memst.SessionStore(str(path))
    return str(path)


def sid():
    return str(uuid.uuid4())


def test_entities_are_shared_across_sessions(store):
    first, second = sid(), sid()
    graph = memst.Graph.open(store)
    assert graph.backend == "helix"
    assert graph.is_shared
    assert graph.owner == "default"

    rust = graph.add_entity(first, "Rust", "language", json.dumps({"since": 2015}))
    assert rust["created"] is True
    assert rust["attributes"] == {"since": 2015}

    again = graph.add_entity(second, "rust", "language")
    assert again["created"] is False
    assert again["id"] == rust["id"]
    assert again["seen_in"] == [second]

    separate = graph.add_entity(second, "Rust", "language", resolve=False)
    assert separate["id"] != rust["id"]

    alice = graph.add_entity(first, "Alice", "person")
    edge = graph.add_relationship(first, alice["id"], "uses", rust["id"], confidence=0.8)
    assert (edge["subject"], edge["predicate"], edge["object"]) == ("Alice", "uses", "Rust")
    assert edge["confidence"] == pytest.approx(0.8)

    assert {e["name"] for e in graph.entities(session_id=first)} == {"Rust", "Alice"}
    assert len(graph.entities()) == 3
    assert len(graph.entities(entity_type="person")) == 1
    assert len(graph.entities(limit=1)) == 1
    assert len(graph.relationships()) == 1
    assert len(graph.relationships(session_id=second)) == 0
    assert len(graph.relationships(entity_id=rust["id"])) == 1

    near = graph.neighbors(alice["id"], hops=1, direction="out")
    assert [(n["name"], n["hops"]) for n in near] == [("Rust", 1)]

    stats = graph.stats()
    assert (stats["entity_count"], stats["relationship_count"]) == (3, 1)
    assert stats["predicates"] == {"uses": 1}


def test_search_and_manual_ingestion(store):
    session = sid()
    graph = memst.Graph.open(store)
    extraction = {
        "entities": [
            {"name": "Alice", "entity_type": "person", "attributes": {"role": "engineer"}, "confidence": 0.9},
            {"name": "Lyon", "entity_type": "city", "attributes": {}, "confidence": 0.9},
        ],
        "relationships": [
            {"subject": "Alice", "predicate": "lives_in", "object": "Lyon", "confidence": 0.8},
            {"subject": "Alice", "predicate": "knows", "object": "Nobody", "confidence": 0.8},
        ],
        "events": [],
        "confidence": 0.9,
    }
    message = sid()
    report = graph.ingest_extraction(session, json.dumps(extraction), message_id=message)
    assert report["entities_created"] == 2
    assert report["relationships_created"] == 1
    assert report["relationships_skipped"] == 1
    assert report["entities_embedded"] == 0

    again = graph.ingest_extraction(session, json.dumps(extraction))
    assert (again["entities_created"], again["entities_reused"]) == (0, 2)
    assert again["relationships_reinforced"] == 1

    alice = graph.entities(entity_type="person")[0]
    assert alice["source_message_id"] == message

    result = graph.search("engineer")
    assert result["method"] in ("native", "fallback")
    assert [hit["name"] for hit in result["hits"]] == ["Alice"]
    assert result["hits"][0]["score"] > 0

    assert graph.search("alice", session_id=sid())["hits"] == []
    assert graph.search("zzz-nothing")["hits"] == []


def test_owners_are_isolated(store):
    session = sid()
    alice = memst.Graph.open(store, owner="alice")
    secret = alice.add_entity(session, "Project Falcon", "project")
    del alice

    bob = memst.Graph.open(store, owner="bob")
    assert bob.entities() == []
    assert bob.search("falcon")["hits"] == []
    mine = bob.add_entity(session, "Bob", "person")
    with pytest.raises(RuntimeError):
        bob.add_relationship(session, mine["id"], "spies_on", secret["id"])


def test_one_writer_many_readers(store):
    session = sid()
    writer = memst.Graph.open(store)
    writer.add_entity(session, "Rust", "language")

    with pytest.raises(RuntimeError, match="open for writing by another process"):
        memst.Graph.open(store)

    reader = memst.Graph.open(store, read_only=True)
    assert [e["name"] for e in reader.entities()] == ["Rust"]
    with pytest.raises(RuntimeError, match="read-only"):
        reader.add_entity(session, "nope", "x")


def test_bad_arguments_are_value_errors(store):
    graph = memst.Graph.open(store)
    with pytest.raises(ValueError):
        graph.add_entity("not-a-uuid", "x", "y")
    with pytest.raises(ValueError):
        graph.add_entity(sid(), "x", "y", attributes="{not json")
    with pytest.raises(ValueError):
        graph.neighbors(str(uuid.uuid4()), direction="sideways")
    with pytest.raises(ValueError):
        graph.ingest_extraction(sid(), "{}")
    with pytest.raises(ValueError):
        memst.Graph.open(store + "-other", backend="oracle")
