# Memory in use, round 7 (LongMemEval-S questions put to an agent with memory tools): results

60 questions (30 of each of: knowledge-update, multi-session; seed 7). Agent: a plain MCP client with `deepseek-flash`, reading transcripts only. Judge: `deepseek-flash`, the harness's.

| | Judged correct | abstention | knowledge-update | multi-session | Prompt tokens per question |
|---|---|---|---|---|---|
| agent with memory tools | 61.7% (n=60) | 0.0% | 81.5% | 55.6% | 6,395 |
| raw-bm25 | 63.3% (n=60) | 83.3% | 85.2% | 37.0% | 4,985 |
| raw-dense | 68.3% (n=60) | 100.0% | 85.2% | 44.4% | 4,413 |
| dense-wide-reason | 91.7% (n=60) | 100.0% | 96.3% | 85.2% | 8,640 |
| rerank-reason | 91.7% (n=60) | 100.0% | 88.9% | 92.6% | 4,952 |

The agent against each pipeline on the same questions, in points (95% interval):

- against `raw-bm25`: -1.7 (-15.0 to +11.7)
- against `raw-dense`: -6.7 (-18.3 to +6.7)
- against `dense-wide-reason`: -30.0 (-41.7 to -16.7)
- against `rerank-reason`: -30.0 (-41.7 to -18.3)

L1: **fail** (agent minus raw-dense: -6.7 points).

The agent, per question: 2.4 model calls, 1.9 tool calls, 3.4 s. 0 question(s) had an error; 0 got no answer.
