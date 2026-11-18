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
| 5 | Evaluation harness | **Done** (pilots on LoCoMo and LongMemEval-S) | One command reproduces every table; MemSt is compared against full-context and raw-history baselines |
| 5b | Full evaluation and read path | **Done** | Full runs reported with paired tests and a second judge; a read path that beats plain retrieval on the pilots |
| 5c | A narrower job for the graph | **Done**: not passed, reported as a negative result | The graph beats raw retrieval on temporal and knowledge-update questions by a paired test, or is reported as a negative result |
| 6 | Security and MCP | **Done** | The MCP adapter is connected to a store and refuses unscoped access; deletion design written |

The manuscript was rewritten after sprint 6, around these results (see `docs/research/memst-and-the-field.md`, "Manuscript").

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

**5. Evaluation harness.** Done: `benchmarks/harness` runs retrieval
pipelines under one reader on LoCoMo and LongMemEval-S, and reports F1 with
intervals, judged accuracy, evidence coverage, prompt tokens, retrieval
latency, write-path cost and an oracle-router bound.

Retrieval is a configurable pipeline of stages (`facts`, `entities`,
`fact_sources`, `turns`, `all_turns`), defined in JSON, and every question
leaves a trace of what each stage returned and whether it reached the answer.
The default pipeline searches the graph by relationship, follows the facts to
the messages they were stated in, and adds raw turns.

Findings so far (figures and caveats in `benchmarks/harness/README.md`):

- LoCoMo, 138 questions: the default pipeline is at 54 F1, level with the
  best raw-history baseline (53) and below full context (59). The graph alone
  is at 46, up from 39 when it was searched by entity.
- LongMemEval-S, 12 questions with 127k-token histories: retrieval pipelines
  match full context at 4% of the prompt tokens, but the sample is too small
  to rank them.

Running against a real model exposed engine faults, now fixed: the extraction
output budget was too small for reasoning models, the parser threw away a
whole batch over one malformed item, the LLM client did not retry dropped
connections, facts carried neither a date nor a link to the message that
states them, and relationship search fetched its hits one at a time (0.9 s
per query, now 0.04 s).

Not done: a LongMemEval run large enough to rank pipelines, other benchmarks,
repeated or independent judging, and any measurement of consolidation, decay
or the memory tiers.

**5b. Full evaluation and the read path (18 November 2026).** Done. The full
runs (1,986 LoCoMo questions, 60 LongMemEval-S questions) settle the earlier
question: **the knowledge graph does not beat retrieval over raw turns.** On
LoCoMo the default graph pipeline is 3.0 points of judged accuracy behind
`raw-dense` on the same questions (interval 0.9 to 4.9); on LongMemEval-S they
are level. A router by kind of question recovers none of the oracle router's
13 points.

The reader loses more points than retrieval does, so the read path was
worked on next:

- Reports now give answerable-only figures (the 1,540-question LoCoMo subset
  others publish), intervals on judged accuracy, and paired differences.
- A second judge can grade a run (`--judge-model`, or `rescore` on saved
  answers). A stricter judge agrees with the original on 98–100% of
  LongMemEval answers and 92% of LoCoMo ones, where the reader's own model is
  lenient by 6 to 10 points; the ordering of pipelines holds.
- LongMemEval's preference questions were being asked for a short fact and
  graded as one; the reader prompt and the judge now follow the dataset's own
  rules.
- New pipeline stages `rerank` and `neighbors`, and a reader that works
  through the evidence before answering. On the pilots, a wider context with
  that reader (`dense-wide-reason`) and reranking with it (`rerank-reason`)
  gain 9 to 10 points of judged accuracy over `raw-dense` on LoCoMo and 17 to
  22 on LongMemEval-S, and match or beat reading the whole history at under a
  fifth of the tokens. Both are built in.

Not done: those two pipelines at full scale, all 500 LongMemEval-S questions,
and a judge from another provider. The commands are in
`benchmarks/harness/RUNNING.md`.

**5c. A narrower job for the graph.** Done; the test was not passed.

The question was whether the graph earns its extraction cost on questions
about dates and changed facts, when added to the best raw pipeline and read
by the same reasoning reader. The criterion, fixed beforehand, was a reliable
gain in judged accuracy over `dense-wide-reason` on the same questions.

- **LoCoMo, all 321 temporal questions:** no use of the graph gives a
  reliable gain, by either judge. The best one (the graph as an index to
  source messages) is 2.5 points ahead, interval −0.6 to +5.9, the same as
  reranking, which needs no extraction. Showing the reader extracted facts,
  as lines or as a timeline, does not help. The graph alone is 12 points
  behind.
- **LongMemEval-S, knowledge-update and temporal questions:** on 22 questions (10 of each type and
  2 abstention ones) the best raw pipeline answers every one correctly, so
  there is nothing for the graph to add; every graph variant is level or
  one to three answers behind.
- **An engine fault was found and fixed.** Ingestion dated a fact with its
  message's date when the text gave none, so "we went two weeks ago" was
  dated to the day it was said. `stated_at` and `valid_from` are now
  separate, with the precision of the date. This raised the graph pipeline
  with a direct reader by 12 points on temporal questions and did not change
  the verdict.
- **Extraction cost cannot simply be cut.** Batches four times larger give a
  quarter of the calls and half the facts.

What this means: on these benchmarks, with this reader, the knowledge graph
is not a retrieval advantage in any role tried. It is at best level with raw
retrieval at 18 to 47 extra model calls per history. The position this
supports is MemSt as a versioned, auditable store whose retrieval is raw
turns with a reasoning reader, with the graph as a structured, time-aware
view for uses these benchmarks do not measure (browsing, audit, correction,
rollback), not as the thing that answers questions.

Still untested, and the only places left where the graph could win:
questions that need facts combined across many sessions at a scale where
retrieval cannot return all of them (LongMemEval-M, about 1.5 million
tokens per history), and tasks that edit memory (corrections, deletion,
"as of" queries), which need a benchmark of their own.

**6. Security and MCP.** Done.

- **A real MCP server.** `memst mcp serve` speaks JSON-RPC over stdio and is
  backed by the store; the earlier adapter returned fixed text and had no
  transport. Tools: `memory_search`, `memory_read`, `memory_get`,
  `memory_status`, `memory_write`, `memory_forget`. Checked against the
  official MCP Python SDK client. `skill_lookup` and `context_build`, which
  were never implemented, are no longer advertised.
- **Scope.** A server is bound to one owner at start (`--owner` is
  required). No tool takes an owner, session or path; an argument a tool
  does not define is refused. `--read-only` removes the writing tools.
- **Write gating.** Notes that read like instructions are quarantined until
  the store's owner approves them on the command line; limits on size, rate
  and count; tags that claim trust or ownership are dropped; every write,
  deletion and approval is logged without content.
- **Reading back.** Returned notes are fenced, labelled with their source
  and whether they were approved, and introduced as records, not
  instructions.
- **A poisoning test suite**: ten attacks, each with the defence that stops
  it (`memst-mcp/tests/poisoning.rs`).
- **Deletion.** `docs/deletion-design.md` sets out four levels of deletion,
  what is implemented and the design for the rest. `memory_forget` removes a
  note with no copy left under the store. Deleting a session now also
  removes it from the search index; before, its text stayed searchable.
- **The store lock.** A store handle holds an exclusive lock for its whole
  life, so a long-running server would have blocked everything else. The MCP
  server takes it per operation.

Limits, stated in `docs/mcp.md`: screening catches plainly phrased
instructions, not a false statement dressed as a fact; owners are separated
by the server, not by the file system; there is no network transport and so
no authentication of its own; the command line and Python module bypass the
MCP checks. Not built: deleting a single message, cascading deletion to
derived facts, and deletion from history (designed, in the deletion note).

## Known gaps not yet scheduled

- Ontology-guided extraction is not reachable from the CLI, Python or the
  server (see sprint 4).
- The server keeps its own memory tiers in `.jsonl` files instead of using the
  engine's.

- Three-way merge in `memst-repo` does not load memories from commits.
- `memst-py` reports version 0.1.x while the workspace is 1.0.0.
- Root-level Python helpers (`llm/`, `demo/`, `pyproject.toml`) sit in the Rust
  workspace root.
