# Lens — Database

Parent: [README](../README.md) · Contract: [07-data-and-db-contract](../07-data-and-db-contract.md) ·
Cost: [16-performance](../16-performance.md)

## What this lens owns

[LAW-174-6](../01-first-principles.md). Row-owned tables carry stored
`classification_rank` and `compartment_ids`. Policies call leakproof
functions as InitPlans. Graph nodes do not get a single mask.

## Lessons already in this repository

| Migration | Lesson for 220 |
|-----------|----------------|
| 096 | `set_config(..., true)` only. Pool release resets |
| 167 | `edgequake_tenant_access` is `NOBYPASSRLS`. Second policy is `AS RESTRICTIVE` |
| 206 | Do not put `jsonb ->>` in a policy. Copy the label onto real columns |
| 208 | `(SELECT current_clearance_rank())`, not a bare call per row |
| 170 | Plain graph tables. Contributions are new rows beside them |

`eq_graph_search_nodes` is security definer and applies the tenant itself.
The enforcement-on search path must apply `clearance_covers` inside that
function or a sibling. Otherwise the owner function skips `clearance_guard`.

## Enable is not the migration

220 creates empty structure. Rank backfill runs while `enforcement` is
still false. The flag flips in the last statement
([RB-174-06](../17-roadblocks.md), [EC-174-41](../12-edge-cases.md)).
NULL rank after that flip fails closed ([EC-174-12](../12-edge-cases.md)).

KV is outside RLS. `load_document_body` must not run on a deny
([RB-174-01](../17-roadblocks.md)). The document list must apply
`clearance_covers` before it fetches metadata blobs
([RB-174-02](../17-roadblocks.md)).

## Predicate

`clearance_covers` is true when enforcement is off. When it is on: rank
compare, `compartments <@ subject`, empty-set bit, steward, break-glass,
and the uploader exception only for rank 5. NULL rank returns false.

Tag delete is `ON DELETE RESTRICT` ([EC-174-11](../12-edge-cases.md)).

## Plan check

W10 EXPLAIN on 2_000 chunks, half hidden, must not be a sequential scan
of `chunk_embeddings` with the predicate applied after `LIMIT`. Iterative
HNSW scan is `SET LOCAL` on the large-visible-set statement only.
<https://github.com/pgvector/pgvector#filtering>
