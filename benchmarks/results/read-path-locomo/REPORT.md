# Evaluation report

Dataset: LoCoMo. 138 questions (seed 7); histories average about 22,179 tokens.
Reader and extraction model: `deepseek-flash` (reasoning off). Judge: `deepseek-flash` (the same model as the reader, so a cross-check and not an independent measurement). Embeddings: `local`.

| Pipeline | F1 (95% interval) | Judged correct (95% interval) | Answer turn in context | Prompt tokens | Model calls per question | Retrieval p50 / p95 | Write-path LLM calls |
|---|---|---|---|---|---|---|---|
| full-context | 59.2 (52.3–66.0) | 71.0% (63.0–78.3) | 100.0% | 24,698 | 1.0 | 0.3 / 0.4 ms | 0.0 |
| raw-dense | 53.3 (46.1–60.0) | 66.7% (58.7–73.2) | 77.2% | 1,074 | 1.0 | 38.2 / 69.3 ms | 0.0 |
| raw-hybrid | 51.3 (44.5–58.3) | 61.6% (53.6–69.6) | 69.8% | 1,116 | 1.0 | 55.3 / 79.8 ms | 0.0 |
| default | 54.2 (47.4–60.7) | 64.5% (56.5–72.5) | 72.1% | 1,439 | 1.0 | 117.2 / 204.6 ms | 15.3 |
| dense-wide | 56.7 (49.8–63.2) | 71.7% (63.8–79.0) | 83.8% | 2,053 | 1.0 | 35.8 / 63.7 ms | 0.0 |
| dense+neighbors | 54.2 (47.8–60.5) | 67.4% (59.4–74.6) | 86.0% | 1,670 | 1.0 | 40.2 / 96.2 ms | 0.0 |
| rerank | 57.8 (51.2–64.3) | 72.5% (65.2–79.7) | 86.0% | 4,468 | 2.0 | 37.2 / 66.2 ms | 0.0 |
| rerank-strict | 50.6 (43.4–57.6) | 65.9% (58.0–73.9) | 82.3% | 3,788 | 2.0 | 37.2 / 56.8 ms | 0.0 |
| dense-reason | 56.8 (49.9–63.2) | 72.5% (65.2–79.7) | 77.2% | 1,130 | 1.0 | 42.3 / 57.2 ms | 0.0 |
| dense-wide-reason | 59.6 (52.6–65.8) | 76.1% (68.8–82.6) | 83.8% | 2,109 | 1.0 | 34.5 / 59.1 ms | 0.0 |
| rerank-reason | 60.5 (53.7–67.0) | 76.8% (70.3–83.3) | 86.0% | 4,524 | 2.0 | 37.7 / 60.6 ms | 0.0 |
| rerank-dense-reason | 59.6 (52.9–66.0) | 75.4% (68.1–81.9) | 82.3% | 4,413 | 2.0 | 45.2 / 61.0 ms | 0.0 |
| default-reason | 57.6 (50.7–64.4) | 71.0% (63.8–78.3) | 72.1% | 1,495 | 1.0 | 111.6 / 177.9 ms | 15.3 |
| full-context-reason | 63.1 (56.8–70.0) | 77.5% (70.3–84.1) | 100.0% | 24,754 | 1.0 | 0.3 / 0.7 ms | 0.0 |

Answerable questions only (108; without adversarial). This is the subset most published figures use:

| Pipeline | F1 (95% interval) | Judged correct (95% interval) |
|---|---|---|
| full-context | 50.6 (43.2–58.6) | 65.7% (57.4–74.1) |
| raw-dense | 42.1 (35.5–50.3) | 59.3% (50.0–68.5) |
| raw-hybrid | 40.6 (33.2–48.9) | 53.7% (43.5–63.0) |
| default | 43.3 (35.9–50.5) | 56.5% (47.2–65.7) |
| dense-wide | 46.5 (39.2–54.4) | 65.7% (57.4–75.0) |
| dense+neighbors | 44.2 (37.3–51.4) | 61.1% (51.8–69.4) |
| rerank | 48.8 (41.4–57.0) | 67.6% (58.3–76.8) |
| rerank-strict | 40.5 (33.6–48.9) | 60.2% (51.8–69.4) |
| dense-reason | 48.5 (41.3–56.1) | 68.5% (59.3–77.8) |
| dense-wide-reason | 52.1 (45.0–59.6) | 73.2% (64.8–81.5) |
| rerank-reason | 53.2 (45.6–60.6) | 74.1% (65.7–82.4) |
| rerank-dense-reason | 52.1 (44.6–59.7) | 72.2% (63.9–80.6) |
| default-reason | 48.6 (41.3–56.5) | 65.7% (56.5–75.0) |
| full-context-reason | 56.5 (49.7–63.9) | 75.0% (66.7–82.4) |

Difference from `raw-dense` on the same questions, in points (95% interval). An interval that excludes zero is a real difference:

| Pipeline | F1 | Judged correct |
|---|---|---|
| full-context | +5.9 (+0.9 to +11.4) | +4.3 (-3.6 to +12.3) |
| raw-hybrid | -2.0 (-6.3 to +2.6) | -5.1 (-10.9 to +1.5) |
| default | +0.9 (-4.2 to +6.0) | -2.2 (-10.1 to +6.5) |
| dense-wide | +3.4 (+0.0 to +7.4) | +5.1 (+0.0 to +10.9) |
| dense+neighbors | +0.9 (-3.3 to +4.9) | +0.7 (-5.8 to +6.5) |
| rerank | +4.5 (-0.2 to +9.2) | +5.8 (+0.0 to +12.3) |
| rerank-strict | -2.7 (-8.3 to +3.1) | -0.7 (-8.0 to +7.2) |
| dense-reason | +3.5 (-1.0 to +8.3) | +5.8 (+0.7 to +11.6) |
| dense-wide-reason | +6.4 (+1.5 to +11.2) | +9.4 (+3.6 to +15.2) |
| rerank-reason | +7.2 (+1.5 to +12.8) | +10.1 (+3.6 to +17.4) |
| rerank-dense-reason | +6.3 (+0.7 to +11.5) | +8.7 (+2.2 to +15.2) |
| default-reason | +4.3 (-1.4 to +10.2) | +4.3 (-2.9 to +11.6) |
| full-context-reason | +9.8 (+4.2 to +15.3) | +10.9 (+4.3 to +18.8) |

Judged correct by category:

| Pipeline | adversarial | multi-hop | open-domain | single-hop | temporal |
|---|---|---|---|---|---|
| full-context | 90.0% (n=30) | 63.3% (n=30) | 27.8% (n=18) | 96.7% (n=30) | 60.0% (n=30) |
| raw-dense | 93.3% (n=30) | 43.3% (n=30) | 33.3% (n=18) | 76.7% (n=30) | 73.3% (n=30) |
| raw-hybrid | 90.0% (n=30) | 33.3% (n=30) | 27.8% (n=18) | 70.0% (n=30) | 73.3% (n=30) |
| default | 93.3% (n=30) | 46.7% (n=30) | 27.8% (n=18) | 66.7% (n=30) | 73.3% (n=30) |
| dense-wide | 93.3% (n=30) | 63.3% (n=30) | 33.3% (n=18) | 80.0% (n=30) | 73.3% (n=30) |
| dense+neighbors | 90.0% (n=30) | 46.7% (n=30) | 27.8% (n=18) | 83.3% (n=30) | 73.3% (n=30) |
| rerank | 90.0% (n=30) | 63.3% (n=30) | 33.3% (n=18) | 90.0% (n=30) | 70.0% (n=30) |
| rerank-strict | 86.7% (n=30) | 70.0% (n=30) | 27.8% (n=18) | 80.0% (n=30) | 50.0% (n=30) |
| dense-reason | 86.7% (n=30) | 53.3% (n=30) | 44.4% (n=18) | 80.0% (n=30) | 86.7% (n=30) |
| dense-wide-reason | 86.7% (n=30) | 73.3% (n=30) | 44.4% (n=18) | 76.7% (n=30) | 86.7% (n=30) |
| rerank-reason | 86.7% (n=30) | 73.3% (n=30) | 44.4% (n=18) | 83.3% (n=30) | 83.3% (n=30) |
| rerank-dense-reason | 86.7% (n=30) | 70.0% (n=30) | 33.3% (n=18) | 83.3% (n=30) | 86.7% (n=30) |
| default-reason | 90.0% (n=30) | 60.0% (n=30) | 38.9% (n=18) | 66.7% (n=30) | 86.7% (n=30) |
| full-context-reason | 86.7% (n=30) | 73.3% (n=30) | 38.9% (n=18) | 90.0% (n=30) | 83.3% (n=30) |

F1 by category:

| Pipeline | adversarial | multi-hop | open-domain | single-hop | temporal |
|---|---|---|---|---|---|
| full-context | 90.0 | 53.4 | 13.6 | 68.6 | 52.2 |
| raw-dense | 93.3 | 34.7 | 16.0 | 54.5 | 52.8 |
| raw-hybrid | 90.0 | 31.4 | 14.8 | 50.2 | 55.5 |
| default | 93.3 | 43.6 | 12.2 | 44.1 | 60.8 |
| dense-wide | 93.3 | 48.2 | 16.1 | 55.3 | 54.2 |
| dense+neighbors | 90.0 | 42.1 | 12.2 | 57.8 | 52.0 |
| rerank | 90.0 | 46.8 | 15.6 | 66.2 | 53.3 |
| rerank-strict | 86.7 | 50.1 | 14.8 | 54.0 | 33.0 |
| dense-reason | 86.7 | 34.9 | 20.9 | 57.9 | 69.3 |
| dense-wide-reason | 86.7 | 51.1 | 20.8 | 55.1 | 68.9 |
| rerank-reason | 86.7 | 53.5 | 20.0 | 61.3 | 64.7 |
| rerank-dense-reason | 86.7 | 49.5 | 20.6 | 57.3 | 68.3 |
| default-reason | 90.0 | 51.7 | 21.2 | 47.4 | 63.0 |
| full-context-reason | 86.7 | 59.7 | 20.7 | 65.8 | 65.7 |

Oracle router over raw-dense, raw-hybrid, default, dense-wide, dense+neighbors, rerank, rerank-strict, dense-reason, dense-wide-reason, rerank-reason, rerank-dense-reason, default-reason, full-context-reason: F1 76.1, against 63.1 for the best single one; judged 89.9% against 77.5%. That gap is the most a per-question router could add.

What each stage contributed ("reached the answer" = the stage returned a turn holding the answer, or a fact stated in one):

| Pipeline | Stage | Reached the answer | Facts added | Turns added | p50 |
|---|---|---|---|---|---|
| full-context | all_turns | 100.0% | 0.0 | 489.4 | 0.1 ms |
| raw-dense | turns (method=dense, k=20) | 77.2% | 0.0 | 20.0 | 38.1 ms |
| raw-hybrid | turns (method=hybrid, k=20) | 69.8% | 0.0 | 20.0 | 55.2 ms |
| default | facts (method=hybrid, k=30) | 53.7% | 30.0 | 0.0 | 72.8 ms |
| default | fact_sources (max=8) | 44.1% | 0.0 | 8.0 | 0.0 ms |
| default | turns (method=hybrid, k=12) | 33.1% | 0.0 | 10.7 | 45.4 ms |
| dense-wide | turns (method=dense, k=40) | 83.8% | 0.0 | 40.0 | 35.7 ms |
| dense+neighbors | turns (method=dense, k=12) | 68.4% | 0.0 | 12.0 | 40.1 ms |
| dense+neighbors | neighbors (window=1) | 22.1% | 0.0 | 19.0 | 0.0 ms |
| rerank | turns (method=hybrid, k=60) | 87.5% | 0.0 | 60.0 | 36.9 ms |
| rerank | rerank (keep=20) | 86.0% | 0.0 | 20.0 | 0.2 ms |
| rerank-strict | turns (method=hybrid, k=60) | 87.5% | 0.0 | 60.0 | 37.0 ms |
| rerank-strict | rerank (keep=20, fill=False) | 82.3% | 0.0 | 6.4 | 0.2 ms |
| dense-reason | turns (method=dense, k=20) | 77.2% | 0.0 | 20.0 | 42.2 ms |
| dense-wide-reason | turns (method=dense, k=40) | 83.8% | 0.0 | 40.0 | 34.5 ms |
| rerank-reason | turns (method=hybrid, k=60) | 87.5% | 0.0 | 60.0 | 37.4 ms |
| rerank-reason | rerank (keep=20) | 86.0% | 0.0 | 20.0 | 0.2 ms |
| rerank-dense-reason | turns (method=dense, k=60) | 85.3% | 0.0 | 60.0 | 44.9 ms |
| rerank-dense-reason | rerank (keep=20) | 82.3% | 0.0 | 20.0 | 0.2 ms |
| default-reason | facts (method=hybrid, k=30) | 53.7% | 30.0 | 0.0 | 71.0 ms |
| default-reason | fact_sources (max=8) | 44.1% | 0.0 | 8.0 | 0.0 ms |
| default-reason | turns (method=hybrid, k=12) | 33.1% | 0.0 | 10.7 | 40.6 ms |
| full-context-reason | all_turns | 100.0% | 0.0 | 489.4 | 0.1 ms |

"Adversarial" and "abstention" questions have no answer in the history; they are scored on whether the pipeline says so.
