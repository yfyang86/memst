# Memory in use, round 3 (memory at size): results

Agent: Pi 0.73.1 with `deepseek-proxy/deepseek-flash`, with memory, no offline pass. 20 scenario runs; 0 had a session that failed.

Correct final answers, by how many other notes the memory holds:

| Scenario | 500 notes | 5000 notes |
|---|---|---|
| recall | 0/0 | 0/0 |
| update | 0/0 | 0/0 |
| stale | 9/10 | 9/10 |
| abstain | 0/0 | 0/0 |

| # | Result | Detail |
|---|---|---|
| S1 | **fail** | recall: 0/10 at 500 notes; 0/10 at 5000 notes |
| S2 | **fail** | update: 0/10 at 500 notes; 0/10 at 5000 notes |
| S3 | **pass** | stale: 9/10 at 500 notes; 9/10 at 5000 notes |
| S4 | **fail** | abstain: 0/10 at 500 notes; 0/10 at 5000 notes |
| S5 | **pass** | prompt tokens of the final session: 22,282 at 500 notes; 23,527 at 5000 notes |

Final session, on average:

| Notes | Prompt tokens | Model calls | Tool calls | Seconds |
|---|---|---|---|---|
| 500 | 22,282 | 3.5 | 3.0 | 7.4 |
| 5000 | 23,527 | 3.3 | 2.9 | 9.1 |
