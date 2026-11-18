# Evaluation report

Dataset: LongMemEval-S. 18 questions (seed 7); histories average about 126,822 tokens.
Reader and extraction model: `deepseek-flash` (reasoning off). Judge: `deepseek-flash` (the same model as the reader, so a cross-check and not an independent measurement). Embeddings: `local`.

| Pipeline | F1 (95% interval) | Judged correct (95% interval) | Answer turn in context | Prompt tokens | Model calls per question | Retrieval p50 / p95 | Write-path LLM calls |
|---|---|---|---|---|---|---|---|
| full-context | 54.8 (35.8–74.4) | 77.8% (55.6–94.4) | 100.0% | 111,646 | 1.0 | 0.7 / 1.6 ms | 0.0 |
| raw-bm25 | 58.1 (37.4–78.9) | 66.7% (44.4–88.9) | 83.3% | 4,732 | 1.0 | 1.6 / 4.0 ms | 0.0 |
| raw-dense | 57.2 (36.8–78.0) | 66.7% (44.4–88.9) | 100.0% | 4,244 | 1.0 | 112.3 / 185.4 ms | 0.0 |
| raw-hybrid | 57.2 (36.8–78.1) | 72.2% (50.0–88.9) | 94.4% | 4,495 | 1.0 | 47.2 / 76.5 ms | 0.0 |
| dense-wide | 57.4 (36.8–78.2) | 72.2% (50.0–88.9) | 100.0% | 7,701 | 1.0 | 46.1 / 84.2 ms | 0.0 |
| dense-reason | 62.7 (43.3–80.0) | 83.3% (61.1–100.0) | 100.0% | 4,300 | 1.0 | 45.3 / 85.4 ms | 0.0 |
| dense-wide-reason | 62.7 (43.2–80.0) | 83.3% (61.1–100.0) | 100.0% | 7,757 | 1.0 | 45.2 / 79.3 ms | 0.0 |
| rerank | 57.2 (36.8–78.0) | 66.7% (44.4–88.9) | 100.0% | 10,160 | 2.0 | 1219.9 / 2496.5 ms | 0.0 |
| rerank-reason | 68.8 (50.8–85.6) | 88.9% (72.2–100.0) | 100.0% | 10,216 | 2.0 | 47.3 / 77.5 ms | 0.0 |
| full-context-reason | 62.4 (43.0–79.6) | 83.3% (61.1–100.0) | 100.0% | 111,702 | 1.0 | 0.9 / 1.2 ms | 0.0 |

Answerable questions only (16; without abstention). This is the subset most published figures use:

| Pipeline | F1 (95% interval) | Judged correct (95% interval) |
|---|---|---|
| full-context | 49.2 (29.1–71.0) | 75.0% (56.2–93.8) |
| raw-bm25 | 52.9 (30.8–75.8) | 62.5% (37.5–87.5) |
| raw-dense | 51.8 (29.2–75.0) | 62.5% (37.5–87.5) |
| raw-hybrid | 51.8 (29.2–75.0) | 68.8% (43.8–87.5) |
| dense-wide | 52.1 (29.7–75.3) | 68.8% (43.8–87.5) |
| dense-reason | 58.0 (37.0–77.1) | 81.2% (56.2–93.8) |
| dense-wide-reason | 58.0 (36.9–77.1) | 81.2% (56.2–93.8) |
| rerank | 51.8 (29.2–75.0) | 62.5% (37.5–87.5) |
| rerank-reason | 64.9 (45.2–83.9) | 87.5% (68.8–100.0) |
| full-context-reason | 57.7 (36.9–77.1) | 81.2% (56.2–93.8) |

Difference from `raw-dense` on the same questions, in points (95% interval). An interval that excludes zero is a real difference:

| Pipeline | F1 | Judged correct |
|---|---|---|
| full-context | -2.3 (-12.7 to +5.2) | +11.1 (-16.7 to +38.9) |
| raw-bm25 | +1.0 (-1.5 to +4.4) | +0.0 (-16.7 to +16.7) |
| raw-hybrid | +0.0 (+0.0 to +0.1) | +5.6 (+0.0 to +16.7) |
| dense-wide | +0.2 (+0.0 to +0.7) | +5.6 (+0.0 to +16.7) |
| dense-reason | +5.5 (-7.4 to +20.3) | +16.7 (+0.0 to +33.3) |
| dense-wide-reason | +5.5 (-7.4 to +20.3) | +16.7 (+0.0 to +33.3) |
| rerank | +0.0 (+0.0 to +0.0) | +0.0 (+0.0 to +0.0) |
| rerank-reason | +11.7 (-3.7 to +29.6) | +22.2 (+5.6 to +44.4) |
| full-context-reason | +5.2 (-7.7 to +19.2) | +16.7 (+0.0 to +33.3) |

Judged correct by category:

| Pipeline | abstention | knowledge-update | multi-session | single-session-assistant | single-session-preference | single-session-user | temporal-reasoning |
|---|---|---|---|---|---|---|---|
| full-context | 100.0% (n=2) | 66.7% (n=3) | 33.3% (n=3) | 100.0% (n=3) | 66.7% (n=3) | 100.0% (n=2) | 100.0% (n=2) |
| raw-bm25 | 100.0% (n=2) | 100.0% (n=3) | 33.3% (n=3) | 100.0% (n=3) | 33.3% (n=3) | 100.0% (n=2) | 0.0% (n=2) |
| raw-dense | 100.0% (n=2) | 100.0% (n=3) | 33.3% (n=3) | 100.0% (n=3) | 33.3% (n=3) | 100.0% (n=2) | 0.0% (n=2) |
| raw-hybrid | 100.0% (n=2) | 100.0% (n=3) | 66.7% (n=3) | 100.0% (n=3) | 33.3% (n=3) | 100.0% (n=2) | 0.0% (n=2) |
| dense-wide | 100.0% (n=2) | 100.0% (n=3) | 66.7% (n=3) | 100.0% (n=3) | 33.3% (n=3) | 100.0% (n=2) | 0.0% (n=2) |
| dense-reason | 100.0% (n=2) | 100.0% (n=3) | 66.7% (n=3) | 100.0% (n=3) | 33.3% (n=3) | 100.0% (n=2) | 100.0% (n=2) |
| dense-wide-reason | 100.0% (n=2) | 100.0% (n=3) | 66.7% (n=3) | 100.0% (n=3) | 33.3% (n=3) | 100.0% (n=2) | 100.0% (n=2) |
| rerank | 100.0% (n=2) | 100.0% (n=3) | 33.3% (n=3) | 100.0% (n=3) | 33.3% (n=3) | 100.0% (n=2) | 0.0% (n=2) |
| rerank-reason | 100.0% (n=2) | 100.0% (n=3) | 66.7% (n=3) | 100.0% (n=3) | 66.7% (n=3) | 100.0% (n=2) | 100.0% (n=2) |
| full-context-reason | 100.0% (n=2) | 100.0% (n=3) | 66.7% (n=3) | 100.0% (n=3) | 33.3% (n=3) | 100.0% (n=2) | 100.0% (n=2) |

F1 by category:

| Pipeline | abstention | knowledge-update | multi-session | single-session-assistant | single-session-preference | single-session-user | temporal-reasoning |
|---|---|---|---|---|---|---|---|
| full-context | 100.0 | 55.6 | 33.3 | 87.7 | 13.6 | 100.0 | 8.0 |
| raw-bm25 | 100.0 | 88.9 | 33.3 | 84.4 | 8.7 | 100.0 | 0.0 |
| raw-dense | 100.0 | 88.9 | 33.3 | 84.4 | 3.0 | 100.0 | 0.0 |
| raw-hybrid | 100.0 | 88.9 | 33.3 | 84.4 | 3.2 | 100.0 | 0.0 |
| dense-wide | 100.0 | 88.9 | 34.7 | 84.4 | 3.0 | 100.0 | 0.0 |
| dense-reason | 100.0 | 66.7 | 33.3 | 84.4 | 2.9 | 100.0 | 83.3 |
| dense-wide-reason | 100.0 | 66.7 | 33.3 | 84.4 | 2.7 | 100.0 | 83.3 |
| rerank | 100.0 | 88.9 | 33.3 | 84.4 | 3.0 | 100.0 | 0.0 |
| rerank-reason | 100.0 | 66.7 | 66.7 | 84.4 | 6.2 | 100.0 | 83.3 |
| full-context-reason | 100.0 | 66.7 | 33.3 | 82.7 | 2.8 | 100.0 | 83.3 |

Oracle router over raw-bm25, raw-dense, raw-hybrid, dense-wide, dense-reason, dense-wide-reason, rerank, rerank-reason, full-context-reason: F1 74.0, against 68.8 for the best single one; judged 94.4% against 88.9%. That gap is the most a per-question router could add.

What each stage contributed ("reached the answer" = the stage returned a turn holding the answer, or a fact stated in one):

| Pipeline | Stage | Reached the answer | Facts added | Turns added | p50 |
|---|---|---|---|---|---|
| full-context | all_turns | 100.0% | 0.0 | 513.3 | 0.1 ms |
| raw-bm25 | turns (method=bm25, k=20) | 83.3% | 0.0 | 20.0 | 1.5 ms |
| raw-dense | turns (method=dense, k=20) | 100.0% | 0.0 | 20.0 | 112.3 ms |
| raw-hybrid | turns (method=hybrid, k=20) | 94.4% | 0.0 | 20.0 | 47.1 ms |
| dense-wide | turns (method=dense, k=40) | 100.0% | 0.0 | 40.0 | 46.0 ms |
| dense-reason | turns (method=dense, k=20) | 100.0% | 0.0 | 20.0 | 45.2 ms |
| dense-wide-reason | turns (method=dense, k=40) | 100.0% | 0.0 | 40.0 | 45.1 ms |
| rerank | turns (method=hybrid, k=60) | 100.0% | 0.0 | 60.0 | 46.3 ms |
| rerank | rerank (keep=20) | 100.0% | 0.0 | 20.0 | 1176.3 ms |
| rerank-reason | turns (method=hybrid, k=60) | 100.0% | 0.0 | 60.0 | 47.0 ms |
| rerank-reason | rerank (keep=20) | 100.0% | 0.0 | 20.0 | 0.3 ms |
| full-context-reason | all_turns | 100.0% | 0.0 | 513.3 | 0.1 ms |

"Adversarial" and "abstention" questions have no answer in the history; they are scored on whether the pipeline says so.
