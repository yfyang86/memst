# Evaluation harness

Compares ways of retrieving from memory on long-conversation question
answering, with every one read by the same model under the same prompt. It
talks to the engine through the `memst` Python module; no server is needed.

```bash
# from the repository root, with the memst extension installed (see memst-py)
python -m benchmarks.harness.run --conversations 3 --per-category 10      # LoCoMo
python -m benchmarks.harness.run --dataset longmemeval --per-category 2   # LongMemEval-S
python -m benchmarks.harness.run --list-pipelines
```

To run the complete evaluation on your own machine (setup, model access,
embeddings on CPU or a GPU server, cost, resuming), see
[RUNNING.md](RUNNING.md) and `run_all.sh`.

Results go to `--out` (default `benchmarks/results/harness/`): `REPORT.md`
(the tables), `summary.json`, `answers.jsonl` (every question, answer and
score) and `traces.jsonl` (what each stage retrieved for every question).

## Pipelines

Retrieval is a pipeline: a list of stages, each adding facts from the
knowledge graph or turns of the conversation to what the reader sees. The
built-in pipelines are in [`pipelines.json`](pipelines.json):

| Pipeline | Stages | Memory built beforehand |
|---|---|---|
| `default` | Search the graph **by relationship** (30 facts) → the messages those facts were stated in (up to 8) → top 12 raw turns | LLM extraction into the graph; embeddings |
| `graph-facts` | Search the graph by relationship → the messages those facts were stated in | LLM extraction; embeddings |
| `graph-entities` | Search the graph by entity, take the facts around each hit → their source messages | LLM extraction; embeddings |
| `raw-hybrid` | Top 20 turns, BM25 and embeddings fused | Turn embeddings |
| `raw-dense` | Top 20 turns by embedding similarity | Turn embeddings |
| `raw-bm25` | Top 20 turns by BM25 | None |
| `full-context` | The whole conversation | None |

`full-context` and the `raw-*` pipelines are the baselines the field now
expects: a memory system has to beat retrieval over the untouched history, and
has to justify itself against simply reading everything.

### Writing your own

A pipeline is JSON, so a new combination needs no code:

```json
{
  "name": "facts-then-dense",
  "stages": [
    {"stage": "facts", "method": "bm25", "k": 15},
    {"stage": "fact_sources", "max": 6},
    {"stage": "turns", "method": "dense", "k": 16}
  ],
  "context": {"max_facts": 15, "max_turns": 20}
}
```

```bash
python -m benchmarks.harness.run --pipeline-file mine.json --pipelines default,facts-then-dense
```

The file holds one pipeline or a list; a name that matches a built-in one
replaces it. Unknown stages, methods or options are rejected before anything
runs.

| Stage | What it adds | Options |
|---|---|---|
| `facts` | Relationships ("Alice lives in Lyon (since 8 May 2023)") ranked against the question | `method`: `bm25` (the engine's relationship search, BM25 in HelixDB), `dense` (embedding of the fact) or `hybrid` (both, fused; the default); `k` (30) |
| `entities` | Entities found by the engine's entity search and by embedding, fused | `k` (8); `expand` (true): also the facts around each entity |
| `fact_sources` | The turns that the facts selected so far were stated in | `max` (8) |
| `turns` | Raw turns ranked against the question | `method`: `bm25`, `dense` or `hybrid` (default); `k` (20) |
| `all_turns` | Every turn | none |

`context` caps what reaches the reader: `max_facts` fact lines (60) and
`max_turns` turns (no cap). Turns are always shown in conversation order.

### Traces

`traces.jsonl` has one line per question and pipeline:

```json
{"pipeline": "default", "question_id": "conv-26:12", "category": "temporal",
 "question": "When did Caroline go to the LGBTQ support group?",
 "stages": [
   {"stage": "facts", "params": {"method": "hybrid", "k": 30}, "ms": 48.1,
    "facts_added": 30, "turns_added": 0, "reached_evidence": true,
    "top": ["Caroline attended LGBTQ support group (since 7 May 2023)", "..."]},
   {"stage": "fact_sources", "params": {"max": 8}, "ms": 0.0, "facts_added": 0,
    "turns_added": 8, "reached_evidence": true, "top": ["D1:3", "..."]},
   {"stage": "turns", "params": {"method": "hybrid", "k": 12}, "ms": 22.0,
    "facts_added": 0, "turns_added": 11, "reached_evidence": false, "top": ["D1:3", "..."]}],
 "context": {"turns": ["D1:3", "..."], "characters": 5210, "evidence_in_context": true}}
```

`reached_evidence` says whether that stage, on its own, returned a turn the
dataset marks as holding the answer, or a fact stated in such a turn. A stage
reports only what it added, so a later stage that finds turns already present
shows fewer. `REPORT.md` aggregates this per stage, which shows which stage
is doing the work.

## What is reported

- **F1** against the reference answer, with a 95% bootstrap interval.
- **Judged correct**: a second model call grading each answer. It is one pass
  with the same model as the reader, so treat it as a cross-check on F1, not
  as an independent measurement.
- **Answer turn in context**: whether a turn the dataset marks as holding the
  answer was among the turns in the context.
  This separates "the evidence was not retrieved" from "the reader did not use
  it".
- **Prompt tokens** per question, as reported by the API.
- **Retrieval latency**, p50 and p95, excluding the reader call.
- **Write-path LLM calls** per conversation: what building the memory cost.
- **Oracle router**: the score if, for each question, the best of the
  retrieval pipelines had been picked. The gap to the best single one is the
  most a per-question router could gain.

Questions LoCoMo calls adversarial have no answer in the conversation. They
are scored on whether the pipeline says so, instead of being dropped. The
same holds for LongMemEval's abstention questions.

Category names follow the LoCoMo paper: 1 multi-hop, 2 temporal, 3 open-domain,
4 single-hop, 5 adversarial. The older `benchmarks/locomo_loader.py` labels
them differently, so category columns in the older results are not comparable
with these.

## Choices that affect the numbers

- **Reasoning is off by default** (`--thinking off`) for the reader, the judge
  and extraction, so that every call is cheap and every pipeline has the same
  reader. With `--thinking on`, budgets are raised to leave room for it.
- **Sampling.** `--per-category N` takes N questions of each category from
  each conversation (seeded), which over-represents rare categories relative
  to the dataset. Use `--per-category 0 --conversations 0` for everything.
- **Caching.** Model calls and embeddings are cached under `--work`, and each
  conversation's graph is built once and reused. Delete the work directory to
  measure from scratch. Latency of cached reader calls is the recorded one.
- **Embeddings** are computed from the first 2,000 characters of a text; the
  whole text still goes to the reader.
- **Extraction batches** are 6,000 characters of conversation per call for
  LoCoMo and 12,000 for LongMemEval (`--batch-chars`).
- **Token estimates** for the write path are characters divided by four; the
  read path uses the API's own counts.

## Data

Neither dataset is committed.

```bash
mkdir -p third/locomo/data third/longmemeval
curl -L -o third/locomo/data/locomo10.json https://raw.githubusercontent.com/snap-research/locomo/main/data/locomo10.json
curl -L -o third/longmemeval/longmemeval_s_cleaned.json https://huggingface.co/datasets/xiaowu0162/longmemeval-cleaned/resolve/main/longmemeval_s_cleaned.json
```

LongMemEval-S has 500 questions, each with its own history of about 50
sessions (roughly 120,000 tokens). Each history is extracted and embedded
separately, so `--per-category N` takes N questions of each of the six types
before anything is built. Its `single-session-preference` answers are
descriptions of a good answer, not short answers: read the judged column for
those, not F1.

## Model access

`--api-url` and `--model` name any OpenAI-compatible endpoint. The key is read
from `MEMST_EVAL_API_KEY`, for the reader, the judge and extraction, and is
not written to the caches or results. Embeddings use the local BGE-M3 model by default
(`--embedder local`), so no embedding API is needed.

## Results (15 November 2026)

`deepseek-flash` with reasoning off as reader, judge and extractor; local
BGE-M3 embeddings. Both are small samples: read them as a pilot.

### LoCoMo

3 conversations, 138 questions (up to 10 per category from each). Full output
in `benchmarks/results/pipelines-locomo/`.

| Pipeline | F1 (95% interval) | Judged correct | Answer turn in context | Prompt tokens | Retrieval p50 / p95 | Write-path LLM calls |
|---|---|---|---|---|---|---|
| full-context | 59.2 (52.3–66.0) | 71.0% | 100.0% | 24,698 | 0.2 / 0.3 ms | 0.0 |
| raw-bm25 | 45.5 (38.2–52.6) | 54.4% | 61.8% | 1,132 | 0.4 / 0.8 ms | 0.0 |
| raw-dense | 53.3 (46.1–60.0) | 66.7% | 77.2% | 1,074 | 20.6 / 32.7 ms | 0.0 |
| raw-hybrid | 51.3 (44.5–58.3) | 61.6% | 69.8% | 1,116 | 21.5 / 41.8 ms | 0.0 |
| graph-entities | 39.7 (32.6–46.9) | 44.9% | 27.9% | 830 | 36.8 / 62.6 ms | 15.3 |
| graph-facts | 45.7 (38.7–52.7) | 52.9% | 48.5% | 1,132 | 50.7 / 76.8 ms | 15.3 |
| default | 54.2 (47.4–60.7) | 64.5% | 72.1% | 1,439 | 76.4 / 113.2 ms | 15.3 |

F1 by category:

| Pipeline | adversarial | multi-hop | open-domain | single-hop | temporal |
|---|---|---|---|---|---|
| full-context | 90.0 (n=30) | 53.4 (n=30) | 13.6 (n=18) | 68.6 (n=30) | 52.2 (n=30) |
| raw-bm25 | 90.0 (n=30) | 28.0 (n=30) | 2.2 (n=18) | 49.7 (n=30) | 40.4 (n=30) |
| raw-dense | 93.3 (n=30) | 34.7 (n=30) | 16.0 (n=18) | 54.5 (n=30) | 52.8 (n=30) |
| raw-hybrid | 90.0 (n=30) | 31.4 (n=30) | 14.8 (n=18) | 50.2 (n=30) | 55.5 (n=30) |
| graph-entities | 90.0 (n=30) | 28.5 (n=30) | 7.8 (n=18) | 22.8 (n=30) | 36.6 (n=30) |
| graph-facts | 90.0 (n=30) | 42.1 (n=30) | 6.5 (n=18) | 21.8 (n=30) | 52.5 (n=30) |
| default | 93.3 (n=30) | 43.6 (n=30) | 12.2 (n=18) | 44.1 (n=30) | 60.8 (n=30) |

Oracle router over raw-bm25, raw-dense, raw-hybrid, graph-entities, graph-facts, default: F1 67.5, against 54.2 for the best single one. That gap is the most a per-question router could add.

What the stages of the graph pipelines contributed:

| Pipeline | Stage | Reached the answer | Facts added | Turns added | p50 |
|---|---|---|---|---|---|
| graph-entities | entities (k=8, expand=True) | 31.6% | 23.8 | 0.0 | 36.7 ms |
| graph-entities | fact_sources (max=12) | 27.9% | 0.0 | 7.6 | 0.0 ms |
| graph-facts | facts (method=hybrid, k=30) | 53.7% | 30.0 | 0.0 | 50.5 ms |
| graph-facts | fact_sources (max=12) | 48.5% | 0.0 | 12.0 | 0.0 ms |
| default | facts (method=hybrid, k=30) | 53.7% | 30.0 | 0.0 | 53.7 ms |
| default | fact_sources (max=8) | 44.1% | 0.0 | 8.0 | 0.0 ms |
| default | turns (method=hybrid, k=12) | 33.1% | 0.0 | 10.7 | 23.5 ms |

Reading it:

- **Searching the graph by relationship finds the answer far more often than
  searching by entity**: the facts returned lead to a turn holding the answer
  for 54% of questions, against 32%. With the graph alone that is 46 F1
  against 40, and 53% judged correct against 45%.
- **The default pipeline (54 F1) is level with the best raw baseline
  (`raw-dense`, 53)**; the intervals overlap almost entirely, and the judge
  puts it slightly behind (64.5% against 66.7%). It is ahead on multi-hop (44
  against 35) and temporal questions (61 against 53) and behind on single-hop
  ones (44 against 55); with 30 questions per category none of these
  differences is established.
- **The graph alone is still below raw retrieval** (46 against 53), at 15
  extraction calls per conversation that the raw baselines do not need.
- **Full context is the strongest** (59), at 17 times the prompt tokens.
  These conversations fit in the reader's context, which is the known
  weakness of LoCoMo as a test of memory.
- **A per-question router has about 13 points of headroom** over the best
  single pipeline. That is an upper bound.

### LongMemEval-S

12 questions (2 of each type; one of them is an abstention question), each
with its own history of about 127,000 tokens. Full output in
`benchmarks/results/pipelines-longmemeval/`.

| Pipeline | F1 (95% interval) | Judged correct | Answer turn in context | Prompt tokens | Retrieval p50 / p95 | Write-path LLM calls |
|---|---|---|---|---|---|---|
| full-context | 53.4 (27.8–77.8) | 58.3% | 100.0% | 111,886 | 0.9 / 1.2 ms | 0.0 |
| raw-bm25 | 58.3 (33.3–83.3) | 58.3% | 75.0% | 4,299 | 1.5 / 2.8 ms | 0.0 |
| raw-dense | 59.7 (33.3–83.3) | 58.3% | 100.0% | 3,805 | 32.3 / 43.8 ms | 0.0 |
| raw-hybrid | 62.5 (37.5–87.5) | 58.3% | 75.0% | 4,027 | 29.0 / 45.9 ms | 0.0 |
| graph-entities | 32.5 (8.3–57.5) | 25.0% | 50.0% | 2,626 | 83.1 / 139.3 ms | 46.9 |
| graph-facts | 51.4 (26.4–76.4) | 50.0% | 50.0% | 4,136 | 127.1 / 153.8 ms | 46.9 |
| default | 59.7 (34.7–84.7) | 58.3% | 75.0% | 5,017 | 146.5 / 179.8 ms | 46.9 |

F1 by category:

| Pipeline | abstention | knowledge-update | multi-session | single-session-assistant | single-session-preference | single-session-user | temporal-reasoning |
|---|---|---|---|---|---|---|---|
| full-context | 100.0 (n=1) | 100.0 (n=2) | 10.5 (n=2) | 100.0 (n=2) | 9.7 (n=2) | 50.0 (n=2) | 0.0 (n=1) |
| raw-bm25 | 100.0 (n=1) | 100.0 (n=2) | 0.0 (n=2) | 100.0 (n=2) | 0.0 (n=2) | 100.0 (n=2) | 0.0 (n=1) |
| raw-dense | 100.0 (n=1) | 83.3 (n=2) | 25.0 (n=2) | 100.0 (n=2) | 0.0 (n=2) | 100.0 (n=2) | 0.0 (n=1) |
| raw-hybrid | 100.0 (n=1) | 100.0 (n=2) | 25.0 (n=2) | 100.0 (n=2) | 0.0 (n=2) | 100.0 (n=2) | 0.0 (n=1) |
| graph-entities | 100.0 (n=1) | 20.0 (n=2) | 25.0 (n=2) | 0.0 (n=2) | 0.0 (n=2) | 100.0 (n=2) | 0.0 (n=1) |
| graph-facts | 100.0 (n=1) | 100.0 (n=2) | 25.0 (n=2) | 50.0 (n=2) | 0.0 (n=2) | 83.3 (n=2) | 0.0 (n=1) |
| default | 100.0 (n=1) | 100.0 (n=2) | 25.0 (n=2) | 100.0 (n=2) | 0.0 (n=2) | 83.3 (n=2) | 0.0 (n=1) |

Oracle router over raw-bm25, raw-dense, raw-hybrid, graph-entities, graph-facts, default: F1 62.5, against 62.5 for the best single one. That gap is the most a per-question router could add.

Reading it:

- **Twelve questions cannot separate these pipelines.** Every interval spans
  about 50 points, and five of the seven pipelines get the same 7 of 12
  judged correct.
- What can be said: the retrieval pipelines answer **about as well as full
  context with 4% of its prompt tokens** (4–5 thousand against 112 thousand).
  That is the regime memory is for, and LoCoMo cannot show it.
- The ordering among graph pipelines matches LoCoMo: by entity (33) below by
  relationship (51) below the default (60).
- Building the graph costs about 47 extraction calls and 6 minutes per
  history. The raw baselines cost only embeddings.
- The 12 questions were drawn with an earlier sampler; the current one picks
  a different 12 for the same settings (and lets a larger sample extend a
  smaller one).
- A larger run needs time, not code: each further question takes about 6
  minutes of extraction (several run in parallel) and 4 minutes of CPU
  embedding. `--per-category 10` (60 questions) is roughly five hours on a
  4-core machine.

### History of the LoCoMo numbers

| Run | Graph alone, F1 (judged) | Graph with raw turns, F1 (judged) |
|---|---|---|
| Facts with no date or source (`benchmarks/results/harness-before-dated-facts/`) | 32.3 | 47.0 |
| Facts cite their message and date; search by entity (`benchmarks/results/harness/`) | 39.0 (44.9%) | 51.7 (60.9%) |
| Search by relationship; dates written out (`benchmarks/results/pipelines-locomo/`) | 45.7 (52.9%) | 54.2 (64.5%) |

The baselines are the same in all three (their answers are cached). Part of
the last step is scoring, not retrieval: writing dates as "8 May 2023"
instead of "2023-05-08" in fact lines raised the two pipelines by about 6 F1
and 1–2 points of judged accuracy, because the reader copies the format it is
shown and token F1 scores a correct ISO date as wrong.

Limits: small samples, one model, reasoning off, a stratified sample (overall
figures are not comparable with published scores), one judging pass by the
same model, extraction that is not deterministic, pipeline settings (k, caps)
chosen once and not tuned, and only the knowledge-graph part of MemSt.

## Not covered yet

- A LongMemEval run large enough to separate pipelines, and the other
  benchmarks in the research notes.
- Repeated judging, and a judge independent of the reader.
- Reranking and query rewriting stages.
- Sleep-time consolidation, decay and the memory tiers: only the knowledge
  graph path of MemSt is measured here.
