# 07 — Data and database contract

Parent: [README](README.md) · Prev: [06-screens](06-screens.md) ·
Next: [08-api-contract](08-api-contract.md)

[LAW-177-3](01-first-principles.md) · [LAW-177-4](01-first-principles.md) ·
[LAW-177-9](01-first-principles.md).

## What stays

| Table | Role after this pack |
|-------|----------------------|
| `provider_connections` | URL, shape, probe, `secret_ref`. New rows have `tenant_id` null. |
| `managed_secrets` | Ciphertext. Offer rows do not point at it. |
| `model_catalog_policy` | Tenant ceiling and workspace narrowing of `(provider, model, usage)`. |
| `workspaces` / `tenants` metadata | `llm_roles` and `default_*`. Not the offer. |
| `server_config` | Fleet defaults. Not the offer. |

`settings` JSONB on tenants and workspaces stays `{}`.

Migration [221](../../edgequake/migrations/221_spec175_model_catalog_policy.sql)
already created `workspaces_tenant_id_workspace_id_uq`. Migration 223
reuses it. It does not add a second unique index.

Config tables are still outside the tenant RLS of migration 167. The API
checks the tenant id and the workspace id in the same functions that
guard model policy. This pack does not pretend RLS protects offers.

## Migration 223

File: `edgequake/migrations/223_spec177_provider_offers.sql`. Expand-only.
No backfill. No rewrite of `api_key_ciphertext` or `secret_ref`.

```sql
-- SPEC-177: which providers a tenant or workspace may see.
-- Absence of rows means "every provider the parent offers".

CREATE TABLE deployment_provider_overrides (
    provider_slug  text PRIMARY KEY,
    withdrawn      boolean NOT NULL,
    updated_at     timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT deployment_provider_overrides_slug_chk CHECK (
        char_length(provider_slug) BETWEEN 1 AND 64
        AND provider_slug = lower(provider_slug)
    )
);

CREATE TABLE provider_offers (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id     uuid NOT NULL REFERENCES tenants (tenant_id) ON DELETE CASCADE,
    workspace_id  uuid NULL REFERENCES workspaces (workspace_id) ON DELETE CASCADE,
    provider_slug text NOT NULL,
    enabled       boolean NOT NULL,
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT provider_offers_slug_chk CHECK (
        char_length(provider_slug) BETWEEN 1 AND 64
        AND provider_slug = lower(provider_slug)
    )
);

CREATE UNIQUE INDEX provider_offers_tenant_uq
    ON provider_offers (tenant_id, provider_slug)
    WHERE workspace_id IS NULL;

CREATE UNIQUE INDEX provider_offers_workspace_uq
    ON provider_offers (tenant_id, workspace_id, provider_slug)
    WHERE workspace_id IS NOT NULL;

CREATE INDEX provider_offers_tenant_idx
    ON provider_offers (tenant_id)
    WHERE workspace_id IS NULL;

ALTER TABLE provider_offers
    ADD CONSTRAINT provider_offers_workspace_tenant_fk
    FOREIGN KEY (tenant_id, workspace_id)
    REFERENCES workspaces (tenant_id, workspace_id);

-- Models confirmed for a fleet connection. The provider id is the connection slug.
CREATE TABLE connection_models (
    connection_id  uuid NOT NULL REFERENCES provider_connections (id) ON DELETE CASCADE,
    model          text NOT NULL,
    supports_chat  boolean NOT NULL DEFAULT false,
    supports_vision boolean NOT NULL DEFAULT false,
    supports_embedding boolean NOT NULL DEFAULT false,
    embedding_dimension integer NULL,
    created_at     timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT connection_models_model_chk CHECK (
        char_length(model) BETWEEN 1 AND 256
    ),
    PRIMARY KEY (connection_id, model)
);
```

Postgres unique constraints treat nulls as distinct, so the two partial
indexes are required. The same pattern is migration 221.

`enabled = false` is stored only when the scope is constrained and the
admin saved an explicit off. A replace of the tenant list deletes the
tenant's `workspace_id IS NULL` rows and inserts the enabled slugs. The
implementation may store only `enabled = true` rows. A constrained tenant
is "has at least one row **or** a mode bit." Storing only enabled rows
cannot tell "constrained, everything off" from "unconstrained."

That case is real: an admin may offer nothing. Migration 223 therefore
adds the mode on the parent, not a sentinel slug:

```sql
ALTER TABLE tenants
    ADD COLUMN provider_offer_constrained boolean NOT NULL DEFAULT false;

ALTER TABLE workspaces
    ADD COLUMN provider_offer_constrained boolean NOT NULL DEFAULT false;
```

Default false preserves [LAW-177-4](01-first-principles.md). The first
save sets the column true, including a save whose enabled list is empty.
A later save that the UI labels "use every provider the parent offers"
sets the column false and deletes the rows.

These two columns are the mode. They are not a second copy of the slug
list. `provider_offers` holds the enabled slugs when the column is true.
When the column is false the rows are ignored and should be empty.

`tenants` and `workspaces` gain a column. SPEC-175 avoided new columns
because model policy used the empty table as the mode. The empty-list
case makes that encoding lossy for providers. The column is the smallest
fix. It is not a JSON settings blob.

## Reads

```text
  global = compute_global_offer(env, fleet, overrides, allowlist)
  tenant mode = tenants.provider_offer_constrained
  if not tenant mode:
       tenant = global
  else:
       tenant = global intersect enabled tenant rows
  workspace mode = workspaces.provider_offer_constrained
  if not workspace mode:
       workspace = tenant
  else:
       workspace = tenant intersect enabled workspace rows
```

The function that implements this lives in `edgequake-core` and takes
plain slices. It does not open SQL. [DIP in 01](01-first-principles.md).

A tenant row whose slug left the global offer is skipped on read. The
next tenant save drops it. A workspace row whose slug left the tenant
offer is skipped the same way.

## Writes

| Write | Rule |
|-------|------|
| Tenant replace | Platform Admin or `TenantAccess::Manage`. Every slug must be in the global offer. Sets `provider_offer_constrained`. |
| Workspace replace | Workspace manage of that workspace. Every slug must be in the tenant offer. Sets the workspace column. |
| Withdraw | Platform Admin. Upsert `deployment_provider_overrides`. Does not delete env vars. |
| Connection create | Platform Admin. `tenant_id` null. Unknown shape still 400 from SPEC-176. |
| Connection delete | Deletes `connection_models` by FK. Application deletes `provider_offers` for the slug in the same transaction. |

`model_catalog_policy` writes gain one check: `provider` must be in the
offer of that scope's parent. A ceiling row for a slug outside the global
offer is `422 provider_not_in_parent`.

## Legacy rows

`provider_connections.tenant_id IS NOT NULL` is unchanged. No UPDATE moves
them to null. The list for the owning tenant returns them under `legacy`.
They are excluded from `compute_global_offer`. [EC-177-33](11-edge-cases.md).

## Caches and lineage

SPEC-175 cache identity stays. An offer change does not delete
`llm_cache`, `decision_cache`, or embedding memo rows. Old keys miss when
the resolved model id changes, which is the safe outcome.

`messages.llm_provider` and `pdf_documents.vision_model` stay lineage.
Withdrawing a provider does not update them and does not reprocess.

## Down migration

Not shipped. Expand-only, same policy as SPEC-150. Dropping the tables is
a forward migration in a later pack if the feature is removed.
