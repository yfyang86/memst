# Evaluation report

Dataset: LoCoMo. 138 questions from 3 conversation(s), 10 per category per conversation (seed 7).
Reader, judge and extraction model: `deepseek-flash` (reasoning off). Embeddings: `local`.

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

"Adversarial" questions have no answer in the conversation; they are scored on whether the system says so.
