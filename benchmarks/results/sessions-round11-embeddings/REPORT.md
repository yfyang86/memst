# Memory in use, round 7 (LongMemEval-S questions put to an agent with memory tools): results

16 questions (8 of each of: knowledge-update, multi-session; seed 7). Agent: a plain MCP client with `deepseek-flash`, reading transcripts only. Judge: `deepseek-flash`, the harness's.

| | Judged correct | knowledge-update | multi-session | Prompt tokens per question |
|---|---|---|---|---|
| agent with memory tools | 87.5% (n=16) | 87.5% | 87.5% | 11,204 |
| raw-bm25 | 68.8% (n=16) | 87.5% | 50.0% | 5,063 |
| raw-dense | 75.0% (n=16) | 87.5% | 62.5% | 4,562 |
| dense-wide-reason | 93.8% (n=16) | 100.0% | 87.5% | 8,514 |
| rerank-reason | 93.8% (n=16) | 100.0% | 87.5% | 4,744 |

The agent against each pipeline on the same questions, in points (95% interval):

- against `raw-bm25`: +18.8 (-6.2 to +43.8)
- against `raw-dense`: +12.5 (-12.5 to +37.5)
- against `dense-wide-reason`: -6.2 (-18.8 to +0.0)
- against `rerank-reason`: -6.2 (-18.8 to +0.0)

Reading abstentions fairly, the agent is at 87.5%.

L1: **pass** (agent minus raw-dense: 12.5 points).

The agent, per question: 2.19 model calls, 1.5 tool calls, 6.6 s. 0 question(s) had an error; 0 got no answer.

Before the question, every entry of the history was embedded with `local`: 291.3 s a history on average.

Against the compared run on the same 16 questions: 87.5% here, 75.0% there; +12.5 points (+0.0 to +31.2). Reading abstentions fairly: 87.5% here, 75.0% there. By type (correct here, there, of): {'knowledge-update': [7.0, 7.0, 8], 'multi-session': [7.0, 5.0, 8]}. Prompt tokens per question: 11,204 here, 11,484 there.
