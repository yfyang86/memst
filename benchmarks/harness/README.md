# Evaluation harness

Compares memory systems on long-conversation question answering, with every
system read by the same model under the same prompt. It talks to the engine
through the `memst` Python module; no server is needed.

```bash
# from the repository root, with the memst extension installed (see memst-py)
python -m benchmarks.harness.run --conversations 3 --per-category 10
```

Results go to `benchmarks/results/harness/`: `REPORT.md` (the tables),
`summary.json`, and `answers.jsonl` (every question, context size, answer and
score, for inspection).

## What is compared

| System | Context given to the reader | Memory built beforehand |
|---|---|---|
| `full-context` | The whole conversation | None |
| `raw-bm25` | Top 20 turns by BM25 | None |
| `raw-dense` | Top 20 turns by embedding similarity | Turn embeddings |
| `raw-hybrid` | Top 20 turns, BM25 and embeddings fused | Turn embeddings |
| `memst-graph` | Entities found by MemSt's graph search, the dated relationships around them, and up to 12 of the messages those facts were stated in | LLM extraction into the knowledge graph |
| `memst-graph+raw` | The graph's facts, the top 12 raw turns, and up to 6 messages the facts point at | Both |

`full-context` and the `raw-*` systems are the baselines the field now
expects: a memory system has to beat retrieval over the untouched history, and
has to justify itself against simply reading everything.

## What is reported

- **F1** against the reference answer, with a 95% bootstrap interval.
- **Judged correct**: a second model call grading each answer. It is one pass
  with the same model as the reader, so treat it as a cross-check on F1, not
  as an independent measurement.
- **Answer turn in context**: for systems that return conversation turns,
  whether a turn the dataset marks as holding the answer was in the context.
  This separates "the evidence was not retrieved" from "the reader did not use
  it".
- **Prompt tokens** per question, as reported by the API.
- **Retrieval latency**, p50 and p95, excluding the reader call.
- **Write-path LLM calls** per conversation: what building the memory cost.
- **Oracle router**: the score if, for each question, the best of the
  retrieval systems had been picked. The gap to the best single system is the
  most a per-question router could gain.

Questions LoCoMo calls adversarial have no answer in the conversation. They
are scored on whether the system says so, instead of being dropped.

Category names follow the LoCoMo paper: 1 multi-hop, 2 temporal, 3 open-domain,
4 single-hop, 5 adversarial. The older `benchmarks/locomo_loader.py` labels
them differently, so category columns in the older results are not comparable
with these.

## Choices that affect the numbers

- **Reasoning is off by default** (`--thinking off`) for the reader, the judge
  and extraction, so that every call is cheap and every system has the same
  reader. With `--thinking on`, budgets are raised to leave room for it.
- **Sampling.** `--per-category N` takes N questions of each category from
  each conversation (seeded), which over-represents rare categories relative
  to the dataset. Use `--per-category 0 --conversations 0` for everything.
- **Caching.** Model calls and embeddings are cached under `--work`, and each
  conversation's graph is built once and reused. Delete the work directory to
  measure from scratch. Latency of cached reader calls is the recorded one.
- **Token estimates** for the write path are characters divided by four; the
  read path uses the API's own counts.

## Data

LoCoMo (`third/locomo/data/locomo10.json`, not committed):

```bash
mkdir -p third/locomo/data
curl -L -o third/locomo/data/locomo10.json https://raw.githubusercontent.com/snap-research/locomo/main/data/locomo10.json
```

## Model access

`--api-url` and `--model` name any OpenAI-compatible endpoint. The key is read
from `MEMST_EVAL_API_KEY`. Embeddings use the local BGE-M3 model by default
(`--embedder local`), so no embedding API is needed.

## Pilot result (15 November 2026)

3 LoCoMo conversations, 138 questions (up to 10 per category from each),
`deepseek-flash` with reasoning off, local BGE-M3 embeddings. Full output in
`benchmarks/results/harness/`.

| System | F1 (95% interval) | Judged correct | Answer turn in context | Prompt tokens | Retrieval p50 / p95 | Write-path LLM calls |
|---|---|---|---|---|---|---|
| full-context | 59.2 (52.3–66.0) | 71.0% | 100.0% | 24,698 | 0.2 / 0.4 ms | 0.0 |
| raw-bm25 | 45.5 (38.2–52.6) | 54.4% | 61.8% | 1,132 | 0.7 / 1.8 ms | 0.0 |
| raw-dense | 53.3 (46.1–60.0) | 66.7% | 77.2% | 1,074 | 180.5 / 446.4 ms | 0.0 |
| raw-hybrid | 51.3 (44.5–58.3) | 61.6% | 69.8% | 1,116 | 193.6 / 388.0 ms | 0.0 |
| memst-graph | 39.0 (32.0–46.2) | 44.9% | 27.9% | 946 | 37.2 / 230.0 ms | 15.3 |
| memst-graph+raw | 51.7 (45.0–58.5) | 60.9% | 68.4% | 1,423 | 80.5 / 597.0 ms | 15.3 |

F1 by category:

| System | adversarial | multi-hop | open-domain | single-hop | temporal |
|---|---|---|---|---|---|
| full-context | 90.0 (n=30) | 53.4 (n=30) | 13.6 (n=18) | 68.6 (n=30) | 52.2 (n=30) |
| raw-bm25 | 90.0 (n=30) | 28.0 (n=30) | 2.2 (n=18) | 49.7 (n=30) | 40.4 (n=30) |
| raw-dense | 93.3 (n=30) | 34.7 (n=30) | 16.0 (n=18) | 54.5 (n=30) | 52.8 (n=30) |
| raw-hybrid | 90.0 (n=30) | 31.4 (n=30) | 14.8 (n=18) | 50.2 (n=30) | 55.5 (n=30) |
| memst-graph | 90.0 (n=30) | 29.3 (n=30) | 7.8 (n=18) | 25.7 (n=30) | 29.6 (n=30) |
| memst-graph+raw | 93.3 (n=30) | 42.5 (n=30) | 11.6 (n=18) | 49.1 (n=30) | 46.1 (n=30) |

Oracle router over raw-bm25, raw-dense, raw-hybrid, memst-graph, memst-graph+raw: F1 65.2, against 53.3 for the best single one. That gap is the most a per-question router could add.

Reading it:

- **The knowledge graph on its own is the weakest system** (39 F1), below
  retrieval over the raw turns (53 with embeddings) and below reading the
  whole conversation (59). Its interval does not overlap theirs.
- **The graph together with raw turns (52) is level with raw retrieval alone
  (53)**; the intervals overlap almost entirely. It is ahead on multi-hop
  questions (43 against 35), which is where a graph should help, and behind on
  temporal ones, but with 30 questions per category neither difference is
  established.
- **The graph rarely leads to the right message.** For only 28% of questions
  was a turn holding the answer among the messages its facts pointed at,
  against 77% for embedding retrieval over the turns. The facts found are
  about the right entities but mostly not the fact the question asks for.
- **Extraction is the only write-path cost**: about 15 LLM calls per
  conversation, for a result the raw baselines match with none.
- **A per-question router has about 12 points of headroom** over the best
  single retrieval system on this sample. That is an upper bound.
- **Full context is the strongest**, at about 20 times the prompt tokens.
  These conversations fit comfortably in the reader's context, which is the
  known weakness of LoCoMo as a test of memory.

### What changed between the two runs

The first run (`benchmarks/results/harness-before-dated-facts/`) used facts
with no date and no link to their source message: graph 32 F1 (temporal 10),
graph plus raw 47. Extraction was then changed to cite, for every
relationship, the message that states it and the date it became true, and the
graph systems were changed to show those dates and messages. That is the run
above: graph 39 (temporal 30), graph plus raw 52. The baselines are identical
in both runs (their answers are cached).

Limits of this pilot: three conversations, one model, reasoning off, a
stratified sample (so the overall figures are not comparable with published
LoCoMo scores), one judging pass by the same model, extraction that is not
deterministic (entity counts differ between runs), and only the knowledge
graph part of MemSt. It is enough to say the graph does not yet earn its
extraction cost on this task; it is not a final measurement.

## Not covered yet

- LongMemEval and the other benchmarks in the research notes; LoCoMo alone is
  known to be a weak test (see `docs/research/memst-and-the-field.md`).
- Repeated judging, and a judge independent of the reader.
- Sleep-time consolidation, decay and the memory tiers: only the knowledge
  graph path of MemSt is measured here.
