# 06 — Provider lifecycle

Parent: [README](README.md) · Prev: [05-gate-law](05-gate-law.md) ·
Next: [07-data-and-db-contract](07-data-and-db-contract.md)

## WHY

A provider is not a row in `models.toml`. It is a credential, a base URL, a
probe, a model list, and a place in the offer. The lifecycle below is the
only way those facts change. Organization and Workspace never collect a key.
That rule is [LAW-177-3](../177-provider-cascade/01-first-principles.md) and stays.

## States

```text
  draft --> tested --> offered --> refreshed --> registered
                |          |            |
                v          v            v
              failed    withdrawn     stale
                           |
                           v
                        deleted
```

| State | Fact |
|-------|------|
| draft | `provider_connections` row, secret stored, `last_test_ok` is null |
| tested | `last_test_ok = true`. Eligible to be in the global offer |
| failed | `last_test_ok = false`, `last_test_error` set. Not offered |
| offered | slug in `global_offer` and not withdrawn |
| refreshed | `provider_refresh_state.last_refreshed_at` set for the slug |
| registered | row in `connection_models` (fleet) or a `models.toml` card (env) |
| withdrawn | `deployment_provider_overrides.withdrawn = true` |
| stale | `base_url`, key, or shape changed. `last_test_ok` cleared. Not offered until retested |
| deleted | row gone. Offers for the slug gone. Assignments become broken at next gate call |

Env providers start at `offered` when their credential env is set. They have
no connection row. Withdraw uses the override table. Refresh uses process
env for that provider, because that is the credential.

## Create

1. `parse_slug`. Reject empty, mixed case, spaces, and slugs longer than 64.
2. Reject a slug that collides with an env provider slug.
3. Reject a base URL with userinfo or a query string. SSRF checks still run.
4. Insert the connection row.
5. Store the secret. On secret failure, delete the row.
6. Do not mark `last_test_ok`.

## Rotate and edit

- A new `api_key` overwrites `eq-conn-{id}`.
- An absent or blank `api_key` keeps the secret. The response says `kept`.
- Changing `base_url`, `api_shape`, or the key sets `last_test_ok = null` and
  `last_test_error = null`.
- Slug rename is refused with `422 slug_rename` while any `provider_offers`
  row or override references the old slug. The admin withdraws, then creates.

## Delete and test

- Fleet rows are addressed with no tenant header. A request that resolves a
  tenant id gets `422 connection_scope_global` on delete and on test, matching
  create.
- Delete returns an impact list. `confirm=true` is required when the list is
  non-empty.

## Refresh

`POST /api/v1/deployment/providers/{slug}/refresh`

1. Platform Admin.
2. Load the fleet connection or the env provider.
3. Call the provider's model list with that base URL and secret.
4. Upsert `provider_model_cache`. Stamp `first_seen_at` once, `last_seen_at` always.
5. Write `provider_refresh_state` with status `ok` or `error`, counts of added and missing.
6. Return the diff. Do not write `connection_models`. Register is a separate PUT.

Upstream 401, timeout, and a partial list set status `error` and keep the
previous cache rows. The UI shows the error and the previous time.

## Register

`PUT /api/v1/deployment/providers/{slug}/models`

Each item has `model`, `supports_chat`, `supports_vision`, `supports_embedding`,
and `embedding_dimension`. An item with `supports_embedding` and a null
dimension is `422 embedding_dimension_required`. The gate copies the
dimension onto the catalog card so workspace embedding assignment can store it.

## Impact

`GET /api/v1/deployment/providers/{slug}/impact` and the same shape on offer
and policy writes.

```text
  { assignments: [ { scope, tenant_id, workspace_id, usage, provider, model } ] }
```

The list is tenants, workspaces, and roles whose stored pair uses the slug
or the model. Confirm does not rewrite them. The gate fails them closed.
