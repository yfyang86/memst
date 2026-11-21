# Memory in use, round 7 (LongMemEval-S questions put to an agent with memory tools): results

60 questions (30 of each of: knowledge-update, multi-session; seed 7). Agent: a plain MCP client with `deepseek-flash`, reading transcripts only. Judge: `deepseek-flash`, the harness's.

| | Judged correct | abstention | knowledge-update | multi-session | Prompt tokens per question |
|---|---|---|---|---|---|
| agent with memory tools | 83.3% (n=60) | 0.0% | 96.3% | 88.9% | 11,885 |
| raw-bm25 | 63.3% (n=60) | 83.3% | 85.2% | 37.0% | 4,985 |
| raw-dense | 68.3% (n=60) | 100.0% | 85.2% | 44.4% | 4,413 |
| dense-wide-reason | 91.7% (n=60) | 100.0% | 96.3% | 85.2% | 8,640 |
| rerank-reason | 91.7% (n=60) | 100.0% | 88.9% | 92.6% | 4,952 |

The agent against each pipeline on the same questions, in points (95% interval):

- against `raw-bm25`: +20.0 (+5.0 to +35.0)
- against `raw-dense`: +15.0 (+0.0 to +30.0)
- against `dense-wide-reason`: -8.3 (-20.0 to +3.3)
- against `rerank-reason`: -8.3 (-20.0 to +3.3)

Reading abstentions fairly, the agent is at 93.3%.

L1: **pass** (agent minus raw-dense: 15.0 points).

The agent, per question: 2.32 model calls, 1.77 tool calls, 3.6 s. 0 question(s) had an error; 0 got no answer.

Before the question, every entry of the history was embedded with `http`: 16.7 s a history on average.

Against the compared run on the same 60 questions: 83.3% here, 71.7% there; +11.7 points (+1.7 to +21.7). Reading abstentions fairly: 93.3% here, 81.7% there. By type (correct here, there, of): {'abstention': [0.0, 0.0, 6], 'knowledge-update': [26.0, 25.0, 27], 'multi-session': [24.0, 18.0, 27]}. Prompt tokens per question: 11,885 here, 12,411 there.
