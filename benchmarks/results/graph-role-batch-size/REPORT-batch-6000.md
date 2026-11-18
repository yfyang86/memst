# Evaluation report

Dataset: LoCoMo. 138 questions (seed 7); histories average about 22,179 tokens.
Reader and extraction model: `deepseek-flash` (reasoning off). Judge: `deepseek-flash` (the same model as the reader, so a cross-check and not an independent measurement). Embeddings: `local`.

| Pipeline | F1 (95% interval) | Judged correct (95% interval) | Answer turn in context | Prompt tokens | Model calls per question | Retrieval p50 / p95 | Write-path LLM calls |
|---|---|---|---|---|---|---|---|
| dense-wide-reason | 59.6 (52.6–65.8) | 76.1% (68.8–82.6) | 83.8% | 2,109 | 1.0 | 39.7 / 63.3 ms | 0.0 |
| default | 55.8 (48.7–62.6) | 66.7% (58.7–73.9) | 73.5% | 1,493 | 1.0 | 107.9 / 179.8 ms | 15.3 |
| default-reason | 57.4 (50.0–64.1) | 71.7% (64.5–79.0) | 73.5% | 1,549 | 1.0 | 113.7 / 170.3 ms | 15.3 |
| sources-only-reason | 55.5 (48.5–62.3) | 67.4% (60.1–74.6) | 73.5% | 1,133 | 1.0 | 109.6 / 163.0 ms | 15.3 |
| wide+sources-reason | 62.3 (55.6–68.7) | 76.1% (68.8–82.6) | 85.3% | 2,516 | 1.0 | 106.9 / 190.1 ms | 15.3 |
| timeline-only-reason | 51.7 (44.2–58.9) | 62.3% (54.4–70.3) | 50.0% | 1,403 | 1.0 | 71.9 / 110.5 ms | 15.3 |

Answerable questions only (108; without adversarial). This is the subset most published figures use:

| Pipeline | F1 (95% interval) | Judged correct (95% interval) |
|---|---|---|
| dense-wide-reason | 52.1 (45.0–59.6) | 73.2% (64.8–81.5) |
| default | 45.4 (37.9–53.2) | 59.3% (50.0–68.5) |
| default-reason | 48.4 (40.8–56.4) | 66.7% (57.4–75.0) |
| sources-only-reason | 46.8 (39.0–54.2) | 62.0% (52.8–71.3) |
| wide+sources-reason | 54.6 (47.2–62.0) | 72.2% (63.9–80.6) |
| timeline-only-reason | 40.1 (32.5–47.9) | 53.7% (44.4–62.0) |

Difference from `dense-wide-reason` on the same questions, in points (95% interval). An interval that excludes zero is a real difference:

| Pipeline | F1 | Judged correct |
|---|---|---|
| default | -3.8 (-8.9 to +1.5) | -9.4 (-16.7 to -2.2) |
| default-reason | -2.2 (-7.2 to +3.1) | -4.3 (-10.9 to +2.2) |
| sources-only-reason | -4.2 (-8.1 to -0.2) | -8.7 (-13.8 to -3.6) |
| wide+sources-reason | +2.7 (-0.5 to +6.3) | +0.0 (-4.3 to +4.3) |
| timeline-only-reason | -8.0 (-13.6 to -2.3) | -13.8 (-21.0 to -6.5) |

Judged correct by category:

| Pipeline | adversarial | multi-hop | open-domain | single-hop | temporal |
|---|---|---|---|---|---|
| dense-wide-reason | 86.7% (n=30) | 73.3% (n=30) | 44.4% (n=18) | 76.7% (n=30) | 86.7% (n=30) |
| default | 93.3% (n=30) | 50.0% (n=30) | 22.2% (n=18) | 73.3% (n=30) | 76.7% (n=30) |
| default-reason | 90.0% (n=30) | 66.7% (n=30) | 44.4% (n=18) | 70.0% (n=30) | 76.7% (n=30) |
| sources-only-reason | 86.7% (n=30) | 50.0% (n=30) | 33.3% (n=18) | 66.7% (n=30) | 86.7% (n=30) |
| wide+sources-reason | 90.0% (n=30) | 73.3% (n=30) | 38.9% (n=18) | 76.7% (n=30) | 86.7% (n=30) |
| timeline-only-reason | 93.3% (n=30) | 60.0% (n=30) | 33.3% (n=18) | 43.3% (n=30) | 70.0% (n=30) |

F1 by category:

| Pipeline | adversarial | multi-hop | open-domain | single-hop | temporal |
|---|---|---|---|---|---|
| dense-wide-reason | 86.7 | 51.1 | 20.8 | 55.1 | 68.9 |
| default | 93.3 | 40.2 | 12.9 | 47.1 | 68.4 |
| default-reason | 90.0 | 49.8 | 22.1 | 45.9 | 65.2 |
| sources-only-reason | 86.7 | 41.1 | 18.5 | 46.1 | 70.3 |
| wide+sources-reason | 90.0 | 56.0 | 20.5 | 55.5 | 72.8 |
| timeline-only-reason | 93.3 | 43.8 | 18.9 | 31.2 | 58.0 |

Oracle router over dense-wide-reason, default, default-reason, sources-only-reason, wide+sources-reason, timeline-only-reason: F1 68.9, against 62.3 for the best single one; judged 83.3% against 76.1%. That gap is the most a per-question router could add.

What each stage contributed ("reached the answer" = the stage returned a turn holding the answer, or a fact stated in one):

| Pipeline | Stage | Reached the answer | Facts added | Turns added | p50 |
|---|---|---|---|---|---|
| dense-wide-reason | turns (method=dense, k=40) | 83.8% | 0.0 | 40.0 | 39.6 ms |
| default | facts (method=hybrid, k=30) | 57.4% | 30.0 | 0.0 | 70.6 ms |
| default | fact_sources (max=8) | 46.3% | 0.0 | 8.0 | 0.0 ms |
| default | turns (method=hybrid, k=12) | 30.9% | 0.0 | 10.6 | 38.6 ms |
| default-reason | facts (method=hybrid, k=30) | 57.4% | 30.0 | 0.0 | 71.7 ms |
| default-reason | fact_sources (max=8) | 46.3% | 0.0 | 8.0 | 0.0 ms |
| default-reason | turns (method=hybrid, k=12) | 30.9% | 0.0 | 10.6 | 38.9 ms |
| sources-only-reason | facts (method=hybrid, k=30) | 57.4% | 30.0 | 0.0 | 68.8 ms |
| sources-only-reason | fact_sources (max=8) | 46.3% | 0.0 | 8.0 | 0.0 ms |
| sources-only-reason | turns (method=hybrid, k=12) | 30.9% | 0.0 | 10.6 | 40.0 ms |
| wide+sources-reason | turns (method=dense, k=40) | 83.8% | 0.0 | 40.0 | 38.8 ms |
| wide+sources-reason | facts (method=hybrid, k=30) | 57.4% | 30.0 | 0.0 | 69.0 ms |
| wide+sources-reason | fact_sources (max=10) | 1.5% | 0.0 | 7.2 | 0.0 ms |
| timeline-only-reason | facts (method=hybrid, k=40) | 58.1% | 40.0 | 0.0 | 71.7 ms |
| timeline-only-reason | fact_sources (max=12) | 50.0% | 0.0 | 12.0 | 0.0 ms |

"Adversarial" and "abstention" questions have no answer in the history; they are scored on whether the pipeline says so.
