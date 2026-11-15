"""Offline tests for the harness: no model, no network beyond localhost."""

import json
import threading
from http.server import BaseHTTPRequestHandler, HTTPServer

import pytest

from benchmarks.harness import scoring
from benchmarks.harness.data import Conversation, Question, Turn, load_locomo
from benchmarks.harness.llm import ChatClient
from benchmarks.harness.systems import BM25, FullContext, RawBM25, cosine_rank, rrf


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


def test_systems_return_context_and_the_turns_it_came_from():
    full = FullContext()
    full.prepare(conversation())
    text, ids = full.context("anything")
    assert ids == ["D1:1", "D1:2"] and "[1 May 2023] Alice: I moved to Lyon" in text

    bm25 = RawBM25(k=1)
    bm25.prepare(conversation())
    assert bm25.context("Where does Alice live? Lyon")[1] == ["D1:1"]


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
