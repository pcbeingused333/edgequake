# 09 — Cross-ref matrix

Parent: [README](README.md) · Prev: [08 E2E test matrix](08-e2e-test-matrix.md)

Validate with `python3 scripts/validate-cross-ref.py`.

## Law, WHY, finding, edge case, wave, test, lens

| Law | WHY | Findings | Edge cases | Wave | Tests | Lens |
|-----|-----|----------|------------|------|-------|------|
| LAW-171-1 | WHY-171-1 | F-171-02, F-171-07, F-171-08, F-171-09, F-171-10, F-171-18 | EC-171-01, EC-171-02, EC-171-07, EC-171-16, EC-171-34, EC-171-38, EC-171-39 | W2 | T-171-17, T-171-18, T-171-21, T-171-22, T-171-28 | postgres, reliability |
| LAW-171-2 | WHY-171-2 | F-171-01, F-171-04, F-171-05, F-171-06, F-171-17 | EC-171-08, EC-171-09, EC-171-10, EC-171-11, EC-171-12, EC-171-22, EC-171-24, EC-171-25, EC-171-30, EC-171-31, EC-171-37 | W3 | T-171-01, T-171-02, T-171-03, T-171-07, T-171-08, T-171-09, T-171-14, T-171-25, T-171-29 | reliability, full-stack |
| LAW-171-3 | WHY-171-3 | F-171-03, F-171-11, F-171-12 | EC-171-04, EC-171-05, EC-171-06, EC-171-13, EC-171-15, EC-171-17, EC-171-18, EC-171-26, EC-171-28, EC-171-29, EC-171-36, EC-171-40 | W4 | T-171-19, T-171-31, T-171-32, T-171-33, T-171-35 | database, system-design |
| LAW-171-4 | WHY-171-4 | F-171-14, F-171-15, F-171-16 | EC-171-19, EC-171-23, EC-171-33, EC-171-35 | W1, W6 | T-171-10, T-171-20, T-171-23, T-171-24, T-171-36 | product, full-stack |
| LAW-171-5 | WHY-171-5 | F-171-13 | EC-171-14, EC-171-21, EC-171-32 | W5 | T-171-12, T-171-34, T-171-37, T-171-38 | system-design, reliability |
| LAW-171-6 | WHY-171-1 | F-171-09 | EC-171-03, EC-171-20, EC-171-27 | W2 | T-171-15, T-171-16, T-171-26 | postgres, database |

## Finding to symbol

| Finding | Symbol |
|---------|--------|
| F-171-01 | `worker.rs` `Err(_elapsed)` arm |
| F-171-02 | `persist.rs` `persist_with_providers_progress_and_embedder` |
| F-171-03 | `persist.rs` fairness `drop` |
| F-171-04 | `persist.rs` `storage_errors` then `Ok` |
| F-171-05 | `finalize.rs` status `failed` then `Ok` |
| F-171-06 | `ingestion_reliability.rs` `GraphMerge` |
| F-171-07 | `connection.rs` `apply_session_baseline` |
| F-171-08 | idle-in-transaction versus `wait_event_type = Lock` |
| F-171-09 | `statement_timeout.rs` `LocalTimeoutTx` unused by ingest |
| F-171-10 | `ingestion_committer.rs` `FOR UPDATE` |
| F-171-11 | `merger/mod.rs` `merge_with_progress` |
| F-171-12 | `graph/mod.rs` community advisory lock holds a connection |
| F-171-13 | `server.rs` `close_db_pools` before `worker_pool.shutdown` |
| F-171-14 | persist phase logs |
| F-171-15 | elapsed message uses `config.processing_timeout_secs` |
| F-171-16 | no `pg_stat_activity` snapshot |
| F-171-17 | `task.rs` `mark_failed` typed-timeout check |
| F-171-18 | `large_document_profile.rs` `PERSIST_BUFFER_SECS` under the 7200 s floor |

## WHY to incident

| WHY | Incident | Raw |
|-----|----------|-----|
| WHY-171-1 | A, persist silence | [report](00-raw/Ingest%20merge-step%20timeout%20%E2%80%94%20incident%20report.md) |
| WHY-171-2 | A, terminal timeout | [ppd log](00-raw/raw-logs-search-result%281%29.csv) |
| WHY-171-3 | A, shared entity rows | same report, recommendations R4 |
| WHY-171-4 | A, empty search window | same report, observability gaps |
| WHY-171-5 | B, closed pool | [prod log](00-raw/All-Messages-search-result%20%281%29.csv) |

## Wave to recommendation

| Wave | Recommendations | Edge-case map in [06](06-edge-cases.md) |
|------|-----------------|------------------------------------------|
| W0 | this pack | — |
| W1 | R5, R7 | EC-171-33, EC-171-35, EC-171-38 |
| W2 | R1, R3 | EC-171-01, EC-171-02, EC-171-03, EC-171-07, EC-171-08, EC-171-09, EC-171-20, EC-171-27, EC-171-34, EC-171-37, EC-171-39 |
| W3 | R2 | EC-171-10, EC-171-11, EC-171-12, EC-171-22, EC-171-24, EC-171-25, EC-171-30, EC-171-31 |
| W4 | R4 | EC-171-04, EC-171-05, EC-171-06, EC-171-13, EC-171-15, EC-171-16, EC-171-17, EC-171-18, EC-171-26, EC-171-28, EC-171-29, EC-171-36, EC-171-40 |
| W5 | R8 | EC-171-14, EC-171-21, EC-171-32 |
| W6 | R6 | EC-171-19, EC-171-23 |

## Prior specs

| Spec | What this pack takes from it |
|------|------------------------------|
| [SPEC-156](../156-ingestion-first-principles/README.md) | LAW-156-3 single-writer. LAW-171-3 is that law on the merge. |
| [SPEC-150](../150-reliable-migration-system/README.md) | `lock_timeout` already exists for DDL. Ingest DML did not get it. |
| SPEC-112 | `application_name` `edgequake:*`, pool close on shutdown. This pack fixes the order of that close. |
| SPEC-083 | Drain budget. Workers must run that drain before pools close. |
| SPEC-045 | `GraphMerge` stays permanent. New class is added beside it. |
| SPEC-038 | `LargeDocumentProfile` floor of 7200 s. The persist budget is inside it, not instead of it. |
| GH-336 | Server cancel before the Rust future is dropped. `statement_timeout.rs` already says so. |
