# Evaluation report

Dataset: LoCoMo. 321 questions (seed 7); histories average about 25,736 tokens.
Reader and extraction model: `deepseek-flash` (reasoning off). Judge: `deepseek-v4-pro`. Embeddings: `local`.

| Pipeline | F1 (95% interval) | Judged correct (95% interval) | Answer turn in context | Prompt tokens | Model calls per question | Retrieval p50 / p95 | Write-path LLM calls |
|---|---|---|---|---|---|---|---|
| raw-dense | 43.4 (39.6–47.1) | 57.0% (51.4–62.3) | 85.4% | 1,082 | 1.0 | 54.4 / 63.9 ms | 0.0 |
| dense-wide-reason | 59.1 (54.7–63.3) | 72.0% (67.0–76.6) | 89.7% | 2,095 | 1.0 | 54.2 / 75.0 ms | 0.0 |
| rerank-reason | 60.8 (56.7–64.9) | 73.5% (68.8–78.2) | 91.0% | 4,521 | 2.0 | 56.6 / 74.4 ms | 0.0 |
| default | 56.5 (52.2–60.6) | 62.0% (56.4–67.3) | 89.4% | 1,500 | 1.0 | 138.5 / 202.4 ms | 17.8 |
| default-reason | 58.2 (53.9–62.5) | 69.2% (63.5–73.8) | 89.4% | 1,556 | 1.0 | 139.8 / 205.1 ms | 17.8 |
| sources-only-reason | 59.6 (55.0–63.4) | 70.4% (65.1–75.1) | 89.4% | 1,127 | 1.0 | 141.0 / 203.5 ms | 17.8 |
| wide+facts-reason | 58.1 (53.8–62.4) | 67.3% (62.0–72.3) | 89.7% | 2,524 | 1.0 | 139.8 / 187.2 ms | 17.8 |
| wide+timeline-reason | 60.0 (56.0–63.9) | 70.1% (64.8–74.8) | 89.7% | 2,541 | 1.0 | 139.7 / 193.1 ms | 17.8 |
| wide+sources-reason | 62.4 (58.2–66.5) | 74.5% (69.8–79.1) | 92.8% | 2,486 | 1.0 | 138.5 / 190.3 ms | 17.8 |
| timeline-only-reason | 51.4 (47.0–56.1) | 57.0% (51.4–62.3) | 71.7% | 1,437 | 1.0 | 84.5 / 125.1 ms | 17.8 |

Difference from `dense-wide-reason` on the same questions, in points (95% interval). An interval that excludes zero is a real difference:

| Pipeline | F1 | Judged correct |
|---|---|---|
| raw-dense | -15.7 (-19.8 to -11.6) | -14.9 (-20.2 to -10.0) |
| rerank-reason | +1.7 (-1.4 to +4.9) | +1.6 (-1.9 to +5.0) |
| default | -2.6 (-6.3 to +1.2) | -10.0 (-15.3 to -4.7) |
| default-reason | -0.9 (-4.6 to +3.1) | -2.8 (-7.8 to +2.2) |
| sources-only-reason | +0.5 (-3.0 to +3.9) | -1.6 (-5.9 to +2.2) |
| wide+facts-reason | -0.9 (-4.6 to +2.7) | -4.7 (-9.0 to +0.3) |
| wide+timeline-reason | +0.9 (-2.3 to +4.5) | -1.9 (-6.2 to +2.5) |
| wide+sources-reason | +3.3 (+0.8 to +6.1) | +2.5 (-0.6 to +5.6) |
| timeline-only-reason | -7.7 (-12.0 to -3.0) | -14.9 (-20.2 to -9.3) |

Judged correct by category:

| Pipeline | temporal |
|---|---|
| raw-dense | 57.0% (n=321) |
| dense-wide-reason | 72.0% (n=321) |
| rerank-reason | 73.5% (n=321) |
| default | 62.0% (n=321) |
| default-reason | 69.2% (n=321) |
| sources-only-reason | 70.4% (n=321) |
| wide+facts-reason | 67.3% (n=321) |
| wide+timeline-reason | 70.1% (n=321) |
| wide+sources-reason | 74.5% (n=321) |
| timeline-only-reason | 57.0% (n=321) |

F1 by category:

| Pipeline | temporal |
|---|---|
| raw-dense | 43.4 |
| dense-wide-reason | 59.1 |
| rerank-reason | 60.8 |
| default | 56.5 |
| default-reason | 58.2 |
| sources-only-reason | 59.6 |
| wide+facts-reason | 58.1 |
| wide+timeline-reason | 60.0 |
| wide+sources-reason | 62.4 |
| timeline-only-reason | 51.4 |

Oracle router over raw-dense, dense-wide-reason, rerank-reason, default, default-reason, sources-only-reason, wide+facts-reason, wide+timeline-reason, wide+sources-reason, timeline-only-reason: F1 77.8, against 62.4 for the best single one; judged 88.2% against 74.5%. That gap is the most a per-question router could add.

What each stage contributed ("reached the answer" = the stage returned a turn holding the answer, or a fact stated in one):

| Pipeline | Stage | Reached the answer | Facts added | Turns added | p50 |
|---|---|---|---|---|---|
| raw-dense | turns (method=dense, k=20) | 85.4% | 0.0 | 20.0 | 54.3 ms |
| dense-wide-reason | turns (method=dense, k=40) | 89.7% | 0.0 | 40.0 | 54.1 ms |
| rerank-reason | turns (method=hybrid, k=60) | 92.8% | 0.0 | 60.0 | 56.3 ms |
| rerank-reason | rerank (keep=20) | 91.0% | 0.0 | 20.0 | 0.2 ms |
| default | facts (method=hybrid, k=30) | 73.5% | 30.0 | 0.0 | 83.5 ms |
| default | fact_sources (max=8) | 67.9% | 0.0 | 8.0 | 0.0 ms |
| default | turns (method=hybrid, k=12) | 25.2% | 0.0 | 10.1 | 55.9 ms |
| default-reason | facts (method=hybrid, k=30) | 73.5% | 30.0 | 0.0 | 83.0 ms |
| default-reason | fact_sources (max=8) | 67.9% | 0.0 | 8.0 | 0.0 ms |
| default-reason | turns (method=hybrid, k=12) | 25.2% | 0.0 | 10.1 | 56.8 ms |
| sources-only-reason | facts (method=hybrid, k=30) | 73.5% | 30.0 | 0.0 | 84.5 ms |
| sources-only-reason | fact_sources (max=8) | 67.9% | 0.0 | 8.0 | 0.0 ms |
| sources-only-reason | turns (method=hybrid, k=12) | 25.2% | 0.0 | 10.1 | 56.9 ms |
| wide+facts-reason | turns (method=dense, k=40) | 89.7% | 0.0 | 40.0 | 55.4 ms |
| wide+facts-reason | facts (method=hybrid, k=30) | 73.5% | 30.0 | 0.0 | 84.4 ms |
| wide+timeline-reason | turns (method=dense, k=40) | 89.7% | 0.0 | 40.0 | 55.4 ms |
| wide+timeline-reason | facts (method=hybrid, k=30) | 73.5% | 30.0 | 0.0 | 84.7 ms |
| wide+sources-reason | turns (method=dense, k=40) | 89.7% | 0.0 | 40.0 | 55.3 ms |
| wide+sources-reason | facts (method=hybrid, k=30) | 73.5% | 30.0 | 0.0 | 82.9 ms |
| wide+sources-reason | fact_sources (max=10) | 5.3% | 0.0 | 6.8 | 0.0 ms |
| timeline-only-reason | facts (method=hybrid, k=40) | 77.3% | 40.0 | 0.0 | 84.2 ms |
| timeline-only-reason | fact_sources (max=12) | 71.7% | 0.0 | 12.0 | 0.0 ms |

"Adversarial" and "abstention" questions have no answer in the history; they are scored on whether the pipeline says so.
