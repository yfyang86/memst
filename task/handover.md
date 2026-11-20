# Handover

Written 2026-11-20 and updated the same day after sprint 16. Sprints 1 to
15 are merged into `dev` (pull requests 1 to 7). Sprint 16 is on the
working branch `claude/loving-allen-qbfcg1` until its pull request is
merged.

Where to read more: `docs/ROADMAP.md` (what each sprint did, in detail),
`CodeReview/CR-20261114.md` (findings 1 to 103, each marked fixed or open),
`benchmarks/sessions/README.md` (the memory-in-use evaluation, rounds 1 to
8), `docs/mcp.md` (the agent memory and its tools),
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
| 20 | Pages and the offline pass at size | Next |
| 21 | Release and a client we did not write | Planned |

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

- **98.** The memory tools read a long history far worse than the
  harness's read paths: 61.7% (71.7% read fairly) against 91.7% on sixty
  LongMemEval-S questions. The read path from sprint 5b is in the
  evaluation harness only. The largest known gap.
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
- **103.** Forgetting and the stale-note match are by shared words;
  matching by meaning needs an embedding model at read time, not chosen.
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
