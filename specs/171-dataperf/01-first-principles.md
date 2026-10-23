# 01 — First principles

Parent: [README](README.md) · Prev: [00 WHY](00-why.md) · Next: [02 Incident catalogue](02-incident-catalogue.md)

These laws are the contract for [07 Implementation plan](07-implementation-plan.md). Each one answers a WHY in [00](00-why.md). Findings that violate them are in [03](03-code-facts.md).

## LAW-171-1 — The server cancels first

Every blocking Postgres wait on the ingest path has a server-side deadline. A Rust `tokio::time::timeout` is only the outer bound. It must lose the race: the backend is already cancelled, or the connection is closed, before the future is abandoned.

WHY: [WHY-171-1](00-why.md). A client that stops waiting does not stop Postgres (GH-336, already written in `statement_timeout.rs`). `idle_in_transaction_session_timeout` does not cover an `active` lock wait. Official definitions: [Client Connection Defaults](https://www.postgresql.org/docs/current/runtime-config-client.html).

Do not raise `TASK_PROCESSING_TIMEOUT_SECS`. The 7200 s floor in `LargeDocumentProfile` is the outer budget for conversion and extraction. It is not a persist budget.

## LAW-171-2 — A deadline is transient until the budget is spent

A persist deadline, `lock_not_available` (`55P03`), `deadlock_detected` (`40P01`), and `query_canceled` (`57014`) are transient while `retry_count < max_retries` and the circuit breaker is closed. The retry loads the checkpoint and does not pay for extraction again. Exhausted retries, `GraphMerge`, deleted documents, and real data errors stay permanent.

WHY: [WHY-171-2](00-why.md). The classifier order is an edge case: [EC-171-08](06-edge-cases.md). `GraphMerge` remains permanent (SPEC-045). A new class is added. The old class is not redefined.

## LAW-171-3 — One writer per workspace, and only for the write

One workspace has one graph-merge writer at a time, across API replicas. Extraction stays concurrent. The summary LLM stays outside the lock. The lock does not pin a pool connection outside the write transaction.

WHY: [WHY-171-3](00-why.md). This extends SPEC-156 LAW-156-3 (side effects are idempotent and single-writer). The product lock is `pg_advisory_xact_lock(hashtext('edgequake.merge'), hashtext(workspace_id))` inside the write transaction ([advisory locks](https://www.postgresql.org/docs/current/functions-admin.html#FUNCTIONS-ADVISORY-LOCKS)). An in-process `Mutex` does not span replicas. It may exist only as a test double.

The fast path (this incident: under 8 description fragments and under 1200 tokens) takes the lock, upserts, and commits. The slow path reads under the lock, drops it, calls the LLM, then writes back with an optimistic check. See [05](05-architecture.md).

## LAW-171-4 — The stall is nameable

A phase log line carries `document_id` and `task_id` as top-level fields. A timeout message names the last progress stage and reads `attempt N of M`. It does not say "permanently failed" when a retry will run. `retry_count` after `mark_failed` is not a retry.

WHY: [WHY-171-4](00-why.md).

## LAW-171-5 — Pools close after workers stop

Workers stop claiming, and in-flight tasks are cancelled or requeued, before `Pool::close`. `PoolClosed` on `claim_next` breaks the `'worker` loop. It is not an error on every poll. The fairness sweep treats the same error as shutdown.

WHY: [WHY-171-5](00-why.md). sqlx `Pool::close` rejects new acquires immediately and waits for connections already checked out ([Pool::close](https://docs.rs/sqlx/latest/sqlx/struct.Pool.html#method.close)).

## LAW-171-6 — One helper

One module owns timeout literals. One module owns the failure class. One function takes the workspace merge lock. Callers do not format `SET` strings and do not invent a second classifier or a second advisory-lock style.

The literal helper is the one in `statement_timeout.rs`, promoted so migrate, vector DDL, and ingest all call it. `sanitize_pg_setting` in `vector/ddl.rs` and `validate_timeout_literal` in the migrate runner are the duplicates this law retires. A timeout string that is not `0` or a short unit literal is rejected. User text is never interpolated.

## How the laws fit together

```text
┌────────────┐   ┌────────────┐   ┌────────────┐   ┌────────────┐
│ LAW-171-1  │   │ LAW-171-2  │   │ LAW-171-3  │   │ LAW-171-5  │
│ server     │   │ transient  │   │ one writer │   │ drain then │
│ cancels    │   │ deadline   │   │ per write  │   │ close pool │
└────────────┘   └────────────┘   └────────────┘   └────────────┘
      │                │                │                │       
      v                v                v                v       
┌───────────────────────────────────────────────────────────────┐
│ LAW-171-4 names the stage    LAW-171-6 one helper             │
└───────────────────────────────────────────────────────────────┘
```

## DRY and SOLID, as laws

| Principle | Where it binds |
|-----------|----------------|
| SRP | Deadline policy, session GUCs, the advisory lock, and shutdown order each have one owner. |
| OCP | Add `IngestionFailureClass::PersistContention`. Do not change the meaning of `GraphMerge`. |
| LSP | A transient persist error is a `TaskError`. It is not an `Ok` result that happens to carry `final_status = failed`. |
| ISP | The activity snapshot is a function with its own 5 s budget. The timeout arm does not know the SQL. |
| DIP | The worker depends on the classifier in `ingestion_reliability`. It does not depend on sqlx. |
| DRY | One literal helper. One retry arm. The elapsed arm calls that arm. |

Edge cases for each law are [06](06-edge-cases.md). Tests are [08](08-e2e-test-matrix.md). The matrix is [09](09-cross-ref.md).
