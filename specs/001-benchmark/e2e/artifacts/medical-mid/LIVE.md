# bench001 LIVE — `medical-mid`

- **updated:** `2026-10-19T14:12:11Z`
- **started:** `2026-10-19T14:11:23Z`
- **run elapsed:** `29m47s`
- **phase:** `report` (done)
- **progress:** `50/50`
- **ETA (phase):** `0s`
- **detail:** valid=True archive=medical-mid-20261019T141211Z

## Corpus / chunking

- **docs:** `—`  (done `—`)
- **chunk size / overlap:** `—` / `—`
- **indexed chunks:** `—`
- **corpus chars:** `—`  capped=`None`
- **questions:** `—`

## Pipeline

○ prepare → ○ ingest_eq → ○ query_parallel → ✓ score_parallel → ✓ report

## Recent ticks

| at (UTC) | phase | status | detail | eta | run |
|----------|-------|--------|--------|-----|-----|
| 2026-10-19T14:12:11Z | score_parallel | running | EQ=done LR=done eval∥=24 | — | 29m46s |
| 2026-10-19T14:12:11Z | score_parallel | done | elapsed=1m45s | — | 29m46s |
| 2026-10-19T14:12:11Z | report | done | valid=True archive=medical-mid-20261019T141211Z | — | 29m47s |

## Monitor

```bash
make bench001-watch STAGE=medical-mid
# or:  watch -n 2 cat /Users/raphaelmansuy/Github/03-working/edgequake/specs/001-benchmark/e2e/artifacts/medical-mid/LIVE.md
# or:  tail -f /Users/raphaelmansuy/Github/03-working/edgequake/specs/001-benchmark/e2e/artifacts/medical-mid/logs/progress.jsonl
```
