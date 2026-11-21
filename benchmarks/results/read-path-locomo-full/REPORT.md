# Evaluation report

Dataset: LoCoMo. 1986 questions (seed 7); histories average about 25,736 tokens.
Reader and extraction model: `deepseek-flash` (reasoning off). Judge: `deepseek-flash` (the same model as the reader, so a cross-check and not an independent measurement). Embeddings: `http`.

| Pipeline | F1 (95% interval) | Judged correct (95% interval) | Answer turn in context | Prompt tokens | Model calls per question | Retrieval p50 / p95 | Write-path LLM calls |
|---|---|---|---|---|---|---|---|
| raw-dense | 60.2 (58.3–62.1) | 72.6% (70.6–74.4) | 76.5% | 1,068 | 1.0 | 45.0 / 93.5 ms | 0.0 |
| default | 60.0 (58.1–61.9) | 70.8% (68.8–72.8) | 77.1% | 1,526 | 1.0 | 83.1 / 94.2 ms | 17.8 |
| dense-wide-reason | 64.6 (62.8–66.3) | 79.9% (78.0–81.7) | 83.9% | 2,093 | 1.0 | 43.2 / 45.9 ms | 0.0 |
| rerank-reason | 66.4 (64.8–68.0) | 81.7% (79.9–83.3) | 89.6% | 4,516 | 2.0 | 726.4 / 1464.6 ms | 0.0 |

Answerable questions only (1540; without adversarial). This is the subset most published figures use:

| Pipeline | F1 (95% interval) | Judged correct (95% interval) |
|---|---|---|
| raw-dense | 50.8 (48.7–52.9) | 66.7% (64.1–69.1) |
| default | 50.3 (48.1–52.3) | 64.2% (61.9–66.6) |
| dense-wide-reason | 57.1 (55.0–59.0) | 76.8% (74.7–78.8) |
| rerank-reason | 59.1 (57.2–61.2) | 78.8% (76.7–80.9) |

Difference from `raw-dense` on the same questions, in points (95% interval). An interval that excludes zero is a real difference:

| Pipeline | F1 | Judged correct |
|---|---|---|
| default | -0.2 (-1.8 to +1.2) | -1.8 (-3.7 to +0.2) |
| dense-wide-reason | +4.4 (+2.9 to +5.7) | +7.3 (+5.6 to +8.9) |
| rerank-reason | +6.2 (+4.7 to +7.7) | +9.1 (+7.4 to +10.8) |

Judged correct by category:

| Pipeline | adversarial | multi-hop | open-domain | single-hop | temporal |
|---|---|---|---|---|---|
| raw-dense | 92.8% (n=446) | 49.3% (n=282) | 27.1% (n=96) | 79.3% (n=841) | 60.8% (n=321) |
| default | 93.5% (n=446) | 45.4% (n=282) | 21.9% (n=96) | 73.2% (n=841) | 69.5% (n=321) |
| dense-wide-reason | 90.8% (n=446) | 65.6% (n=282) | 43.8% (n=96) | 84.2% (n=841) | 77.0% (n=321) |
| rerank-reason | 91.5% (n=446) | 66.7% (n=282) | 42.7% (n=96) | 87.9% (n=841) | 76.6% (n=321) |

F1 by category:

| Pipeline | adversarial | multi-hop | open-domain | single-hop | temporal |
|---|---|---|---|---|---|
| raw-dense | 92.8 | 36.2 | 15.6 | 62.8 | 42.6 |
| default | 93.5 | 34.0 | 11.8 | 57.4 | 57.4 |
| dense-wide-reason | 90.8 | 43.8 | 24.0 | 64.3 | 59.5 |
| rerank-reason | 91.5 | 43.6 | 25.6 | 67.7 | 60.4 |

Oracle router over raw-dense, default, dense-wide-reason, rerank-reason: F1 76.5, against 66.4 for the best single one; judged 89.5% against 81.7%. That gap is the most a per-question router could add.

What each stage contributed ("reached the answer" = the stage returned a turn holding the answer, or a fact stated in one):

| Pipeline | Stage | Reached the answer | Facts added | Turns added | p50 |
|---|---|---|---|---|---|
| raw-dense | turns (method=dense, k=20) | 76.5% | 0.0 | 20.0 | 45.0 ms |
| default | facts (method=hybrid, k=30) | 48.5% | 30.0 | 0.0 | 38.9 ms |
| default | fact_sources (max=8) | 38.5% | 0.0 | 8.0 | 0.0 ms |
| default | turns (method=hybrid, k=12) | 42.4% | 0.0 | 10.7 | 44.2 ms |
| dense-wide-reason | turns (method=dense, k=40) | 83.9% | 0.0 | 40.0 | 43.2 ms |
| rerank-reason | turns (method=hybrid, k=60) | 90.9% | 0.0 | 60.0 | 44.2 ms |
| rerank-reason | rerank (keep=20) | 89.6% | 0.0 | 20.0 | 679.5 ms |

"Adversarial" and "abstention" questions have no answer in the history; they are scored on whether the pipeline says so.
