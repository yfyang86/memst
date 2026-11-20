# Memory in use, round 3 (memory at size): results

Agent: Pi 0.73.1 with `deepseek-proxy/deepseek-flash`, with memory, no offline pass. 80 scenario runs; 0 had a session that failed.

Correct final answers, by how many other notes the memory holds:

| Scenario | 500 notes | 5000 notes |
|---|---|---|
| recall | 10/10 | 10/10 |
| update | 10/10 | 9/10 |
| stale | 9/10 | 10/10 |
| abstain | 10/10 | 9/10 |

| # | Result | Detail |
|---|---|---|
| S1 | **pass** | recall: 10/10 at 500 notes; 10/10 at 5000 notes |
| S2 | **pass** | update: 10/10 at 500 notes; 9/10 at 5000 notes |
| S3 | **pass** | stale: 9/10 at 500 notes; 10/10 at 5000 notes |
| S4 | **pass** | abstain: 10/10 at 500 notes; 9/10 at 5000 notes |
| S5 | **pass** | prompt tokens of the final session: 14,466 at 500 notes; 19,331 at 5000 notes |

Final session, on average:

| Notes | Prompt tokens | Model calls | Tool calls | Seconds |
|---|---|---|---|---|
| 500 | 14,466 | 2.5 | 2.02 | 4.9 |
| 5000 | 19,331 | 2.98 | 2.73 | 7.1 |
