# Evaluation report

Dataset: LongMemEval-S. 500 questions (seed 7); histories average about 126,460 tokens.
Reader and extraction model: `deepseek-flash` (reasoning off). Judge: `deepseek-flash` (the same model as the reader, so a cross-check and not an independent measurement). Embeddings: `http`.

| Pipeline | F1 (95% interval) | Judged correct (95% interval) | Answer turn in context | Prompt tokens | Model calls per question | Retrieval p50 / p95 | Write-path LLM calls |
|---|---|---|---|---|---|---|---|
| raw-bm25 | 50.5 (46.6–54.4) | 65.8% (62.0–70.2) | 90.8% | 5,054 | 1.0 | 0.7 / 1.6 ms | 0.0 |
| raw-dense | 52.9 (49.3–56.8) | 67.6% (63.6–71.8) | 98.8% | 4,143 | 1.0 | 33.0 / 62.4 ms | 0.0 |
| raw-hybrid | 51.6 (47.8–55.3) | 67.2% (63.2–71.2) | 96.0% | 4,818 | 1.0 | 33.7 / 44.1 ms | 0.0 |
| dense-wide-reason | 62.1 (58.8–65.3) | 88.2% (85.2–90.8) | 99.6% | 8,029 | 1.0 | 32.7 / 45.3 ms | 0.0 |
| rerank-reason | 62.2 (58.9–65.4) | 88.6% (86.0–91.2) | 99.2% | 10,558 | 2.0 | 34.2 / 1281.2 ms | 0.0 |

Answerable questions only (470; without abstention). This is the subset most published figures use:

| Pipeline | F1 (95% interval) | Judged correct (95% interval) |
|---|---|---|
| raw-bm25 | 47.7 (44.0–51.6) | 64.0% (59.8–68.3) |
| raw-dense | 50.1 (46.5–54.0) | 65.7% (61.7–70.0) |
| raw-hybrid | 48.7 (45.1–52.7) | 65.3% (61.1–69.8) |
| dense-wide-reason | 60.5 (57.0–63.8) | 88.3% (85.3–91.1) |
| rerank-reason | 60.2 (56.7–63.5) | 88.3% (85.5–91.1) |

Difference from `raw-dense` on the same questions, in points (95% interval). An interval that excludes zero is a real difference:

| Pipeline | F1 | Judged correct |
|---|---|---|
| raw-bm25 | -2.5 (-4.6 to -0.4) | -1.8 (-4.4 to +1.2) |
| raw-hybrid | -1.4 (-3.3 to +0.3) | -0.4 (-3.0 to +2.4) |
| dense-wide-reason | +9.1 (+6.1 to +12.3) | +20.6 (+16.6 to +24.6) |
| rerank-reason | +9.3 (+6.1 to +12.4) | +21.0 (+17.2 to +25.0) |

Judged correct by category:

| Pipeline | abstention | knowledge-update | multi-session | single-session-assistant | single-session-preference | single-session-user | temporal-reasoning |
|---|---|---|---|---|---|---|---|
| raw-bm25 | 93.3% (n=30) | 87.5% (n=72) | 42.1% (n=121) | 94.6% (n=56) | 46.7% (n=30) | 96.9% (n=64) | 45.7% (n=127) |
| raw-dense | 96.7% (n=30) | 86.1% (n=72) | 44.6% (n=121) | 98.2% (n=56) | 43.3% (n=30) | 95.3% (n=64) | 50.4% (n=127) |
| raw-hybrid | 96.7% (n=30) | 87.5% (n=72) | 47.9% (n=121) | 94.6% (n=56) | 36.7% (n=30) | 95.3% (n=64) | 48.0% (n=127) |
| dense-wide-reason | 86.7% (n=30) | 97.2% (n=72) | 78.5% (n=121) | 98.2% (n=56) | 63.3% (n=30) | 96.9% (n=64) | 89.8% (n=127) |
| rerank-reason | 93.3% (n=30) | 93.1% (n=72) | 80.2% (n=121) | 98.2% (n=56) | 66.7% (n=30) | 98.4% (n=64) | 89.0% (n=127) |

F1 by category:

| Pipeline | abstention | knowledge-update | multi-session | single-session-assistant | single-session-preference | single-session-user | temporal-reasoning |
|---|---|---|---|---|---|---|---|
| raw-bm25 | 93.3 | 74.7 | 22.8 | 79.8 | 6.2 | 88.5 | 31.4 |
| raw-dense | 96.7 | 71.5 | 28.5 | 83.0 | 6.9 | 87.5 | 35.5 |
| raw-hybrid | 96.7 | 69.8 | 27.6 | 79.0 | 4.6 | 87.6 | 34.3 |
| dense-wide-reason | 86.7 | 72.4 | 48.4 | 83.5 | 7.4 | 82.2 | 56.6 |
| rerank-reason | 93.3 | 69.9 | 47.0 | 84.1 | 9.6 | 83.2 | 57.2 |

Oracle router over raw-bm25, raw-dense, raw-hybrid, dense-wide-reason, rerank-reason: F1 71.9, against 62.2 for the best single one; judged 93.8% against 88.6%. That gap is the most a per-question router could add.

What each stage contributed ("reached the answer" = the stage returned a turn holding the answer, or a fact stated in one):

| Pipeline | Stage | Reached the answer | Facts added | Turns added | p50 |
|---|---|---|---|---|---|
| raw-bm25 | turns (method=bm25, k=20) | 90.8% | 0.0 | 20.0 | 0.7 ms |
| raw-dense | turns (method=dense, k=20) | 98.8% | 0.0 | 20.0 | 33.0 ms |
| raw-hybrid | turns (method=hybrid, k=20) | 96.0% | 0.0 | 20.0 | 33.7 ms |
| dense-wide-reason | turns (method=dense, k=40) | 99.6% | 0.0 | 40.0 | 32.7 ms |
| rerank-reason | turns (method=hybrid, k=60) | 99.6% | 0.0 | 60.0 | 33.8 ms |
| rerank-reason | rerank (keep=20) | 99.2% | 0.0 | 20.0 | 0.2 ms |

"Adversarial" and "abstention" questions have no answer in the history; they are scored on whether the pipeline says so.
