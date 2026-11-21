# Evaluation report

Dataset: LoCoMo. 321 questions (seed 7); histories average about 25,736 tokens.
Reader and extraction model: `deepseek-flash` (reasoning off). Judge: `deepseek-flash` (the same model as the reader, so a cross-check and not an independent measurement). Embeddings: `local`.

| Pipeline | F1 (95% interval) | Judged correct (95% interval) | Answer turn in context | Prompt tokens | Model calls per question | Retrieval p50 / p95 | Write-path LLM calls |
|---|---|---|---|---|---|---|---|
| raw-dense | 43.4 (39.6–47.1) | 60.4% (54.8–65.7) | 85.4% | 1,082 | 1.0 | 115.6 / 168.0 ms | 0.0 |
| dense-wide-reason | 59.1 (54.7–63.3) | 75.4% (70.7–80.1) | 89.7% | 2,095 | 1.0 | 55.0 / 77.1 ms | 0.0 |
| rerank-reason | 60.8 (56.7–64.9) | 77.9% (73.2–82.2) | 91.0% | 4,521 | 2.0 | 1145.9 / 1623.0 ms | 0.0 |
| default | 51.3 (47.2–55.3) | 57.3% (51.7–62.9) | 88.5% | 1,449 | 1.0 | 141.0 / 188.4 ms | 17.8 |
| default-reason | 57.2 (53.1–61.1) | 75.4% (70.4–79.8) | 88.5% | 1,505 | 1.0 | 140.1 / 186.6 ms | 17.8 |
| sources-only-reason | 58.2 (53.9–62.6) | 74.1% (68.8–78.5) | 88.5% | 1,126 | 1.0 | 137.5 / 198.9 ms | 17.8 |
| wide+facts-reason | 59.9 (55.8–63.9) | 78.5% (73.5–82.9) | 89.7% | 2,474 | 1.0 | 137.9 / 187.3 ms | 17.8 |
| wide+timeline-reason | 57.3 (52.9–61.3) | 73.2% (68.2–77.9) | 89.7% | 2,515 | 1.0 | 140.3 / 187.4 ms | 17.8 |
| wide+sources-reason | 62.5 (58.1–66.6) | 77.9% (73.5–82.2) | 92.2% | 2,484 | 1.0 | 139.1 / 188.2 ms | 17.8 |
| timeline-only-reason | 46.8 (42.7–51.2) | 60.4% (55.1–65.4) | 67.9% | 1,406 | 1.0 | 84.7 / 114.6 ms | 17.8 |

Difference from `dense-wide-reason` on the same questions, in points (95% interval). An interval that excludes zero is a real difference:

| Pipeline | F1 | Judged correct |
|---|---|---|
| raw-dense | -15.7 (-19.8 to -11.6) | -14.9 (-20.6 to -9.7) |
| rerank-reason | +1.7 (-1.4 to +4.9) | +2.5 (-0.9 to +6.2) |
| default | -7.8 (-12.3 to -3.1) | -18.1 (-24.6 to -11.5) |
| default-reason | -1.9 (-5.8 to +1.7) | +0.0 (-4.7 to +4.7) |
| sources-only-reason | -0.9 (-3.8 to +2.1) | -1.2 (-5.0 to +2.2) |
| wide+facts-reason | +0.8 (-2.8 to +4.5) | +3.1 (-1.2 to +7.8) |
| wide+timeline-reason | -1.8 (-5.5 to +1.9) | -2.2 (-6.9 to +2.5) |
| wide+sources-reason | +3.4 (+0.7 to +6.1) | +2.5 (-0.6 to +5.3) |
| timeline-only-reason | -12.3 (-16.9 to -7.7) | -14.9 (-20.6 to -9.0) |

Judged correct by category:

| Pipeline | temporal |
|---|---|
| raw-dense | 60.4% (n=321) |
| dense-wide-reason | 75.4% (n=321) |
| rerank-reason | 77.9% (n=321) |
| default | 57.3% (n=321) |
| default-reason | 75.4% (n=321) |
| sources-only-reason | 74.1% (n=321) |
| wide+facts-reason | 78.5% (n=321) |
| wide+timeline-reason | 73.2% (n=321) |
| wide+sources-reason | 77.9% (n=321) |
| timeline-only-reason | 60.4% (n=321) |

F1 by category:

| Pipeline | temporal |
|---|---|
| raw-dense | 43.4 |
| dense-wide-reason | 59.1 |
| rerank-reason | 60.8 |
| default | 51.3 |
| default-reason | 57.2 |
| sources-only-reason | 58.2 |
| wide+facts-reason | 59.9 |
| wide+timeline-reason | 57.3 |
| wide+sources-reason | 62.5 |
| timeline-only-reason | 46.8 |

Oracle router over raw-dense, dense-wide-reason, rerank-reason, default, default-reason, sources-only-reason, wide+facts-reason, wide+timeline-reason, wide+sources-reason, timeline-only-reason: F1 78.5, against 62.5 for the best single one; judged 94.4% against 78.5%. That gap is the most a per-question router could add.

What each stage contributed ("reached the answer" = the stage returned a turn holding the answer, or a fact stated in one):

| Pipeline | Stage | Reached the answer | Facts added | Turns added | p50 |
|---|---|---|---|---|---|
| raw-dense | turns (method=dense, k=20) | 85.4% | 0.0 | 20.0 | 115.6 ms |
| dense-wide-reason | turns (method=dense, k=40) | 89.7% | 0.0 | 40.0 | 55.0 ms |
| rerank-reason | turns (method=hybrid, k=60) | 92.8% | 0.0 | 60.0 | 57.6 ms |
| rerank-reason | rerank (keep=20) | 91.0% | 0.0 | 20.0 | 1082.6 ms |
| default | facts (method=hybrid, k=30) | 70.1% | 30.0 | 0.0 | 82.6 ms |
| default | fact_sources (max=8) | 62.0% | 0.0 | 8.0 | 0.0 ms |
| default | turns (method=hybrid, k=12) | 29.6% | 0.0 | 10.1 | 56.6 ms |
| default-reason | facts (method=hybrid, k=30) | 70.1% | 30.0 | 0.0 | 81.6 ms |
| default-reason | fact_sources (max=8) | 62.0% | 0.0 | 8.0 | 0.0 ms |
| default-reason | turns (method=hybrid, k=12) | 29.6% | 0.0 | 10.1 | 57.0 ms |
| sources-only-reason | facts (method=hybrid, k=30) | 70.1% | 30.0 | 0.0 | 80.3 ms |
| sources-only-reason | fact_sources (max=8) | 62.0% | 0.0 | 8.0 | 0.0 ms |
| sources-only-reason | turns (method=hybrid, k=12) | 29.6% | 0.0 | 10.1 | 56.9 ms |
| wide+facts-reason | turns (method=dense, k=40) | 89.7% | 0.0 | 40.0 | 55.5 ms |
| wide+facts-reason | facts (method=hybrid, k=30) | 70.1% | 30.0 | 0.0 | 81.1 ms |
| wide+timeline-reason | turns (method=dense, k=40) | 89.7% | 0.0 | 40.0 | 55.5 ms |
| wide+timeline-reason | facts (method=hybrid, k=30) | 70.1% | 30.0 | 0.0 | 82.5 ms |
| wide+sources-reason | turns (method=dense, k=40) | 89.7% | 0.0 | 40.0 | 55.3 ms |
| wide+sources-reason | facts (method=hybrid, k=30) | 70.1% | 30.0 | 0.0 | 81.9 ms |
| wide+sources-reason | fact_sources (max=10) | 4.4% | 0.0 | 6.8 | 0.0 ms |
| timeline-only-reason | facts (method=hybrid, k=40) | 74.5% | 40.0 | 0.0 | 84.5 ms |
| timeline-only-reason | fact_sources (max=12) | 67.9% | 0.0 | 12.0 | 0.0 ms |

"Adversarial" and "abstention" questions have no answer in the history; they are scored on whether the pipeline says so.
