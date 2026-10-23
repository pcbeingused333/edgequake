# 02 — Incident catalogue

Parent: [README](README.md) · Prev: [01 First principles](01-first-principles.md) · Next: [03 Code facts](03-code-facts.md)

Two deployments, one day. Raw files stay in [00-raw](00-raw/). This page is the index. Code that still matches the logs is [03](03-code-facts.md).

## Incident A — PPD persist stall

| Field | Value |
|-------|--------|
| When | 2026-10-21 06:56:07 Z to 08:56:07 Z |
| Host | `quantalogic-ppd-edgequake`, worker 1 |
| Document | `01a15d3e-edbe-730c-b1da-0481207f4915` (`Cv_FR_2_.pdf`) |
| Task | `insert-7fb6804f-7e7c-47b1-8dee-363d0859e67b` |
| Tenant | `be1dd9bb-c64b-454f-a98c-ab1539bed6cc` |
| Workspace | `6bd5cbe4-7fa9-44a0-92ec-7d9f0238e8f4` |
| Providers | extraction and embedding `mistral` / `mistral-small-latest` |
| Result | `Task processing timed out after 7200 seconds`, document `failed`, `retry_count: 1` |

Source narrative: [Ingest merge-step timeout — incident report](00-raw/Ingest%20merge-step%20timeout%20%E2%80%94%20incident%20report.md).
Source lines: [raw-logs-search-result(1).csv](00-raw/raw-logs-search-result%281%29.csv).

| Time (UTC) | Event |
|------------|--------|
| 06:56:07.746 | Pipeline start. Last useful prepare line. |
| 06:56:32.271 | Extraction done. 2/2 chunks. |
| 06:56:32.273 | Entity embed. 112 unique of 126 mentions. |
| 06:56:32.274 | Relationship embed. 54 unique of 55 mentions. |
| 06:56:33.609 | Checkpoint saved. 2 chunks, 126 entities, 55 relationships, 225781 bytes, `typed_write: true`. Last line from this task. |
| 06:56:33 → 08:56:07 | No further line from this task. 1 h 59 min 34 s. |
| 08:56:07.715 | Worker: permanently failed, `timeout_secs: 7200`. |
| 08:56:07.715 | API: document status `failed`. `retry_count: 1`. Circuit breaker not tripped. |

The gap is 7194 s of silence inside persist, then a timeout that is exactly the worker default. A later upload of the same PDF, alone, completed. The file is not the cause.

WHY: [WHY-171-1](00-why.md), [WHY-171-2](00-why.md), [WHY-171-3](00-why.md), [WHY-171-4](00-why.md).

### Hypotheses

| Id | Hypothesis | Kept? | Why |
|----|------------|-------|-----|
| H-A1 | Row-lock wait on a shared entity or on committer `FOR UPDATE`, with no `lock_timeout` | Kept | Shared CV entities. Solo run succeeded. A true deadlock would have raised `40P01` inside `deadlock_timeout`. |
| H-A2 | Pool slot held while a second acquire waits | Kept | Persist nests merger, committer, and typed-embedding acquires. Postgres cannot see a client-side pool wait. |
| H-A3 | Summary LLM hang | Dropped for this CV | Each call is capped (default 600 s). The 8-fragment / 1200-token gate in `description_merge.rs` is not crossed by a 2-chunk CV. |
| H-A4 | Apache AGE still on the cutover ledger | Check once, not a fix | v0.36.0 product path is plain `eq_graph_*`. Confirm with `SELECT status, age_absent FROM public.eq_graph_cutover WHERE id = 1`. |

## Incident B — Prod closed pool

| Field | Value |
|-------|--------|
| When | 2026-10-21 12:18:52 Z to 12:18:54 Z |
| Host | `quantalogic-prod-edgequake` |
| Workers | 0, 1, 2, 3 |
| Error | `Storage error: Failed to acquire connection for claim_next: attempted to acquire a connection on a closed pool` |
| Also | `stale fairness park sweep failed` with the same closed-pool error |

Source: [All-Messages-search-result (1).csv](00-raw/All-Messages-search-result%20%281%29.csv).

This is not the PPD stall. It is the shutdown order: pools close, then workers are asked to stop. While `Pool::close` waits for a checked-out connection, every worker that polls `claim_next` logs an error and comes back on the next tick. A task that still holds a connection cannot persist its outcome after the pool rejects new acquires. The document stays in processing until lease reclaim, if the process lives that long. Kubernetes will SIGKILL the pod if `pool.close()` is blocked on the stuck backend past `terminationGracePeriodSeconds`.

WHY: [WHY-171-5](00-why.md).

## What the email described

The note says the worker failed to save and the document stayed in processing. Incident A is that save, inside persist, with the checkpoint already on disk. Incident B is a later prod window where workers could not take a connection at all. Both are in scope. Neither is fixed by a larger wall clock.

## Operator queries for the next occurrence

```sql
SELECT status, age_absent FROM public.eq_graph_cutover WHERE id = 1;

SELECT a.pid, a.application_name, a.wait_event_type, a.wait_event,
       now() - a.xact_start AS xact_age, pg_blocking_pids(a.pid) AS blocked_by,
       left(a.query, 120) AS query
FROM pg_stat_activity a
WHERE a.application_name LIKE 'edgequake:%'
  AND (a.wait_event_type = 'Lock' OR now() - a.xact_start > interval '2 minutes');
```

`application_name` is already pinned to `edgequake:ingest` / `edgequake:queue` (SPEC-112). The view is [pg_stat_activity](https://www.postgresql.org/docs/current/monitoring-stats.html#MONITORING-PG-STAT-ACTIVITY-VIEW). Wave W6 runs the same snapshot from the timeout arm, with a 5 s budget, and never fails the handler if the snapshot fails. See [EC-171-19](06-edge-cases.md) and [EC-171-23](06-edge-cases.md).
