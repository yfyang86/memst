# Evaluation report

Dataset: LongMemEval-S. 22 questions (seed 7); histories average about 126,404 tokens.
Reader and extraction model: `deepseek-flash` (reasoning off). Judge: `deepseek-flash` (the same model as the reader, so a cross-check and not an independent measurement). Embeddings: `local`.

| Pipeline | F1 (95% interval) | Judged correct (95% interval) | Answer turn in context | Prompt tokens | Model calls per question | Retrieval p50 / p95 | Write-path LLM calls |
|---|---|---|---|---|---|---|---|
| raw-dense | 64.2 (45.9–82.2) | 68.2% (50.0–86.4) | 100.0% | 4,214 | 1.0 | 51.0 / 130.0 ms | 0.0 |
| dense-wide-reason | 75.4 (63.2–87.5) | 100.0% (100.0–100.0) | 100.0% | 7,608 | 1.0 | 44.5 / 71.7 ms | 0.0 |
| rerank-reason | 71.8 (56.3–85.6) | 90.9% (77.3–100.0) | 100.0% | 10,418 | 2.0 | 1248.6 / 1683.0 ms | 0.0 |
| default | 62.7 (44.2–80.2) | 77.3% (59.1–95.5) | 95.5% | 5,381 | 1.0 | 178.7 / 216.6 ms | 46.5 |
| default-reason | 72.0 (56.9–85.8) | 90.9% (77.3–100.0) | 95.5% | 5,437 | 1.0 | 176.7 / 224.6 ms | 46.5 |
| sources-only-reason | 64.8 (49.1–80.2) | 86.4% (72.7–100.0) | 95.5% | 4,918 | 1.0 | 185.3 / 221.0 ms | 46.5 |
| wide+facts-reason | 72.9 (58.3–85.9) | 95.5% (86.4–100.0) | 100.0% | 8,126 | 1.0 | 179.9 / 205.0 ms | 46.5 |
| wide+timeline-reason | 78.3 (65.3–90.3) | 95.5% (86.4–100.0) | 100.0% | 8,125 | 1.0 | 188.6 / 211.3 ms | 46.5 |
| wide+sources-reason | 72.5 (58.3–85.9) | 95.5% (86.4–100.0) | 100.0% | 9,727 | 1.0 | 177.6 / 214.8 ms | 46.5 |

Answerable questions only (20; without abstention). This is the subset most published figures use:

| Pipeline | F1 (95% interval) | Judged correct (95% interval) |
|---|---|---|
| raw-dense | 60.6 (42.5–79.5) | 65.0% (45.0–85.0) |
| dense-wide-reason | 73.0 (59.2–85.5) | 100.0% (100.0–100.0) |
| rerank-reason | 68.9 (52.5–83.9) | 90.0% (75.0–100.0) |
| default | 58.9 (39.8–78.0) | 75.0% (55.0–95.0) |
| default-reason | 69.2 (53.3–83.5) | 90.0% (75.0–100.0) |
| sources-only-reason | 61.3 (44.1–76.8) | 85.0% (70.0–100.0) |
| wide+facts-reason | 70.2 (55.7–83.7) | 95.0% (85.0–100.0) |
| wide+timeline-reason | 76.1 (61.6–88.6) | 95.0% (85.0–100.0) |
| wide+sources-reason | 69.8 (54.6–84.0) | 95.0% (85.0–100.0) |

Difference from `dense-wide-reason` on the same questions, in points (95% interval). An interval that excludes zero is a real difference:

| Pipeline | F1 | Judged correct |
|---|---|---|
| raw-dense | -11.2 (-29.2 to +5.9) | -31.8 (-50.0 to -13.6) |
| rerank-reason | -3.7 (-14.1 to +3.1) | -9.1 (-22.7 to +0.0) |
| default | -12.8 (-30.5 to +3.2) | -22.7 (-40.9 to -4.5) |
| default-reason | -3.4 (-17.5 to +9.3) | -9.1 (-22.7 to +0.0) |
| sources-only-reason | -10.6 (-25.5 to +1.3) | -13.6 (-27.3 to +0.0) |
| wide+facts-reason | -2.5 (-15.5 to +9.8) | -4.5 (-13.6 to +0.0) |
| wide+timeline-reason | +2.8 (-6.7 to +14.3) | -4.5 (-13.6 to +0.0) |
| wide+sources-reason | -2.9 (-9.1 to +0.3) | -4.5 (-13.6 to +0.0) |

Judged correct by category:

| Pipeline | abstention | knowledge-update | temporal-reasoning |
|---|---|---|---|
| raw-dense | 100.0% (n=2) | 100.0% (n=10) | 30.0% (n=10) |
| dense-wide-reason | 100.0% (n=2) | 100.0% (n=10) | 100.0% (n=10) |
| rerank-reason | 100.0% (n=2) | 90.0% (n=10) | 90.0% (n=10) |
| default | 100.0% (n=2) | 100.0% (n=10) | 50.0% (n=10) |
| default-reason | 100.0% (n=2) | 100.0% (n=10) | 80.0% (n=10) |
| sources-only-reason | 100.0% (n=2) | 90.0% (n=10) | 80.0% (n=10) |
| wide+facts-reason | 100.0% (n=2) | 100.0% (n=10) | 90.0% (n=10) |
| wide+timeline-reason | 100.0% (n=2) | 90.0% (n=10) | 100.0% (n=10) |
| wide+sources-reason | 100.0% (n=2) | 100.0% (n=10) | 90.0% (n=10) |

F1 by category:

| Pipeline | abstention | knowledge-update | temporal-reasoning |
|---|---|---|---|
| raw-dense | 100.0 | 86.7 | 34.6 |
| dense-wide-reason | 100.0 | 76.7 | 69.3 |
| rerank-reason | 100.0 | 66.7 | 71.2 |
| default | 100.0 | 83.3 | 34.5 |
| default-reason | 100.0 | 83.3 | 55.1 |
| sources-only-reason | 100.0 | 70.0 | 52.6 |
| wide+facts-reason | 100.0 | 78.9 | 61.6 |
| wide+timeline-reason | 100.0 | 70.0 | 82.2 |
| wide+sources-reason | 100.0 | 76.7 | 62.9 |

Oracle router over raw-dense, dense-wide-reason, rerank-reason, default, default-reason, sources-only-reason, wide+facts-reason, wide+timeline-reason, wide+sources-reason: F1 90.7, against 78.3 for the best single one; judged 100.0% against 100.0%. That gap is the most a per-question router could add.

What each stage contributed ("reached the answer" = the stage returned a turn holding the answer, or a fact stated in one):

| Pipeline | Stage | Reached the answer | Facts added | Turns added | p50 |
|---|---|---|---|---|---|
| raw-dense | turns (method=dense, k=20) | 100.0% | 0.0 | 20.0 | 50.9 ms |
| dense-wide-reason | turns (method=dense, k=40) | 100.0% | 0.0 | 40.0 | 44.4 ms |
| rerank-reason | turns (method=hybrid, k=60) | 100.0% | 0.0 | 60.0 | 47.2 ms |
| rerank-reason | rerank (keep=20) | 100.0% | 0.0 | 20.0 | 1184.7 ms |
| default | facts (method=hybrid, k=30) | 68.2% | 30.0 | 0.0 | 133.0 ms |
| default | fact_sources (max=8) | 68.2% | 0.0 | 8.0 | 0.0 ms |
| default | turns (method=hybrid, k=12) | 50.0% | 0.0 | 9.0 | 44.1 ms |
| default-reason | facts (method=hybrid, k=30) | 68.2% | 30.0 | 0.0 | 131.7 ms |
| default-reason | fact_sources (max=8) | 68.2% | 0.0 | 8.0 | 0.0 ms |
| default-reason | turns (method=hybrid, k=12) | 50.0% | 0.0 | 9.0 | 45.7 ms |
| sources-only-reason | facts (method=hybrid, k=30) | 68.2% | 30.0 | 0.0 | 134.4 ms |
| sources-only-reason | fact_sources (max=8) | 68.2% | 0.0 | 8.0 | 0.0 ms |
| sources-only-reason | turns (method=hybrid, k=12) | 50.0% | 0.0 | 9.0 | 46.6 ms |
| wide+facts-reason | turns (method=dense, k=40) | 100.0% | 0.0 | 40.0 | 43.3 ms |
| wide+facts-reason | facts (method=hybrid, k=30) | 68.2% | 30.0 | 0.0 | 129.1 ms |
| wide+timeline-reason | turns (method=dense, k=40) | 100.0% | 0.0 | 40.0 | 48.7 ms |
| wide+timeline-reason | facts (method=hybrid, k=30) | 68.2% | 30.0 | 0.0 | 139.2 ms |
| wide+sources-reason | turns (method=dense, k=40) | 100.0% | 0.0 | 40.0 | 45.1 ms |
| wide+sources-reason | facts (method=hybrid, k=30) | 68.2% | 30.0 | 0.0 | 135.7 ms |
| wide+sources-reason | fact_sources (max=10) | 0.0% | 0.0 | 5.5 | 0.0 ms |

"Adversarial" and "abstention" questions have no answer in the history; they are scored on whether the pipeline says so.
