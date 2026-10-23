# 08 — E2E test matrix

Parent: [README](README.md) · Prev: [07 Implementation plan](07-implementation-plan.md) · Next: [09 Cross-ref](09-cross-ref.md)

Proof for [06](06-edge-cases.md). Unit tests do not need Mistral. Postgres tests use a held `FOR UPDATE` and the mock provider. A test that needs a database is marked `postgres`. The rest are `--lib`.

## Unit tests

| Id | Crate | Asserts | Covers |
|----|-------|---------|--------|
| T-171-01 | `edgequake-tasks` | Elapsed, `retry_count < max_retries`, transient text → requeue, no `on_permanent_failure` | EC-171-11 inverse, W3 |
| T-171-02 | `edgequake-tasks` | Elapsed at `max_retries` → `on_permanent_failure` | EC-171-11 |
| T-171-03 | `edgequake-tasks` | `persist deadline exceeded` is not permanent | EC-171-08 |
| T-171-04 | `edgequake-tasks` | `55P03` and `lock_not_available` are not permanent | EC-171-08, EC-171-17 |
| T-171-05 | `edgequake-tasks` | `40P01` and `deadlock_detected` are not permanent | EC-171-18 |
| T-171-06 | `edgequake-tasks` | `57014` and `query_canceled` are not permanent | EC-171-37 |
| T-171-07 | `edgequake-tasks` | `knowledge-graph merge error(s) during persist` stays permanent | EC-171-08 |
| T-171-08 | `edgequake-tasks` | Tombstone text stays permanent and beats a deadline prefix if both are present only when the class is `DocumentDeleted` alone. A pure deadline is not deleted. | EC-171-24 |
| T-171-09 | `edgequake-tasks` | Breaker tripped → no requeue on elapsed | EC-171-12 |
| T-171-10 | `edgequake-tasks` | Message contains effective seconds, stage, `attempt N of M` | EC-171-33, EC-171-38 |
| T-171-11 | `edgequake-tasks` | Metadata override is the number in the message, not the config default | EC-171-38 |
| T-171-12 | `edgequake-tasks` | `closed pool` breaks the worker loop. One warning. | EC-171-21 |
| T-171-13 | `edgequake-tasks` | Cancel during persist stays `Cancelled` | EC-171-30 |
| T-171-14 | `edgequake-tasks` | `retry_count` increments once on the elapsed-then-retry path | EC-171-11, F-171-17 |
| T-171-15 | `edgequake-storage` | Literal `1s; DROP TABLE` rejected | EC-171-20 |
| T-171-16 | `edgequake-storage` | Ingest policy rejects `0`. Migrate policy accepts `0`. | EC-171-39 |
| T-171-17 | `edgequake-storage` | `lock_timeout >= statement_timeout` rejected for the ingest pair | EC-171-02 |
| T-171-18 | `edgequake-storage` | Version 160000 does not emit `transaction_timeout`. 170000 does. | EC-171-01 |
| T-171-19 | `edgequake-storage` | Lock SQL is `pg_advisory_xact_lock(hashtext('edgequake.merge'), hashtext($1))` | EC-171-04 |
| T-171-20 | `edgequake-storage` | Snapshot redacts `sk-`. Failure of the query returns `Err` the caller may ignore. | EC-171-19, EC-171-23 |
| T-171-21 | `edgequake-api` | Persist budget 900 clamped under a 120 s task timeout | EC-171-34 |
| T-171-22 | `edgequake-api` | Page count scales the budget and the clamp still holds | EC-171-16 |
| T-171-23 | `edgequake-api` | Two short chunks do not request a summary LLM | EC-171-35 |
| T-171-24 | `edgequake-api` | Phase log helper requires `document_id` and `task_id` fields | EC-171-33, LAW-171-4 |
| T-171-25 | `edgequake-api` | Transient persist error is `Err`, not `Ok` with `final_status = failed` | EC-171-09 |

## Postgres integration

| Id | Asserts | Covers |
|----|---------|--------|
| T-171-26 | Second checkout after `RESET ALL` still shows `lock_timeout` `30s` on `edgequake:ingest` | EC-171-03, EC-171-27 |
| T-171-27 | Committer transaction issues `SET LOCAL lock_timeout` before `FOR UPDATE` | EC-171-17, R3 |
| T-171-28 | Held `FOR UPDATE`: persist returns within the budget, SQLSTATE or `persist deadline exceeded`, and `pg_stat_activity` has no leftover `active` `edgequake:ingest` for that backend | EC-171-07, EC-171-28 |
| T-171-29 | Release the holder during backoff. Second attempt loads the checkpoint, zero extraction calls, document completed, one row per entity | EC-171-10, EC-171-25, EC-171-31 |
| T-171-30 | No checkpoint row: retry still finishes and extraction ran | EC-171-22 |
| T-171-31 | Same workspace: write intervals do not overlap. Different workspaces: they do. | EC-171-15, EC-171-26 |
| T-171-32 | Pool max 1: merge completes. Checked-out count is 0 during the fake LLM. | EC-171-05, EC-171-06, EC-171-40 |
| T-171-33 | Two connections, two processes or two pools: same workspace serializes | EC-171-13 |
| T-171-34 | Abort mid-transaction, then idle timer: no `idle in transaction` row remains past 60 s, and the task is not stuck `processing` with a live lease | EC-171-14 |
| T-171-35 | Fairness permit is taken during extract and is none during the write. A second tenant claims. | EC-171-29 |
| T-171-36 | Snapshot query uses a 5 s timeout. A cancelled snapshot does not change the task outcome. | EC-171-23 |
| T-171-37 | Sweep function on a closed pool returns without an error loop | EC-171-32 |

## Shutdown order

| Id | Asserts | Covers |
|----|---------|--------|
| T-171-38 | A test double records call order: `worker_shutdown` then `pool_close`. The production `main` path is the one under test, not a reimplementation. | EC-171-21, LAW-171-5 |

T-171-12 and T-171-38 together are the incident B proof. T-171-28 and T-171-29 together are the incident A proof.

## Commands

```text
cargo test -p edgequake-tasks --lib
cargo test -p edgequake-storage --lib
cargo test -p edgequake-api --lib
cargo test -p edgequake-storage --test <postgres suite>   # T-171-26 .. T-171-37
cargo clippy --workspace --all-targets -- -D warnings
python3 specs/171-dataperf/scripts/validate-cross-ref.py
```

`make bench` on medical-mid is the uncontended latency check for [EC-171-36](06-edge-cases.md) when W4 lands. It is not a W0 gate.

## Coverage

EC-171-01 T-171-18. EC-171-02 T-171-17. EC-171-03 T-171-26. EC-171-04 T-171-19.
EC-171-05 T-171-32. EC-171-06 T-171-32. EC-171-07 T-171-28. EC-171-08 T-171-03 T-171-04 T-171-07.
EC-171-09 T-171-25. EC-171-10 T-171-29. EC-171-11 T-171-02 T-171-14. EC-171-12 T-171-09.
EC-171-13 T-171-33. EC-171-14 T-171-34. EC-171-15 T-171-31. EC-171-16 T-171-22.
EC-171-17 T-171-04 T-171-27. EC-171-18 T-171-05. EC-171-19 T-171-20. EC-171-20 T-171-15.
EC-171-21 T-171-12 T-171-38. EC-171-22 T-171-30. EC-171-23 T-171-20 T-171-36. EC-171-24 T-171-08.
EC-171-25 T-171-29. EC-171-26 T-171-31. EC-171-27 T-171-26. EC-171-28 T-171-28.
EC-171-29 T-171-35. EC-171-30 T-171-13. EC-171-31 T-171-29. EC-171-32 T-171-37.
EC-171-33 T-171-10 T-171-24. EC-171-34 T-171-21. EC-171-35 T-171-23. EC-171-36 T-171-19.
EC-171-37 T-171-06. EC-171-38 T-171-11. EC-171-39 T-171-16. EC-171-40 T-171-32.
