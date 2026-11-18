# Evaluation report

Dataset: LoCoMo. 138 questions (seed 7); histories average about 22,179 tokens.
Reader and extraction model: `deepseek-flash` (reasoning off). Judge: `deepseek-flash` (the same model as the reader, so a cross-check and not an independent measurement). Embeddings: `local`.

| Pipeline | F1 (95% interval) | Judged correct (95% interval) | Answer turn in context | Prompt tokens | Model calls per question | Retrieval p50 / p95 | Write-path LLM calls |
|---|---|---|---|---|---|---|---|
| dense-wide-reason | 59.6 (52.6–65.8) | 76.1% (68.8–82.6) | 83.8% | 2,109 | 1.0 | 35.5 / 64.7 ms | 0.0 |
| default | 55.7 (49.0–62.5) | 66.7% (58.7–73.9) | 71.3% | 1,510 | 1.0 | 89.2 / 141.2 ms | 8.0 |
| default-reason | 54.4 (46.9–61.5) | 66.7% (58.7–74.6) | 71.3% | 1,566 | 1.0 | 90.3 / 150.1 ms | 8.0 |
| sources-only-reason | 53.8 (46.4–60.6) | 67.4% (58.7–74.6) | 71.3% | 1,172 | 1.0 | 90.6 / 155.0 ms | 8.0 |
| wide+sources-reason | 58.1 (51.1–64.7) | 75.4% (68.1–82.6) | 84.6% | 2,576 | 1.0 | 86.3 / 148.5 ms | 8.0 |
| timeline-only-reason | 49.1 (41.5–56.5) | 56.5% (48.5–65.2) | 42.6% | 1,427 | 1.0 | 51.6 / 85.1 ms | 8.0 |

Answerable questions only (108; without adversarial). This is the subset most published figures use:

| Pipeline | F1 (95% interval) | Judged correct (95% interval) |
|---|---|---|
| dense-wide-reason | 52.1 (45.0–59.6) | 73.2% (64.8–81.5) |
| default | 46.2 (38.3–54.5) | 60.2% (50.9–69.4) |
| default-reason | 44.5 (37.0–52.3) | 60.2% (50.9–69.4) |
| sources-only-reason | 43.7 (36.8–51.9) | 61.1% (51.8–70.4) |
| wide+sources-reason | 50.2 (43.0–57.8) | 72.2% (63.9–80.6) |
| timeline-only-reason | 36.8 (28.8–44.9) | 46.3% (37.0–55.6) |

Difference from `dense-wide-reason` on the same questions, in points (95% interval). An interval that excludes zero is a real difference:

| Pipeline | F1 | Judged correct |
|---|---|---|
| default | -3.9 (-8.9 to +2.1) | -9.4 (-16.7 to -2.2) |
| default-reason | -5.2 (-9.8 to -0.4) | -9.4 (-15.9 to -3.6) |
| sources-only-reason | -5.9 (-10.2 to -2.1) | -8.7 (-14.5 to -2.9) |
| wide+sources-reason | -1.5 (-4.3 to +1.4) | -0.7 (-4.3 to +2.9) |
| timeline-only-reason | -10.5 (-17.0 to -3.5) | -19.6 (-28.3 to -11.6) |

Judged correct by category:

| Pipeline | adversarial | multi-hop | open-domain | single-hop | temporal |
|---|---|---|---|---|---|
| dense-wide-reason | 86.7% (n=30) | 73.3% (n=30) | 44.4% (n=18) | 76.7% (n=30) | 86.7% (n=30) |
| default | 90.0% (n=30) | 43.3% (n=30) | 33.3% (n=18) | 66.7% (n=30) | 86.7% (n=30) |
| default-reason | 90.0% (n=30) | 60.0% (n=30) | 38.9% (n=18) | 60.0% (n=30) | 73.3% (n=30) |
| sources-only-reason | 90.0% (n=30) | 50.0% (n=30) | 38.9% (n=18) | 63.3% (n=30) | 83.3% (n=30) |
| wide+sources-reason | 86.7% (n=30) | 66.7% (n=30) | 44.4% (n=18) | 76.7% (n=30) | 90.0% (n=30) |
| timeline-only-reason | 93.3% (n=30) | 50.0% (n=30) | 27.8% (n=18) | 26.7% (n=30) | 73.3% (n=30) |

F1 by category:

| Pipeline | adversarial | multi-hop | open-domain | single-hop | temporal |
|---|---|---|---|---|---|
| dense-wide-reason | 86.7 | 51.1 | 20.8 | 55.1 | 68.9 |
| default | 90.0 | 40.0 | 16.2 | 47.4 | 69.2 |
| default-reason | 90.0 | 47.1 | 21.4 | 41.8 | 58.4 |
| sources-only-reason | 90.0 | 41.3 | 19.1 | 42.1 | 62.6 |
| wide+sources-reason | 86.7 | 46.7 | 19.4 | 52.7 | 69.6 |
| timeline-only-reason | 93.3 | 40.2 | 12.8 | 25.2 | 59.4 |

Oracle router over dense-wide-reason, default, default-reason, sources-only-reason, wide+sources-reason, timeline-only-reason: F1 69.9, against 59.6 for the best single one; judged 84.1% against 76.1%. That gap is the most a per-question router could add.

What each stage contributed ("reached the answer" = the stage returned a turn holding the answer, or a fact stated in one):

| Pipeline | Stage | Reached the answer | Facts added | Turns added | p50 |
|---|---|---|---|---|---|
| dense-wide-reason | turns (method=dense, k=40) | 83.8% | 0.0 | 40.0 | 35.4 ms |
| default | facts (method=hybrid, k=30) | 47.8% | 30.0 | 0.0 | 50.0 ms |
| default | fact_sources (max=8) | 33.8% | 0.0 | 8.0 | 0.0 ms |
| default | turns (method=hybrid, k=12) | 39.7% | 0.0 | 11.0 | 40.1 ms |
| default-reason | facts (method=hybrid, k=30) | 47.8% | 30.0 | 0.0 | 49.7 ms |
| default-reason | fact_sources (max=8) | 33.8% | 0.0 | 8.0 | 0.0 ms |
| default-reason | turns (method=hybrid, k=12) | 39.7% | 0.0 | 11.0 | 39.8 ms |
| sources-only-reason | facts (method=hybrid, k=30) | 47.8% | 30.0 | 0.0 | 49.7 ms |
| sources-only-reason | fact_sources (max=8) | 33.8% | 0.0 | 8.0 | 0.0 ms |
| sources-only-reason | turns (method=hybrid, k=12) | 39.7% | 0.0 | 11.0 | 43.3 ms |
| wide+sources-reason | turns (method=dense, k=40) | 83.8% | 0.0 | 40.0 | 36.8 ms |
| wide+sources-reason | facts (method=hybrid, k=30) | 47.8% | 30.0 | 0.0 | 49.5 ms |
| wide+sources-reason | fact_sources (max=10) | 1.5% | 0.0 | 7.8 | 0.0 ms |
| timeline-only-reason | facts (method=hybrid, k=40) | 52.2% | 40.0 | 0.0 | 51.4 ms |
| timeline-only-reason | fact_sources (max=12) | 42.6% | 0.0 | 12.0 | 0.0 ms |

"Adversarial" and "abstention" questions have no answer in the history; they are scored on whether the pipeline says so.
