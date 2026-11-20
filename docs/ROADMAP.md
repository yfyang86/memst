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
| 7 | Four-layer memory in the store | **Done** | Every note change can be diffed and reverted; existing MCP notes migrate; the index stays within its cap |
| 8 | Pi as the default agent | **Done** | A fresh Pi session recalls something from an earlier one, in a scripted test |
| 9 | Dreaming on a branch | **Done** | A planted wrong note is corrected against the transcript; a bad pass can be rolled back; unverified notes are not laundered into the trusted index |
| 10 | Evaluate memory in use | Done | 60 cross-session scenarios under three conditions, seven criteria fixed beforehand: five pass, one fails on the answer check, one failed and led to a fix (forgetting did not reach transcripts) |
| 11 | Hardening and release | Planned | Parallel writers are safe; the open deletion work is done; packages are published |

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

Full-scale follow-up (20 November 2026): both raw read paths are now measured
on all 1,986 LoCoMo questions and all 500 LongMemEval-S questions. On LoCoMo,
`dense-wide-reason` and `rerank-reason` gain 7.3 and 9.1 judged points over
`raw-dense` (intervals 5.6 to 8.9 and 7.4 to 10.8), and beat the earlier
full-context run at 7% and 15% of its prompt tokens. On LongMemEval-S they
reach 88.2% and 88.6%, gains of 20.6 and 21.0 points. Reranking adds 1.8 points
over wide dense retrieval on LoCoMo (0.4 to 3.3), but only 0.4 on
LongMemEval-S (−1.8 to 2.6). Reports are in `read-path-locomo-full/` and
`full-longmemeval-raw/`; both are complete with no answer errors.

Not done: a judge from another provider, strict regrading of these full runs,
and full-context or graph pipelines on all 500 LongMemEval-S questions.

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

**7. Four-layer memory in the store.** Done. Memory is now modelled on the
layers Anthropic's engineering note of November 2026 describes
(transcripts, notes, representations, injected context), as plain files
under `<store>/memory/<owner>/`.

- **Notes are Markdown files with history.** A new module,
  `memst_repo::versioned_dir`, keeps every committed state of a directory as
  content-addressed blobs, trees and commits: log, read an earlier version,
  restore, and purge. The MCP server writes each note as a file and each
  change as a revision with its author.
- **The index** (`MEMORY.md`) is built by the server from the notes, within
  200 lines and 8,000 bytes, pinned notes first. The agent cannot write it.
- **Transcripts** are append-only files per session, searched only when
  asked for.
- **Edits outside the server are not trusted.** A changed file is taken in as
  unverified, screened, and recorded as changed by "external".
- **Forgetting reaches history**: a note's earlier versions are purged with
  it. This is the first use of the versioned repository's design for
  something the evaluation does not measure but an owner needs: seeing what
  changed, undoing it, and removing it for good.
- **Migration**: notes written by sprint 6's server are moved in on first
  open.
- Tools added: `memory_context`, `memory_update`, `memory_history`,
  `transcript_append`. Owner commands added: `notes`, `index`, `show`,
  `history`, `restore`, `pin`, `unpin`.

Limits: each operation reads every note, so writes slow down as notes
accumulate (limit 10,000 per owner); search had no stemming (added in sprint 11); pages (the
third layer) are not built until sprint 9.

**8. Pi as the default agent.** Done. `integrations/pi` is a native Pi
extension (Pi has no built-in MCP). It runs `memst mcp serve` as a child
process and speaks its protocol over the child's pipes.

- The index is added to the system prompt, fetched once per session so the
  prompt stays the same from turn to turn; an empty memory is still
  announced, so the model knows it can write.
- Seven tools for the model: search, read, get, history, write, update,
  forget. What the user and the assistant say is recorded as a transcript;
  tool output is not.
- `/memory` lets the user review, approve, pin, forget, see the history and
  go back, from inside Pi. The model cannot run it.
- Pi's file tools are blocked from the memory files, so that notes change
  only through tools that record who did what.
- Tests: six offline tests drive the extension with a stand-in for Pi
  against the real `memst` program, in CI. A live test with Pi 0.73.1 and a
  real model ran two separate sessions; the second recalled both facts the
  first was told and remarked that they were unverified.

Limits: `bash` can still reach the memory files (a changed file is taken in
as unverified by the next session, but another owner's notes can be read);
the extension has been run with one Pi version and one model; the index is
fixed at the start of a session, so a note written during a session shows in
the index only from the next one.

**9. Dreaming on a branch.** Done. `memst mcp dream` is an offline pass that
reads the notes against what was said and proposes corrections, missing
notes, pages and pins. A model drafts; the server checks.

- Every item must quote its source, and the quote is looked up there. A
  correction must rest on the user's own words.
- An approved note is never rewritten: a correction to one becomes an
  unverified note that disputes it.
- Everything the pass writes is unverified and screened; quarantined notes
  are not shown to the model; pins are only suggested.
- A proposal waits for review, then lands as one revision by "dream".
  Restoring the revision before it undoes the pass. A note that changed
  while the proposal waited is left alone, which is what "on a branch" comes
  to without real branches.
- Pages (the third layer) are Markdown files in which every statement
  carries the words it rests on. Forgetting a source removes the pages built
  on it.

All three criteria were met, in tests with a scripted model and in a run
with a real one: three wrong notes were found and corrected or disputed from
the transcript, three missing notes and a page were added, and one `restore`
undid the pass.

Limits: the pass checks that something was said, not that it is true; one
model has been tried; it runs on demand, not on a schedule; history is
linear, so two passes at once are not supported.

**10. Evaluate memory in use** *(done)*, with criteria fixed beforehand, as
in 5c. Sixty scenarios (recall, update, correction by the offline pass,
forgetting, a planted instruction, abstention), each under three conditions
(no memory, memory, memory with the offline pass), with Pi and
`deepseek-flash`; every session a separate process. Details and every
answer: [benchmarks/sessions](../benchmarks/sessions/README.md).

- Passed: recall across sessions (10/10 against 0/10 without memory); the
  offline pass correcting a wrong note (10/10 against 0/10 without the
  pass); a planted instruction never followed (0 of 20) with the page's fact
  still given; abstention (10/10); the pass doing no harm.
- Failed on the check as committed: updates (6/10). Every failing answer
  gives the new value and names the old one as history; the check was too
  strict, and the criterion is still reported as failed.
- Failed: forgetting. The note was gone, the fact was still in the
  transcripts in 20 of 20 runs, and one answer found it. Fixed
  (`memory_forget` now removes transcript entries by their words); on
  re-run, 0 of 20.
- Cost: about 5,200 prompt tokens for a final session with memory against
  560 without, and one second more.

Limits: short scenarios written by us, a nearly empty memory, ten instances
per cell, one model and one agent. Not done: pages plus search against
search alone in the question-answering harness.

**11. Hardening and release** *(done, except where marked open)*.

- **A store in use no longer makes the next process wait in silence.** A
  store handle holds the lock for as long as it lives; a second process now
  waits 30 seconds (`MEMST_LOCK_TIMEOUT_SECS`) and then says the store is in
  use. The lock is still held for the handle's life: per-operation locking
  in the engine is **open**.
- **One offline pass at a time** on a memory; a second is refused.
- **Search matches a word with its own endings** ("seat"/"seats",
  "deploy"/"deployed").
- **Forgetting reaches the transcripts** (from sprint 10).
- **Release.** Workspace, Python module and command line all report 1.1.0
  (the module said 0.1.0 and the command line 0.1.0). A release workflow
  builds `memst` for Linux x86-64 and macOS arm64 on a `v*` tag or by hand;
  it has **not been run**, since it only runs from the default branch or a
  tag.
- **Manuscript.** A section on memory in use, with the sprint 10 results.

**12. Contradicted notes and a sounder forget** *(done)*.

- A note is shown with what the user said on its subject since it was
  written, and marked in the index; the later statement holds if they
  differ.
- After a forget, entries on the same subject that were not removed are
  listed and can be removed singly; the assistant's replies on that subject
  are not recorded for the rest of the session.
- The session evaluation judges an update by which value is given first.
- Round 2 of the evaluation (70 runs, criteria fixed beforehand): one of
  three criteria passed on the first run (forgetting a fact said in other
  words). The stale-note criterion reached 6/10 and one forget run left the
  assistant's confirmation on disk. After two fixes, all three pass on a
  second run (stale 10/10, against 0/10 in round 1 without the offline
  pass).

**13. Memory at realistic size** *(done; one part not done)*.

- Each note file is read once per operation, not up to eight times, and
  the index looks up later statements only for the notes it lists. At
  5,000 notes: the index 2.5 s to 67 ms, a write 757 ms to 129 ms, a search
  131 ms to 50 ms.
- Round 3 of the evaluation: recall, update, stale and abstain with 500 and
  5,000 other notes, a fifth of them near misses (80 runs, criteria fixed
  beforehand). First run: four of five criteria passed; the stale-note
  criterion failed (6/10 at 5,000). One cause was in the harness (a note
  and a later statement stamped with the same millisecond) and one in the
  system (statements sharing a frequent word pushed out the one that
  corrected the note; matches are now ranked by what they share). On a
  second run all five pass.
- Keyword search retrieved the needed note in every case examined, so, as
  decided beforehand, embedding search over notes was not added.
- Found and open: with many notes of the same shape about other teams, the
  agent sometimes reports a conflict or gives another team's value, once
  for a fact it was never told. A session costs two to three times the
  prompt tokens of one with a nearly empty memory.
- **Not done:** pages plus search against search alone.

**14. Deletion and locking in the session store** *(done; the cascade is
partial)*.

- **One message can be deleted** (`memst message delete`,
  `SessionStore::delete_message`): the message file and its index are
  rewritten without it, a tombstone records the id and nothing else, and its
  search entry is removed.
- **The graph follows.** Deleting a message removes the facts extracted
  from it and the entities nothing else refers to; deleting a session does
  the same for the session. Works on the JSON and HelixDB backends.
- **The search index really forgets.** With tantivy, a deleted document's
  text stayed in its segment file until a merge that might never come; this
  applied to session deletion too. Deletion now forces the merge.
- **The store is locked per operation**, not for the life of a handle. A
  long-running process (the Python module, a server) no longer blocks every
  other process, and two handles in one process take turns where they used
  to deadlock. `SessionStore::hold` keeps the lock across several
  operations when that is wanted.

Not reached by deleting a message: memory items and summaries made from it
(no record of their sources), the operations log, and facts a later message
restated. Deleting from the history of the versioned session repository
(design D) is not built.

**15. What versioning is worth** *(done)*.

- **Reading the past.** `memory_as_of` gives the notes as they stood at a
  given time; what was forgotten since is not shown.
- **Merging.** `memst mcp merge` joins two copies of an owner's memory
  three ways: one-sided changes kept, conflicting changes both kept,
  anything either copy forgot stays forgotten (also across later merges),
  everything incoming unverified and screened. Tested without a model.
- **Round 4 of the evaluation** (60 runs; criterion and expectation written
  first): what did the notes say at an earlier moment? With the as-of read
  20/20; with transcripts 19/20; with the current notes alone 8/10 when the
  change had been said and 0/10 when the note had been edited by hand. We
  expected transcripts to match the as-of read, and on accuracy they did.
  They took four times the prompt tokens and five to six times the time.

What this says about versioning: for recall of past values it is not more
accurate than keeping transcripts; it is much cheaper, and it is the only
source for a note's own earlier text. Undo, the record of who changed what,
and merging are things transcripts do not give at all. The three-way merge
in `memst-repo` for the session store is still not connected to anything.

**16. A second agent, a second model, and the release** *(evaluation and
manuscript done; the release is the owner's to make)*.

- **Round 5** (140 runs; criteria and expectations written first): round
  2's scenarios with a plain MCP client that has no extension, and with Pi
  on a larger model. Both pass all three criteria, 70 of 70 each. We had
  doubted that a model with no index in its prompt and nothing telling it
  to write notes would do so; it did. The plain client's sessions cost less
  than half of Pi's; the larger model's cost four times as much for the
  same answers.
- **Full-scale runs** (by the owner): the read-path pipelines on all of
  LoCoMo and the raw pipelines on all 500 LongMemEval-S questions; see
  sprint 5b above.
- **Manuscript**: rounds 2 to 5 of memory in use, deletion and the
  limitations brought up to date.
- **Not done**: the release workflow has not run (it needs a tag or the
  default branch); a client we did not write was not tried.

**17. A harder, independent test** *(done)*. Round 5 was 140 of 140, which
measures nothing. Two rounds with criteria and expectations fixed first.

- **Round 6: facts said in passing, over histories of 24 sessions**, some
  changing twice (192 questions). Pi with the extension: 96 of 96. A plain
  MCP client: 20 of 96. Pi answered stable facts from the transcripts the
  extension had recorded (only 4 of 32 were ever noted); the plain client
  records nothing and noted 4 of 80. Recording sessions is what makes
  memory hold what nobody decided to keep.
- **Round 7: sixty LongMemEval-S questions put to an agent searching
  MemSt's transcripts**, graded by the harness's judge beside the
  harness's pipelines on the same questions. The agent: 61.7% as scored
  (71.7% with six abstentions read as the correct answers they are),
  against 68.3% for one-shot dense retrieval and 91.7% for the two
  reasoning read paths. The criterion (not below dense retrieval) fails as
  scored. The agent is 20 to 30 points below the read paths the harness
  found in sprint 5b: that way of reading a history has not been carried
  into the memory tools.

**18. Whom a note is about** *(done)*. A note carries a subject (the
user, or a named other); the tools show and set it; search ranks the notes
a question is about first and can be limited to one subject. Round 8 (420
runs, criteria first): at 5,000 notes, wrong answers fell from 9 of 120 on
the build before to 3 of 120, all nine of the baseline's on one fact with
many look-alike notes about other teams. The difference could arise by
chance one time in seven. Told the user's fact beside the same facts about
named others, the agent answered both ways correctly in 30 of 30 and
labelled 125 notes about others correctly.

**19. What deletion and forgetting do not reach** *(two of three parts
done)*.

- **Memory items made from a message go with it.** A memory item can say
  which messages and memories it was made from; consolidation records its
  inputs; deleting a message removes what was made from it, and summaries
  made from that, with their search entries. A memory item that does not
  say where it came from is still not reached.
- **The native search index** was rewritten in full on every drop of a
  handle, searches included. Additions now go to a journal: 32 ms to 12.6
  ms per message at 2,000 messages. Still linear, since the index is loaded
  afresh for each operation.
- **A correction.** Sprint 14 recorded that a build with the native backend
  alone never wrote its search index. That was wrong: it wrote it on every
  drop. The test that suggested otherwise failed on how the native
  tokenizer splits a hyphenated word. The commit added then made things
  slower, and is superseded by the journal.
- **Not done: matching by meaning.** Forgetting and the stale-note match
  are still by shared words. An embedding model at read time was not
  chosen, and the local one did not fit on this machine's disk.

**20. Pages and the offline pass at size** *(done)*.

- **The pass was shown every note**, about 350,000 tokens' worth at 5,000
  notes. It is shown 80 at most, chosen statement by statement: for each
  thing the user said, the notes nearest to it. Prompt: about 17,600
  tokens whatever the memory holds.
- **Round 9, the pass over 5,000 notes** (criteria first): the prompt
  bound held; the wrong note was corrected in 3 of 10 on the first run,
  when notes were chosen for a whole stretch of transcript at once, and in
  8 of 10 after choosing them per statement.
- **Round 9, notes and pages beside transcripts**, thirty LongMemEval-S
  questions: 76.7% with the pass against 70.0% with transcripts only (-6.7
  to +20.0 points). Not shown to help; not shown to hurt.

**21. The read path in the memory tools** *(done; half the gap closed)*.
The harness's pilots put the gain in the reader working through the
evidence, not in wider retrieval. `memory_read` with scope `all` now
returns up to forty transcript entries in the order they were said, with
dates, and ends with how to read them. Round 10, round 7's sixty
questions on the new build (criteria first): 71.7% against 61.7%, ten
points (-1.7 to +21.7), and above one-shot dense retrieval. With
abstentions read fairly, 81.7% against 91.7% for the harness's best read
path. Retrieval is still by keyword. A question costs twice the tokens.

Next, in the order we would take them: retrieval by embedding in the
memory tools (the ten points left, and matching by meaning for forgetting
and stale notes, both need an embedding model at read time); have a plain
client's sessions recorded (round 6); then **22**, the release and a
client we did not write.

Open after sprint 11: three-way merge; deletion of single messages with
cascade to derived facts, and from history, for the session store; the cost
of reading every note per operation; a note contradicted by something said
later is not flagged at read time; forgetting by words misses a paraphrase;
the store is within reach of an agent that has a shell.

## Known gaps not yet scheduled

- Ontology-guided extraction is not reachable from the CLI, Python or the
  server (see sprint 4).
- The server keeps its own memory tiers in `.jsonl` files instead of using the
  engine's.

- Three-way merge in `memst-repo` does not load memories from commits.
- Root-level Python helpers (`llm/`, `demo/`, `pyproject.toml`) sit in the Rust
  workspace root.
