# Memory in use, round 4 (what memory held at an earlier time): results

Agent: Pi 0.73.1 with `deepseek-proxy/deepseek-flash`. 60 scenario runs; 0 had a step that failed.

Correct answers (the earlier value, given first), by what the final session could read:

| Scenario | notes | notes+transcripts | notes+asof |
|---|---|---|---|
| asof-said | 8/10 | 10/10 | 10/10 |
| asof-edit | 0/10 | 9/10 | 10/10 |

V1: **pass** (asof-said: 10/10 with the as-of read; asof-edit: 10/10 with the as-of read).

Final session, on average:

| Could read | Prompt tokens | Tool calls | Seconds |
|---|---|---|---|
| notes | 9,486 | 5.2 | 11.7 |
| notes+transcripts | 13,363 | 6.7 | 18.6 |
| notes+asof | 3,342 | 1.05 | 3.3 |
