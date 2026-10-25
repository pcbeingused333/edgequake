# Lens — Database

Parent: [README](../README.md) · Contract: [07-data-and-db-contract](../07-data-and-db-contract.md)

## What this lens owns

[LAW-175-6](../01-first-principles.md), [LAW-175-11](../01-first-principles.md),
and [LAW-175-14](../01-first-principles.md).

Selection stays in `metadata`. The new fact is `model_catalog_policy`.
Secrets stay in `provider_connections`. Probe columns stay there.
`settings` JSONB stays `{}`.

## Migration 221 checks

- Partial unique indexes, not one `UNIQUE` that treats null `workspace_id`
  as distinct.
- `usage` check list matches the enum in [05-resolution-law](../05-resolution-law.md).
- Composite foreign key `(tenant_id, workspace_id)` needs
  `UNIQUE (tenant_id, workspace_id)` on `workspaces`. The primary key is
  `workspace_id` alone (`001_init_database.sql`). Add the unique index
  in the same migration.
- `ON DELETE CASCADE` from tenant and from workspace.
- No backfill. Zero rows means unconstrained ([EC-175-19](../11-edge-cases.md)).
- A ceiling PUT deletes that scope's rows and inserts the body in one
  transaction. A unique conflict rolls back ([EC-175-48](../11-edge-cases.md)).
- RLS is not the isolation check. A foreign tenant id is 404 in the API
  ([EC-175-45](../11-edge-cases.md)).

## What this lens will not do

- Put the model id on `provider_connections`.
- Add RLS in this pack. Isolation stays in the API, matching
  `tenants` and `workspaces` today. A later spec can add policies once
  the table has a proven query shape.
- Delete `llm_cache` on a model change. The hash misses
  ([EC-175-27](../11-edge-cases.md)).

## Answer cache

`hash_query_prompt_with_effort` must take the resolved model id. Keyword
hashes already include the model. Do not add a `model` column if the
hash change is enough.
