# Memory in use, round 6 (facts said in passing, over a long history): results

Client: a plain MCP client (no extension), model `deepseek-flash`. 8 histories of 24 sessions; 96 questions; 0 had a session that failed.

| Kind of fact | Correct | Latest value was in a note | Latest value was in a transcript |
|---|---|---|---|
| stable | 0/32 | 0/32 | 0/32 |
| changed-once | 1/24 | 1/24 | 0/24 |
| changed-twice | 3/24 | 3/24 | 0/24 |
| never-told | 16/16 |  |  |

H2: **fail** (stable 0/32; changed-once 1/24; changed-twice 3/24; never-told 16/16; each needs 80%).

A question, on average: 3,802 prompt tokens, 1.93 tool calls, 3.25 s.
