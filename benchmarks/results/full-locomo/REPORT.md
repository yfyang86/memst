# Evaluation report

Dataset: LoCoMo. 1986 questions (seed 7); histories average about 25,736 tokens.
Reader, judge and extraction model: `deepseek-flash` (reasoning off). Embeddings: `http`.

| Pipeline | F1 (95% interval) | Judged correct | Answer turn in context | Prompt tokens | Retrieval p50 / p95 | Write-path LLM calls |
|---|---|---|---|---|---|---|
| full-context | 63.9 (62.2–65.7) | 75.5% | 99.8% | 29,172 | 0.2 / 0.3 ms | 0.0 |
| raw-bm25 | 54.7 (52.7–56.7) | 63.5% | 68.7% | 1,120 | 0.6 / 0.9 ms | 0.0 |
| raw-dense | 60.6 (58.7–62.4) | 72.2% | 76.5% | 1,067 | 63.1 / 70.2 ms | 0.0 |
| raw-hybrid | 59.9 (57.9–61.7) | 70.1% | 77.0% | 1,125 | 44.5 / 46.9 ms | 0.0 |
| graph-entities | 38.9 (37.1–41.0) | 43.5% | 24.8% | 904 | 24.8 / 35.5 ms | 17.8 |
| graph-facts | 46.3 (44.4–48.4) | 53.0% | 42.0% | 1,168 | 38.3 / 47.6 ms | 17.8 |
| default | 59.7 (57.9–61.6) | 69.2% | 77.3% | 1,475 | 84.0 / 94.3 ms | 17.8 |

F1 by category:

| Pipeline | adversarial | multi-hop | open-domain | single-hop | temporal |
|---|---|---|---|---|---|
| full-context | 91.0 (n=446) | 44.9 (n=282) | 14.2 (n=96) | 69.9 (n=841) | 42.2 (n=321) |
| raw-bm25 | 95.1 (n=446) | 25.2 (n=282) | 8.3 (n=96) | 55.0 (n=841) | 37.7 (n=321) |
| raw-dense | 93.5 (n=446) | 35.9 (n=282) | 13.3 (n=96) | 63.0 (n=841) | 44.5 (n=321) |
| raw-hybrid | 93.3 (n=446) | 34.6 (n=282) | 15.3 (n=96) | 61.5 (n=841) | 44.7 (n=321) |
| graph-entities | 96.0 (n=446) | 17.9 (n=282) | 7.6 (n=96) | 24.1 (n=841) | 26.3 (n=321) |
| graph-facts | 95.7 (n=446) | 27.4 (n=282) | 9.3 (n=96) | 32.4 (n=841) | 41.8 (n=321) |
| default | 92.2 (n=446) | 33.8 (n=282) | 11.9 (n=96) | 59.3 (n=841) | 52.8 (n=321) |

Oracle router over raw-bm25, raw-dense, raw-hybrid, graph-entities, graph-facts, default: F1 73.3, against 60.6 for the best single one. That gap is the most a per-question router could add.

What each stage contributed ("reached the answer" = the stage returned a turn holding the answer, or a fact stated in one):

| Pipeline | Stage | Reached the answer | Facts added | Turns added | p50 |
|---|---|---|---|---|---|
| full-context | all_turns | 99.8% | 0.0 | 601.3 | 0.0 ms |
| raw-bm25 | turns (method=bm25, k=20) | 68.7% | 0.0 | 20.0 | 0.5 ms |
| raw-dense | turns (method=dense, k=20) | 76.5% | 0.0 | 20.0 | 63.0 ms |
| raw-hybrid | turns (method=hybrid, k=20) | 77.0% | 0.0 | 20.0 | 44.5 ms |
| graph-entities | entities (k=8, expand=True) | 31.4% | 27.3 | 0.0 | 24.7 ms |
| graph-entities | fact_sources (max=12) | 24.8% | 0.0 | 8.2 | 0.0 ms |
| graph-facts | facts (method=hybrid, k=30) | 48.1% | 30.0 | 0.0 | 38.3 ms |
| graph-facts | fact_sources (max=12) | 42.0% | 0.0 | 12.0 | 0.0 ms |
| default | facts (method=hybrid, k=30) | 48.1% | 30.0 | 0.0 | 38.2 ms |
| default | fact_sources (max=8) | 37.7% | 0.0 | 8.0 | 0.0 ms |
| default | turns (method=hybrid, k=12) | 44.1% | 0.0 | 10.7 | 44.4 ms |

"Adversarial" and "abstention" questions have no answer in the history; they are scored on whether the pipeline says so.
