"""Offline tests for the harness: no model, no network beyond localhost."""

import json
import threading
from http.server import BaseHTTPRequestHandler, HTTPServer

import pytest

from benchmarks.harness import scoring
from benchmarks.harness.data import Conversation, Question, Turn, load_locomo, load_longmemeval
from benchmarks.harness.llm import ChatClient
from benchmarks.harness.pipeline import Memory, load_pipelines, retrieve, uses_embeddings, uses_graph, validate
from benchmarks.harness.retrieval import BM25, CachedEmbedder, cosine_rank, rrf


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


def test_cached_embedder_retries_and_reuses_disk_cache(tmp_path, monkeypatch):
    calls = []

    class FlakyEmbedder:
        id = "flaky"

        def embed(self, texts):
            calls.append(texts)
            if len(calls) == 1:
                raise RuntimeError("connection closed before message completed")
            return [[float(len(text))] for text in texts]

    monkeypatch.setattr("benchmarks.harness.retrieval.time.sleep", lambda _: None)
    cache = tmp_path / "embeddings.jsonl"
    embedder = CachedEmbedder(FlakyEmbedder(), cache)
    assert [list(v) for v in embedder.embed(["a", "bb"])] == [[1.0], [2.0]]

    resumed = CachedEmbedder(FlakyEmbedder(), cache)
    assert [list(v) for v in resumed.embed(["a", "bb"])] == [[1.0], [2.0]]
    assert len(calls) == 2


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
        {"name": "x", "stages": [{"stage": "summarise"}]},
        {"name": "x", "stages": [{"stage": "all_turns"}], "reader": {"mode": "guess"}},
        {"name": "x", "stages": [{"stage": "all_turns"}], "router": {}},
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


def test_chat_client_retries_transient_redirects(tmp_path, monkeypatch):
    calls = []

    class Handler(BaseHTTPRequestHandler):
        def do_POST(self):
            calls.append(json.loads(self.rfile.read(int(self.headers["Content-Length"]))))
            if len(calls) <= 5:
                self.send_response(307)
                self.send_header("Content-Length", "0")
                self.end_headers()
                return
            body = json.dumps({"choices": [{"message": {"content": "OK"}}]}).encode()
            self.send_response(200)
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, *args):
            pass

    monkeypatch.setattr("benchmarks.harness.llm.time.sleep", lambda _: None)
    server = HTTPServer(("127.0.0.1", 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    url = f"http://127.0.0.1:{server.server_port}"
    try:
        reply = ChatClient(url, "m", tmp_path / "cache.jsonl").complete([{"role": "user", "content": "q"}])
        assert reply.text == "OK"
        assert len(calls) == 6
    finally:
        server.shutdown()


def test_chat_client_follows_same_origin_post_redirect(tmp_path, monkeypatch):
    calls = []

    class Handler(BaseHTTPRequestHandler):
        def do_POST(self):
            payload = self.rfile.read(int(self.headers["Content-Length"]))
            calls.append((self.path, payload, self.headers.get("Authorization")))
            if self.path == "/chat/completions":
                self.send_response(307)
                self.send_header("Location", "/v1/chat/completions")
                self.send_header("Content-Length", "0")
                self.end_headers()
                return
            body = json.dumps({"choices": [{"message": {"content": "OK"}}]}).encode()
            self.send_response(200)
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, *args):
            pass

    monkeypatch.setattr("benchmarks.harness.llm.time.sleep", lambda _: None)
    server = HTTPServer(("127.0.0.1", 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    url = f"http://127.0.0.1:{server.server_port}"
    try:
        reply = ChatClient(url, "m", tmp_path / "cache.jsonl").complete([{"role": "user", "content": "q"}])
        assert reply.text == "OK"
        assert [call[0] for call in calls] == ["/chat/completions", "/v1/chat/completions"]
        assert calls[0][1:] == calls[1][1:]
    finally:
        server.shutdown()


def long_conversation():
    turns = [Turn(f"D1:{i}", "Alice" if i % 2 else "Bob", text, "1 May 2023", "session_1") for i, text in enumerate(
        ["We adopted a dog.", "Its name is Rex.", "I moved to Lyon last month.", "The flat is near the river.", "Rust is fun."], 1)]
    turns.append(Turn("D2:1", "Bob", "Lyon again next week.", "9 May 2023", "session_2"))
    return Conversation("c", turns, [Question("c:0", "Where does Alice live? Lyon", "Lyon", "single-hop", ["D1:3"])])


class FakeReranker:
    def __init__(self, reply):
        self.reply, self.prompts = reply, []

    def complete(self, messages, max_tokens=200):
        from benchmarks.harness.llm import Completion

        self.prompts.append(messages[0]["content"])
        return Completion(self.reply, 321, 5, 0.0, False)


def test_neighbors_stay_inside_the_session(tmp_path):
    conv = long_conversation()
    memory = Memory(conv, WordEmbedder(), tmp_path, llm={}, batch_chars=6000)
    pipeline = {"name": "n", "stages": [{"stage": "turns", "method": "bm25", "k": 2}, {"stage": "neighbors", "window": 1}]}
    validate(pipeline)
    _, shown, trace = retrieve(pipeline, memory, conv.questions[0])
    # BM25 finds the two Lyon turns (D1:3 and D2:1); neighbours come only from D1:3's session.
    assert [t.id for t in shown] == ["D1:2", "D1:3", "D1:4", "D2:1"]
    assert trace[1]["stage"] == "neighbors" and trace[1]["turns_added"] == 2


def test_rerank_keeps_the_models_choice_first_and_counts_the_call(tmp_path):
    conv = long_conversation()
    base = [{"stage": "all_turns"}]
    for reply, fill, expected in (
        ("[3, 6]", False, ["D1:3", "D2:1"]),
        ("[3, 6]", True, ["D1:1", "D1:3", "D2:1"]),  # filled up to keep=3 in the earlier order
        ("no idea", False, ["D1:1", "D1:2", "D1:3"]),  # unreadable reply: earlier order
    ):
        reranker = FakeReranker(reply)
        memory = Memory(conv, WordEmbedder(), tmp_path, llm={}, batch_chars=6000, reranker=reranker)
        pipeline = {"name": "r", "stages": base + [{"stage": "rerank", "keep": 3, "fill": fill}]}
        validate(pipeline)
        _, shown, trace = retrieve(pipeline, memory, conv.questions[0])
        assert [t.id for t in shown] == expected, reply
        assert (trace[1]["llm_calls"], trace[1]["prompt_tokens"]) == (1, 321)
        assert "[3] [1 May 2023] Alice: I moved to Lyon" in reranker.prompts[0]
    with pytest.raises(ValueError):
        retrieve(pipeline, Memory(conv, WordEmbedder(), tmp_path, llm={}, batch_chars=6000), conv.questions[0])


def test_reader_prompts_and_reasoned_answers():
    from benchmarks.harness.run import final_answer, reader_prompt

    question = Question("q", "Where do I live?", "Lyon", "knowledge-update", date="2023/06/01 (Thu) 10:00")
    direct = reader_prompt("longmemeval", "direct", "CONTEXT", question)
    assert direct.endswith("Question: Where do I live?\nAnswer:") and "Today's date: 2023/06/01" in direct and "suggestion" in direct
    assert "suggestion" not in reader_prompt("locomo", "direct", "CONTEXT", question)
    reasoned = reader_prompt("locomo", "reason", "CONTEXT", question)
    assert reasoned.endswith("Reasoning:") and "Question: Where do I live?" in reasoned
    assert final_answer("Paris in May, Lyon in June.\nAnswer: Lyon", "reason") == "Lyon"
    assert final_answer("Answer: Lyon", "direct") == "Answer: Lyon"
    assert final_answer("so it is Lyon", "reason") == "so it is Lyon"


def test_judge_rules_by_question_type():
    class Judge:
        def __init__(self, reply):
            self.reply, self.prompt = reply, None

        def complete(self, messages, max_tokens=10):
            from benchmarks.harness.llm import Completion

            self.prompt = messages[0]["content"]
            return Completion(self.reply, 1, 1, 0.0, False)

    judge = Judge("The dates differ by one day, which is allowed.\nCORRECT")
    assert scoring.judge(judge, "q", "19 days", "18 days", category="temporal-reasoning")
    assert "off by one" in judge.prompt
    plain = Judge("INCORRECT")
    assert not scoring.judge(plain, "q", "a", "b") and "off by one" not in plain.prompt
    assert not scoring.judge(Judge(""), "q", "a", "b")


def test_report_has_answerable_subset_and_paired_differences():
    from benchmarks.harness.report import render, summarise

    def row(pipeline, qid, category, f1, judged):
        return {"pipeline": pipeline, "question_id": qid, "category": category, "f1": f1, "judged_correct": judged,
                "evidence_in_context": True, "prompt_tokens": 100, "retrieval_seconds": 0.01}

    rows = []
    for i in range(20):
        rows.append(row("raw-dense", f"q{i}", "single-hop", 0.5, 1.0))
        rows.append(row("mine", f"q{i}", "single-hop", 1.0, 0.0))
    rows += [row("raw-dense", "a", "adversarial", 1.0, 1.0), row("mine", "a", "adversarial", 0.0, 0.0)]
    rows[1]["stage_llm_calls"], rows[1]["stage_prompt_tokens"] = 1, 2100
    summary = summarise(rows, [], ["raw-dense", "mine"], {}, {"dataset": "LoCoMo", "reader_and_judge_model": "m"}, {}, 1000)
    mine = summary["pipelines"]["mine"]
    assert (mine["answerable"]["n"], mine["answerable"]["f1"], mine["n"]) == (20, 1.0, 21)
    assert mine["prompt_tokens_mean"] == 200 and mine["read_llm_calls_mean"] == 1.05
    assert summary["paired_with"] == "raw-dense"
    difference = summary["paired"]["mine"]["judged"]
    assert difference[0] == -1.0 and difference[2] < 0, "a difference whose interval excludes zero"
    assert summary["oracle_router"]["judged"] == 1.0
    text = render(summary)
    assert "Answerable questions only (20; without adversarial)" in text and "Difference from `raw-dense`" in text
    assert "the same model as the reader" in text


def test_route_analysis_finds_a_router_only_where_question_kind_predicts_the_winner():
    from benchmarks.harness.route_analysis import analyse, kind_of

    assert (kind_of("When did she move?"), kind_of("How many dogs?"), kind_of("Tell me")) == ("when", "how many", "other")
    rows = []
    for i in range(40):
        when = i % 2 == 0
        question = f"When was event {i}?" if when else f"Who did thing {i}?"
        # Pipeline a is right on "when" questions only, b on the others only.
        rows.append({"pipeline": "a", "question_id": f"q{i}", "question": question, "judged_correct": float(when)})
        rows.append({"pipeline": "b", "question_id": f"q{i}", "question": question, "judged_correct": float(not when)})
        rows.append({"pipeline": "full-context", "question_id": f"q{i}", "question": question, "judged_correct": 1.0})
    result = analyse(rows)
    assert result["pipelines"] == ["a", "b"] and result["best_single_score"] == 0.5
    assert result["routed_score"] == 1.0 and result["share_of_oracle_gain"] == 1.0


def test_fact_timeline_orders_by_date_and_marks_the_latest_value():
    from benchmarks.harness.pipeline import fact_timeline

    def fact(i, obj, when, valid_to=None, predicate="works_at"):
        return {"id": f"f{i}", "subject_id": "alice", "subject": "Alice", "predicate": predicate,
                "object_id": obj, "object": obj, "valid_from": when, "valid_to": valid_to}

    lines = fact_timeline(None, [
        fact(1, "Globex", "2023-06-01T00:00:00+00:00"),
        fact(2, "Acme", "2023-01-10T00:00:00+00:00"),
        fact(3, "tea", None, predicate="likes"),
        fact(4, "Lyon", "2023-02-01T00:00:00+00:00", valid_to="2023-05-01T00:00:00+00:00", predicate="lives_in"),
    ])
    assert lines == [
        "10 January 2023: Alice works at Acme (earlier; see the later entry)",
        "1 February 2023: Alice lives in Lyon (no longer true from 1 May 2023)",
        "1 June 2023: Alice works at Globex (latest)",
        "undated: Alice likes tea",
    ]
    # When it happened and when it was said are told apart, and a date is
    # written no more exactly than it is known.
    said = {**fact(5, "park", None, predicate="visited"), "stated_at": "2023-09-09T10:00:00+00:00"}
    in_2022 = {**fact(6, "pottery", "2022-01-01T00:00:00+00:00", predicate="started"), "valid_precision": "year", "stated_at": "2023-09-09T10:00:00+00:00"}
    assert fact_timeline(None, [said, in_2022]) == ["2022: Alice started pottery", "said on 9 September 2023: Alice visited park"]
    assert Memory.fact_text(said) == "Alice visited park (said on 9 September 2023; date of the event not given)"
    assert Memory.fact_text(in_2022) == "Alice started pottery (in 2022)"
    assert Memory.fact_text({**in_2022, "valid_precision": "day"}) == "Alice started pottery (on 1 January 2022)"
    assert Memory.fact_text(fact(7, "Acme", "2023-01-10T00:00:00+00:00")) == "Alice works at Acme (since 10 January 2023)"
    validate({"name": "t", "stages": [{"stage": "all_turns"}], "context": {"facts_as": "timeline", "max_facts": 0}})
    with pytest.raises(ValueError):
        validate({"name": "t", "stages": [{"stage": "all_turns"}], "context": {"facts_as": "table"}})


def test_longmemeval_can_be_limited_to_types_or_ids(tmp_path):
    def sample(qid, kind):
        return {"question_id": qid, "question_type": kind, "question": "q", "answer": "a", "haystack_dates": ["2023/05/01 (Mon) 09:00"],
                "haystack_session_ids": ["s1"], "haystack_sessions": [[{"role": "user", "content": "hi"}]], "answer_session_ids": ["s1"]}

    path = tmp_path / "lme.json"
    path.write_text(json.dumps([sample("a", "knowledge-update"), sample("b", "temporal-reasoning"), sample("c", "multi-session"), sample("d", "knowledge-update")]))
    assert sorted(c.id for c in load_longmemeval(path, types=["knowledge-update"])) == ["a", "d"]
    assert [c.id for c in load_longmemeval(path, per_type=1, ids=["b", "c"])] == ["b", "c"]
    assert len(load_longmemeval(path, per_type=1, types=["knowledge-update", "temporal-reasoning"])) == 2
