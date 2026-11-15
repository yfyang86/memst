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
| `memst-graph` | Entities found by MemSt's graph search, and the relationships around them | LLM extraction into the knowledge graph |
| `memst-graph+raw` | The graph's facts, plus the top 12 raw turns | Both |

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
| full-context | 59.2 (52.3–66.0) | 71.0% | 100.0% | 24,698 | 0.2 / 0.5 ms | 0.0 |
| raw-bm25 | 45.5 (38.2–52.6) | 54.4% | 61.8% | 1,132 | 0.7 / 1.6 ms | 0.0 |
| raw-dense | 53.3 (46.1–60.0) | 66.7% | 77.2% | 1,074 | 205.4 / 415.2 ms | 0.0 |
| raw-hybrid | 51.3 (44.5–58.3) | 61.6% | 69.8% | 1,116 | 185.8 / 429.3 ms | 0.0 |
| memst-graph | 32.3 (25.2–39.6) | 41.3% | n/a | 588 | 39.6 / 232.8 ms | 15.3 |
| memst-graph+raw | 47.0 (40.0–53.8) | 56.5% | 65.4% | 1,217 | 77.7 / 399.6 ms | 15.3 |

F1 by category:

| System | adversarial | multi-hop | open-domain | single-hop | temporal |
|---|---|---|---|---|---|
| full-context | 90.0 (n=30) | 53.4 (n=30) | 13.6 (n=18) | 68.6 (n=30) | 52.2 (n=30) |
| raw-bm25 | 90.0 (n=30) | 28.0 (n=30) | 2.2 (n=18) | 49.7 (n=30) | 40.4 (n=30) |
| raw-dense | 93.3 (n=30) | 34.7 (n=30) | 16.0 (n=18) | 54.5 (n=30) | 52.8 (n=30) |
| raw-hybrid | 90.0 (n=30) | 31.4 (n=30) | 14.8 (n=18) | 50.2 (n=30) | 55.5 (n=30) |
| memst-graph | 90.0 (n=30) | 25.1 (n=30) | 0.0 (n=18) | 23.7 (n=30) | 9.7 (n=30) |
| memst-graph+raw | 86.7 (n=30) | 35.6 (n=30) | 9.4 (n=18) | 51.2 (n=30) | 37.3 (n=30) |

Oracle router over raw-bm25, raw-dense, raw-hybrid, memst-graph, memst-graph+raw: F1 63.8, against 53.3 for the best single one. That gap is the most a per-question router could add.

Reading it:

- **The knowledge graph on its own is the weakest system here** (32 F1), well
  below retrieval over the raw turns (53 with embeddings) and below reading
  the whole conversation (59). The intervals do not overlap.
- **Adding raw turns to the graph's facts recovers most of the loss** but does
  not beat raw retrieval alone: with these settings the graph's facts add
  nothing measurable on top of the evidence they were extracted from.
- **Temporal questions are where the graph fails outright** (10 F1). Its facts
  carry no dates: "Caroline joined a mentorship program last weekend" is
  stored without the date of the message, so "last weekend" cannot be
  resolved. Relationships now have a `valid_from` field; ingestion does not
  fill it.
- **Extraction is the only system with a write-path cost**: about 15 LLM calls
  per conversation, for a result the raw baselines beat with none.
- **A per-question router has about 10 points of headroom** over the best
  single retrieval system on this sample. That is an upper bound, and it would
  still only bring retrieval level with full context.
- **Full context is the strongest**, at about 20 times the prompt tokens.
  These conversations fit comfortably in the reader's context, which is the
  known weakness of LoCoMo as a test of memory.

Limits of this pilot: three conversations, one model, reasoning off, a
stratified sample (so the overall figures are not comparable with published
LoCoMo scores), one judging pass by the same model, and only the knowledge
graph part of MemSt. It is enough to say the graph does not yet earn its
cost on this task; it is not a final measurement.

## Not covered yet

- LongMemEval and the other benchmarks in the research notes; LoCoMo alone is
  known to be a weak test (see `docs/research/memst-and-the-field.md`).
- Repeated judging, and a judge independent of the reader.
- Sleep-time consolidation, decay and the memory tiers: only the knowledge
  graph path of MemSt is measured here.
