# 03 — Code facts

Parent: [README](README.md) · Prev: [02 Incident catalogue](02-incident-catalogue.md) · Next: [04 Postgres theory](04-postgres-theory.md)

Line numbers are from this tree on 2026-10-23. Each finding names the law it breaks. The target shape is [05](05-architecture.md).

## Findings

| Id | Fact | Law |
|----|------|-----|
| F-171-01 | Wall-clock timeout is terminal | LAW-171-2 |
| F-171-02 | Persist has no deadline | LAW-171-1 |
| F-171-03 | Fairness is dropped before persist | LAW-171-3 |
| F-171-04 | Persist errors return `Ok` | LAW-171-2 |
| F-171-05 | Finalize returns success after status `failed` | LAW-171-2 |
| F-171-06 | `GraphMerge` is permanent and matches broadly | LAW-171-2 |
| F-171-07 | Ingest baseline has no `lock_timeout` | LAW-171-1 |
| F-171-08 | Idle-in-transaction does not cover lock waits | LAW-171-1 |
| F-171-09 | Server-cancel helper exists and ingest does not use it | LAW-171-1, LAW-171-6 |
| F-171-10 | Committer `FOR UPDATE` has no `SET LOCAL lock_timeout` | LAW-171-1 |
| F-171-11 | Merger upserts are not one locked transaction | LAW-171-3 |
| F-171-12 | Community advisory lock holds a pool connection | LAW-171-3, LAW-171-6 |
| F-171-13 | Pools close before workers stop, and `PoolClosed` does not stop the loop | LAW-171-5 |
| F-171-14 | Persist does not log phases with both ids | LAW-171-4 |
| F-171-15 | Timeout text omits the stage and uses the config clock | LAW-171-4 |
| F-171-16 | No `pg_stat_activity` snapshot at the deadline | LAW-171-4 |
| F-171-17 | Wall-clock `mark_failed` is not a typed timeout | LAW-171-2, LAW-171-4 |
| F-171-18 | `PERSIST_BUFFER_SECS` (600) is swallowed by the 7200 s floor | LAW-171-1 |

## F-171-01 — Wall-clock timeout is terminal

[worker.rs](../../edgequake/crates/edgequake-tasks/src/worker.rs) wraps `process_with_fairness` in `tokio::time::timeout` (about line 908). The effective duration can come from task metadata `processing_timeout_secs`.

The `Ok(Err(e))` arm (about line 946) classifies once, then retries when `auto_retry && can_retry() && !circuit_breaker && !is_permanent`.

The `Err(_elapsed)` arm (about line 1054) builds `Task processing timed out after {config.processing_timeout_secs} seconds`, calls `task.mark_failed`, logs "marking as permanently failed", and calls `processor.on_permanent_failure`. It does not call `can_retry()`.

Incident A hit this arm. `retry_count: 1` is explained by F-171-17, not by a second attempt.

## F-171-02 — Persist has no deadline

[persist.rs](../../edgequake/crates/edgequake-api/src/processor/text_insert/persist.rs) calls `persist_with_providers_progress_and_embedder` (about line 345) with no `tokio::time::timeout`. Success logs `P-G2 persist completed` (about line 394). Incident A never reached that line.

## F-171-03 — Fairness is dropped before persist

The same file drops the `FairnessPermit` before materialize (about lines 23–32), by SPEC-091 so other tenants can claim during the write. That is correct for tenants. It also lets several documents in one workspace enter merge together. LAW-171-3 serializes the write, not the extraction, and does not put the permit back.

Default pool width is `num_cpus * 4`, and the per-tenant cap is three quarters of that ([worker.rs](../../edgequake/crates/edgequake-tasks/src/worker.rs) `Default` for `WorkerPoolConfig`, about lines 263–275).

## F-171-04 — Persist errors return `Ok`

On `Err` from persist, the function pushes `Knowledge graph persist failed: …` into `storage_errors` and continues (about lines 398–403). It then returns `Ok(TextInsertPersisted)` (about line 485), including when `final_status` is `failed` (about lines 468–478).

A `55P03` raised inside persist therefore never becomes the `Ok(Err(e))` arm in the worker. Routing R2 through the worker is not enough. The persist function must return `TaskError` for a transient class. See [EC-171-09](06-edge-cases.md).

## F-171-05 — Finalize returns success after status `failed`

[finalize.rs](../../edgequake/crates/edgequake-api/src/processor/text_insert/finalize.rs) writes the document status, logs "Document finalize completed with failed status" when `final_status == "failed"` (about line 241), and still returns `Ok(json!(…))` (about line 349). The worker then marks the task successful. The document row says failed. No retry is scheduled.

## F-171-06 — `GraphMerge` is permanent and matches broadly

[ingestion_reliability.rs](../../edgequake/crates/edgequake-tasks/src/ingestion_reliability.rs): `IngestionFailureClass::GraphMerge` is permanent (`is_permanent`, about lines 93–104). The matcher (about lines 247–256) includes `knowledge-graph merge error`, `merge error(s) during persist`, `graph error` plus `merge`, typed fleet mirror, relational FK misses, and `documents_valid_status`.

A new message that contains those substrings will not retry. `persist deadline exceeded`, `55P03`, `40P01`, and `57014` must be classified before this arm. `GraphMerge` itself stays permanent. See [EC-171-08](06-edge-cases.md).

## F-171-07 — Ingest baseline has no `lock_timeout`

[connection.rs](../../edgequake/crates/edgequake-storage/src/adapters/postgres/connection.rs) `apply_session_baseline` (about lines 120–141) sets `application_name`, `search_path`, and `idle_in_transaction_session_timeout`. The ingest label is `edgequake:ingest` (`session_application_name`). `after_release` runs `RESET ALL` and re-pins that baseline. A new GUC that is not in the baseline disappears on the next checkout. See [EC-171-03](06-edge-cases.md) and [EC-171-27](06-edge-cases.md).

## F-171-08 — Idle-in-transaction does not cover lock waits

`DEFAULT_IDLE_IN_XACT_TIMEOUT_SECS` is 60. PostgreSQL applies it only when the session is idle inside a transaction, waiting for the next client query ([Client Connection Defaults](https://www.postgresql.org/docs/current/runtime-config-client.html)). A lock wait is `state = active`, `wait_event_type = Lock`. The 60 s timer does not fire. That matches the two-hour silence.

## F-171-09 — Server-cancel helper exists and ingest does not use it

[statement_timeout.rs](../../edgequake/crates/edgequake-storage/src/adapters/postgres/statement_timeout.rs) documents GH-336: `tokio::time::timeout` abandons the future while Postgres keeps executing. `LocalTimeoutTx` applies `SET LOCAL statement_timeout` for interactive reads, with 250 ms headroom. The comment in that file says the worker wall clock is LLM-bound and is not this budget. Ingest DML never calls it.

Two other literal checkers exist and disagree:

- `sanitize_pg_setting` in [ddl.rs](../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector/ddl.rs) (private, alphanumeric only)
- `validate_timeout_literal` in [runner.rs](../../edgequake/crates/edgequake-api/src/state/migration_bootstrap/runner.rs) (allows a space, length under 32)

LAW-171-6 promotes one helper. Callers stop formatting `SET` strings.

## F-171-10 — Committer `FOR UPDATE` has no `SET LOCAL lock_timeout`

[ingestion_committer.rs](../../edgequake/crates/edgequake-storage/src/adapters/postgres/ingestion_committer.rs) takes `FOR UPDATE` on the latest revision (about line 267), on bindings (about line 363), and on the document row (about line 590). No `SET LOCAL lock_timeout` precedes them. DDL already does this dance via `EDGEQUAKE_GRAPH_DDL_LOCK_TIMEOUT` and `EDGEQUAKE_MIGRATE_LOCK_TIMEOUT`. DML does not.

## F-171-11 — Merger upserts are not one locked transaction

[merger/mod.rs](../../edgequake/crates/edgequake-pipeline/src/merger/mod.rs) `merge_with_progress` (about line 874) batches entities and relationships and upserts through the graph storage. Native upserts are `INSERT … ON CONFLICT` in [mutate.rs](../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/nodes_ops/mutate.rs). They are not wrapped in one transaction that holds `pg_advisory_xact_lock`. Two CV merges can interleave row locks on the same `eq_graph_nodes` keys.

The description LLM gate is [description_merge.rs](../../edgequake/crates/edgequake-pipeline/src/merger/description_merge.rs): `DEFAULT_FORCE_LLM_SUMMARY_ON_MERGE = 8`, `DEFAULT_SUMMARY_MAX_TOKENS = 1200`. Incident A’s CV stays on the fast path. The lock must still not surround the slow path. See [05](05-architecture.md).

## F-171-12 — Community advisory lock holds a pool connection

[graph/mod.rs](../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs) takes `pg_try_advisory_lock(hashtext('edgequake.community'), hashtext($1))` and stores the `PgConnection` in a map until release. That is the two-key form LAW-171-3 reuses (`hashtext` is 32-bit; two keys cut the collision class). It is also the hold LAW-171-3 forbids for merge: a checked-out connection across non-SQL work starves the pool (H-A2). Merge uses `pg_advisory_xact_lock`, which dies at commit or rollback ([advisory locks](https://www.postgresql.org/docs/current/functions-admin.html#FUNCTIONS-ADVISORY-LOCKS)).

## F-171-13 — Pools close before workers stop

[server.rs](../../edgequake/crates/edgequake-api/src/server.rs) `close_db_pools` (about line 291) runs before `Server::run` returns. [main.rs](../../edgequake/src/main.rs) calls `worker_pool.shutdown()` only after `server.run().await` (about lines 1746–1750).

[worker.rs](../../edgequake/crates/edgequake-tasks/src/worker.rs) `shutdown` (about line 1127) stops new claims, cancels in-flight work, and joins up to `EDGEQUAKE_SHUTDOWN_DRAIN_SECS` (default 30). That runs too late.

The claim error arm (about lines 536–547) logs `Failed to claim next task`, sleeps 1 s, and `break`s the inner `for _reclaim` loop. The `'worker` loop (about line 482) polls again. `PoolClosed` therefore logs on every tick until the process exits. The fairness sweep is a separate acquire and logs the same way. sqlx text is `attempted to acquire a connection on a closed pool` ([postgres.rs](../../edgequake/crates/edgequake-tasks/src/postgres.rs) about line 790).

## F-171-14 — Persist does not log phases with both ids

`bump_task_progress(task, "extraction", …)` runs at about line 107 of `persist.rs`, before the long call, and `"indexing"` runs only after it returns. Merge progress goes to the WebSocket callback, not to a log line with both ids. A search on top-level `task_id` misses the pipeline span. Incident A’s gap looks empty for that reason.

## F-171-15 — Timeout text omits the stage and uses the config clock

The elapsed arm formats `config.processing_timeout_secs`, not the `timeout_duration` it actually waited (metadata can override). The stage from `bump_task_progress` is not appended. `permanent_failure_reason` (about line 307) is not used, so the line cannot say `attempt N of M`.

## F-171-16 — No activity snapshot

The elapsed arm does not query `pg_stat_activity`. The only moment the blocker is known is discarded. W6 adds a best-effort snapshot. Failure of that query must not change the handler outcome ([EC-171-23](06-edge-cases.md)).

## F-171-17 — Wall-clock `mark_failed` is not a typed timeout

[task.rs](../../edgequake/crates/edgequake-tasks/src/types/task.rs) `mark_failed` (about line 288) increments `retry_count` and clears the lease. It counts a typed timeout only when the text starts with `Operation timed out` or contains `[ingestion_failure_class=Timeout`. Incident A’s text matches neither, so `consecutive_timeout_failures` is reset to 0. The breaker does not trip. The API log shows `retry_count: 1` and `circuit_breaker_tripped: false`. Operators read a retry that did not happen.

`can_retry` (about line 544) requires `Failed`, `retry_count < max_retries`, and `error.retryable`. The elapsed arm never consults it.

## F-171-18 — The 600 s persist buffer never bounds persist

[large_document_profile.rs](../../edgequake/crates/edgequake-api/src/services/large_document_profile.rs): `TASK_TIMEOUT_FLOOR_SECS = 7200` (line 31), `PERSIST_BUFFER_SECS = 600` (line 99). `ingest_timeout_secs` adds the buffer to the extract estimate and then clamps to the floor. A small CV therefore gets a 7200 s ingest budget. The buffer is not an inner deadline. `EDGEQUAKE_PERSIST_TIMEOUT_SECS` (default 900) is a new inner budget, clamped strictly under the effective task timeout. See [EC-171-16](06-edge-cases.md) and [EC-171-34](06-edge-cases.md).

## What is already true and must stay

- Checkpoint save and load exist (`pipeline_checkpoint`, extraction resume). Retry must call them. Do not add a second checkpoint store.
- `ON CONFLICT` upserts are the idempotent write. Retry after a partial batch must hit that path ([EC-171-10](06-edge-cases.md), [EC-171-31](06-edge-cases.md)).
- `application_name` labels exist. Diagnostics filter `edgequake:%`.
- `GraphMerge`, `DocumentDeleted`, and the circuit breaker stay permanent.
