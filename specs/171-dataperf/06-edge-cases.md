# 06 — Edge cases

Parent: [README](README.md) · Prev: [05 Architecture](05-architecture.md) · Next: [07 Implementation plan](07-implementation-plan.md)

Every case has an owner in [07](07-implementation-plan.md) and a proof in [08](08-e2e-test-matrix.md). The mitigation is part of the case. A case with no test is not done.

## Deadlines and server timers

**EC-171-01 — PostgreSQL 16 has no `transaction_timeout`.**
`SET transaction_timeout` on 16 is an error and aborts the connection setup. Detect `server_version_num < 170000` and skip it. `lock_timeout` and `statement_timeout` still apply. The Rust persist deadline bounds the sum of statements.

**EC-171-02 — `lock_timeout` greater than or equal to `statement_timeout` never fires.**
The manual says the statement timer wins first. The ingest statement budget is `B - 1` seconds. `lock_timeout` stays at most 120 s and strictly under that budget. A test sets both equal and expects the helper to reject the pair.

**EC-171-03 — A GUC set only inside one checkout is gone after `RESET ALL`.**
`after_release` clears the session. `lock_timeout` has to be in `apply_session_baseline`, which already re-runs after `RESET ALL`. A test checks the GUC on the second checkout.

**EC-171-07 — Rust timeout without a server cancel leaves a zombie backend.**
The deadline path must observe `55P03` or `57014`, or drop the connection, before it returns the slot to the pool. The proof looks at `pg_stat_activity` for `edgequake:ingest` and expects no `active` row for that backend after the error. This is the GH-336 check.

**EC-171-16 — A large document can honestly need more than 900 s of persist.**
Scale the persist budget with `LargeDocumentProfile` page count, then clamp to `effective_task_timeout - 60`. Never let the inner budget reach the outer one ([EC-171-34](06-edge-cases.md)). Do not raise the 7200 s floor.

**EC-171-34 — Persist budget must stay strictly under the task timeout.**
If an operator sets `EDGEQUAKE_PERSIST_TIMEOUT_SECS` above the task timeout, clamp it. Otherwise the inner deadline never fires and [F-171-01](03-code-facts.md) remains the only outcome.

**EC-171-38 — Metadata `processing_timeout_secs` is the clock that actually ran.**
The elapsed message uses that duration, not `WorkerPoolConfig::processing_timeout_secs`, when the metadata override is present.

**EC-171-39 — `0` and empty do not disable ingest `lock_timeout`.**
PostgreSQL treats `0` as disabled. Empty falls through to 30 s. `0` is rejected and the default is kept. This path has no operator opt-out. Migrate DDL may still use its own literal, through the same helper, where `0` means disabled on purpose. The helper takes a policy argument. Ingest uses `reject_zero`.

**EC-171-20 — A timeout literal cannot carry SQL.**
Values like `1s; DROP TABLE` fail the helper and never reach `SET`. The allowed shape is `0` or a short unit literal (`30s`, `5min`), length under 32, no semicolon.

## Locks and writers

**EC-171-04 — `hashtext` collisions.**
Two-int keys, namespace `edgequake.merge` plus the workspace id. Same residual as the community lock. Do not add a third style. A unit test locks the SQL text to that shape.

**EC-171-05 — The summary LLM stays outside the lock.**
If the 8-fragment or 1200-token gate trips, the connection is not held during the HTTP call. A test spies the pool: checked-out count is 0 while the fake LLM is in progress.

**EC-171-06 — Nested pool acquire.**
The write transaction uses one connection. It does not call another pool acquire before commit. A test with pool size 1 completes a merge. Today that shape can deadlock the pool (H-A2).

**EC-171-13 — Two replicas.**
The lock is `pg_advisory_xact_lock` in Postgres, so two processes serialize. An in-process `Mutex` is not compiled into the product path. A test may use one as a double only when `cfg(test)` and the postgres test is skipped.

**EC-171-15 — Different workspaces do not share the lock.**
Two merges with different workspace ids overlap in time. A test records start and end and expects an overlap.

**EC-171-26 — The same workspace does not overlap.**
Two merges with the same workspace id do not overlap the write transaction. Total time is at least the sum of the solo writes.

**EC-171-17 — An operator leaves `FOR UPDATE` open in `psql`.**
The ingest session hits `lock_timeout` and returns `55P03` inside the persist budget. The task does not sit until 7200 s. After the operator commits, the retry succeeds ([EC-171-25](06-edge-cases.md)).

**EC-171-18 — Deadlock `40P01`.**
PostgreSQL raises it within `deadlock_timeout`. The classifier marks it transient. It does not match `GraphMerge`.

**EC-171-28 — Held row lock fails inside the budget and returns the connection.**
Integration test: begin a transaction, `SELECT … FOR UPDATE` the document row, leave it open, run persist. Expect `persist deadline exceeded` or `55P03` before the budget, and no leftover `active` `edgequake:ingest` backend.

**EC-171-36 — Uncontended lock is one extra statement.**
A solo merge issues the advisory lock and does not take a second round trip for a session-level lock. Medical-mid smoke in the release notes of [07](07-implementation-plan.md) watches ingest latency for a regression on the uncontended path.

**EC-171-40 — No connection checked out across the LLM.**
Same proof as EC-171-05, stated for the pool slot rather than the lock. Both must pass. A slow LLM with a held connection is H-A2.

## Classification and retry

**EC-171-08 — Classifier order.**
`persist deadline exceeded`, `55P03`, `lock_not_available`, `40P01`, `deadlock_detected`, and `57014` / `query_canceled` match `PersistContention` before the `GraphMerge` substrings. The existing sentence `knowledge-graph merge error(s) during persist` stays `GraphMerge` and stays permanent.

**EC-171-09 — Persist must not return `Ok` for a transient failure.**
`storage_errors` plus `final_status = failed` plus `Ok` from finalize ([F-171-04](03-code-facts.md), [F-171-05](03-code-facts.md)) never runs for `PersistContention`. The worker sees `Err`. The document row is not moved to `failed` until retries are exhausted.

**EC-171-10 — Checkpoint retry is idempotent.**
The second persist uses `ON CONFLICT` upserts and the committer’s replace-the-spine write. Entity counts do not double.

**EC-171-11 — Budget spent.**
`retry_count >= max_retries` on a deadline calls `on_permanent_failure`. The document becomes `failed`. The message says `attempt N of M`.

**EC-171-12 — Circuit breaker already open.**
`can_retry()` is false. A deadline does not requeue. The existing breaker tests stay green.

**EC-171-22 — No checkpoint.**
Retry still runs. Extraction runs because the loader finds nothing. That path is not a crash.

**EC-171-24 — Document deleted between checkpoint and retry.**
`DocumentDeleted` stays permanent and wins over `PersistContention` if both could match. Recommended action stays `finish_delete_or_reupload`.

**EC-171-25 — The blocker is gone before the retry fires.**
Release the `psql` lock during backoff. The second attempt logs checkpoint resume, makes no extraction LLM call, and reaches `Document processed successfully`.

**EC-171-30 — Cancel during persist.**
Shutdown or an operator cancel is `Cancelled`, not a retry storm, and not `PersistContention`. The existing cancel arm stays the one that preserves `Cancelled`.

**EC-171-31 — A partial upsert already committed.**
Retry runs the same upserts. `ON CONFLICT` updates in place. The test inserts one entity, fails the next statement, retries, and expects one row.

**EC-171-35 — A 2-chunk CV is not an LLM hang.**
The gate is 8 fragments or 1200 tokens. The incident narrative must not add an LLM timeout as the cause. A test with two chunks and short descriptions does not call the summary LLM.

**EC-171-37 — `57014` is transient.**
`query_canceled` from `statement_timeout` or `transaction_timeout` takes the retry arm while budget remains.

**EC-171-33 — The wall-clock message names the effective seconds and the stage.**
`attempt N of M` is present. The word "permanently" appears only when no retry will run.

## Shutdown and the pool

**EC-171-14 — Drain budget exceeded aborts a worker inside a transaction.**
Abort can leave a backend `idle in transaction` until the client connection drops or `idle_in_transaction_session_timeout` (60 s) fires. Shutdown must cancel the token first, wait the drain, and only then close pools, so the session sees a client disconnect. The test asserts no `idle in transaction` `edgequake:ingest` row past the idle timer after a forced abort.

**EC-171-21 — `PoolClosed` stops the worker loop.**
The error `attempted to acquire a connection on a closed pool` breaks `'worker`, logs one warning, and does not sleep-and-poll. Today it breaks only the inner attempt ([F-171-13](03-code-facts.md)).

**EC-171-32 — The fairness sweep on a closed pool is shutdown, not an error storm.**
The sweep returns. It does not log at `error` on every tick.

**EC-171-27 — Second checkout still has `lock_timeout`.**
After `RESET ALL`, the baseline re-applies `edgequake:ingest` and the lock timeout. A test selects `current_setting('lock_timeout')` on checkout two.

**EC-171-29 — Fairness stays released during persist.**
The permit is not re-acquired for the merge. Another tenant can claim while this workspace waits on its advisory lock. The advisory lock is per workspace, not per tenant.

## Diagnostics

**EC-171-19 — The snapshot query is truncated and has no secrets.**
`left(query, 120)`. No bind values. `application_name LIKE 'edgequake:%'`. Document ids in SQL text are acceptable. Provider keys are not in these statements. If a statement ever contains a secret, the snapshot redacts it. The test feeds a query string with a fake `sk-` token and expects the log to drop it.

**EC-171-23 — The snapshot failing must not fail the handler.**
A down database, a 5 s timeout, or a permission error on `pg_stat_activity` is logged at `warn`. The task still retries or fails according to the deadline, not according to the snapshot.

## Map

| Ids | Wave |
|-----|------|
| EC-171-33, EC-171-35, EC-171-38 | W1 |
| EC-171-01, EC-171-02, EC-171-03, EC-171-07, EC-171-08, EC-171-09, EC-171-20, EC-171-27, EC-171-34, EC-171-37, EC-171-39 | W2 |
| EC-171-10, EC-171-11, EC-171-12, EC-171-22, EC-171-24, EC-171-25, EC-171-30, EC-171-31 | W3 |
| EC-171-04, EC-171-05, EC-171-06, EC-171-13, EC-171-15, EC-171-16, EC-171-17, EC-171-18, EC-171-26, EC-171-28, EC-171-29, EC-171-36, EC-171-40 | W4 |
| EC-171-14, EC-171-21, EC-171-32 | W5 |
| EC-171-19, EC-171-23 | W6 |

W0 is this pack. It has no runtime edge case.
