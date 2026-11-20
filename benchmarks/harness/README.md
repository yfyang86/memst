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
| `rerank-reason` | 60 candidate turns (BM25 and embeddings fused) → a model keeps the 20 most useful; the reader works through the evidence | Turn embeddings |
| `dense-wide-reason` | Top 40 turns by embedding; the reader works through the evidence | Turn embeddings |
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

Pipelines that are being tried out live in
[`experiments/`](experiments/); `experiments/read-path.json` has the wider,
reranked and reasoning variants reported below.

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
| `neighbors` | The turns just before and after each turn selected so far, within the same session | `window` (1): turns each way |
| `rerank` | A model reads the turns selected so far and keeps the most useful, best first. One extra model call per question, counted in the report | `keep` (20); `fill` (true): if the model chooses fewer, the rest follow in their earlier order |

`"reader": {"mode": "reason"}` on a pipeline has the reader work through the
evidence (list the dated statements, count, do date arithmetic) before a
final `Answer:` line; the default, `"direct"`, asks for the answer alone.
It works with any model, with or without built-in reasoning.

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
- **Judged correct**, with an interval: a model grades each answer against
  the reference. By default the judge is the reader's model, which makes it a
  cross-check and not an independent measurement; `--judge-model` (and
  `--judge-api-url`, key in `MEMST_JUDGE_API_KEY`) names another. LongMemEval's
  temporal, knowledge-update and preference questions are graded by the rules
  of its own evaluation script.
- **Answerable questions only**: the same figures without the questions that
  have no answer (LoCoMo's adversarial category, LongMemEval's abstention
  ones). For LoCoMo this is the 1,540-question subset most published figures
  use.
- **Difference from `raw-dense` on the same questions**, with an interval.
  Two pipelines whose separate intervals overlap can still differ reliably;
  this table says whether they do.
- **Answer turn in context**: whether a turn the dataset marks as holding the
  answer was among the turns in the context.
  This separates "the evidence was not retrieved" from "the reader did not use
  it".
- **Prompt tokens** per question, as reported by the API, including the
  prompts of stages that call a model, and **model calls per question**.
- **Retrieval latency**, p50 and p95, excluding the reader call. A rerank
  stage's model call is included, but only when it is actually made: once its
  answer is cached the time is near zero, so read rerank latency from a fresh
  run (about 1.2 s here).
- **Write-path LLM calls** per conversation: what building the memory cost.
- **By category**, judged and F1.
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

## Working from saved answers

Two tools read a result directory and run nothing again:

```bash
python -m benchmarks.harness.rescore benchmarks/results/full-locomo
```

rebuilds `REPORT.md` and `summary.json` from the saved answers, which is how
an older result gets the newer tables. With `--judge-model NAME` (and
`--judge-api-url`, `--out`) it first grades every saved answer again with
that model.

```bash
python -m benchmarks.harness.route_analysis benchmarks/results/full-locomo
```

measures how much of the oracle router's gain a real router would get: it
fits the simplest one (the best pipeline for each kind of question) on four
fifths of the questions and scores it on the rest.

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

## Results (18 November 2026)

Reader and extractor: `deepseek-flash` with reasoning off. BGE-M3 embeddings.

### Full runs

All of LoCoMo (1,986 questions) and 60 LongMemEval-S questions, seven
pipelines. Output and full reports: `benchmarks/results/full-locomo/` and
`benchmarks/results/full-longmemeval/`.

LoCoMo:

| Pipeline | F1 (95% interval) | Judged correct (95% interval) | Answer turn in context | Prompt tokens | Model calls per question | Retrieval p50 / p95 | Write-path LLM calls |
|---|---|---|---|---|---|---|---|
| full-context | 63.9 (62.2–65.7) | 75.5% (73.7–77.4) | 99.8% | 29,172 | 1.0 | 0.2 / 0.3 ms | 0.0 |
| raw-bm25 | 54.7 (52.7–56.7) | 63.5% (61.5–65.7) | 68.7% | 1,120 | 1.0 | 0.6 / 0.9 ms | 0.0 |
| raw-dense | 60.6 (58.7–62.4) | 72.2% (70.3–74.1) | 76.5% | 1,067 | 1.0 | 63.1 / 70.2 ms | 0.0 |
| raw-hybrid | 59.9 (57.9–61.7) | 70.1% (68.0–72.0) | 77.0% | 1,125 | 1.0 | 44.5 / 46.9 ms | 0.0 |
| graph-entities | 38.9 (37.1–41.0) | 43.5% (41.3–45.7) | 24.8% | 904 | 1.0 | 24.8 / 35.5 ms | 17.8 |
| graph-facts | 46.3 (44.4–48.4) | 53.0% (50.8–55.2) | 42.0% | 1,168 | 1.0 | 38.3 / 47.6 ms | 17.8 |
| default | 59.7 (57.9–61.6) | 69.2% (67.2–71.2) | 77.3% | 1,475 | 1.0 | 84.0 / 94.3 ms | 17.8 |

| Pipeline | F1 | Judged correct |
|---|---|---|
| full-context | +3.4 (+2.0 to +4.9) | +3.3 (+1.5 to +5.2) |
| raw-bm25 | -5.9 (-7.5 to -4.3) | -8.7 (-10.5 to -6.8) |
| raw-hybrid | -0.7 (-2.0 to +0.6) | -2.1 (-3.6 to -0.5) |
| graph-entities | -21.7 (-23.7 to -19.7) | -28.7 (-31.0 to -26.4) |
| graph-facts | -14.3 (-16.2 to -12.4) | -19.2 (-21.5 to -16.9) |
| default | -0.9 (-2.3 to +0.7) | -3.0 (-4.9 to -0.9) |

LongMemEval-S:

| Pipeline | F1 (95% interval) | Judged correct (95% interval) | Answer turn in context | Prompt tokens | Model calls per question | Retrieval p50 / p95 | Write-path LLM calls |
|---|---|---|---|---|---|---|---|
| full-context | 44.8 (33.5–55.7) | 58.3% (46.7–70.0) | 100.0% | 111,534 | 1.0 | 0.4 / 0.5 ms | 0.0 |
| raw-bm25 | 51.4 (40.2–61.5) | 65.0% (53.3–76.7) | 90.0% | 4,897 | 1.0 | 0.9 / 2.5 ms | 0.0 |
| raw-dense | 49.5 (38.4–59.4) | 65.0% (53.3–76.7) | 100.0% | 4,100 | 1.0 | 63.8 / 77.7 ms | 0.0 |
| raw-hybrid | 50.4 (39.4–60.0) | 68.3% (56.7–80.0) | 95.0% | 4,587 | 1.0 | 54.2 / 65.6 ms | 0.0 |
| graph-entities | 38.2 (28.1–48.5) | 51.7% (40.0–63.3) | 63.3% | 2,615 | 1.0 | 84.3 / 101.0 ms | 46.7 |
| graph-facts | 36.6 (25.9–46.5) | 46.7% (33.3–58.3) | 68.3% | 4,360 | 1.0 | 87.9 / 110.4 ms | 46.7 |
| default | 48.9 (38.3–59.2) | 66.7% (55.0–78.3) | 93.3% | 5,336 | 1.0 | 129.2 / 150.1 ms | 46.7 |

Reading them:

- **The knowledge graph does not beat retrieval over raw turns.** On LoCoMo
  the default pipeline (graph facts, their source messages, raw turns) is 3.0
  points of judged accuracy behind `raw-dense` on the same questions, and the
  interval excludes zero. It costs about 18 extraction calls per conversation
  that `raw-dense` does not need. On LongMemEval-S the two are level.
- **The graph's lead on temporal questions is mostly answer format.** On
  LoCoMo it is 8 points ahead on F1 and 1.5 behind on judged accuracy.
- **Retrieval is not what limits LongMemEval-S.** `raw-dense` has a turn
  holding the answer in its context for all 60 questions and is judged
  correct on 65%. Full context, at 27 times the tokens, does worse (58%).
- **A router by kind of question gains nothing.** The oracle router is 13
  points above the best single pipeline on LoCoMo; a router fitted on
  question kind (`route_analysis`) scores 72.0% against 72.2% for always
  using `raw-dense`. Which pipeline wins a question is not predictable from
  how the question is worded.
- **231 LoCoMo questions (12%) are answered wrong by every pipeline**,
  including full context.

### Is the judge trustworthy?

The answers were graded again by a stronger model, `deepseek-v4-pro`, with
reasoning on.

| Answers | Agreement with the original grades | Effect |
|---|---|---|
| LongMemEval-S full run, 420 | 98.3% | None on the ordering; preference questions lose 3 grades, temporal ones gain 2 |
| LongMemEval-S read-path experiment, 180 | 100% | None |
| LoCoMo read-path experiment, 1,932 | 92.0% | Every pipeline drops 6 to 10 points; 154 answers pass only the original judge, 1 only the stricter one |

So the reader's own model is a lenient judge on LoCoMo. The ordering of
pipelines survives, and differences between them shrink by about a third.
All of LoCoMo has not been re-graded (about 10,000 calls to a reasoning
model); the command is in [RUNNING.md](RUNNING.md). The second judge is from
the same provider, so this is a check on leniency, not a fully independent
judge.

### Read-path experiments

Since the reader, not retrieval, loses most of the points, these vary what
the reader is given and how it answers. LoCoMo pilot (3 conversations, 138
questions); output in `benchmarks/results/read-path-locomo/`, and graded by
the stricter judge in `benchmarks/results/read-path-locomo-pro-judge/`.

| Pipeline | What it does | Judged correct | Stricter judge | Prompt tokens | Model calls |
|---|---|---|---|---|---|
| raw-dense | Top 20 turns by embedding | 66.7% | 60.9% | 1,074 | 1 |
| dense-wide | Top 40 turns | 71.7% | 63.8% | 2,053 | 1 |
| dense+neighbors | Top 12 turns with the turn before and after each | 67.4% | 59.4% | 1,670 | 1 |
| rerank | 60 candidates, a model keeps 20 | 72.5% | 63.0% | 4,468 | 2 |
| rerank-strict | The same, keeping only what the model chose | 65.9% | 56.5% | 3,788 | 2 |
| dense-reason | Top 20 turns; the reader works through the evidence | 72.5% | 63.0% | 1,130 | 1 |
| **dense-wide-reason** | Top 40 turns; reasoning reader | 76.1% | 65.9% | 2,109 | 1 |
| **rerank-reason** | Rerank; reasoning reader | 76.8% | 67.4% | 4,524 | 2 |
| default | Graph facts, their sources, raw turns | 64.5% | 56.5% | 1,439 | 1 |
| default-reason | The same with a reasoning reader | 71.0% | 63.0% | 1,495 | 1 |
| full-context | The whole conversation | 71.0% | 64.5% | 24,698 | 1 |
| full-context-reason | The whole conversation; reasoning reader | 77.5% | 71.0% | 24,754 | 1 |

LongMemEval-S, 18 questions (3 per type), with the reader prompt and judging
rules described above; output in `benchmarks/results/read-path-longmemeval/`.
Both judges give the same grades.

| Pipeline | F1 (95% interval) | Judged correct (95% interval) | Answer turn in context | Prompt tokens | Model calls per question | Retrieval p50 / p95 | Write-path LLM calls |
|---|---|---|---|---|---|---|---|
| full-context | 54.8 (35.8–74.4) | 77.8% (55.6–94.4) | 100.0% | 111,646 | 1.0 | 0.7 / 1.6 ms | 0.0 |
| raw-dense | 57.2 (36.8–78.0) | 66.7% (44.4–88.9) | 100.0% | 4,244 | 1.0 | 112.3 / 185.4 ms | 0.0 |
| raw-hybrid | 57.2 (36.8–78.1) | 72.2% (50.0–88.9) | 94.4% | 4,495 | 1.0 | 47.2 / 76.5 ms | 0.0 |
| dense-wide | 57.4 (36.8–78.2) | 72.2% (50.0–88.9) | 100.0% | 7,701 | 1.0 | 46.1 / 84.2 ms | 0.0 |
| dense-reason | 62.7 (43.3–80.0) | 83.3% (61.1–100.0) | 100.0% | 4,300 | 1.0 | 45.3 / 85.4 ms | 0.0 |
| dense-wide-reason | 62.7 (43.2–80.0) | 83.3% (61.1–100.0) | 100.0% | 7,757 | 1.0 | 45.2 / 79.3 ms | 0.0 |
| rerank | 57.2 (36.8–78.0) | 66.7% (44.4–88.9) | 100.0% | 10,160 | 2.0 | 1219.9 / 2496.5 ms | 0.0 |
| rerank-reason | 68.8 (50.8–85.6) | 88.9% (72.2–100.0) | 100.0% | 10,216 | 2.0 | 47.3 / 77.5 ms | 0.0 |
| full-context-reason | 62.4 (43.0–79.6) | 83.3% (61.1–100.0) | 100.0% | 111,702 | 1.0 | 0.9 / 1.2 ms | 0.0 |

Reading them:

- **Letting the reader work through the evidence is the largest gain**, on
  both datasets and at almost no cost: about 6 points on LoCoMo for the same
  turns (2 by the stricter judge), 17 on LongMemEval-S, where both temporal
  questions go from wrong to right. It costs a few points on LoCoMo's
  unanswerable questions (93% to 87%), where a reader that reasons is more
  willing to guess.
- **More turns help on LoCoMo** (40 instead of 20: 5 points) and do nothing
  on LongMemEval-S, where the answer turn is already retrieved.
- **Reranking alone is not worth its extra call**, and trusting the reranker
  to cut the context hurts. With a reasoning reader it is the best retrieval
  pipeline on both datasets, by a margin too small to separate from
  `dense-wide-reason` on these samples.
- **`dense-wide-reason` and `rerank-reason` match or beat reading the whole
  history directly**: on LoCoMo 76–77% against 71% with 9–18% of the tokens;
  on LongMemEval-S 83–89% against 78% with 7–9%.
- **The graph still does not help.** With the same reasoning reader the
  default pipeline is 5 points behind `dense-wide-reason` on LoCoMo.
- These are small samples. On LoCoMo the gain of `rerank-reason` over
  `raw-dense` is 10.1 points (interval 3.6 to 17.4); by the stricter judge
  6.5 (0.0 to 13.0). On LongMemEval-S it is 22 points (6 to 44) on 18
  questions. The full runs below test whether those gains survive at scale.

### Full-scale read-path confirmation (20 November 2026)

The two promising raw pipelines and their baselines were run on all 1,986
LoCoMo questions. Output: `benchmarks/results/read-path-locomo-full/` (7,944
answers, no reader or judge errors). Judged accuracy uses the original judge;
the stricter judge has not regraded this full run.

| Pipeline | Judged correct | Paired difference from `raw-dense` (95% interval) | Prompt tokens |
|---|---:|---:|---:|
| raw-dense | 72.6% | — | 1,068 |
| default | 70.8% | −1.8 (−3.7 to +0.2) | 1,526 |
| dense-wide-reason | 79.9% | +7.3 (+5.6 to +8.9) | 2,093 |
| rerank-reason | 81.7% | +9.1 (+7.4 to +10.8) | 4,516 |

Both reasoning pipelines beat the 75.5% full-context result from the original
full LoCoMo run on the same questions: by 4.4 points (2.4 to 6.4) for
`dense-wide-reason` and 6.2 (4.3 to 8.1) for `rerank-reason`, at 7% and 15% of
its prompt tokens. Reranking is 1.8 points ahead of wide dense retrieval (0.4
to 3.3). The graph pipeline gives no gain over `raw-dense`.

All 500 LongMemEval-S questions were also run for every raw pipeline. Output:
`benchmarks/results/full-longmemeval-raw/` (2,500 answers, no reader or judge
errors). Full context was deliberately omitted to avoid 500 prompts of about
112,000 tokens each.

| Pipeline | Judged correct | Paired difference from `raw-dense` (95% interval) | Prompt tokens |
|---|---:|---:|---:|
| raw-bm25 | 65.8% | −1.8 (−4.4 to +1.2) | 5,054 |
| raw-dense | 67.6% | — | 4,143 |
| raw-hybrid | 67.2% | −0.4 (−3.0 to +2.4) | 4,818 |
| dense-wide-reason | 88.2% | +20.6 (+16.6 to +24.6) | 8,029 |
| rerank-reason | 88.6% | +21.0 (+17.2 to +25.0) | 10,558 |

The pilot's central result therefore holds at scale: the reasoning read path,
not the graph, is the large improvement. On LongMemEval-S, however, reranking
adds only 0.4 point over `dense-wide-reason` (−1.8 to +2.6), so the cheaper
wide-dense pipeline is sufficient there. Raw dense retrieval already includes
an answer turn for 98.8% of questions while the direct reader answers 67.6%,
which again locates most of the loss in reading rather than retrieval.

Both full runs were resumed after transient provider failures. Accuracy and
prompt-token aggregates include every answer exactly once, but the
`rerank-reason` retrieval timings mix live and cached calls and are not
cold-start latency measurements.

### A narrower job for the graph (roadmap 5c)

The graph is not a better general retriever, so this asks less of it: does
it help on questions about dates, when added to the best raw pipeline and
read by the same reasoning reader? The test was fixed beforehand: a reliable
gain in judged accuracy over `dense-wide-reason` on the same questions.

All 321 temporal questions of LoCoMo, all ten conversations. Output in
`benchmarks/results/graph-role-locomo/`; graded by the stricter judge in
`graph-role-locomo-pro-judge/`. The pipelines are in
`experiments/graph-role.json`.

| Pipeline | What the graph contributes | Judged correct | Difference from `dense-wide-reason` | By the stricter judge |
|---|---|---|---|---|
| dense-wide-reason | nothing (40 raw turns) | 75.4% | | 72.0% |
| rerank-reason | nothing (reranked raw turns) | 77.9% | +2.5 (−0.9 to +6.2) | +1.6 (−1.9 to +5.0) |
| wide+sources-reason | the messages its facts were stated in, added to the 40 turns | 77.9% | +2.5 (−0.6 to +5.9) | +2.5 (−0.6 to +5.6) |
| wide+timeline-reason | its facts as a dated timeline, added to the 40 turns | 76.6% | +1.2 (−2.8 to +5.6) | −1.9 (−6.2 to +2.5) |
| wide+facts-reason | its facts as lines, added to the 40 turns | 73.5% | −1.9 (−6.9 to +2.8) | −4.7 (−9.0 to +0.3) |
| default-reason | facts, their sources and 12 raw turns | 75.4% | 0.0 (−4.7 to +5.0) | −2.8 (−7.8 to +2.2) |
| sources-only-reason | the sources of its facts and 12 raw turns, no fact lines | 74.5% | −0.9 (−5.3 to +2.8) | −1.6 (−5.9 to +2.2) |
| timeline-only-reason | everything: a timeline and its sources, no raw retrieval | 63.5% | −11.8 (−17.1 to −6.2) | −14.9 (−20.2 to −9.3) |

**Verdict on LoCoMo: the test is not passed.** No use of the graph gives a
reliable gain over the best raw pipeline on temporal questions, by either
judge. The best one uses the graph only as an index to source messages and
is 2.5 points ahead with an interval that includes zero, the same as
reranking, which needs no extraction. Showing the reader the extracted facts
does not help, and the graph on its own is 12 points behind.

LongMemEval-S, 22 questions about changed facts and dates (10
knowledge-update, 10 temporal-reasoning, 2 abstention), with graphs
extracted after the date fix. Output in
`benchmarks/results/graph-role-longmemeval/`.

| Pipeline | Judged correct | knowledge-update | temporal-reasoning | Prompt tokens |
|---|---|---|---|---|
| raw-dense (direct reader) | 68.2% | 10 of 10 | 3 of 10 | 4,214 |
| dense-wide-reason | 100.0% | 10 | 10 | 7,608 |
| rerank-reason | 90.9% | 9 | 9 | 10,418 |
| wide+timeline-reason | 95.5% | 9 | 10 | 8,125 |
| wide+facts-reason | 95.5% | 10 | 9 | 8,126 |
| wide+sources-reason | 95.5% | 10 | 9 | 9,727 |
| default-reason | 90.9% | 10 | 8 | 5,437 |
| sources-only-reason | 86.4% | 9 | 8 | 4,918 |
| default (direct reader) | 77.3% | 10 | 5 | 5,381 |

**Verdict on LongMemEval-S: not passed either.** The best raw pipeline
answers all 22 correctly, so the graph has nothing to add, and each graph
variant is level or one to three answers behind. What moves these questions
is the reader: the same 20 turns go from 3 of 10 temporal questions right to
10 of 10 with 40 turns and a reader that works through the dates. Twenty-two
questions is a small sample, and this subset is evidently easy once the
reader reasons; it cannot show a graph advantage, only fail to.

Two things were learnt on the way:

- **Fact dates were wrong, and are fixed.** Ingestion dated a fact with the
  date of its message when the text gave none, and showed that as "true
  since". A trip mentioned on 9 September as "two weeks ago" was dated 9
  September. Now when a fact became true and when it was said are stored
  apart, with the precision of the date. On the same questions this raised
  the default pipeline with a direct reader from 57.3% to 69.5% (interval of
  the gain 6.5 to 17.8), and the timeline no longer hurts (73.2% to 76.6%).
  It did not change the verdict. The run before the fix is in
  `graph-role-locomo-old-dates/`. Of 2,551 facts extracted afterwards, 24%
  carry the date of the event, 39% only the date they were said, and 37% no
  time at all.
- **Extraction cannot simply be made cheaper.** With batches of 6,000, 12,000
  and 24,000 characters the three pilot conversations took 46, 24 and 13
  calls and produced 650, 438 and 321 relationships: the model extracts a
  roughly fixed amount per call. The graph alone drops from 62% to 57% to 49%
  judged; pipelines that also have raw turns do not move
  (`graph-role-batch-size/`).

### Earlier pilots

`benchmarks/results/pipelines-locomo/`, `pipelines-longmemeval/`, `harness/`
and `harness-before-dated-facts/` are the pilots that led here: graph alone
32 F1 with undated facts, 39 once facts cite their message and date, 46 when
searched by relationship. `pipelines-longmemeval/` used an earlier sampler
and reader prompt.

Limits: one reader model with reasoning off, one embedding model, judges from
one provider, extraction that is not deterministic, and pipeline settings chosen
on the 138-question pilot and not tuned further. The main LongMemEval-S graph
comparison uses 60 questions, and the full-scale read-path runs have not been
regraded by the stricter judge. Only the knowledge-graph part of MemSt is
covered.

## Not covered yet

- Full-context and graph pipelines on all 500 LongMemEval-S questions, and the
  other benchmarks in the research notes.
- A judge from a different provider, the official LongMemEval judge, and strict
  regrading of the full-scale read-path runs.
- Sleep-time consolidation, decay and the memory tiers.
