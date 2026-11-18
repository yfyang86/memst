# Evaluation report

Dataset: LoCoMo. 138 questions (seed 7); histories average about 22,179 tokens.
Reader and extraction model: `deepseek-flash` (reasoning off). Judge: `deepseek-flash` (the same model as the reader, so a cross-check and not an independent measurement). Embeddings: `local`.

| Pipeline | F1 (95% interval) | Judged correct (95% interval) | Answer turn in context | Prompt tokens | Model calls per question | Retrieval p50 / p95 | Write-path LLM calls |
|---|---|---|---|---|---|---|---|
| dense-wide-reason | 59.6 (52.6–65.8) | 76.1% (68.8–82.6) | 83.8% | 2,109 | 1.0 | 40.0 / 66.7 ms | 0.0 |
| default | 52.8 (45.3–59.7) | 63.8% (55.8–71.7) | 70.6% | 1,488 | 1.0 | 78.7 / 127.3 ms | 4.3 |
| default-reason | 56.2 (48.8–63.3) | 68.1% (60.9–75.4) | 70.6% | 1,544 | 1.0 | 82.1 / 153.0 ms | 4.3 |
| sources-only-reason | 56.3 (49.5–63.0) | 71.0% (63.0–78.3) | 70.6% | 1,155 | 1.0 | 80.9 / 150.0 ms | 4.3 |
| wide+sources-reason | 60.2 (53.3–66.6) | 76.8% (69.6–83.3) | 85.3% | 2,536 | 1.0 | 96.9 / 148.7 ms | 4.3 |
| timeline-only-reason | 43.1 (35.7–50.0) | 48.5% (40.6–56.5) | 36.0% | 1,400 | 1.0 | 42.2 / 66.0 ms | 4.3 |

Answerable questions only (108; without adversarial). This is the subset most published figures use:

| Pipeline | F1 (95% interval) | Judged correct (95% interval) |
|---|---|---|
| dense-wide-reason | 52.1 (45.0–59.6) | 73.2% (64.8–81.5) |
| default | 43.3 (35.8–51.2) | 57.4% (49.1–66.7) |
| default-reason | 46.9 (39.4–54.9) | 62.0% (53.7–71.3) |
| sources-only-reason | 47.9 (40.8–55.4) | 66.7% (58.3–75.0) |
| wide+sources-reason | 52.9 (46.0–60.7) | 74.1% (65.7–82.4) |
| timeline-only-reason | 28.2 (21.4–34.8) | 35.2% (26.9–43.5) |

Difference from `dense-wide-reason` on the same questions, in points (95% interval). An interval that excludes zero is a real difference:

| Pipeline | F1 | Judged correct |
|---|---|---|
| default | -6.9 (-12.2 to -1.5) | -12.3 (-19.6 to -5.8) |
| default-reason | -3.4 (-7.7 to +1.4) | -8.0 (-13.8 to -1.5) |
| sources-only-reason | -3.3 (-7.9 to +1.0) | -5.1 (-10.9 to +0.0) |
| wide+sources-reason | +0.6 (-2.7 to +4.0) | +0.7 (-2.9 to +4.3) |
| timeline-only-reason | -16.5 (-23.6 to -9.9) | -27.5 (-36.2 to -18.8) |

Judged correct by category:

| Pipeline | adversarial | multi-hop | open-domain | single-hop | temporal |
|---|---|---|---|---|---|
| dense-wide-reason | 86.7% (n=30) | 73.3% (n=30) | 44.4% (n=18) | 76.7% (n=30) | 86.7% (n=30) |
| default | 86.7% (n=30) | 46.7% (n=30) | 27.8% (n=18) | 66.7% (n=30) | 76.7% (n=30) |
| default-reason | 90.0% (n=30) | 60.0% (n=30) | 33.3% (n=18) | 63.3% (n=30) | 80.0% (n=30) |
| sources-only-reason | 86.7% (n=30) | 60.0% (n=30) | 38.9% (n=18) | 70.0% (n=30) | 86.7% (n=30) |
| wide+sources-reason | 86.7% (n=30) | 73.3% (n=30) | 44.4% (n=18) | 76.7% (n=30) | 90.0% (n=30) |
| timeline-only-reason | 96.7% (n=30) | 40.0% (n=30) | 22.2% (n=18) | 20.0% (n=30) | 53.3% (n=30) |

F1 by category:

| Pipeline | adversarial | multi-hop | open-domain | single-hop | temporal |
|---|---|---|---|---|---|
| dense-wide-reason | 86.7 | 51.1 | 20.8 | 55.1 | 68.9 |
| default | 86.7 | 40.7 | 12.5 | 47.5 | 60.3 |
| default-reason | 90.0 | 48.0 | 21.2 | 43.7 | 64.3 |
| sources-only-reason | 86.7 | 43.8 | 20.7 | 50.3 | 65.7 |
| wide+sources-reason | 86.7 | 55.7 | 22.5 | 52.3 | 68.9 |
| timeline-only-reason | 96.7 | 34.5 | 13.7 | 17.8 | 41.2 |

Oracle router over dense-wide-reason, default, default-reason, sources-only-reason, wide+sources-reason, timeline-only-reason: F1 71.3, against 60.2 for the best single one; judged 84.8% against 76.8%. That gap is the most a per-question router could add.

What each stage contributed ("reached the answer" = the stage returned a turn holding the answer, or a fact stated in one):

| Pipeline | Stage | Reached the answer | Facts added | Turns added | p50 |
|---|---|---|---|---|---|
| dense-wide-reason | turns (method=dense, k=40) | 83.8% | 0.0 | 40.0 | 40.0 ms |
| default | facts (method=hybrid, k=30) | 40.4% | 30.0 | 0.0 | 40.1 ms |
| default | fact_sources (max=8) | 32.4% | 0.0 | 8.0 | 0.0 ms |
| default | turns (method=hybrid, k=12) | 42.6% | 0.0 | 11.1 | 39.8 ms |
| default-reason | facts (method=hybrid, k=30) | 40.4% | 30.0 | 0.0 | 40.0 ms |
| default-reason | fact_sources (max=8) | 32.4% | 0.0 | 8.0 | 0.0 ms |
| default-reason | turns (method=hybrid, k=12) | 42.6% | 0.0 | 11.1 | 39.6 ms |
| sources-only-reason | facts (method=hybrid, k=30) | 40.4% | 30.0 | 0.0 | 39.9 ms |
| sources-only-reason | fact_sources (max=8) | 32.4% | 0.0 | 8.0 | 0.0 ms |
| sources-only-reason | turns (method=hybrid, k=12) | 42.6% | 0.0 | 11.1 | 41.3 ms |
| wide+sources-reason | turns (method=dense, k=40) | 83.8% | 0.0 | 40.0 | 52.1 ms |
| wide+sources-reason | facts (method=hybrid, k=30) | 40.4% | 30.0 | 0.0 | 42.6 ms |
| wide+sources-reason | fact_sources (max=10) | 2.2% | 0.0 | 7.6 | 0.0 ms |
| timeline-only-reason | facts (method=hybrid, k=40) | 43.4% | 40.0 | 0.0 | 41.9 ms |
| timeline-only-reason | fact_sources (max=12) | 36.0% | 0.0 | 12.0 | 0.0 ms |

"Adversarial" and "abstention" questions have no answer in the history; they are scored on whether the pipeline says so.
