# Roadmap

Agreed on 14 November 2026. Order of priorities: a solid engine first, then
evaluation, then the manuscript. Findings behind these choices are in
`CodeReview/CR-20261114.md` and `docs/research/memst-and-the-field.md`.

## Decisions

- **Graph scope:** one HelixDB graph per store, shared across sessions.
- **Graph stores:** KG extraction writes entities and relationships into
  HelixDB; SQLite keeps ontologies and job records only.
- **Embeddings:** a local CPU model in the BGE family (bge-m3, about 0.6B
  parameters, 1024 dimensions), so embedding needs no API key.
- **Single writer:** the first process to open a store holds the HelixDB
  writer; a second writer fails with a clear message and can still read.
- **Evaluation:** uses an LLM key supplied when the evaluation sprint starts.

## Sprints

| # | Sprint | Status | Done when |
|---|--------|--------|-----------|
| 1 | Green build and CI | **Done**, CI green | `cargo test --workspace` and the Python tests pass, and run in CI |
| 2 | One graph per store | **Done** | Conformance suite passes on the store-level graph; old stores migrate; a query scoped to one owner never returns another's entities |
| 3 | Extraction into HelixDB | **Done**, two items carried into sprint 4 | Extracting a conversation makes its entities searchable with no manual step; the server has no separate graph store |
| 4 | Non-destructive, time-aware graph | **Done**, one item moved to sprint 5 | A corrected fact outranks and hides the stale one; an entity merge can be undone |
| 5 | Evaluation harness | Next (needs an LLM key) | One command reproduces every table; MemSt is compared against full-context and raw-history baselines |
| 6 | Security and MCP | Planned | The MCP adapter is connected to a store and refuses unscoped access; deletion design written |

The manuscript update follows sprint 5.

## Sprint detail

**1. Green build and CI.** Done. See the addendum in `CodeReview/CR-20261114.md`.

**2. One graph per store.** Done. One HelixDB database per store; entities
carry their owner and the sessions that mention them; every query and search is
restricted to the handle's owner, with search indexes partitioned by owner;
`resolve_or_add_entity` reuses an entity met again; a lock file refuses a
second writer while read-only handles keep working; `memst graph migrate` moves
per-session graphs across. See `docs/graph-backend-helix.md`.

**3. Extraction into HelixDB.** Done:

- `memst-sleep` `kg_ingest` writes LLM extraction into the shared graph:
  known entities reused, repeated relationships reinforced, provenance
  (session, message, run) recorded, new and changed entities embedded.
- `memst-embed`: BGE-M3 on the CPU. `KnowledgeGraph::embed_pending` embeds only
  what is new, changed, or embedded by another model.
- CLI `graph ingest`; Python `memst.Graph`; the server's knowledge-graph
  endpoints read and build the engine's graph, with the old regex graph as
  fallback.

Carried into sprint 4:

- **Ontology-guided extraction (`memst-extract-v2`) is not connected.** Its
  extraction step does not call an LLM: it matches a fixed list of ten names.
  Feeding that into the shared graph would only pollute it, so it was left
  alone. Making it real (LLM call with the ontology prompt, then `kg_ingest`)
  is the remaining work.
- **BM25 search over relationships.** Done at the start of sprint 4.

**4. Non-destructive, time-aware graph.** Done:

- Keyword search over relationships (carried from sprint 3).
- Relationships carry validity in time. A new value of a single-valued
  predicate supersedes the old one, which leaves queries, traversal and search
  but stays in history and in point-in-time views. Ingestion applies this.
- Entity merges are recorded and can be undone.
- Ontology-guided extraction (`memst-extract-v2`) makes a real LLM call when a
  model is configured (`ExtractionService::with_llm`, `extract_graph`) and
  returns the common extraction result that `kg_ingest` writes to the graph.
  It is a library function: the CLI, the Python `ExtractionService` wrapper
  and the server's `/kg/extract` endpoints do not use it yet, and without a
  model the keyword demo remains.
- `search_with_context`: search hits together with the current relationships
  around them.

Moved to sprint 5: raw messages as a retrieval strategy. That is the
raw-history baseline the evaluation needs, so it is built there.

**5. Evaluation harness.** Full-context and raw-history retrieval baselines
under the same reader. LongMemEval-S. Hardened LoCoMo runs. Cost and latency
reporting. Oracle-router bound.

**6. Security and MCP.** Connect the MCP tool handlers to a real store (they
are placeholders today). Write gating, ownership checks and authentication.
Deletion design for a store that keeps history. One memory-poisoning test
suite.

## Known gaps not yet scheduled

- Ontology-guided extraction is not reachable from the CLI, Python or the
  server (see sprint 4).
- The server keeps its own memory tiers in `.jsonl` files instead of using the
  engine's.

- Three-way merge in `memst-repo` does not load memories from commits.
- `memst-py` reports version 0.1.x while the workspace is 1.0.0.
- Root-level Python helpers (`llm/`, `demo/`, `pyproject.toml`) sit in the Rust
  workspace root.
