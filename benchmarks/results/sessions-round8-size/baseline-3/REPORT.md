# Memory in use, round 3 (memory at size): results

Agent: Pi 0.73.1 with `deepseek-proxy/deepseek-flash`, with memory, no offline pass. 40 scenario runs; 0 had a session that failed.

Correct final answers, by how many other notes the memory holds:

| Scenario | 5000 notes |
|---|---|
| recall | 10/10 |
| update | 9/10 |
| stale | 9/10 |
| abstain | 10/10 |

| # | Result | Detail |
|---|---|---|
| S1 | **pass** | recall: 10/10 at 5000 notes |
| S2 | **pass** | update: 9/10 at 5000 notes |
| S3 | **pass** | stale: 9/10 at 5000 notes |
| S4 | **pass** | abstain: 10/10 at 5000 notes |
| S5 | **pass** | prompt tokens of the final session: 28,197 at 5000 notes |

Final session, on average:

| Notes | Prompt tokens | Model calls | Tool calls | Seconds |
|---|---|---|---|---|
| 5000 | 28,197 | 3.4 | 2.88 | 8.4 |
