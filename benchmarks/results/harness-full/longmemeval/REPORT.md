# Evaluation report

Dataset: LongMemEval-S. 60 questions (seed 7); histories average about 126,930 tokens.
Reader, judge and extraction model: `deepseek-flash` (reasoning off). Embeddings: `http`.

| Pipeline | F1 (95% interval) | Judged correct | Answer turn in context | Prompt tokens | Retrieval p50 / p95 | Write-path LLM calls |
|---|---|---|---|---|---|---|
| full-context | 44.8 (33.5–55.7) | 58.3% | 100.0% | 111,534 | 0.4 / 0.5 ms | 0.0 |
| raw-bm25 | 51.4 (40.2–61.5) | 65.0% | 90.0% | 4,897 | 0.9 / 2.5 ms | 0.0 |
| raw-dense | 49.5 (38.4–59.4) | 65.0% | 100.0% | 4,100 | 63.8 / 77.7 ms | 0.0 |
| raw-hybrid | 50.4 (39.4–60.0) | 68.3% | 95.0% | 4,587 | 54.2 / 65.6 ms | 0.0 |
| graph-entities | 38.2 (28.1–48.5) | 51.7% | 63.3% | 2,615 | 84.3 / 101.0 ms | 46.7 |
| graph-facts | 36.6 (25.9–46.5) | 46.7% | 68.3% | 4,360 | 87.9 / 110.4 ms | 46.7 |
| default | 48.9 (38.3–59.2) | 66.7% | 93.3% | 5,336 | 129.2 / 150.1 ms | 46.7 |

F1 by category:

| Pipeline | abstention | knowledge-update | multi-session | single-session-assistant | single-session-preference | single-session-user | temporal-reasoning |
|---|---|---|---|---|---|---|---|
| full-context | 100.0 (n=2) | 64.8 (n=10) | 21.0 (n=10) | 71.7 (n=10) | 8.9 (n=10) | 65.6 (n=9) | 26.1 (n=9) |
| raw-bm25 | 100.0 (n=2) | 84.8 (n=10) | 32.9 (n=10) | 67.2 (n=10) | 0.3 (n=10) | 84.8 (n=9) | 30.2 (n=9) |
| raw-dense | 100.0 (n=2) | 80.6 (n=10) | 25.7 (n=10) | 67.2 (n=10) | 1.1 (n=10) | 85.4 (n=9) | 28.5 (n=9) |
| raw-hybrid | 100.0 (n=2) | 80.6 (n=10) | 28.6 (n=10) | 67.2 (n=10) | 1.5 (n=10) | 85.6 (n=9) | 30.2 (n=9) |
| graph-entities | 100.0 (n=2) | 51.7 (n=10) | 20.7 (n=10) | 59.7 (n=10) | 1.1 (n=10) | 71.6 (n=9) | 13.1 (n=9) |
| graph-facts | 100.0 (n=2) | 46.7 (n=10) | 26.7 (n=10) | 30.6 (n=10) | 1.1 (n=10) | 77.1 (n=9) | 27.7 (n=9) |
| default | 100.0 (n=2) | 70.6 (n=10) | 20.7 (n=10) | 66.1 (n=10) | 2.4 (n=10) | 85.6 (n=9) | 40.6 (n=9) |

Oracle router over raw-bm25, raw-dense, raw-hybrid, graph-entities, graph-facts, default: F1 56.8, against 51.4 for the best single one. That gap is the most a per-question router could add.

What each stage contributed ("reached the answer" = the stage returned a turn holding the answer, or a fact stated in one):

| Pipeline | Stage | Reached the answer | Facts added | Turns added | p50 |
|---|---|---|---|---|---|
| full-context | all_turns | 100.0% | 0.0 | 497.1 | 0.0 ms |
| raw-bm25 | turns (method=bm25, k=20) | 90.0% | 0.0 | 20.0 | 0.9 ms |
| raw-dense | turns (method=dense, k=20) | 100.0% | 0.0 | 20.0 | 63.7 ms |
| raw-hybrid | turns (method=hybrid, k=20) | 95.0% | 0.0 | 20.0 | 54.2 ms |
| graph-entities | entities (k=8, expand=True) | 68.3% | 45.2 | 0.0 | 84.2 ms |
| graph-entities | fact_sources (max=12) | 63.3% | 0.0 | 7.1 | 0.0 ms |
| graph-facts | facts (method=hybrid, k=30) | 70.0% | 30.0 | 0.0 | 87.9 ms |
| graph-facts | fact_sources (max=12) | 68.3% | 0.0 | 11.9 | 0.0 ms |
| default | facts (method=hybrid, k=30) | 70.0% | 30.0 | 0.0 | 95.5 ms |
| default | fact_sources (max=8) | 65.0% | 0.0 | 8.0 | 0.0 ms |
| default | turns (method=hybrid, k=12) | 56.7% | 0.0 | 9.7 | 34.1 ms |

"Adversarial" and "abstention" questions have no answer in the history; they are scored on whether the pipeline says so.
