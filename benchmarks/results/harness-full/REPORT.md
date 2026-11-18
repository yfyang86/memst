# Full benchmark report

Both evaluations are complete: 2,046 questions and 14,322 pipeline answers. The reader, judge, and extraction model was `deepseek-flash` with reasoning off. Embeddings used local BGE-M3 over HTTP (1,024 dimensions).

## LoCoMo

1,986 questions; 13,902 pipeline answers. Histories average about 25,736 tokens.

| Pipeline | F1 (95% interval) | Judged correct | Answer turn in context | Prompt tokens | Retrieval p50 / p95 | Write-path LLM calls |
|---|---|---|---|---|---|---|
| full-context | 63.9 (62.2–65.7) | 75.5% | 99.8% | 29,172 | 0.2 / 0.3 ms | 0.0 |
| raw-bm25 | 54.7 (52.7–56.7) | 63.5% | 68.7% | 1,120 | 0.6 / 0.9 ms | 0.0 |
| raw-dense | 60.6 (58.7–62.4) | 72.2% | 76.5% | 1,067 | 63.1 / 70.2 ms | 0.0 |
| raw-hybrid | 59.9 (57.9–61.7) | 70.1% | 77.0% | 1,125 | 44.5 / 46.9 ms | 0.0 |
| graph-entities | 38.9 (37.1–41.0) | 43.5% | 24.8% | 904 | 24.8 / 35.5 ms | 17.8 |
| graph-facts | 46.3 (44.4–48.4) | 53.0% | 42.0% | 1,168 | 38.3 / 47.6 ms | 17.8 |
| default | 59.7 (57.9–61.6) | 69.2% | 77.3% | 1,475 | 84.0 / 94.3 ms | 17.8 |

Oracle router: F1 73.3, against 60.6 for the best single retrieval pipeline.

Category results, stage details, answers, and traces are in [`../full-locomo/`](../full-locomo/).

## LongMemEval-S

60 questions; 420 pipeline answers. Histories average about 126,930 tokens.

| Pipeline | F1 (95% interval) | Judged correct | Answer turn in context | Prompt tokens | Retrieval p50 / p95 | Write-path LLM calls |
|---|---|---|---|---|---|---|
| full-context | 44.8 (33.5–55.7) | 58.3% | 100.0% | 111,534 | 0.4 / 0.5 ms | 0.0 |
| raw-bm25 | 51.4 (40.2–61.5) | 65.0% | 90.0% | 4,897 | 0.9 / 2.5 ms | 0.0 |
| raw-dense | 49.5 (38.4–59.4) | 65.0% | 100.0% | 4,100 | 63.8 / 77.7 ms | 0.0 |
| raw-hybrid | 50.4 (39.4–60.0) | 68.3% | 95.0% | 4,587 | 54.2 / 65.6 ms | 0.0 |
| graph-entities | 38.2 (28.1–48.5) | 51.7% | 63.3% | 2,615 | 84.3 / 101.0 ms | 46.7 |
| graph-facts | 36.6 (25.9–46.5) | 46.7% | 68.3% | 4,360 | 87.9 / 110.4 ms | 46.7 |
| default | 48.9 (38.3–59.2) | 66.7% | 93.3% | 5,336 | 129.2 / 150.1 ms | 46.7 |

Oracle router: F1 56.8, against 51.4 for the best single retrieval pipeline.

Category results, stage details, answers, and traces are in [`../full-longmemeval/`](../full-longmemeval/).

## Published SOTA context

There is no maintained, unified leaderboard for either dataset. The official repositories provide data and evaluation scripts, not a current cross-system ranking: [LoCoMo](https://github.com/snap-research/locomo), [LongMemEval](https://github.com/xiaowu0162/longmemeval). Published leaders report about 96–97% LLM-judged accuracy, but question subsets, readers, judges, retrieval budgets, and oracle access differ.

### LoCoMo

- **ByteRover: 96.1%**, the strongest documented QA claim found in this search. It evaluates 1,982 questions across four categories using Gemini 3 Flash as judge and its own unified harness. This is not an independent reproduction. [Paper, Table 3](https://arxiv.org/html/2604.01599v1).
- **ContextStream: 95.7%** (1,474/1,540), with **95.8% on a preregistered held-out half**. Categories 1–4, reader, judge, confidence intervals, and methodology are disclosed. [Benchmark methodology](https://contextstream.io/benchmarks).
- ZeroMemory also claims 96.1%, but its public post lacks enough methodology to rank it confidently.
- FluctlightDB's 97.0% is **evidence recall at k=150**, not end-to-end QA; its LoCoMo QA result is approximately 85%. [Paper](https://arxiv.org/html/2608.12365v2).

### LongMemEval-S

- **FluctlightDB: 97.4%** (487/500), the highest number found. Its custom profile uses gold-session reader context on hard question types and reports 100% session recall in that profile, so it is not comparable to ordinary full-history evaluation. [Paper, Section 4.5](https://arxiv.org/html/2608.12365v2).
- **Synap: 97.0%** (485/500) using Gemini 4 Argon with a GPT-4o judge. This is a vendor-reported result: the published per-question artifacts currently document the **95.6% Gemini 3.8 Flash run**, not the Argon run. [Results and methodology](https://github.com/maximem-ai/eval_benchmark_runs_output).
- **ContextStream: 96.8%** (484/500) using the official pinned GPT-4o judge, with **96.0% on a preregistered held-out half**. This is the strongest transparent comparator found in this search. [Benchmark methodology](https://contextstream.io/benchmarks).
- **Exabase: 96.4% at top-50**, using Gemini 3 Flash and one uniform prompt without category-specific tuning. [Methodology](https://exabase.io/research/exabase-achieves-state-of-the-art-on-longmemeval-benchmark).

### Interpretation for this harness

Our results are not directly comparable with these claims:

- LoCoMo includes all **1,986 questions**, including 446 adversarial questions. Many published results use 1,540 Category 1–4 questions; ByteRover reports 1,982.
- LongMemEval-S is a stratified **60-question sample**, not the complete 500-question suite.
- The reader and judge are `deepseek-flash`, rather than the stronger readers or official GPT-4o judge used by many leaders.
- Token F1 is not interchangeable with LLM-judged answer accuracy.

We therefore do **not** claim SOTA. The default graph pipeline is below the best raw retrieval pipeline on LoCoMo (3.0 points of judged accuracy behind `raw-dense` on the same questions, interval 0.9 to 4.9) and level with it on LongMemEval-S.

Temporal questions are the one category where it leads on F1: LoCoMo **52.8** for `default` against **44.5** for `raw-dense`, and LongMemEval-S **40.6** against **30.2** for `raw-hybrid`. On LoCoMo the judge does not confirm this (58.6% against 60.1% judged correct), so that lead is mostly answer format: the pipeline shows the reader dates written the way the references are. On LongMemEval-S the judge agrees in direction (56% against 44%), on 9 questions.

A meaningful follow-up comparison should run all 500 LongMemEval-S questions and LoCoMo Categories 1–4 with the official judge and identical prompts, while also retaining the current full-suite results. Evaluation-target choices can themselves reverse conclusions; see [TIAP's ACL 2026 analysis](https://aclanthology.org/2026.findings-emnlp.1305/).

## Result files

- `result.json`: both complete summaries, configuration, source dataset hashes, and artifact paths.
The per-dataset files are in `../full-locomo/` and `../full-longmemeval/` (they were duplicated here before):

- `REPORT.md`: the full report, rebuilt on 18 November 2026 with answerable-only figures, paired differences and judged accuracy by category.
- `answers.jsonl`: every answer and score.
- `traces.jsonl`: retrieval traces.

Judged correctness is a same-model cross-check, not an independent evaluation. LongMemEval preference scores should be read primarily through the judged column.
