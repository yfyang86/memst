# Evaluation report

Dataset: LoCoMo. 1986 questions (seed 7); histories average about 25,736 tokens.
Reader and extraction model: `deepseek-flash` (reasoning off). Judge: `deepseek-flash` (the same model as the reader, so a cross-check and not an independent measurement). Embeddings: `http`.

| Pipeline | F1 (95% interval) | Judged correct (95% interval) | Answer turn in context | Prompt tokens | Model calls per question | Retrieval p50 / p95 | Write-path LLM calls |
|---|---|---|---|---|---|---|---|
| full-context | 63.9 (62.2–65.7) | 75.5% (73.7–77.4) | 99.8% | 29,172 | 1.0 | 0.2 / 0.3 ms | 0.0 |
| raw-bm25 | 54.7 (52.7–56.7) | 63.5% (61.5–65.7) | 68.7% | 1,120 | 1.0 | 0.6 / 0.9 ms | 0.0 |
| raw-dense | 60.6 (58.7–62.4) | 72.2% (70.3–74.1) | 76.5% | 1,067 | 1.0 | 63.1 / 70.2 ms | 0.0 |
| raw-hybrid | 59.9 (57.9–61.7) | 70.1% (68.0–72.0) | 77.0% | 1,125 | 1.0 | 44.5 / 46.9 ms | 0.0 |
| graph-entities | 38.9 (37.1–41.0) | 43.5% (41.3–45.7) | 24.8% | 904 | 1.0 | 24.8 / 35.5 ms | 17.8 |
| graph-facts | 46.3 (44.4–48.4) | 53.0% (50.8–55.2) | 42.0% | 1,168 | 1.0 | 38.3 / 47.6 ms | 17.8 |
| default | 59.7 (57.9–61.6) | 69.2% (67.2–71.2) | 77.3% | 1,475 | 1.0 | 84.0 / 94.3 ms | 17.8 |

Answerable questions only (1540; without adversarial). This is the subset most published figures use:

| Pipeline | F1 (95% interval) | Judged correct (95% interval) |
|---|---|---|
| full-context | 56.1 (54.0–58.1) | 71.0% (68.6–73.2) |
| raw-bm25 | 43.0 (40.9–45.1) | 54.4% (51.9–56.8) |
| raw-dense | 51.1 (48.9–53.1) | 66.0% (63.6–68.4) |
| raw-hybrid | 50.2 (48.0–52.3) | 63.4% (60.9–65.9) |
| graph-entities | 22.4 (20.7–24.2) | 28.2% (26.1–30.6) |
| graph-facts | 32.0 (29.9–34.0) | 40.6% (38.1–43.0) |
| default | 50.3 (48.2–52.3) | 62.5% (60.1–65.1) |

Difference from `raw-dense` on the same questions, in points (95% interval). An interval that excludes zero is a real difference:

| Pipeline | F1 | Judged correct |
|---|---|---|
| full-context | +3.4 (+2.0 to +4.9) | +3.3 (+1.5 to +5.2) |
| raw-bm25 | -5.9 (-7.5 to -4.3) | -8.7 (-10.5 to -6.8) |
| raw-hybrid | -0.7 (-2.0 to +0.6) | -2.1 (-3.6 to -0.5) |
| graph-entities | -21.7 (-23.7 to -19.7) | -28.7 (-31.0 to -26.4) |
| graph-facts | -14.3 (-16.2 to -12.4) | -19.2 (-21.5 to -16.9) |
| default | -0.9 (-2.3 to +0.7) | -3.0 (-4.9 to -0.9) |

Judged correct by category:

| Pipeline | adversarial | multi-hop | open-domain | single-hop | temporal |
|---|---|---|---|---|---|
| full-context | 91.0% (n=446) | 61.0% (n=282) | 25.0% (n=96) | 85.6% (n=841) | 55.1% (n=321) |
| raw-bm25 | 95.1% (n=446) | 31.2% (n=282) | 16.7% (n=96) | 68.1% (n=841) | 50.2% (n=321) |
| raw-dense | 93.5% (n=446) | 48.9% (n=282) | 26.0% (n=96) | 78.6% (n=841) | 60.1% (n=321) |
| raw-hybrid | 93.3% (n=446) | 42.9% (n=282) | 27.1% (n=96) | 76.2% (n=841) | 58.6% (n=321) |
| graph-entities | 96.0% (n=446) | 20.2% (n=282) | 15.6% (n=96) | 30.8% (n=841) | 32.4% (n=321) |
| graph-facts | 95.7% (n=446) | 36.9% (n=282) | 18.8% (n=96) | 41.4% (n=841) | 48.3% (n=321) |
| default | 92.2% (n=446) | 44.0% (n=282) | 22.9% (n=96) | 74.8% (n=841) | 58.6% (n=321) |

F1 by category:

| Pipeline | adversarial | multi-hop | open-domain | single-hop | temporal |
|---|---|---|---|---|---|
| full-context | 91.0 | 44.9 | 14.2 | 69.9 | 42.2 |
| raw-bm25 | 95.1 | 25.2 | 8.3 | 55.0 | 37.7 |
| raw-dense | 93.5 | 35.9 | 13.3 | 63.0 | 44.5 |
| raw-hybrid | 93.3 | 34.6 | 15.3 | 61.5 | 44.7 |
| graph-entities | 96.0 | 17.9 | 7.6 | 24.1 | 26.3 |
| graph-facts | 95.7 | 27.4 | 9.3 | 32.4 | 41.8 |
| default | 92.2 | 33.8 | 11.9 | 59.3 | 52.8 |

Oracle router over raw-bm25, raw-dense, raw-hybrid, graph-entities, graph-facts, default: F1 73.3, against 60.6 for the best single one; judged 85.4% against 72.2%. That gap is the most a per-question router could add.

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
