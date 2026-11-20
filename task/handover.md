# Handover

Written 2026-11-20, at the start of sprint 16. Everything up to and
including sprint 15 is merged into `dev` (pull requests 1 to 6; the last
merge commit is `16d4392`). The working branch
`claude/loving-allen-qbfcg1` is at the same commit plus this file.

Where to read more: `docs/ROADMAP.md` (what each sprint did, in detail),
`CodeReview/CR-20261114.md` (findings 1 to 94, each marked fixed or open),
`benchmarks/sessions/README.md` (the memory-in-use evaluation, rounds 1 to
4), `docs/mcp.md` (the agent memory and its tools),
`docs/deletion-design.md`.

## Sprints

| Id | Name | Status |
|---|---|---|
| 1 | Green build and CI | Done |
| 2 | One graph per store | Done |
| 3 | Extraction into HelixDB | Done |
| 4 | Non-destructive, time-aware graph | Done |
| 5 | Evaluation harness | Done |
| 5b | Full evaluation and read path | Done; the full-scale read-path and 500-question runs are yours to do (see below) |
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
| 16 | A second agent, a second model, and the release | **Started, nothing committed.** Plan below |

## Sprint 16: what is left to do

Nothing of sprint 16 is in the repository. Two things were checked and
both work, so the plan below is not blocked:

- The MCP server sends its `instructions` in the `initialize` reply, so a
  plain MCP client has something to put in its prompt.
- `deepseek-v4-pro` accepts tool calls through the same endpoint as
  `deepseek-flash` (it returns `reasoning_content` beside `tool_calls`).

### 16a. Round 5: a second model and a second client

Every result so far is one agent (Pi with the MemSt extension) and one
model (`deepseek-flash`). Round 5 should repeat round 2's seven kinds of
scenario (`build_round2()` in `benchmarks/sessions/scenarios.py`: recall,
update, stale, forget, forget2, poison, abstain; 70 runs) twice:

1. **Second model, same client:** Pi with `deepseek-v4-pro`. Add the model
   to Pi's `models.json` (probably with `"reasoning": true`) and pass
   `--model <provider>/deepseek-v4-pro` to the runner. Check first that Pi
   handles the model's `reasoning_content` in a tool-calling loop.
2. **Second client, same model:** a plain MCP client with no extension. It
   needs writing: a small loop (about 100 lines of Python) that starts
   `memst mcp --owner <name> serve`, sends `initialize` and `tools/list`,
   gives the model the server's instructions and the tools, and runs
   tool calls until the model answers. Add it to
   `benchmarks/sessions/run.py` as a `--client mcp` option beside Pi.
   What differs from Pi, and is the point of the test: no index in the
   prompt at the start, and nothing records the session unless the model
   calls `transcript_append` itself. Expect `stale` and the two `forget`
   kinds to behave differently for that reason.

Do it the way rounds 1 to 4 were done: write the criteria in
`benchmarks/sessions/README.md` and commit them before the first run; use
round 2's thresholds (R1 to R3) for each of the two; report the first run
as the outcome and any re-run after a fix separately. If a real second
agent is wanted (Claude Code over MCP was the suggestion), that needs an
account or key you supply; it was not available here.

### 16b. The release (yours to do)

`.github/workflows/release.yml` has never run: it runs from a tag or from
the default branch, and the work is on `dev`. To release 1.1.0:

1. Merge `dev` into the default branch (`main`).
2. Move the "Unreleased" items in `Release Note.md` under a version. They
   were added after 1.1.0 was written up, so either fold them into 1.1.0
   or bump the workspace version in `Cargo.toml` (and `memst-py`,
   `pyproject.toml`) to 1.2.0.
3. Tag and push: `git tag v1.1.0 && git push origin v1.1.0`. The workflow
   builds `memst` for Linux x86-64 and macOS arm64 and attaches the
   archives to a GitHub release. Expect to fix something on its first run.

### 16c. The manuscript

`manuscript/memst/memst-yifan.tex` has a section "Memory in Use" that
covers round 1 only (written in sprint 11). It needs:

- rounds 2 to 4: the stale-note result, memory at 500 and 5,000 notes, and
  the as-of comparison (20/20 against 19/20 with transcripts, at a quarter
  of the prompt tokens);
- the deletion paragraph in "Security and Deletion" brought up to date
  (single messages, the graph cascade, the search index that kept deleted
  text);
- "Limitations": "Not implemented" still lists three-way merge and
  deletion of single messages, both now built for the parts described
  above;
- round 5, once run;
- your full-scale numbers (next section), once run.

Tables are generated by `manuscript/memst/make_tables.py` from
`benchmarks/results`; the "Memory in Use" table is written by hand in the
`.tex`. Rebuild with `pdflatex`, `bibtex`, `pdflatex` twice.

## Runs that are yours (they need more compute than this environment)

No new results have been pushed since 16 November. Still to run on the
Mac, with `benchmarks/harness/RUNNING.md` and `run_all.sh`:

1. The read-path pipelines (`experiments/read-path.json`) on all of
   LoCoMo. The +9 to +10 point gain is from a pilot.
2. The 500-question LongMemEval-S run for the raw pipelines.
3. Optional: regrade the full LoCoMo run with the stricter judge
   (`rescore.py --judge-model deepseek-v4-pro`).

Then `python manuscript/memst/make_tables.py` and rebuild the PDF.

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

- **86.** Among many notes of the same shape about other people or teams,
  the agent sometimes gives another's value, once for a fact it was never
  told. Notes do not record whom they are about in a form search can use.
- **80, 81.** Matching a note to what the user said later, and finding
  what still says a forgotten thing, are by shared words. Statements in
  wholly different words are missed.
- **87.** Deleting a message does not reach memory items and summaries
  made from it, or the operations log.
- **91.** With the native search backend alone, each message rewrites the
  whole index.
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
