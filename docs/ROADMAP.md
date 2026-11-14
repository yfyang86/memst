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
| 1 | Green build and CI | **Done** | `cargo test --workspace` and the Python tests pass, and run in CI |
| 2 | One graph per store | **Done** | Conformance suite passes on the store-level graph; old stores migrate; a query scoped to one owner never returns another's entities |
| 3 | Extraction into HelixDB | Next | Extracting a conversation makes its entities searchable with no manual step; the server has no separate graph store |
| 4 | Non-destructive, time-aware graph | Planned | A corrected fact outranks and hides the stale one; an entity merge can be undone |
| 5 | Evaluation harness | Planned | One command reproduces every table; MemSt is compared against full-context and raw-history baselines |
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

**3. Extraction into HelixDB.** Extraction writes to the graph with provenance
(source message, extraction run). Local embedding model; entities embedded at
write and re-embedded when their text changes. BM25 over relationships. Graph
exposed in `memst-py`; the server reads through it and drops its own graph
tables.

**4. Non-destructive, time-aware graph.** Valid-from, valid-to and
superseded-by on relationships; superseded facts marked rather than only
decayed. Reversible entity merges. Search and graph expansion in one query.
Raw messages offered as a retrieval strategy.

**5. Evaluation harness.** Full-context and raw-history retrieval baselines
under the same reader. LongMemEval-S. Hardened LoCoMo runs. Cost and latency
reporting. Oracle-router bound.

**6. Security and MCP.** Connect the MCP tool handlers to a real store (they
are placeholders today). Write gating, ownership checks and authentication.
Deletion design for a store that keeps history. One memory-poisoning test
suite.

## Known gaps not yet scheduled

- Three-way merge in `memst-repo` does not load memories from commits.
- `memst-py` reports version 0.1.x while the workspace is 1.0.0.
- Root-level Python helpers (`llm/`, `demo/`, `pyproject.toml`) sit in the Rust
  workspace root.
