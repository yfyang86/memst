# Memory in use, round 6 (facts said in passing, over a long history): results

Client: Pi with the MemSt extension, model `deepseek-proxy/deepseek-flash`. 8 histories of 24 sessions; 96 questions; 0 had a session that failed.

| Kind of fact | Correct | Latest value was in a note | Latest value was in a transcript |
|---|---|---|---|
| stable | 32/32 | 4/32 | 32/32 |
| changed-once | 24/24 | 22/24 | 24/24 |
| changed-twice | 24/24 | 23/24 | 24/24 |
| never-told | 16/16 |  |  |

H1: **pass** (stable 32/32; changed-once 24/24; changed-twice 24/24; never-told 16/16; each needs 80%).

A question, on average: 6,638 prompt tokens, 1.62 tool calls, 4.54 s.
