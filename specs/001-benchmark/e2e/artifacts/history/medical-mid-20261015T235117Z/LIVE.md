# bench001 LIVE — `medical-mid`

- **updated:** `2026-10-15T23:51:16Z`
- **started:** `2026-10-15T23:34:25Z`
- **run elapsed:** `16m52s`
- **phase:** `score_parallel` (done)
- **progress:** `50/50`
- **ETA (phase):** `0s`
- **detail:** elapsed=10m01s

## Corpus / chunking

- **docs:** `1`  (done `—`)
- **chunk size / overlap:** `1200` / `100`
- **indexed chunks:** `—`
- **corpus chars:** `1052159`  capped=`False`
- **questions:** `200`

## Pipeline

✓ prepare → ○ ingest_eq → ○ query_parallel → ✓ score_parallel → ○ report  |  ✓ query_eq

## Recent ticks

| at (UTC) | phase | status | detail | eta | run |
|----------|-------|--------|--------|-----|-----|
| 2026-10-15T23:34:25Z | prepare | done | n_questions=200 docs=1 chars=1052159 chunk=1200/100 | — | 0s |
| 2026-10-15T23:41:15Z | query_eq | running | EQ query 200/200 id=Medical-65be7ca5 | 0s | 6m51s |
| 2026-10-15T23:41:15Z | query_eq | done | n=200 | — | 6m51s |
| 2026-10-15T23:51:16Z | score_parallel | running | EQ=done LR=done eval∥=1 | — | 16m52s |
| 2026-10-15T23:51:16Z | score_parallel | done | elapsed=10m01s | — | 16m52s |

## Monitor

```bash
make bench001-watch STAGE=medical-mid
# or:  watch -n 2 cat /Users/raphaelmansuy/Github/03-working/edgequake/specs/001-benchmark/e2e/artifacts/medical-mid/LIVE.md
# or:  tail -f /Users/raphaelmansuy/Github/03-working/edgequake/specs/001-benchmark/e2e/artifacts/medical-mid/logs/progress.jsonl
```
