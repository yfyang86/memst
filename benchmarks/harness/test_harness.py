"""Offline tests for the harness: no model, no network beyond localhost."""

import json
import threading
from http.server import BaseHTTPRequestHandler, HTTPServer

import pytest

from benchmarks.harness import scoring
from benchmarks.harness.data import Conversation, Question, Turn, load_locomo, load_longmemeval
from benchmarks.harness.llm import ChatClient
from benchmarks.harness.pipeline import Memory, load_pipelines, retrieve, uses_embeddings, uses_graph, validate
from benchmarks.harness.retrieval import BM25, cosine_rank, rrf


def test_token_f1_and_abstention():
    assert scoring.token_f1("7 May 2023", "May 7, 2023") == 1.0
    assert scoring.token_f1("the Paris office", "Paris") == pytest.approx(2 / 3)
    assert scoring.token_f1("Berlin", "Paris") == 0.0
    assert scoring.is_abstention("No information available.")
    assert scoring.is_abstention("The conversation does not mention that.")
    assert not scoring.is_abstention("She moved to Lyon.")


def test_bootstrap_interval_brackets_the_mean():
    mean, low, high = scoring.mean_with_interval([0.0, 1.0] * 50)
    assert mean == 0.5 and low < 0.5 < high and high - low < 0.3
    assert scoring.mean_with_interval([]) == (0.0, 0.0, 0.0)
    assert scoring.percentile([1, 2, 3, 4], 0.5) == 3


def test_bm25_ranks_the_matching_document_first():
    index = BM25(["alice lives in lyon", "bob likes rust", "alice and bob met in paris"])
    ranked = [i for i, _ in index.rank("where does alice live? lyon")]
    assert ranked[0] == 0 and 1 not in ranked
    assert index.rank("zebra") == []


def test_rank_fusion_and_cosine():
    assert rrf([[1, 2, 3], [2, 1, 4]])[:2] in ([1, 2], [2, 1])
    assert rrf([[5], [5], [6]])[0] == 5
    assert [i for i, _ in cosine_rank([1, 0], [[0, 1], [1, 0.1]])] == [1, 0]


def conversation():
    turns = [
        Turn("D1:1", "Alice", "I moved to Lyon last month.", "1 May 2023"),
        Turn("D1:2", "Bob", "Nice. I still write Rust every day.", "1 May 2023"),
    ]
    return Conversation("c", turns, [Question("c:0", "Where does Alice live?", "Lyon", "single-hop", ["D1:1"])])


class WordEmbedder:
    """One dimension per known word: enough to rank by meaning in a test."""

    words = ["lyon", "rust", "live", "moved", "write"]

    def embed(self, texts):
        return [[float(w in t.lower()) + 0.01 for w in self.words] for t in texts]


def memory(tmp_path):
    return Memory(conversation(), WordEmbedder(), tmp_path, llm={}, batch_chars=6000)


def test_raw_pipelines_return_context_turns_and_a_trace(tmp_path):
    pipelines = load_pipelines()
    question = conversation().questions[0]

    text, shown, trace = retrieve(pipelines["full-context"], memory(tmp_path), question)
    assert [t.id for t in shown] == ["D1:1", "D1:2"] and "[1 May 2023] Alice: I moved to Lyon" in text
    assert trace[0]["stage"] == "all_turns" and trace[0]["reached_evidence"] is True

    for method in ("bm25", "dense", "hybrid"):
        one = {"name": "one", "stages": [{"stage": "turns", "method": method, "k": 1}]}
        validate(one)
        question.text = "Where does Alice live? Lyon"
        _, shown, trace = retrieve(one, memory(tmp_path), question)
        assert [t.id for t in shown] == ["D1:1"], method
        assert trace[0]["params"] == {"method": method, "k": 1} and trace[0]["turns_added"] == 1
        assert trace[0]["top"] == ["D1:1"] and trace[0]["ms"] >= 0


def test_pipeline_definitions_are_checked(tmp_path):
    pipelines = load_pipelines()
    assert "default" in pipelines and pipelines["default"]["stages"][0]["stage"] == "facts"
    assert uses_graph(pipelines["default"]) and not uses_graph(pipelines["raw-bm25"])
    assert uses_embeddings(pipelines["raw-dense"]) and not uses_embeddings(pipelines["raw-bm25"])
    for bad in (
        {"name": "x", "stages": []},
        {"name": "x", "stages": [{"stage": "rerank"}]},
        {"name": "x", "stages": [{"stage": "turns", "method": "magic"}]},
        {"name": "x", "stages": [{"stage": "turns", "top": 3}]},
        {"name": "x", "stages": [{"stage": "all_turns"}], "context": {"budget": 1}},
    ):
        with pytest.raises(ValueError):
            validate(bad)
    # A file can add pipelines and replace built-in ones.
    extra = tmp_path / "extra.json"
    extra.write_text(json.dumps({"name": "raw-bm25", "stages": [{"stage": "turns", "method": "bm25", "k": 3}]}))
    assert load_pipelines(extra)["raw-bm25"]["stages"][0]["k"] == 3


def test_graph_pipeline_searches_facts_and_follows_them_to_their_source(tmp_path):
    memst = pytest.importorskip("memst")
    # A store as an earlier run would have left it: Alice's fact was stated
    # in the first turn.
    store = tmp_path / "store-c"
    memst.SessionStore(str(store))
    graph = memst.Graph.open(str(store))
    message = "00000000-0000-4000-8000-000000000001"
    extraction = {
        "entities": [
            {"name": "Alice", "entity_type": "person", "attributes": {}, "confidence": 0.9},
            {"name": "Lyon", "entity_type": "place", "attributes": {}, "confidence": 0.9},
        ],
        "relationships": [{"subject": "Alice", "predicate": "lives_in", "object": "Lyon", "confidence": 0.9}],
        "events": [],
        "confidence": 0.9,
    }
    graph.ingest_extraction("00000000-0000-4000-8000-000000000002", json.dumps(extraction), message_id=message)
    del graph
    (store / "message-turns.json").write_text(json.dumps({message: 0}))
    (store / "ingest-report.json").write_text(json.dumps({"llm_calls": 3, "llm_input_chars": 400}))

    built = memory(tmp_path)
    question = conversation().questions[0]
    question.text = "Where does Alice live? Lyon"
    for name in ("graph-facts", "graph-entities", "default"):
        text, shown, trace = retrieve(load_pipelines()[name], built, question)
        assert "Alice lives in Lyon" in text, name
        assert "D1:1" in [t.id for t in shown], name
        assert trace[0]["facts_added"] == 1 and trace[0]["reached_evidence"] is True, name
        assert trace[1]["stage"] == "fact_sources" and trace[1]["turns_added"] == 1, name
    assert built.graph_cost.llm_calls == 3
    # In the default pipeline the raw-turn stage adds only what is new.
    assert trace[2]["stage"] == "turns" and trace[2]["turns_added"] == 1


def test_longmemeval_loader_orders_sessions_and_keeps_evidence(tmp_path):
    sample = {
        "question_id": "q1",
        "question_type": "knowledge-update",
        "question": "Where do I live?",
        "answer": "Lyon",
        "question_date": "2023/06/01 (Thu) 10:00",
        "haystack_dates": ["2023/05/20 (Sat) 02:21", "2023/05/01 (Mon) 09:00"],
        "haystack_session_ids": ["s2", "s1"],
        "haystack_sessions": [
            [{"role": "user", "content": "I moved to Lyon.", "has_answer": True}],
            [{"role": "user", "content": "I live in Paris."}, {"role": "assistant", "content": "Noted."}],
        ],
        "answer_session_ids": ["s2"],
    }
    path = tmp_path / "lme.json"
    path.write_text(json.dumps([sample, {**sample, "question_id": "q2_abs"}]))
    first, second = load_longmemeval(path)
    assert [t.id for t in first.turns] == ["s1:0", "s1:1", "s2:0"], "sessions are put in time order"
    assert first.turns[2].timestamp().startswith("2023-05-20T02:21")
    question = first.questions[0]
    assert (question.evidence, question.date, question.category) == (["s2:0"], "2023/06/01 (Thu) 10:00", "knowledge-update")
    assert question.evidence_among(first.turns[2:]) and not question.evidence_among(first.turns[:2])
    assert second.questions[0].abstain and second.questions[0].category == "abstention"


def test_locomo_loader_marks_unanswerable_questions(tmp_path):
    sample = {
        "sample_id": "s1",
        "conversation": {
            "speaker_a": "A",
            "speaker_b": "B",
            "session_2_date_time": "2 May",
            "session_2": [{"dia_id": "D2:1", "speaker": "B", "text": "later"}],
            "session_1_date_time": "1 May",
            "session_1": [{"dia_id": "D1:1", "speaker": "A", "text": "hi", "blip_caption": "a dog"}],
        },
        "qa": [
            {"question": "q1", "answer": 7, "evidence": ["D1:1"], "category": 2},
            {"question": "q2", "adversarial_answer": "x", "evidence": [], "category": 5},
        ],
    }
    path = tmp_path / "locomo.json"
    path.write_text(json.dumps([sample]))
    loaded = load_locomo(path)[0]
    assert [t.id for t in loaded.turns] == ["D1:1", "D2:1"], "sessions are ordered by number"
    assert "a dog" in loaded.turns[0].text
    assert (loaded.questions[0].answer, loaded.questions[0].category) == ("7", "temporal")
    assert loaded.questions[1].abstain and loaded.questions[1].category == "adversarial"


def test_chat_client_caches_and_records_usage(tmp_path):
    calls = []

    class Handler(BaseHTTPRequestHandler):
        def do_POST(self):
            calls.append(json.loads(self.rfile.read(int(self.headers["Content-Length"]))))
            body = json.dumps(
                {"choices": [{"message": {"content": " Lyon "}}], "usage": {"prompt_tokens": 12, "completion_tokens": 1}}
            ).encode()
            self.send_response(200)
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, *args):
            pass

    server = HTTPServer(("127.0.0.1", 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    url = f"http://127.0.0.1:{server.server_port}"
    cache = tmp_path / "cache.jsonl"
    try:
        first = ChatClient(url, "m", cache).complete([{"role": "user", "content": "q"}])
        assert (first.text, first.prompt_tokens, first.cached) == ("Lyon", 12, False)
        # A new client reads the cache from disk: no second request.
        second = ChatClient(url, "m", cache).complete([{"role": "user", "content": "q"}])
        assert second.cached and second.text == "Lyon" and len(calls) == 1
        ChatClient(url, "m", cache).complete([{"role": "user", "content": "other"}])
        assert len(calls) == 2
    finally:
        server.shutdown()
