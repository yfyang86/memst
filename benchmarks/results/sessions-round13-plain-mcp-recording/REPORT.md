# Memory in use, round 6 (facts said in passing, over a long history): results

Client: a plain MCP client (no extension), model `deepseek-flash`. 8 histories of 24 sessions; 96 questions; 0 had a session that failed.

| Kind of fact | Correct | Latest value was in a note | Latest value was in a transcript |
|---|---|---|---|
| stable | 6/32 | 6/32 | 0/32 |
| changed-once | 7/24 | 8/24 | 0/24 |
| changed-twice | 15/24 | 15/24 | 0/24 |
| never-told | 16/16 |  |  |

H2: **fail** (stable 6/32; changed-once 7/24; changed-twice 15/24; never-told 16/16; each needs 80%).

A question, on average: 4,344 prompt tokens, 2.03 tool calls, 2.59 s.
