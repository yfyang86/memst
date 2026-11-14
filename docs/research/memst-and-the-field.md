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
| Knowledge graph | Entities and relationships with half-life decay, merging, deprecation; now on HelixDB | Decay-only. No valid-time / transaction-time, no supersession marking. Entity merge is not reversible. Graphs are per session, while the field has moved to user-level graphs. |
| Consolidation (`memst-sleep`) | Sleep-time jobs, A-MEM style evolution | In line with current practice, but consolidation rewrites rather than appends, and extraction hallucination is not measured. |
| Raw evidence | Messages are kept in the session store | Good: the raw episodes exist. They are not offered as a retrieval strategy alongside the consolidated ones. |
| Skills | Procedural memory with lookup | No update / deprecate path, flat retrieval, no import or export in the common `SKILL.md` layout. |
| Security | None | No provenance beyond `session_id` / `source_message_id`, no write gating, no ownership check at retrieval, MCP adapter unauthenticated. |
| Graph substrate | Embedded HelixDB (graph + vector + BM25). Entity search uses its BM25 and vector indexes, fused by reciprocal rank, with MemSt's substring matching as fallback | Matches the hybrid substrate the field converged on. Entities are embedded only on request, and relationships are not indexed for search; see [graph-backend-helix.md](../graph-backend-helix.md). |

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
time, combining search with graph expansion in one query, and one graph per
user rather than per session.

**6. Interoperability.** Skills import and export in the `SKILL.md` layout, and
a backend for the file-style memory tool command set, next to MCP.

## Manuscript

`manuscript/memst/memst-yifan.tex` has **not** been edited. Refreshing it is
more than adding citations: the evaluation critique above bears on the paper's
central claim, and that is the author's call. Candidate references are in
`manuscript/memst/cite-2026-candidates.bib`. Their author lists are truncated
and must be completed, and venues checked, before any are cited.

The limitations section says conflict-aware three-way merge is not
implemented. That is still accurate: `memst-repo` detects conflicts between
memories and fast-forwards, but does not yet load memories from commits.
