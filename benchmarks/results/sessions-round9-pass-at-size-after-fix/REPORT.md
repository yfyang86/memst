# Memory in use, round 9 (the offline pass over a large memory): results

10 runs: a wrong note and a transcript that contradicts it, among 5,000 other notes and 500 other sessions. One pass with `deepseek-flash`. 0 run(s) had an error.

- P1: **pass** (the wrong note was corrected in 8/10).
- P2: **pass** (the largest prompt was about 17,568 tokens; the limit is 30,000).

A pass read 514.3 transcript entries beside 80.0 of the notes, in a prompt of about 17,534 tokens, and took 4.1 s. Showing it every note, as before this sprint, would have taken about 352,676 tokens for the notes alone.

Besides the seeded note, a pass corrected 0.0 other notes, added 0.2 and wrote 0.1 pages on average, from the filler's sessions.
