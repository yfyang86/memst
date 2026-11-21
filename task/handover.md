# Handover

Written 2026-11-20 and updated on 2026-11-21, part of the way through
sprint 22. Sprints 1 to 21 are merged into `dev` (pull requests 1 to 10).
Sprint 22's code is merged with this file; its measurement is yours to
run on the Mac (see "Tasks for the Mac" below).

Where to read more: `docs/ROADMAP.md` (what each sprint did, in detail),
`CodeReview/CR-20261114.md` (findings 1 to 108, each marked fixed or open),
`benchmarks/sessions/README.md` (the memory-in-use evaluation, rounds 1 to
11), `docs/mcp.md` (the agent memory and its tools),
`docs/deletion-design.md`.

## Sprints

| Id | Name | Status |
|---|---|---|
| 1 | Green build and CI | Done |
| 2 | One graph per store | Done |
| 3 | Extraction into HelixDB | Done |
| 4 | Non-destructive, time-aware graph | Done |
| 5 | Evaluation harness | Done |
| 5b | Full evaluation and read path | Done, including the full-scale runs |
| 5c | A narrower job for the graph | Done: the graph did not pass; reported as a negative result |
| 6 | Security and MCP | Done |
| 7 | Four-layer memory in the store | Done |
| 8 | Pi as the default agent | Done |
| 9 | Dreaming on a branch (the offline pass) | Done |
| 10 | Evaluate memory in use | Done: 5 of 7 criteria passed; forgetting failed and was fixed |
| 11 | Hardening and release | Done, except the release workflow has never run |
| 12 | Contradicted notes and a sounder forget | Done: 1 of 3 criteria on the first run, 3 of 3 after fixes |
| 13 | Memory at realistic size | Done: 4 of 5 on the first run, 5 of 5 after fixes. Not done: pages plus search against search alone |
| 14 | Deletion and locking in the session store | Done; the cascade from a deleted message is partial |
| 15 | What versioning is worth | Done: as-of reads, merging two copies, round 4 |
| 16 | A second agent, a second model, and the release | Done, except the release itself (below) |
| 17 | A harder, independent test | Done: Pi passes a long history of facts said in passing (96/96), a plain client fails it (20/96); an agent on LongMemEval questions is 20 to 30 points below the harness's read paths |
| 18 | Whom a note is about | Done: wrong answers at 5,000 notes fell from 9 to 3 of 120 (could be chance one time in seven) |
| 19 | What deletion and forgetting do not reach | Done in two of three parts; matching by meaning not done |
| 20 | Pages and the offline pass at size | Done: the pass is bounded at 5,000 notes and corrects 8 of 10 after a fix; pages and notes not shown to help or hurt |
| 21 | The read path in the memory tools | Done: 61.7% to 71.7% on sixty LongMemEval-S questions; ten points still to the harness's best |
| 22 | Search by meaning in the memory tools | **Built and unit-tested; not yet measured.** The measurement is the first task for the Mac |
| 23 | Release and a client we did not write | Planned |

Candidates ahead of 21, by size of the known gap: give `memory_read` the
harness's read path (finding 98); record a plain client's sessions
(finding 97); matching by meaning (finding 103).

## Sprint 22: where it stands

**What was built.** With an embedder, memory search ranks by keyword and
by nearness of meaning and joins the two orders, so a note or a transcript
entry can be found without sharing a word with the question.

- `memst mcp --owner <name> --embedder local serve` (or set
  `MEMST_EMBEDDER=local`; the Pi extension passes it on). `local` is BGE-M3
  on this machine: 2.2 GB, downloaded on first use to `MEMST_MODEL_DIR` or
  `~/.cache/memst/models`. The default is still `none`.
- `memst mcp --owner <name> --embedder local embed` makes every missing
  vector up front. A search makes at most 64 as it goes.
- Vectors are kept in `<store>/memory/<owner>/vectors/<model>/`, one file
  for the notes and one per session, keyed by a hash of the text. Every
  kind of forgetting removes the affected vectors at once. A server run
  without the embedder cannot tell which are stale and drops them all on a
  forget; they are made again.
- A text is listed by meaning only at cosine 0.45 or more (`NEAR_ENOUGH` in
  `memst-mcp/src/service.rs`), a figure chosen for BGE-M3 and not tuned.

**What is verified.** The unit tests, with a stand-in model
(`memst-mcp/tests/vectors.rs`): found without a shared word, vectors
reused, vectors gone on forgetting. And one manual run with the real
model: a 551-turn history embedded in 332 s on four CPU cores, a read in
3.5 s (most of it loading the model).

**What is not.**

- The whole workspace's tests were not run after this change here (the
  machine was busy embedding); only the memory crate's. CI runs them.
- **No measurement of whether it helps.** A 16-question run (round 11) was
  started on the build machine and had not finished when this was written;
  if its results are in `benchmarks/results/sessions-round11-embeddings`,
  they are that run. Sixteen questions can show only a large effect.
- Matching by meaning for forgetting and for stale notes (finding 103) is
  not built. The embedder it waited on is now there.

## Tasks for the Mac

In order. Each is one command; all need the model key in the environment
(`MEMST_LLM_API_KEY`) and `third/longmemeval/longmemeval_s_cleaned.json`
(see `benchmarks/harness/README.md` for the download).

- [ ] **0. Build and test.**
  ```bash
  cargo build --release -p memst-cli
  cargo test --workspace
  (cd integrations/pi && npm ci && MEMST_BIN=$PWD/../../target/release/memst npm test)
  ```
  There were 519 Rust tests before this sprint and it adds two, so about
  521, and 6 Pi tests; the whole set was not run here after the change.
  If anything fails, stop and fix that first.

- [ ] **1. Round 11 in full: does search by meaning help?** The criterion
  and what we expected are already in `benchmarks/sessions/README.md`,
  "Round 11", written before any run; the full run replaces the sixteen
  questions there with all sixty.
  ```bash
  python -m benchmarks.sessions.lme \
      --data third/longmemeval/longmemeval_s_cleaned.json \
      --memst target/release/memst --model deepseek-flash \
      --embedder local --workers 1 \
      --compare benchmarks/results/sessions-round10-read-path/answers.jsonl \
      --out benchmarks/results/sessions-round11-embeddings-full
  ```
  The first run downloads BGE-M3. Each of the sixty histories is embedded
  before its question; on four CPU cores that was five and a half minutes
  a history. `--workers 1` because embedding already uses every core; if
  the Mac's ONNX build uses the GPU, try `--workers 2`.
  Read `REPORT.md` in the output folder. The line to look at is "Against
  the compared run on the same 60 questions": round 10 was 71.7% as scored
  and 81.7% with abstentions read fairly; the harness's best read path is
  91.7%. If the interval on the difference includes zero, the honest
  reading is that it is not shown to help.

- [ ] **2. The same without the wide read's competition for attention**
  (optional). To see retrieval by itself, compare how often the answer's
  turn is among what `memory_read` returns, with and without the embedder.
  There is no script for this yet; it would be a small addition to
  `benchmarks/sessions/lme.py` using each question's `evidence` turn ids.

- [ ] **3. Memory at size with embeddings** (optional, slow). Round 8's
  size test with `MEMST_EMBEDDER=local` in the environment:
  ```bash
  for seed in 1 2 3; do MEMST_EMBEDDER=local python -m benchmarks.sessions.run \
      --round 3 --sizes 5000 --filler-seed $seed --subjects \
      --pi pi --memst target/release/memst --model <provider>/deepseek-flash \
      --workers 1 --timeout 3600 \
      --out benchmarks/results/sessions-round8-size/embeddings-$seed; done
  ```
  5,000 notes are embedded for each scenario run, so this is hours. The
  comparison is `benchmarks/results/sessions-round8-size/subjects-on-every-note-*`
  (3 wrong of 120). Do it only if task 1 shows a gain.

- [ ] **4. Push the results** to `dev`, as with the full-scale harness
  runs, and say which tasks were run. The write-up in
  `benchmarks/sessions/README.md` ("Round 11 results"), the roadmap and
  the review log (finding 108) are then to be filled in from them.

- [ ] **5. The release**, when you want it: see "The release" below.

What to expect to go wrong: the model download needs network access to
Hugging Face; a run interrupted part-way keeps nothing (the runner writes
its answers at the end), so run it where it will not be put to sleep.

## Sprint 16: done, except the release

Updated 2026-11-20, after sprint 16.

- **Round 5** is run and written up (`benchmarks/sessions/README.md`,
  "Round 5"): a plain MCP client with no extension
  (`benchmarks/sessions/mcp_agent.py`, `--client mcp`) and Pi on
  `deepseek-v4-pro` each pass all of round 2's criteria, 70 of 70.
- **The manuscript** covers rounds 1 to 5, the deletion work and your
  full-scale runs. Rebuilt PDF: 15 pages.
- **A client we did not write** (Claude Code over MCP) was not tried; it
  needs an account or key.

### The release (yours to do)

`.github/workflows/release.yml` has never run: it runs from a tag or from
the default branch, and the work is on `dev`. To release:

1. Merge `dev` into the default branch (`main`).
2. Move the "Unreleased" items in `Release Note.md` under a version. They
   were added after 1.1.0 was written up, so either fold them into 1.1.0
   or bump the workspace version in `Cargo.toml` (and `memst-py`,
   `pyproject.toml`) to 1.2.0.
3. Tag and push: `git tag v1.1.0 && git push origin v1.1.0`. The workflow
   builds `memst` for Linux x86-64 and macOS arm64 and attaches the
   archives to a GitHub release. Expect to fix something on its first run.

## Runs that were yours

Done and merged on 20 November (commit `326f9c6`): the read-path pipelines
on all of LoCoMo and the raw pipelines on all 500 LongMemEval-S questions.
Still optional: regrade the full LoCoMo run with the stricter judge
(`rescore.py --judge-model deepseek-v4-pro`).

## How to run things

```bash
# Tests
cargo test --workspace                     # 515 pass; needs Python <= 3.13 for memst-lib
(cd integrations/pi && npm ci && npm test) # 6 pass; needs MEMST_BIN or a built target/debug/memst

# Memory for an agent
memst --store ~/.memst mcp --owner <name> serve        # MCP server over stdio
memst --store ~/.memst mcp --owner <name> notes|review|history|dream|merge ...

# The memory-in-use evaluation (Pi and a model key are needed)
python -m benchmarks.sessions.run --round 2 --pi pi --memst target/release/memst --model <provider/model>
```

The model key is never in the repository. The harness and the offline pass
read `MEMST_LLM_API_URL`, `MEMST_LLM_MODEL` and `MEMST_LLM_API_KEY` from
the environment; Pi reads its own `models.json`.

Build notes: `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0` keeps
the HelixDB test binaries small; `HELIX_TELEMETRY_LEVEL=off` in CI.

## Open items worth knowing

From the code review log, the ones most likely to matter next:

- **107.** An agent with the memory tools is ten points below the
  harness's best read path on sixty LongMemEval-S questions (81.7% against
  91.7%, abstentions read fairly), after sprint 21 closed half the gap.
  Retrieval in the tools is by keyword; the harness's is by embedding. An
  embedding model at read time is the decision that item, and finding 103,
  wait on.
- **97.** A client without the Pi extension records no sessions, so its
  memory holds only what the model chose to note: 4 of 80 facts said in
  passing.
- **100.** Look-alike notes about other teams: 9 wrong of 120 before notes
  had subjects, 3 after. Two of the three are the stale-note scenario.
- **80, 81.** Matching a note to what the user said later, and finding
  what still says a forgotten thing, are by shared words. Statements in
  wholly different words are missed.
- **101.** Deleting a message removes memory items that say they were
  made from it; ones that do not say, and the operations log, are not
  reached.
- **103.** Forgetting and the stale-note match are by shared words. The
  embedder they waited on exists from sprint 22; the matching is not yet
  built on it.
- **102.** The native search index is loaded afresh for every operation,
  so indexing is still linear in its size (12.6 ms a message at 2,000).
  Finding 91, as first written, was wrong; see the log.
- **68, 83.** Memory operations are linear in the number of notes (about
  130 ms a write at 5,000). The limit is 10,000 notes per owner.
- **70.** An agent with a shell can reach the memory files directly.
- **94.** A note does not show that it has earlier versions; with the notes
  alone an agent reports the present text as the past.
- Not done in sprints 10 and 13: pages plus search against search alone.
- The three-way merge in `memst-repo` (for the session store) is still
  connected to nothing; the merge that works is the one for agent memory
  (`memst mcp merge`).

## A caution about the evaluation numbers

Each cell of the memory-in-use tables is ten runs. An unchanged repeat of
one cell gave 9/10 where the first run gave 6/10. Differences of one or two
in ten between conditions are not findings. The scenarios were written by
the people who built the system, and every second run was made on the
scenarios its fixes were written against.
