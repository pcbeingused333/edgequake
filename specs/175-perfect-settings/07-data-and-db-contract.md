# 07 — Data and database contract

Parent: [README](README.md) · Prev: [06-catalog](06-catalog.md) ·
Next: [08-api-contract](08-api-contract.md)

[LAW-175-6](01-first-principles.md) · [LAW-175-11](01-first-principles.md) ·
[LAW-175-14](01-first-principles.md).

## What stays

No new columns on `tenants` or `workspaces`. Selection remains metadata
written by `build_tenant_metadata` and `apply_llm_config_update_with_tenant`.

| Key | Table metadata | Role |
|-----|----------------|------|
| `llm_provider`, `llm_model` | workspace | family default for chat usages |
| `embedding_provider`, `embedding_model`, `embedding_dimension` | workspace | embed default |
| `vision_llm_provider`, `vision_llm_model` | workspace | vlm family default |
| `llm_roles` | workspace | per-usage `RoleLlmConfig` |
| `default_reasoning_effort` | workspace and tenant | effort fallback |
| `default_llm_*`, `default_embedding_*`, `default_vision_llm_*` | tenant | tenant layer |
| `llm_defaults` | `server_config` | server layer |
| `extraction_mode`, `decision_*` | workspace | decision usage |
| `pdf_parser_backend`, chunking, entity types, vision prompts | workspace | ingestion scope, not the resolver |

`provider_connections` ([migration 169](../../edgequake/migrations/169_spec163_provider_connections.sql))
keeps secrets. It does not gain a model column. `load_connection_spec`
still receives the model from the resolver.

`settings` JSONB on tenants and workspaces stays `{}`. It is not the
settings store. Do not start writing it.

Config tables are not under tenant RLS today (`167_tenant_access_rls.sql`
does not cover `tenants`, `workspaces`, `provider_connections`,
`server_config`). This pack does not pretend RLS protects policy. The
API checks tenant and workspace ids in the same functions that already
guard `PUT /workspaces/{id}` and `PUT /tenants/{id}`.

## Migration 221

File: `edgequake/migrations/221_spec175_model_catalog_policy.sql`.

Postgres unique constraints treat nulls as distinct, so a single
`UNIQUE (tenant_id, workspace_id, provider, model, usage)` would allow
duplicate ceiling rows. Use two partial unique indexes. `workspace_id`
null means ceiling.

```sql
CREATE TABLE model_catalog_policy (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id     uuid NOT NULL REFERENCES tenants (tenant_id) ON DELETE CASCADE,
    workspace_id  uuid NULL REFERENCES workspaces (workspace_id) ON DELETE CASCADE,
    provider      text NOT NULL,
    model         text NOT NULL,
    usage         text NOT NULL,
    enabled       boolean NOT NULL DEFAULT true,
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT model_catalog_policy_usage_chk CHECK (
        usage IN (
            'extract', 'query', 'summary', 'vlm', 'keyword',
            'embedding', 'decision', 'rerank'
        )
    ),
    CONSTRAINT model_catalog_policy_provider_chk CHECK (
        char_length(provider) BETWEEN 1 AND 64
    ),
    CONSTRAINT model_catalog_policy_model_chk CHECK (
        char_length(model) BETWEEN 1 AND 256
    )
);

CREATE UNIQUE INDEX model_catalog_policy_ceiling_uq
    ON model_catalog_policy (tenant_id, provider, model, usage)
    WHERE workspace_id IS NULL;

CREATE UNIQUE INDEX model_catalog_policy_workspace_uq
    ON model_catalog_policy (tenant_id, workspace_id, provider, model, usage)
    WHERE workspace_id IS NOT NULL;

CREATE INDEX model_catalog_policy_tenant_idx
    ON model_catalog_policy (tenant_id, usage)
    WHERE enabled;

ALTER TABLE model_catalog_policy
    ADD CONSTRAINT model_catalog_policy_workspace_tenant_fk
    FOREIGN KEY (tenant_id, workspace_id)
    REFERENCES workspaces (tenant_id, workspace_id);
```

`workspaces` already has `UNIQUE (tenant_id, slug)` and primary key
`workspace_id`. The composite foreign key requires a unique pair on
`workspaces (tenant_id, workspace_id)`. Migration 221 adds that unique
index if it is not already present:

```sql
CREATE UNIQUE INDEX IF NOT EXISTS workspaces_tenant_id_workspace_id_uq
    ON workspaces (tenant_id, workspace_id);
```

Confirm the primary key name in `001_init_database.sql` before writing
the migration. `workspace_id` is the primary key, so
`(tenant_id, workspace_id)` is unique once the extra index exists. The
composite FK stops a row whose `workspace_id` belongs to another tenant.
[EC-175-15](11-edge-cases.md).

No secrets. No probe columns. Probe state stays on
`provider_connections.last_test_ok`.

`rerank` is in the check list so a later spec can store it. W1–W6 do
not insert rerank rows. The API rejects rerank policy writes with
`usage_not_configurable`.

## Reads

Unconstrained: `NOT EXISTS` a row for the tenant. Constrained: at least
one ceiling row. Narrowed: at least one row with that `workspace_id`.

Eligibility query, in words:

```text
  candidates = catalog_for(usage) filtered by probe and capability
  if ceiling rows exist:
      candidates = candidates where a ceiling row is enabled
  if workspace rows exist:
      candidates = candidates where a workspace row is enabled
                   or no workspace row names that pair and the
                   ceiling (or unconstrained set) still has it
  a workspace enabled=false row removes the pair
  a workspace enabled=true row cannot add a pair the ceiling omitted
```

The storage function returns the list. `resolve_usage` receives it. The
resolver does not embed SQL. [DIP in 01](01-first-principles.md).

## Assignments

`llm_roles` values stay JSON. OpenAPI gains a real schema
([08-api-contract](08-api-contract.md)). `embedding` may be a full
`RoleLlmConfig`. Readers accept the old shape that only has
`connection_id`.

Clearing an assignment deletes that role key and repaints display fields
the way `workspace_model_update.rs` already clears overrides. The
effective model then comes from the next layer. It is not a broken row
unless the next layer itself fails the probe.

## Caches

| Cache | Model identity today | Required |
|-------|----------------------|----------|
| Keyword `llm_cache` | hash includes model (`hash_keyword_args`) | keep |
| Answer `llm_cache` | hash of the prompt; model is not its own field | include resolved model id |
| Multimodal | `analysis_cache_key(..., model, ...)` | keep |
| `decision_cache` | `model` column and `key_hash` | keep |
| Embedding memo | name, model, dim | keep |

[EC-175-27](11-edge-cases.md). Do not add a column to `llm_cache` if the
hash input changes; old keys miss, which is the safe outcome. Do not
delete the cache on a model change.

## Lineage

`messages.llm_provider` / `messages.llm_model` and
`pdf_documents.vision_model` stay the record of what ran. They are not
configuration. A model change does not update them and does not
reprocess. [EC-175-18](11-edge-cases.md) · [EC-175-26](11-edge-cases.md).

## Backfill

None. Empty `model_catalog_policy` is unconstrained. No rewrite of
`llm_roles`.
