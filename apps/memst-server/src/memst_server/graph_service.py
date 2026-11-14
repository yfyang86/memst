"""The store's knowledge graph, for the server.

The engine keeps one graph per store (embedded HelixDB, shared by all
sessions). This module is the server's only door to it:

- ``session_graph`` returns a session's entities and relationships in the
  ``{"nodes": [...], "edges": [...]}`` shape the UI draws;
- ``ingest_session`` runs LLM extraction over a session's messages and writes
  the result into the graph.

Only one process can write a store's graph, so the server keeps a single
handle open per (store, owner) for its lifetime. A CLI run against the same
store while the server is up can read but not write.
"""

import os
import threading
from typing import Any, Dict, List, Optional

try:
    import memst

    GRAPH_AVAILABLE = hasattr(memst, "Graph")
except ImportError:  # pragma: no cover - the engine is optional for the server
    memst = None
    GRAPH_AVAILABLE = False

# Owner the server reads and writes as. The CLI's default owner is the same,
# so what one writes the other sees.
DEFAULT_OWNER = "default"

_handles: Dict[tuple, Any] = {}
_handles_lock = threading.Lock()


def _graph(store_path: str, owner: str = DEFAULT_OWNER):
    """The server's long-lived handle on a store's graph, or None."""
    if not GRAPH_AVAILABLE:
        return None
    key = (str(store_path), owner)
    with _handles_lock:
        if key not in _handles:
            try:
                _handles[key] = memst.Graph.open(str(store_path), owner=owner)
            except RuntimeError as error:
                # Another process is writing: read what it has written.
                print(f"Knowledge graph opened read-only: {error}")
                try:
                    _handles[key] = memst.Graph.open(str(store_path), owner=owner, read_only=True)
                except Exception as read_error:
                    print(f"Knowledge graph unavailable: {read_error}")
                    return None
            except Exception as error:
                print(f"Knowledge graph unavailable: {error}")
                return None
        return _handles[key]


def close_all() -> None:
    """Release every graph handle (and with it the writer lock)."""
    with _handles_lock:
        _handles.clear()


def session_graph(store_path: str, session_id: str, owner: str = DEFAULT_OWNER) -> Optional[Dict[str, Any]]:
    """A session's part of the graph, in the shape the UI draws.

    Returns None when the graph is unavailable or has nothing for the
    session, so the caller can fall back to its previous behaviour.
    """
    graph = _graph(store_path, owner)
    if graph is None:
        return None
    try:
        entities = graph.entities(session_id=session_id)
        if not entities:
            return None
        ids = {entity["id"] for entity in entities}
        nodes = [
            {
                "id": entity["id"],
                "label": entity["name"],
                "type": entity["entity_type"],
                "connections": 0,
                "confidence": entity["confidence"],
                "attributes": {k: v for k, v in entity["attributes"].items() if not k.startswith("_")},
                "shared": bool(entity["seen_in"]),
            }
            for entity in entities
        ]
        by_id = {node["id"]: node for node in nodes}
        edges = []
        seen = set()
        # An edge belongs to the session's view when both of its ends do.
        for entity in entities:
            for rel in graph.relationships(entity_id=entity["id"]):
                if rel["id"] in seen or rel["subject_id"] not in ids or rel["object_id"] not in ids:
                    continue
                seen.add(rel["id"])
                edges.append(
                    {
                        "from": rel["subject_id"],
                        "to": rel["object_id"],
                        "label": rel["predicate"],
                        "confidence": rel["confidence"],
                    }
                )
                by_id[rel["subject_id"]]["connections"] += 1
                by_id[rel["object_id"]]["connections"] += 1
        return {"nodes": nodes, "edges": edges, "source": "graph"}
    except Exception as error:
        print(f"Reading the knowledge graph failed: {error}")
        return None


def ingest_session(
    store_path: str,
    session_id: str,
    messages: List[Dict[str, Any]],
    llm: Dict[str, Any],
    owner: str = DEFAULT_OWNER,
    embedder: Optional[str] = None,
) -> Dict[str, Any]:
    """Extract a session's messages into the graph with the configured LLM.

    Raises RuntimeError with the reason when it cannot (no engine, no LLM
    configured, graph open read-only, LLM failure).
    """
    graph = _graph(store_path, owner)
    if graph is None:
        raise RuntimeError("the knowledge graph is not available")
    # "auto" uses the configured embedding API, else the local BGE-M3 model
    # (a 2.2 GB download on first use). MEMST_SERVER_EMBEDDER overrides it;
    # "none" skips embedding.
    embedder = embedder or os.environ.get("MEMST_SERVER_EMBEDDER", "auto")
    api_url = (llm or {}).get("api_url", "")
    model = (llm or {}).get("model", "")
    if not api_url or not model:
        raise RuntimeError("no LLM is configured for extraction")
    pairs = [
        (str(message["id"]), str(message.get("content", "")))
        for message in messages
        if message.get("id") and str(message.get("content", "")).strip()
    ]
    if not pairs:
        raise RuntimeError("the session has no messages to extract from")
    return graph.ingest_messages(
        session_id,
        pairs,
        llm_api_url=api_url,
        llm_model=model,
        llm_api_key=(llm or {}).get("api_key") or None,
        embedder=embedder,
    )
