# Memory in use, round 7 (LongMemEval-S questions put to an agent with memory tools): results

30 questions (15 of each of: knowledge-update, multi-session; seed 7). Agent: a plain MCP client with `deepseek-flash`, reading transcripts only. Judge: `deepseek-flash`, the harness's.

| | Judged correct | abstention | knowledge-update | multi-session | Prompt tokens per question |
|---|---|---|---|---|---|
| agent with memory tools | 70.0% (n=30) | 0.0% | 92.3% | 60.0% | 4,188 |
| raw-bm25 | 63.3% (n=30) | 100.0% | 76.9% | 46.7% | 4,926 |
| raw-dense | 66.7% (n=30) | 100.0% | 76.9% | 53.3% | 4,547 |
| dense-wide-reason | 96.7% (n=30) | 100.0% | 100.0% | 93.3% | 8,649 |
| rerank-reason | 93.3% (n=30) | 100.0% | 92.3% | 93.3% | 4,969 |

The agent against each pipeline on the same questions, in points (95% interval):

- against `raw-bm25`: +6.7 (-10.0 to +23.3)
- against `raw-dense`: +3.3 (-13.3 to +20.0)
- against `dense-wide-reason`: -26.7 (-43.3 to -10.0)
- against `rerank-reason`: -23.3 (-36.7 to -10.0)

L1: **pass** (agent minus raw-dense: 3.3 points).

The agent, per question: 2.2 model calls, 1.6 tool calls, 2.8 s. 0 question(s) had an error; 0 got no answer.
