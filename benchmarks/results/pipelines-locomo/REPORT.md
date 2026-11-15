# Evaluation report

Dataset: LoCoMo. 138 questions (seed 7); histories average about 22,179 tokens.
Reader, judge and extraction model: `deepseek-flash` (reasoning off). Embeddings: `local`.

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

What each stage contributed ("reached the answer" = the stage returned a turn holding the answer, or a fact stated in one):

| Pipeline | Stage | Reached the answer | Facts added | Turns added | p50 |
|---|---|---|---|---|---|
| full-context | all_turns | 100.0% | 0.0 | 489.4 | 0.0 ms |
| raw-bm25 | turns (method=bm25, k=20) | 61.8% | 0.0 | 20.0 | 0.4 ms |
| raw-dense | turns (method=dense, k=20) | 77.2% | 0.0 | 20.0 | 20.5 ms |
| raw-hybrid | turns (method=hybrid, k=20) | 69.8% | 0.0 | 20.0 | 21.5 ms |
| graph-entities | entities (k=8, expand=True) | 31.6% | 23.8 | 0.0 | 36.7 ms |
| graph-entities | fact_sources (max=12) | 27.9% | 0.0 | 7.6 | 0.0 ms |
| graph-facts | facts (method=hybrid, k=30) | 53.7% | 30.0 | 0.0 | 50.5 ms |
| graph-facts | fact_sources (max=12) | 48.5% | 0.0 | 12.0 | 0.0 ms |
| default | facts (method=hybrid, k=30) | 53.7% | 30.0 | 0.0 | 53.7 ms |
| default | fact_sources (max=8) | 44.1% | 0.0 | 8.0 | 0.0 ms |
| default | turns (method=hybrid, k=12) | 33.1% | 0.0 | 10.7 | 23.5 ms |

"Adversarial" and "abstention" questions have no answer in the history; they are scored on whether the pipeline says so.
