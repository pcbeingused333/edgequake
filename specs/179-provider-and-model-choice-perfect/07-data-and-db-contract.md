# 07 — Data and database contract

Parent: [README](README.md) · Prev: [06-provider-lifecycle](06-provider-lifecycle.md) ·
Next: [08-api-contract](08-api-contract.md)

## WHY

The offer, the policy, and the connection already have tables. What is
missing is the refresh result, a revision the UI can send back, and an
audit row that cannot contain a secret. Migration 224 adds those. It does
not add a second offer table and it does not move `llm_roles` out of
workspace metadata. [LAW-178-4](../178-provider-and-model-choice/01-first-principles.md)
still holds for the choice itself.

## Migration 224

File: `edgequake/migrations/224_spec179_provider_refresh.sql`. Expand only.
`compat_serve_max` becomes 224.

```text
  provider_model_cache
  +------------------+---------------------------+
  | provider_slug    | PK part, lowercase        |
  | model            | PK part, 1..256           |
  | supports_chat    | bool                      |
  | supports_vision  | bool                      |
  | supports_embedding | bool                    |
  | embedding_dimension | int null               |
  | context_length   | int null                  |
  | source           | text, default 'refresh'  |
  | first_seen_at    | timestamptz               |
  | last_seen_at     | timestamptz               |
  +------------------+---------------------------+

  provider_refresh_state
  +------------------+---------------------------+
  | provider_slug    | PK                        |
  | last_refreshed_at| timestamptz               |
  | status           | 'ok' or 'error'           |
  | error            | text null                 |
  | added_count      | int                       |
  | removed_count    | int                       |
  +------------------+---------------------------+

  provider_config_audit
  +------------------+---------------------------+
  | id               | uuid PK                   |
  | actor_id         | uuid null                 |
  | action           | text                      |
  | scope            | text                      |
  | subject          | text                      |
  | before_hash      | text null                 |
  | after_hash       | text null                 |
  | created_at       | timestamptz               |
  +------------------+---------------------------+
```

No secret column. A test selects `information_schema.columns` and fails if
`ciphertext`, `api_key`, or `nonce` appears on these three tables.

## Revisions

`provider_offers_revision` and `model_policy_revision` are integers on
`tenants` (org scope) and `workspaces` (workspace scope), default 1.
A replace increments the matching counter in the same transaction as the
delete and insert. The client sends `If-Match: "<revision>"`. A mismatch is
`409 stale_revision` and writes nothing.

Deployment overrides use one row in `server_config` key
`provider_config_revision`.

## Indexes

- `provider_offers (provider_slug)` so withdraw and impact do not scan.
- `model_catalog_policy (tenant_id, workspace_id, usage)` without a
  `WHERE enabled` predicate, so narrowing rows (enabled false) are indexed.
  The existing partial index stays.

## Unchanged

`provider_connections`, `managed_secrets`, `connection_models`,
`deployment_provider_overrides`, `model_catalog_policy`, workspace
`llm_roles`. Slug checks stay in the application (`parse_slug`) and in the
existing CHECK constraints.

## Read path

The gate reads, in one function:

1. `load_workspace_offer` or the tenant or global variant.
2. `load_rows` for ceiling and narrowing.
3. `load_catalog` plus `append_connection_models`.
4. The fleet connection row for the resolved slug, if any.

Impact reads `tenants.metadata`, `workspaces.metadata` (`llm_roles` and the
legacy family keys), and `server_config` fleet defaults. It does not read
secrets.
