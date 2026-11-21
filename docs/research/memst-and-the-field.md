# MemSt and the state of agent-memory research (November 2026)

MemSt's design notes and manuscript were written against the literature up to
early 2025. This note records what has changed since and what it means for this
codebase. The evidence behind every statement is in the companion survey,
[agent-memory-survey-2026-11.md](agent-memory-survey-2026-11.md), which tags each
source by how it was verified. Items first published after June 2026 are
unreviewed preprints and are marked there.

## What changed in the field

1. **The vocabulary moved.** The working / short-term / long-term / archival
   split is now one axis among several. Current surveys describe memory by
   *form* (external text, parametric, latent), *function* (factual,
   experiential, working) and *operations* (write, consolidate, update, index,
   forget, retrieve, and, since 2026, govern, audit, roll back).

2. **LoCoMo alone is no longer accepted as evidence.** The conversations fit in
   a context window, so a full-context baseline beats most memory systems. An
   independent audit found 6.4% wrong gold answers and a judge that accepts most
   plausible wrong answers. Several late-2026 studies find plain retrieval over
   raw history matches or beats engineered memory. The expected practice is a
   benchmark portfolio (LongMemEval, BEAM, MemoryAgentBench, operation-level
   probes), two mandatory baselines (full context and raw-history RAG under the
   same reader), and cost reporting.

3. **Non-destructive memory won.** Systems that rewrite or summarise away their
   source lose evidence they cannot get back. Mem0's 2026 redesign is add-only;
   newer systems keep complete records and consolidate beside them.

4. **Git-like memory is no longer unusual.** Letta's Context Repositories
   (February 2026) version memory in git with worktrees and merges; ChronoMem,
   GitOfThoughts and Kumiho study versioned memory directly. One of them reports
   a useful negative result: versioning buys auditability and mergeability, not
   retrieval accuracy.

5. **Temporal graphs moved from decay to supersession.** Bi-temporal edges
   (when a fact was true, and when it was recorded) with invalidation instead of
   deletion are the reference design. A stale competing fact is measurably worse
   than an irrelevant one, so lowering its rank is not enough.

6. **Security became a literature of its own.** Memory poisoning through normal
   interaction, cross-user leakage and unsafe consolidation are documented, and
   the consensus defences are provenance, write gating, ownership checks at
   retrieval time, and versioned rollback. Forgetting on request is essentially
   unsolved, and is harder in a store that keeps history.

7. **Vendors ship primitives, not architectures.** File-style memory tools,
   context editing and compaction, session stores and skill bundles are now
   platform features. A memory system is increasingly judged by whether it can
   sit behind those interfaces.

## Where MemSt stands

| Area | MemSt today | Reading against the field |
|------|-------------|---------------------------|
| Versioned store (`memst-repo`) | Content-addressed objects, WAL, worktrees, fast-forward merge and conflict detection. A full three-way merge is not there yet: memories are not loaded from commits | The strongest differentiator, but it now has direct neighbours that must be cited and compared against. Nothing yet measures what versioning buys (rollback, audit, merge). |
| Evaluation (`benchmarks/`) | LoCoMo only. MemSt's own tables show full context at 80.7 F1 against 42.5–49.2 for MemSt strategies | This is exactly the pattern the field now treats as "memory not shown to be needed". No raw-history RAG baseline under the same reader, no cost-to-break-even, category 5 dropped. |
| Query-adaptive routing | Rule-based classifier, 80% accuracy | A 2026 study finds per-query routing realises little of the oracle headroom; regime-level selection does better. The oracle bound and realised fraction should be reported. |
| Knowledge graph | Entities and relationships with half-life decay, merging, deprecation; now on HelixDB | Relationships now carry validity in time (kept apart from when they were stated) and are superseded rather than only decayed; entity merges can be undone. Cascading invalidation of dependent facts is not done. The graph is now one per store with per-owner isolation, in line with the field's user-level graphs. |
| Consolidation (`memst-sleep`) | Sleep-time jobs, A-MEM style evolution | In line with current practice, but consolidation rewrites rather than appends, and extraction hallucination is not measured. |
| Raw evidence | Messages are kept in the session store | Good: the raw episodes exist. They are not offered as a retrieval strategy alongside the consolidated ones. |
| Skills | Procedural memory with lookup | No update / deprecate path, flat retrieval, no import or export in the common `SKILL.md` layout. |
| Security | An MCP server bound to one owner, with quarantine of instruction-like writes, server-set trust and ownership tags, labelled reads, limits, an audit log and a read-only mode | Covers the consensus defences at the tool boundary: provenance, write gating, ownership checks, labelled retrieval. Not covered: detection of false but plausible notes, encryption, and the command line and Python paths, which bypass the checks. Deletion from history is designed, not built (`docs/deletion-design.md`). |
| Graph substrate | Embedded HelixDB (graph + vector + BM25). Entity search uses its BM25 and vector indexes, fused by reciprocal rank, with MemSt's substring matching as fallback | Matches the hybrid substrate the field converged on. Relationships are searchable by BM25. Entities are embedded only on request; see [graph-backend-helix.md](../graph-backend-helix.md). |

## What MemSt's own evaluation now shows

`benchmarks/harness` has the baselines the field asks for, on all of LoCoMo
(1,986 questions) and a 60-question LongMemEval-S sample, with one model.

- **The graph does not beat raw-history retrieval.** LoCoMo judged accuracy:
  full context 75.5%, embedding retrieval over raw turns 72.2%, the graph
  with raw turns 69.2% (3.0 points behind on the same questions, interval 0.9
  to 4.9), the graph alone 53.0%. On LongMemEval-S, raw retrieval 65–68%, the
  graph with raw turns 66.7%, the graph alone 47–52%, full context 58.3%.
- **Per-question routing realises none of its headroom**, as the field
  reports: the oracle is 13 points above the best single pipeline, a router
  by kind of question is 0.2 below it.
- **The reader is the bottleneck on long histories.** On LongMemEval-S the
  answer turn is retrieved for every question and 65% are answered correctly.
  A reader that works through the evidence, over a wider or reranked context,
  gains 9 to 10 points on a LoCoMo pilot and 17 to 22 on a LongMemEval-S
  pilot, and matches full context at under a fifth of the tokens.
- **A narrower job for the graph did not hold up either.** Asked only to help
  on questions about dates and changed facts, added to the best raw pipeline
  with the same reasoning reader, no use of the graph gives a reliable gain:
  +2.5 points at best on LoCoMo's 321 temporal questions (interval −0.6 to
  +5.9), and nothing on 22 LongMemEval-S knowledge-update and temporal
  questions, which the raw pipeline answers in full. This matches the 2026
  reports that retrieval over raw history equals engineered memory.
- **Valid time and statement time must be stored apart.** MemSt dated facts
  with the day they were said; fixing that was worth 12 points to the graph
  pipeline on temporal questions.
- **The reader's own model is a lenient judge on LoCoMo** (6 to 10 points
  above a stricter one), consistent with the audit cited above.

Details and caveats are in `benchmarks/harness/README.md`. The older tables
under `benchmarks/` were produced through the server with different category
labels and are not comparable.

## Recommended order of work

**1. Evaluation.** This carries the most risk to the manuscript's claims.

- Add full-context and raw-history RAG (BM25, and dense with rerank) baselines
  under the same reader, embedding model and token budget. MemSt already has
  BM25, HNSW and RRF, so a "raw episodes only" ablation is cheap.
- Add LongMemEval-S, and one of MemoryAgentBench, BEAM or an operation-level
  benchmark.
- For the LoCoMo numbers that remain: state the 93.57% ceiling, score category 5
  as abstention, repeat the judging and report intervals.
- Report write-path cost, read tokens, p50/p95 latency and break-even turns
  against full context.
- Report the oracle-router upper bound next to the realised routing gain.

**2. Make the versioned store the headline, and make it non-destructive.**

- Position against Letta Context Repositories, ChronoMem, GitOfThoughts, Mem++
  and Kumiho in the related work.
- Add a rollback evaluation: answer as if later updates never happened.
- Make consolidation and entity merging append-only, so a merge can be undone
  from history.

**3. Bi-temporal graph.** Add `valid_from`, `valid_to` and `superseded_by` to
relationships, mark superseded edges instead of only decaying them, and filter
or label them at retrieval. The HelixDB backend makes these indexed properties;
the version log already supplies transaction time.

**4. Security and governance.** Provenance on every item, ownership gating at
retrieval, authentication and scoping on the MCP adapter, and a design for
deletion in a store that keeps history.

**5. Finish the move onto HelixDB.** Entity search already uses its BM25 and
vector indexes. Still open: embedding entities automatically at extraction
time, and combining search with graph expansion in one query.

**6. Interoperability.** Skills import and export in the `SKILL.md` layout, and
a backend for the file-style memory tool command set, next to MCP.

## Manuscript

`manuscript/memst/memst-yifan.tex` was rewritten on 18 November 2026 around
the results above. The earlier text claimed 49.2% F1 for query-adaptive
routing; the new one reports the like-for-like evaluation, the negative
result for the graph and for routing, the reader finding and the fault in
how facts were dated, and positions MemSt as a versioned store. Its tables
and every number in its evaluation are generated from the result files by
`manuscript/memst/make_tables.py`.

Claims of the earlier text that had no evidence in the repository were
removed and not replaced: the importance-score weights "chosen by grid
search", the comparison with an exact integer-programming solution, 87%
extraction precision "by manual analysis", the 12–18% deduplication saving,
the per-message cost and latency figures, and the GPT-4o-mini judge. The six
figures of the earlier evaluation are no longer used.

References added in the rewrite were checked against arXiv metadata. Five
older entries that the new text cites were wrong (a placeholder arXiv number,
wrong author names, a wrong title) and were corrected. The other entries in
`cite.bib` are not cited by the new text and have not been checked;
`cite-2026-candidates.bib` still has truncated author lists.
