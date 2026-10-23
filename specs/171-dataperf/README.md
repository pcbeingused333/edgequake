# SPEC-171 — Persist reliability and Postgres bounds

> **Status:** Implemented (W1–W6). Unit gates and `e2e_spec171_persist` (lock re-pin and workspace merge lock) passed on 2026-10-23.
> **Date:** 2026-10-23
> **Product pin:** EdgeQuake v0.36.0 (schema 214 on the incident hosts)
> **Postgres:** 16, 17, and 18 ([version matrix](../../docs/data-layer/version-matrix.md)). `current` docs are PostgreSQL 18.
> **Source:** [00-raw](00-raw/) — PPD merge stall and prod closed-pool logs, 2026-10-21
> **Inherits:** [SPEC-156](../156-ingestion-first-principles/README.md) LAW-156-3 (single-writer) · [SPEC-150](../150-reliable-migration-system/README.md) (lock timeout on DDL, not on ingest DML)
> **Peers:** SPEC-112 (pool labels, `application_name`) · SPEC-083 (shutdown drain) · SPEC-045 (failure classes) · SPEC-038 (`LargeDocumentProfile`) · GH-336 (server cancel before the Rust future is dropped)

## What operators and implementers need to know

| Goal | Do this |
|------|---------|
| Understand why a document sat in processing for two hours | Read [00-why](00-why.md) |
| Know the laws | Read [01-first-principles](01-first-principles.md) |
| See both incidents | Read [02-incident-catalogue](02-incident-catalogue.md) |
| See the code that matches the logs | Read [03-code-facts](03-code-facts.md) |
| See which Postgres timeout covers which wait | Read [04-postgres-theory](04-postgres-theory.md) |
| See the target shape | Read [05-architecture](05-architecture.md) |
| Cover edge cases | Read [06-edge-cases](06-edge-cases.md) |
| Ship in waves | Follow [07-implementation-plan](07-implementation-plan.md) |
| Prove every edge case | Run gates in [08-e2e-test-matrix](08-e2e-test-matrix.md) |
| Check cross-refs | [09-cross-ref](09-cross-ref.md) and `python3 scripts/validate-cross-ref.py` |
| Read one lens | [`lenses/`](lenses/) |

## Start here

1. [00-why.md](00-why.md)
2. [01-first-principles.md](01-first-principles.md)
3. [02-incident-catalogue.md](02-incident-catalogue.md)
4. [03-code-facts.md](03-code-facts.md)
5. [04-postgres-theory.md](04-postgres-theory.md)
6. [05-architecture.md](05-architecture.md)
7. [06-edge-cases.md](06-edge-cases.md)
8. [07-implementation-plan.md](07-implementation-plan.md)
9. [08-e2e-test-matrix.md](08-e2e-test-matrix.md)
10. [09-cross-ref.md](09-cross-ref.md)
11. Lenses: [product](lenses/LENS-product-owner.md) · [full stack](lenses/LENS-full-stack.md) · [database](lenses/LENS-database.md) · [system design](lenses/LENS-system-design.md) · [postgres](lenses/LENS-postgres.md) · [reliability](lenses/LENS-reliability.md)

## One-screen job

```text
┌────────────────────────────────────────────────────────────┐
│ extract + embed + checkpoint                        26 s   │
└────────────────────────────┬───────────────────────────────┘
                             │                                
                             v                                
┌────────────────────────────────────────────────────────────┐
│ persist / merge                                            │
│ no deadline, no lock_timeout, fairness already dropped     │
└────────────────────────────┬───────────────────────────────┘
                             │                                
              ┌──────────────┴──────────────┐                 
              v                             v                 
┌──────────────────────────┐  ┌──────────────────────────────┐
│ target                   │  │ today                        │
│ minutes, then retry      │  │ silence until 7200 s         │
│ from the checkpoint      │  │ permanent, checkpoint unused │
└──────────────────────────┘  └──────────────────────────────┘
```

A second failure the same day: workers call `claim_next` after `Pool::close`. The claim error breaks only the inner attempt. The worker loop keeps polling. See [WHY-171-5](00-why.md).

## Locked decisions

| # | Decision | Law |
|---|----------|-----|
| 1 | Spec number is **171**. Raw evidence stays in `00-raw/`. | — |
| 2 | Every ingest lock wait and every runaway statement dies in the server, before the Rust future is abandoned. | LAW-171-1 |
| 3 | Deadline, `55P03`, `40P01`, and `57014` are transient while retries remain. `GraphMerge` stays permanent. | LAW-171-2 |
| 4 | One workspace has one graph-merge writer across replicas. The lock is `pg_advisory_xact_lock` inside the write transaction. The summary LLM stays outside that transaction. | LAW-171-3 |
| 5 | Phase logs carry top-level `document_id` and `task_id`. Timeout text names the stage and `attempt N of M`. | LAW-171-4 |
| 6 | Workers stop claiming before pools close. `PoolClosed` breaks the worker loop. | LAW-171-5 |
| 7 | One timeout-literal helper, promoted into `statement_timeout.rs`. Callers do not format `SET` strings. | LAW-171-6 |
| 8 | Do not raise `TASK_PROCESSING_TIMEOUT_SECS`. The 7200 s floor is the outer budget, not the persist budget. | LAW-171-1 |
| 9 | No in-process `Mutex` as the product lock. | LAW-171-3 |
| 10 | This pack is the specification. Implementation follows [07](07-implementation-plan.md). | — |

## Non-goals

- Query-path, ANN, and migration rewrites (SPEC-089, SPEC-090, SPEC-150)
- Raising the worker wall clock
- Schema changes, or turning Apache AGE back on
- Holding the workspace lock across the summary LLM
- An in-process mutex as the multi-replica lock (a test double is allowed)

## Recommendations carried forward

R1–R7 are Steven’s list in the [incident report](00-raw/Ingest%20merge-step%20timeout%20%E2%80%94%20incident%20report.md). R8 is the prod closed-pool incident. The wave map is in [07](07-implementation-plan.md).
