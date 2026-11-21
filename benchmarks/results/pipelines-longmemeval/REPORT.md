# Evaluation report

Dataset: LongMemEval-S. 12 questions (seed 7); histories average about 127,303 tokens.
Reader, judge and extraction model: `deepseek-flash` (reasoning off). Embeddings: `local`.

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

What each stage contributed ("reached the answer" = the stage returned a turn holding the answer, or a fact stated in one):

| Pipeline | Stage | Reached the answer | Facts added | Turns added | p50 |
|---|---|---|---|---|---|
| full-context | all_turns | 100.0% | 0.0 | 504.8 | 0.2 ms |
| raw-bm25 | turns (method=bm25, k=20) | 75.0% | 0.0 | 20.0 | 1.4 ms |
| raw-dense | turns (method=dense, k=20) | 100.0% | 0.0 | 20.0 | 32.2 ms |
| raw-hybrid | turns (method=hybrid, k=20) | 75.0% | 0.0 | 20.0 | 28.9 ms |
| graph-entities | entities (k=8, expand=True) | 50.0% | 46.9 | 0.0 | 83.0 ms |
| graph-entities | fact_sources (max=12) | 50.0% | 0.0 | 7.6 | 0.0 ms |
| graph-facts | facts (method=hybrid, k=30) | 50.0% | 30.0 | 0.0 | 127.0 ms |
| graph-facts | fact_sources (max=12) | 50.0% | 0.0 | 12.0 | 0.0 ms |
| default | facts (method=hybrid, k=30) | 50.0% | 30.0 | 0.0 | 117.1 ms |
| default | fact_sources (max=8) | 50.0% | 0.0 | 8.0 | 0.0 ms |
| default | turns (method=hybrid, k=12) | 25.0% | 0.0 | 9.8 | 31.0 ms |

"Adversarial" and "abstention" questions have no answer in the history; they are scored on whether the pipeline says so.
