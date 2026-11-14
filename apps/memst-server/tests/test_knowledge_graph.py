"""The knowledge-graph endpoints are served from the engine's shared graph.

Needs the server's dependencies and the `memst` extension installed. A small
local HTTP server stands in for the LLM.
"""

import json
import os
import sys
import threading
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

memst = pytest.importorskip("memst")
pytest.importorskip("fastapi")
if not hasattr(memst, "Graph"):
    pytest.skip("memst was built without the knowledge graph", allow_module_level=True)

EXTRACTION = {
    "entities": [
        {"name": "Alice", "type": "person", "attributes": {"role": "engineer"}, "confidence": 0.9},
        {"name": "Rust", "type": "language", "attributes": {}, "confidence": 0.9},
    ],
    "relationships": [{"subject": "Alice", "predicate": "writes", "object": "Rust", "confidence": 0.9}],
    "events": [],
}


class FakeLlm(BaseHTTPRequestHandler):
    def do_POST(self):
        self.rfile.read(int(self.headers["Content-Length"]))
        body = json.dumps(
            {
                "id": "x",
                "object": "chat.completion",
                "created": 0,
                "model": "fake",
                "choices": [
                    {
                        "index": 0,
                        "message": {"role": "assistant", "content": json.dumps(EXTRACTION)},
                        "finish_reason": "stop",
                    }
                ],
                "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2},
            }
        ).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *args):
        pass


@pytest.fixture
def llm_url():
    server = HTTPServer(("127.0.0.1", 0), FakeLlm)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    yield f"http://127.0.0.1:{server.server_port}/v1"
    server.shutdown()


@pytest.fixture
def routes(tmp_path, monkeypatch):
    store = tmp_path / "store"
    (tmp_path / "config.toml").write_text(f'[server]\nstore_path = "{store}"\n')
    monkeypatch.chdir(tmp_path)
    monkeypatch.setenv("MEMST_CONFIG_PATH", str(tmp_path / "config.toml"))
    monkeypatch.setenv("MEMST_SERVER_EMBEDDER", "none")
    monkeypatch.setenv("NO_PROXY", "127.0.0.1")
    monkeypatch.setenv("no_proxy", "127.0.0.1")
    for name in [n for n in sys.modules if n.startswith("memst_server")]:
        del sys.modules[name]
    from memst_server.api import routes as module
    from memst_server import graph_service

    yield module
    graph_service.close_all()


def new_session(routes):
    session = routes._memst_client.create_session("kg test", "model")
    assert session, "the engine must be available to the server"
    return session["id"]


def test_graph_endpoint_serves_the_engine_graph(routes, llm_url):
    session_id = new_session(routes)
    routes._memst_client.add_message(session_id, "user", "I'm Alice, an engineer. I write Rust.")
    routes.get_db().update_settings(llm={"api_url": llm_url, "model": "fake", "api_key": ""})

    result = routes.parse_knowledge_graph(session_id)

    assert "warning" not in result, result.get("warning")
    assert result["source"] == "graph"
    assert result["ingest"]["entities_created"] == 2
    assert result["ingest"]["relationships_created"] == 1
    labels = {node["label"]: node for node in result["nodes"]}
    assert set(labels) == {"Alice", "Rust"}
    assert labels["Alice"]["type"] == "person"
    assert labels["Alice"]["attributes"] == {"role": "engineer"}, "bookkeeping attributes are hidden"
    assert labels["Alice"]["connections"] == 1
    assert result["edges"] == [
        {"from": labels["Alice"]["id"], "to": labels["Rust"]["id"], "label": "writes", "confidence": pytest.approx(0.9)}
    ]

    # Reading it back does not need the LLM, and a second parse adds nothing.
    assert routes.get_knowledge_graph(session_id)["source"] == "graph"
    again = routes.parse_knowledge_graph(session_id)
    assert again["ingest"]["entities_created"] == 0
    assert len(again["nodes"]) == 2

    # The CLI and other tools see the same graph: it is the store's graph.
    store_path = str(routes._memst_client.store_path)
    from memst_server import graph_service

    graph_service.close_all()
    graph = memst.Graph.open(store_path)
    assert {e["name"] for e in graph.entities(session_id=session_id)} == {"Alice", "Rust"}


def test_falls_back_to_the_regex_graph_without_an_llm(routes):
    session_id = new_session(routes)
    routes._memst_client.add_message(session_id, "user", "hello")
    routes.get_db().update_settings(llm={"api_url": "", "model": ""})

    result = routes.parse_knowledge_graph(session_id)

    assert result["source"] == "regex"
    assert "no LLM is configured" in result["warning"]
    assert "ingest" not in result
