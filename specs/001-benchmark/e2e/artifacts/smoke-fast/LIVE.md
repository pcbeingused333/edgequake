# bench001 LIVE — `smoke-fast`

- **updated:** `2026-10-15T05:59:50Z`
- **started:** `2026-10-15T05:56:23Z`
- **run elapsed:** `0s`
- **phase:** `score_parallel` (running)
- **progress:** `50/50`
- **ETA (phase):** `0s`
- **detail:** judge generation 50/50

## Corpus / chunking

- **docs:** `—`  (done `—`)
- **chunk size / overlap:** `—` / `—`
- **indexed chunks:** `—`
- **corpus chars:** `—`  capped=`None`
- **questions:** `—`

## Pipeline

○ prepare → ○ ingest_eq → ○ query_parallel → ● score_parallel → ○ report

## Recent ticks

| at (UTC) | phase | status | detail | eta | run |
|----------|-------|--------|--------|-----|-----|
| 2026-10-15T05:59:50Z | score_parallel | running | judge generation 50/50 | 0s | 0s |

## Monitor

```bash
make bench001-watch STAGE=smoke-fast
# or:  watch -n 2 cat /Users/raphaelmansuy/Github/03-working/edgequake/specs/001-benchmark/e2e/artifacts/smoke-fast/LIVE.md
# or:  tail -f /Users/raphaelmansuy/Github/03-working/edgequake/specs/001-benchmark/e2e/artifacts/smoke-fast/logs/progress.jsonl
```
