# memst-py

Python bindings for MemSt (PyO3 extension module).

This package exposes the MemSt session store, tiered memory helpers, and selected advanced search and git-like object wrappers.

## Install (from source)

```bash
cd memst-py
maturin develop --release

python -c "import memst; print(memst.__version__)"
```

## Quick start

```python
import memst

# Create or open a store directory
store = memst.SessionStore("./memst-store")

# Create a session
session = store.create_session("My Chat", "gpt-4")

# Add messages
store.add_message(session.id, memst.Role.User, "Hello!")
store.add_message(session.id, memst.Role.Assistant, "Hi there!")

# Read message history
messages = store.get_session_messages(session.id)
print([m["role"] for m in messages])

# Keyword search (across all sessions)
results = store.search("hello", limit=10)
for r in results:
    print(r["score"], r["snippet"])
```

## Memory tiers

```python
import memst

store = memst.SessionStore("./memst-store")
session = store.create_session("Memory", "gpt-4")

store.add_memory(session.id, memst.MemoryTier.Working, "User prefers dark mode", tags=["preference"])

working = store.get_session_memory(session.id, memst.MemoryTier.Working)
print(len(working))
```

## Semantic search (HNSW)

```python
import memst

cfg = memst.HnswConfig()
cfg.m = 16
cfg.ef_construction = 200

index = memst.HnswIndex(2, cfg)
index.add_document("doc-a", [1.0, 0.0], session_id="s1", doc_type="message", content="hello")
index.add_document("doc-b", [0.0, 1.0], session_id="s2", doc_type="message", content="world")

results = index.search([1.0, 0.0], limit=1)
print(results[0].id, results[0].score)
```

## Query routing

```python
import memst

router = memst.QueryRouter()
strategy = router.analyze_query("find messages about rust ownership")
explain = router.explain_recommendation("find messages about rust ownership", "hybrid")
print(strategy)
print(explain)
```

## License

Apache-2.0 (see repository `LICENSE`).
