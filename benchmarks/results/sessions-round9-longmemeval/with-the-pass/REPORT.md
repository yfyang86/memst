# Memory in use, round 7 (LongMemEval-S questions put to an agent with memory tools): results

30 questions (15 of each of: knowledge-update, multi-session; seed 7). Agent: a plain MCP client with `deepseek-flash`, reading transcripts only. Judge: `deepseek-flash`, the harness's.

| | Judged correct | abstention | knowledge-update | multi-session | Prompt tokens per question |
|---|---|---|---|---|---|
| agent with memory tools | 76.7% (n=30) | 0.0% | 84.6% | 80.0% | 3,949 |
| raw-bm25 | 63.3% (n=30) | 100.0% | 76.9% | 46.7% | 4,926 |
| raw-dense | 66.7% (n=30) | 100.0% | 76.9% | 53.3% | 4,547 |
| dense-wide-reason | 96.7% (n=30) | 100.0% | 100.0% | 93.3% | 8,649 |
| rerank-reason | 93.3% (n=30) | 100.0% | 92.3% | 93.3% | 4,969 |

The agent against each pipeline on the same questions, in points (95% interval):

- against `raw-bm25`: +13.3 (-3.3 to +33.3)
- against `raw-dense`: +10.0 (-6.7 to +30.0)
- against `dense-wide-reason`: -20.0 (-36.7 to -6.7)
- against `rerank-reason`: -16.7 (-30.0 to -3.3)

L1: **pass** (agent minus raw-dense: 10.0 points).

The agent, per question: 2.07 model calls, 1.47 tool calls, 2.5 s. 0 question(s) had an error; 0 got no answer.

Before the question, the offline pass read the history in 4.6 passes on average (49.4 s) and left 56.4 notes and 20.3 pages.

Against the compared run on the same 30 questions: 76.7% here, 70.0% there; +6.7 points (-6.7 to +20.0).
