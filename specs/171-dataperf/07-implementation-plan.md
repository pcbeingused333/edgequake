# 07 — Implementation plan

Parent: [README](README.md) · Prev: [06 Edge cases](06-edge-cases.md) · Next: [08 E2E test matrix](08-e2e-test-matrix.md)

Waves are the order of code. Each wave is shippable. Later waves do not reopen an earlier law. Tests for the wave are in [08](08-e2e-test-matrix.md). Do not raise `TASK_PROCESSING_TIMEOUT_SECS`.

## SOLID and DRY, applied

| Rule | What to do | What not to do |
|------|------------|----------------|
| SRP | One module owns the persist budget. One function owns the merge lock. One function owns the activity snapshot. `main` owns shutdown order. | Do not teach `worker.rs` how to format `SET lock_timeout`. |
| OCP | Add `IngestionFailureClass::PersistContention` and match it first. | Do not change `GraphMerge::is_permanent` or its recommended action. |
| LSP | Transient persist is `TaskError`. Callers that match `Ok` keep meaning success. | Do not overload `TextInsertPersisted.final_status = failed` as a retry signal. |
| ISP | The snapshot function returns rows or a warning. It does not decide retry. | Do not pass a `PgPool` into the task state machine. |
| DIP | Worker calls `is_permanent_ingestion_failure`. Storage owns SQLSTATE mapping and returns a message the classifier understands. | Do not parse sqlx errors in `worker.rs`. |
| DRY | Promote the timeout-literal check into `statement_timeout.rs`. Migrate runner, vector DDL, and ingest call it. The elapsed arm calls the existing retry decision. | Do not copy `sanitize_pg_setting`. Do not add a second `tokio::time::timeout` wrapper type. |

`LocalTimeoutTx` stays the read-path helper. Ingest adds `SET LOCAL` through the same literal helper, inside the write transaction. That is extension, not a fork.

## Recommendation map

| Id | Change | Wave |
|----|--------|------|
| R1 | Persist deadline, server cancel first | W2 |
| R2 | Timeout is retryable while budget remains, resume from checkpoint | W3 |
| R3 | `lock_timeout` on the ingest session and `SET LOCAL` in the write | W2 |
| R4 | `pg_advisory_xact_lock` per workspace, LLM outside | W4 |
| R5 | Phase logs with top-level `document_id` and `task_id` | W1 |
| R6 | `pg_stat_activity` snapshot on the deadline | W6 |
| R7 | Stage and `attempt N of M` in the message | W1 |
| R8 | Drain workers, then close pools. `PoolClosed` stops the loop | W5 |

R1–R7 are the incident report. R8 is incident B ([02](02-incident-catalogue.md)).

## W0 — This pack

Documents and `scripts/validate-cross-ref.py`. No product code. Gate: `python3 specs/171-dataperf/scripts/validate-cross-ref.py`.

## W1 — Name the stall

Touch:

- `edgequake-api` persist phase logs, one helper, fields `document_id` and `task_id`
- `edgequake-tasks` elapsed message: effective seconds, last stage, `attempt N of M`
- Do not change retry yet. The elapsed arm stays terminal until W3, so W1 cannot pretend a retry happened

Phases: `generation`, `chunk_vectors`, `merger`, `committer`, `fleet_vectors`. Per-entity progress stays on the WebSocket. The log is phases.

Edge cases: [EC-171-33](06-edge-cases.md), [EC-171-35](06-edge-cases.md), [EC-171-38](06-edge-cases.md).

Gate: `cargo test -p edgequake-tasks --lib` for the message shape, and the log-shape test named in [08](08-e2e-test-matrix.md).

## W2 — Server deadline and a real error

Touch:

- `statement_timeout.rs`: public literal helper with `reject_zero` and `allow_zero` policies. Replace the private checkers in vector DDL and the migrate runner with calls. Behavior of migrate `0` stays "disabled".
- `apply_session_baseline`: for `edgequake:ingest` only, pin `lock_timeout` from `EDGEQUAKE_DB_INGEST_LOCK_TIMEOUT_SECS` (default 30, clamp 1..=120). Re-pin after `RESET ALL`.
- `after_connect`: if `server_version_num >= 170000`, the write transaction may set `transaction_timeout`. Do not set it on 16.
- Committer and the new merge write transaction: `SET LOCAL lock_timeout` and `SET LOCAL statement_timeout` via the helper, before `FOR UPDATE` and before upserts.
- `EDGEQUAKE_PERSIST_TIMEOUT_SECS` default 900, clamp `30 .. effective_task_timeout - 60`. Scale with `LargeDocumentProfile` under that clamp.
- Wrap the persist call in `tokio::time::timeout` only as the outer bound. Prefer the SQLSTATE when the server wins.
- On the Rust deadline, drop the connection instead of returning a live backend to the pool.
- New class `PersistContention`, matched before `GraphMerge`, not permanent.
- Persist returns `TaskError` for that class. It does not push it into `storage_errors`.

Edge cases: [EC-171-01](06-edge-cases.md), [EC-171-02](06-edge-cases.md), [EC-171-03](06-edge-cases.md), [EC-171-07](06-edge-cases.md), [EC-171-08](06-edge-cases.md), [EC-171-09](06-edge-cases.md), [EC-171-20](06-edge-cases.md), [EC-171-27](06-edge-cases.md), [EC-171-34](06-edge-cases.md), [EC-171-37](06-edge-cases.md), [EC-171-39](06-edge-cases.md).

Gate: `cargo test -p edgequake-storage --lib`, `cargo test -p edgequake-tasks --lib`, `cargo test -p edgequake-api --lib`.

## W3 — Retry from the checkpoint

Touch:

- The `Err(_elapsed)` arm calls the same decision as `Ok(Err)`: one `mark_failed` (or the details variant), then retry or permanent. No double increment of `retry_count`.
- A `PersistContention` error while `can_retry()` is true requeues with the existing backoff (`calculate_backoff_delay`). It does not call `on_permanent_failure`.
- Document status stays `indexing` until success or the budget is spent. Finalize’s `failed` write runs only on the permanent path.
- Extraction’s existing checkpoint load is the resume. Do not add a store.
- `Cancelled` still bypasses retry.
- `GraphMerge` and `DocumentDeleted` still do not retry.

Edge cases: [EC-171-10](06-edge-cases.md), [EC-171-11](06-edge-cases.md), [EC-171-12](06-edge-cases.md), [EC-171-22](06-edge-cases.md), [EC-171-24](06-edge-cases.md), [EC-171-25](06-edge-cases.md), [EC-171-30](06-edge-cases.md), [EC-171-31](06-edge-cases.md).

Gate: `cargo test -p edgequake-tasks --lib` plus the checkpoint resume test in [08](08-e2e-test-matrix.md).

## W4 — One writer per workspace

Touch:

- One function `lock_workspace_merge(tx, workspace_id)` issuing the two-key `pg_advisory_xact_lock` from [04](04-postgres-theory.md).
- Fast path: that call inside the upsert transaction, then commit, then the committer transaction.
- Slow path: read version under the lock, commit, LLM with no connection held, write back if the version matches. One extra LLM. A third conflict is `PersistContention`.
- Do not copy the community path that stores a `PgConnection`.
- Do not add a production `tokio::sync::Mutex`.

Edge cases: [EC-171-04](06-edge-cases.md), [EC-171-05](06-edge-cases.md), [EC-171-06](06-edge-cases.md), [EC-171-13](06-edge-cases.md), [EC-171-15](06-edge-cases.md), [EC-171-16](06-edge-cases.md), [EC-171-17](06-edge-cases.md), [EC-171-18](06-edge-cases.md), [EC-171-26](06-edge-cases.md), [EC-171-28](06-edge-cases.md), [EC-171-29](06-edge-cases.md), [EC-171-36](06-edge-cases.md), [EC-171-40](06-edge-cases.md).

Gate: postgres integration tests in [08](08-e2e-test-matrix.md). Uncontended medical-mid smoke (`make bench` is the release Acc gate, not part of W0) must not show a step-change in ingest latency. If it does, the lock statement is the only new round trip. Look there first.

## W5 — Drain, then close

Touch:

- `Server::run` does not close pools.
- `main` calls `worker_pool.shutdown()`, then `close_db_pools()`, then drops the ingest runtime.
- Claim error arm: if the message contains `closed pool`, log one `warn` and `break 'worker`.
- Fairness sweep: the same string returns, no error loop.
- Cancelled in-flight tasks go through the existing cancel path so the lease is cleared or the row is reclaimable. They are not left `processing`.

Edge cases: [EC-171-14](06-edge-cases.md), [EC-171-21](06-edge-cases.md), [EC-171-32](06-edge-cases.md).

Gate: a unit test on the claim loop with a closed pool, and a shutdown-order test that fails if `close` is invoked while a worker is still inside `claim_next`.

## W6 — Snapshot

Touch:

- One function in the storage crate. SQL is the query in [02](02-incident-catalogue.md). `statement_timeout` 5 s via the helper. `left(query, 120)`. Redact `sk-` prefixes.
- The timeout arm logs the rows. Any error from the function is `warn` and is ignored.

Edge cases: [EC-171-19](06-edge-cases.md), [EC-171-23](06-edge-cases.md).

## Done when

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --lib`
- The postgres tests in [08](08-e2e-test-matrix.md) that need a database
- `python3 specs/171-dataperf/scripts/validate-cross-ref.py` still passes
- `.env.example` documents `EDGEQUAKE_PERSIST_TIMEOUT_SECS` and `EDGEQUAKE_DB_INGEST_LOCK_TIMEOUT_SECS` when those waves land

W0 does not edit `.env.example`. Later waves do, in the same change as the knob.
