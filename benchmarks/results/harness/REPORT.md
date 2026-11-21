# Evaluation report

Dataset: LoCoMo. 138 questions from 3 conversation(s), 10 per category per conversation (seed 7).
Reader, judge and extraction model: `deepseek-flash` (reasoning off). Embeddings: `local`.

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

"Adversarial" questions have no answer in the conversation; they are scored on whether the system says so.
