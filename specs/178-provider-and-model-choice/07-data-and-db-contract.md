# 07 — Data and database contract

Parent: [README](README.md) · Prev: [06-screens](06-screens.md) ·
Next: [08-api-contract](08-api-contract.md)

[LAW-178-4](01-first-principles.md). This pack adds no table, no column,
no index, and no migration. Head stays
[223_spec177_provider_offers.sql](../../edgequake/migrations/223_spec177_provider_offers.sql).

## Why a table would be the wrong fact

The eligible set is a function of rows that already have a writer. A
cache table would have a second writer, would go stale when a ceiling
row changes, and would let a reader disagree with `eligible_models`.
That is the bug this pack exists to close. The set is computed per
request from the rows below.

```text
  provider_offers ------------------+
  provider_offer_constrained       |
  deployment_provider_overrides    +--> offer(scope)
  provider_connections.last_test_ok|
  env presence (process, not a row)|
                                   |
  model_catalog_policy ------------+--> ceiling(scope), narrowing(scope)
                                   |
  models.toml + connection_models -+--> catalog facts
                                   |
                                   v
                            eligible_models
                                   |
           +-----------------------+------------------------+
           |                       |                        |
           v                       v                        v
  workspaces.metadata      tenants.metadata         server_config
  llm_*  llm_roles         default_*                key llm_defaults
  embedding_*  vision_*
  decision_model
```

## Rows a choice reads

| Relation or document | Columns that matter | Scope |
|----------------------|---------------------|-------|
| `provider_offers` | `tenant_id`, `workspace_id`, `provider_slug`, `enabled` | tenant, workspace |
| `tenants.provider_offer_constrained` | the mode bit | tenant, workspace |
| `workspaces.provider_offer_constrained` | the mode bit | workspace |
| `deployment_provider_overrides` | `provider_slug`, `withdrawn` | all three, via the global offer |
| `provider_connections` | `tenant_id` null, `slug`, `last_test_ok` | global offer |
| `connection_models` | `connection_id`, `model`, capability flags, `embedding_dimension` | catalog facts for fleet slugs |
| `model_catalog_policy` | `tenant_id`, `workspace_id`, `provider`, `model`, `usage`, `enabled` | tenant ceiling, workspace narrowing |
| `workspaces.metadata` | `llm_provider`, `llm_model`, `llm_roles`, `embedding_*`, `vision_llm_*`, `decision_model` | the assignment a run reads |
| `tenants.metadata` | `default_llm_*`, `default_embedding_*`, `default_vision_llm_*` | tenant default |
| `server_config` key `llm_defaults` | `ServerLlmDefaults` JSON | deployment default |

`workspaces.settings` and `tenants.settings` stay unused. Choice does
not start writing them.

## Rows a choice writes

Only after `PairVerdict::Eligible`, and only through the writer that
already owns the fact:

| Body | Writer | Stored |
|------|--------|--------|
| Workspace family, roles, embedding, vision, decision | existing workspace update | metadata keys already used |
| Tenant `default_*` | existing tenant update | tenant metadata keys already used |
| Deployment defaults | [`update_llm_defaults`](../../edgequake/crates/edgequake-api/src/handlers/llm_defaults.rs) | `server_config.llm_defaults` |

The browser pair is not a database row. It lives in the settings store,
keyed by workspace id, and the server re-checks it.
[LAW-178-7](01-first-principles.md).

## Embedding dimension

`embedding_dimension` stays the integer already stored beside
`embedding_model` on the workspace (metadata and the column the update
path maintains). On a choice save the integer is copied from the card:

- static card: `capabilities.embedding_dimension`
- fleet `connection_models.embedding_dimension` when that row is the card

The body may repeat the same integer. A different integer is rejected
and nothing is written. [05-choice-law](05-choice-law.md).

Vector tables are untouched by the save. A dimension change still
requires the existing rebuild endpoint. Until that rebuild, the table
keeps the previous dimension and the mismatch guard in the vector layer
still refuses a write of the wrong width. This pack does not set
`EDGEQUAKE_ALLOW_VECTOR_TABLE_REBUILD`.

## What a delete already does

SPEC-177 deletes offer rows and `connection_models` with the connection
and leaves `llm_roles` in place. This pack keeps that. The leftover
role becomes `broken_reason = not_eligible` on the next read and the
next run. No cascade into metadata. No background rewriter.

## Concurrency

Two admins can save a ceiling and a workspace assignment in either
order. The assignment save checks the ceiling it reads. A ceiling save
that lands after the assignment does not roll the assignment back. The
next run sees the new ceiling and refuses. That is
[EC-178-28](11-edge-cases.md). There is no transaction across the two
handlers, and this pack does not add one. The run is the authority.

## Access, still the existing grants

| Call | Grant |
|------|-------|
| Workspace-scope eligible, and workspace assignment | member may read; owner or admin may write. Same as today's policy GET and workspace PUT. |
| Tenant-scope eligible, and tenant defaults | tenant owner or admin, or a caller who may create a workspace in that tenant. A member of one workspace is not enough. |
| Deployment-scope eligible, and `llm_defaults` | `Role::Admin` |

The checks stay in the handlers that already guard those routes. No new
RLS policy. `model_catalog_policy` RLS from migration 221 is unchanged
because the table is unchanged.

## Invariants the database lens holds

1. No secret column appears on a choice payload. Keys stay on connections.
2. `model_catalog_policy` remains the only model-tick table. Choice does
   not insert a tick in order to remember a selection.
3. An unconstrained tenant (zero ceiling rows) still has an offer. The
   offer is the bound. The absence of policy rows is not the absence of
   a filter.
4. Legacy `provider_connections.tenant_id IS NOT NULL` stays out of the
   global offer and out of every new choice list
   ([EC-177-33](../177-provider-cascade/11-edge-cases.md),
   [EC-178-21](11-edge-cases.md)).
