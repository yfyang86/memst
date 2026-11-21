# Evaluation report

Dataset: LongMemEval-S. 60 questions (seed 7); histories average about 126,930 tokens.
Reader and extraction model: `deepseek-flash` (reasoning off). Judge: `deepseek-flash` (the same model as the reader, so a cross-check and not an independent measurement). Embeddings: `http`.

| Pipeline | F1 (95% interval) | Judged correct (95% interval) | Answer turn in context | Prompt tokens | Model calls per question | Retrieval p50 / p95 | Write-path LLM calls |
|---|---|---|---|---|---|---|---|
| full-context | 44.8 (33.5–55.7) | 58.3% (46.7–70.0) | 100.0% | 111,534 | 1.0 | 0.4 / 0.5 ms | 0.0 |
| raw-bm25 | 51.4 (40.2–61.5) | 65.0% (53.3–76.7) | 90.0% | 4,897 | 1.0 | 0.9 / 2.5 ms | 0.0 |
| raw-dense | 49.5 (38.4–59.4) | 65.0% (53.3–76.7) | 100.0% | 4,100 | 1.0 | 63.8 / 77.7 ms | 0.0 |
| raw-hybrid | 50.4 (39.4–60.0) | 68.3% (56.7–80.0) | 95.0% | 4,587 | 1.0 | 54.2 / 65.6 ms | 0.0 |
| graph-entities | 38.2 (28.1–48.5) | 51.7% (40.0–63.3) | 63.3% | 2,615 | 1.0 | 84.3 / 101.0 ms | 46.7 |
| graph-facts | 36.6 (25.9–46.5) | 46.7% (33.3–58.3) | 68.3% | 4,360 | 1.0 | 87.9 / 110.4 ms | 46.7 |
| default | 48.9 (38.3–59.2) | 66.7% (55.0–78.3) | 93.3% | 5,336 | 1.0 | 129.2 / 150.1 ms | 46.7 |

Answerable questions only (58; without abstention). This is the subset most published figures use:

| Pipeline | F1 (95% interval) | Judged correct (95% interval) |
|---|---|---|
| full-context | 42.9 (31.6–53.8) | 56.9% (43.1–70.7) |
| raw-bm25 | 49.8 (38.8–60.6) | 63.8% (51.7–75.9) |
| raw-dense | 47.8 (36.7–58.4) | 63.8% (51.7–75.9) |
| raw-hybrid | 48.6 (38.0–59.3) | 67.2% (55.2–77.6) |
| graph-entities | 36.1 (25.6–47.2) | 50.0% (37.9–63.8) |
| graph-facts | 34.4 (22.7–44.4) | 44.8% (31.0–56.9) |
| default | 47.1 (36.6–57.2) | 65.5% (53.4–75.9) |

Difference from `raw-dense` on the same questions, in points (95% interval). An interval that excludes zero is a real difference:

| Pipeline | F1 | Judged correct |
|---|---|---|
| full-context | -4.7 (-13.4 to +3.4) | -6.7 (-20.0 to +5.0) |
| raw-bm25 | +1.9 (-0.8 to +5.0) | +0.0 (-8.3 to +8.3) |
| raw-hybrid | +0.9 (-1.2 to +2.7) | +3.3 (-3.3 to +11.7) |
| graph-entities | -11.3 (-19.1 to -4.0) | -13.3 (-23.3 to -1.7) |
| graph-facts | -12.9 (-22.1 to -4.5) | -18.3 (-30.0 to -8.3) |
| default | -0.6 (-5.8 to +4.8) | +1.7 (-5.0 to +10.0) |

Judged correct by category:

| Pipeline | abstention | knowledge-update | multi-session | single-session-assistant | single-session-preference | single-session-user | temporal-reasoning |
|---|---|---|---|---|---|---|---|
| full-context | 100.0% (n=2) | 60.0% (n=10) | 30.0% (n=10) | 90.0% (n=10) | 40.0% (n=10) | 77.8% (n=9) | 44.4% (n=9) |
| raw-bm25 | 100.0% (n=2) | 80.0% (n=10) | 60.0% (n=10) | 100.0% (n=10) | 0.0% (n=10) | 100.0% (n=9) | 44.4% (n=9) |
| raw-dense | 100.0% (n=2) | 90.0% (n=10) | 50.0% (n=10) | 100.0% (n=10) | 10.0% (n=10) | 100.0% (n=9) | 33.3% (n=9) |
| raw-hybrid | 100.0% (n=2) | 90.0% (n=10) | 60.0% (n=10) | 100.0% (n=10) | 10.0% (n=10) | 100.0% (n=9) | 44.4% (n=9) |
| graph-entities | 100.0% (n=2) | 50.0% (n=10) | 40.0% (n=10) | 90.0% (n=10) | 10.0% (n=10) | 77.8% (n=9) | 33.3% (n=9) |
| graph-facts | 100.0% (n=2) | 50.0% (n=10) | 40.0% (n=10) | 60.0% (n=10) | 10.0% (n=10) | 88.9% (n=9) | 22.2% (n=9) |
| default | 100.0% (n=2) | 80.0% (n=10) | 40.0% (n=10) | 100.0% (n=10) | 20.0% (n=10) | 100.0% (n=9) | 55.6% (n=9) |

F1 by category:

| Pipeline | abstention | knowledge-update | multi-session | single-session-assistant | single-session-preference | single-session-user | temporal-reasoning |
|---|---|---|---|---|---|---|---|
| full-context | 100.0 | 64.8 | 21.0 | 71.7 | 8.9 | 65.6 | 26.1 |
| raw-bm25 | 100.0 | 84.8 | 32.9 | 67.2 | 0.3 | 84.8 | 30.2 |
| raw-dense | 100.0 | 80.6 | 25.7 | 67.2 | 1.1 | 85.4 | 28.5 |
| raw-hybrid | 100.0 | 80.6 | 28.6 | 67.2 | 1.5 | 85.6 | 30.2 |
| graph-entities | 100.0 | 51.7 | 20.7 | 59.7 | 1.1 | 71.6 | 13.1 |
| graph-facts | 100.0 | 46.7 | 26.7 | 30.6 | 1.1 | 77.1 | 27.7 |
| default | 100.0 | 70.6 | 20.7 | 66.1 | 2.4 | 85.6 | 40.6 |

Oracle router over raw-bm25, raw-dense, raw-hybrid, graph-entities, graph-facts, default: F1 56.8, against 51.4 for the best single one; judged 78.3% against 68.3%. That gap is the most a per-question router could add.

What each stage contributed ("reached the answer" = the stage returned a turn holding the answer, or a fact stated in one):

| Pipeline | Stage | Reached the answer | Facts added | Turns added | p50 |
|---|---|---|---|---|---|
| full-context | all_turns | 100.0% | 0.0 | 497.1 | 0.0 ms |
| raw-bm25 | turns (method=bm25, k=20) | 90.0% | 0.0 | 20.0 | 0.9 ms |
| raw-dense | turns (method=dense, k=20) | 100.0% | 0.0 | 20.0 | 63.7 ms |
| raw-hybrid | turns (method=hybrid, k=20) | 95.0% | 0.0 | 20.0 | 54.2 ms |
| graph-entities | entities (k=8, expand=True) | 68.3% | 45.2 | 0.0 | 84.2 ms |
| graph-entities | fact_sources (max=12) | 63.3% | 0.0 | 7.1 | 0.0 ms |
| graph-facts | facts (method=hybrid, k=30) | 70.0% | 30.0 | 0.0 | 87.9 ms |
| graph-facts | fact_sources (max=12) | 68.3% | 0.0 | 11.9 | 0.0 ms |
| default | facts (method=hybrid, k=30) | 70.0% | 30.0 | 0.0 | 95.5 ms |
| default | fact_sources (max=8) | 65.0% | 0.0 | 8.0 | 0.0 ms |
| default | turns (method=hybrid, k=12) | 56.7% | 0.0 | 9.7 | 34.1 ms |

"Adversarial" and "abstention" questions have no answer in the history; they are scored on whether the pipeline says so.
