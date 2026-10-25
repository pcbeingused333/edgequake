# 08 — API contract

Parent: [README](README.md) · Prev: [07-data-and-db-contract](07-data-and-db-contract.md) ·
Next: [09-ux-ui-spec](09-ux-ui-spec.md)

JSON uses snake_case. Errors use the existing API error body. New
machine codes below are the `code` field. OpenAPI is regenerated in W2
from the utoipa paths. Handlers do not return a key, a ciphertext, or a
`secret_ref`.

## Global offer

`GET /api/v1/deployment/providers`

Platform Admin. `403` otherwise.

```json
{
  "providers": [
    {
      "slug": "openai",
      "display_name": "openai (env)",
      "origin": "env",
      "api_shape": "openai_chat",
      "offered": true,
      "withdrawn": false,
      "probe": "ok",
      "key_configured": true,
      "models": [
        {
          "id": "gpt-5-nano",
          "usages": ["extract", "query", "summary", "keyword"]
        }
      ]
    }
  ]
}
```

`origin` is `env`, `fleet`, or `legacy` is never used here. Legacy rows
are not on this route.

`probe` is `ok`, `failed`, or `unchecked`. `offered` is true only when
[05-cascade-law](05-cascade-law.md) includes the slug.

`PUT /api/v1/deployment/providers/{slug}`

```json
{ "withdrawn": true }
```

`404` when the slug is not an env candidate and not a fleet connection.
`422 slug_invalid` when the slug fails the check constraint.
Withdrawal of an env slug does not change the process environment.
Response is the same object as one element of `providers`.

`PUT /api/v1/deployment/providers/{slug}/models`

Replaces `connection_models` for the fleet connection with that slug.
`422 provider_not_fleet` for an env slug (env models stay on the catalog
cards). `422 probe_required` when `last_test_ok` is not true.

## Tenant offer

`GET /api/v1/tenants/{tenant_id}/provider-offer`

`TenantAccess::Manage`, or Platform Admin. `403` for a workspace admin.

```json
{
  "tenant_id": "…",
  "offer_mode": "unconstrained",
  "parent_count": 2,
  "enabled_count": 2,
  "providers": [
    {
      "slug": "openai",
      "display_name": "OpenAI",
      "offered_by_parent": true,
      "enabled": true,
      "models": []
    }
  ]
}
```

`providers` contains the global offer only. `enabled` is true for every
row when `offer_mode` is `unconstrained`.

`PUT /api/v1/tenants/{tenant_id}/provider-offer`

```json
{
  "offer_mode": "constrained",
  "slugs": ["openai"]
}
```

`offer_mode: unconstrained` ignores `slugs`, clears the tenant's offer
rows, and sets `provider_offer_constrained = false`.

`422 provider_not_in_parent` when a slug is not in the global offer.
`422 slug_invalid` for an empty slug, a slug longer than 64, or a slug
that is not lowercase ASCII plus digits and hyphens. The server lowercases
before the check and rejects a slug that still fails.
`422 duplicate_slug` when `slugs` repeats a value.

The response is the GET body after the write.

## Workspace offer

`GET /api/v1/workspaces/{workspace_id}/provider-offer`

A member of the workspace may GET. `can_manage` is a boolean on the
response so the editor can hide Save. The slug list is still filtered.

`PUT` requires workspace manage. Same body as the tenant PUT.
`422 provider_not_in_parent` when the slug is outside the tenant offer,
including when the tenant is unconstrained and the slug is outside the
global offer.

## Catalog and eligible

`GET /api/v1/tenants/{tenant_id}/model-catalog` gains the same slug
filter as the tenant offer. Entries whose provider is outside the global
offer are omitted. The response grows `offer_mode` copied from the tenant
provider offer so the model grid and the provider list cannot disagree.

`GET /api/v1/models/eligible?usage=&workspace_id=` applies the workspace
offer before ceiling and narrowing. A slug outside the offer is absent.
The route does not grow a new query parameter.

`GET /api/v1/config/effective` is unchanged in shape. When the resolved
pair's provider is outside the workspace offer and the source is not
`operator_lock`, `eligible` is false and `broken_reason` is
`not_eligible`. When the source is `operator_lock` and the credential
exists, `eligible` stays true and `locked` stays true.

## Connections

`POST /api/v1/connections` and `PUT /api/v1/connections/{id}`:

- Platform Admin, unchanged.
- When `X-Tenant-ID` resolves to a uuid, `422` with code
  `connection_scope_global` and message "Providers are added on the
  deployment."
- Absent tenant header stores `tenant_id` null, unchanged.
- `api_key` is write-only. The response keeps `key_configured` and
  `key_fingerprint`. It does not echo `api_key`.

`GET /api/v1/connections` for a tenant includes fleet rows plus that
tenant's legacy rows. The JSON grows `legacy: true` on rows whose
`tenant_id` equals the caller tenant. Fleet rows have `legacy: false`.
Another tenant's rows are not in the array. This matches
`connection_list_sql` and adds the flag.

`DELETE /api/v1/connections/{id}` of a fleet row deletes offer rows for
that slug in the same transaction. `404` when the id is another tenant's
legacy row and the caller is not that tenant's platform admin looking at
that tenant. Platform Admin may delete a legacy row only when the request
is scoped to that tenant. The response stays `204`.

## Model policy

`PUT /api/v1/tenants/{tenant_id}/model-policy` and the workspace twin
reject a row whose provider is outside the parent offer with
`422 provider_not_in_parent`. Existing 422s (`usage_not_configurable`,
duplicates, widening) stay.

## Status map

| Situation | HTTP | Code |
|-----------|------|------|
| Workspace admin opens tenant offer | 403 | existing forbidden |
| Member PUT workspace offer | 403 | existing forbidden |
| Slug not in parent | 422 | `provider_not_in_parent` |
| Bad slug | 422 | `slug_invalid` |
| Repeated slug | 422 | `duplicate_slug` |
| Tenant header on create | 422 | `connection_scope_global` |
| Model replace on env slug | 422 | `provider_not_fleet` |
| Model replace before a successful test | 422 | `probe_required` |
| Unknown connection on delete | 404 | existing |
| Run or effective, pair outside offer | 422 on a request override; the job's stored assignment surfaces as `broken_reason` on effective and as the SPEC-175 refusal on ingest and query | `not_eligible` |

Request overrides that name an ineligible pair stay `422 model_not_eligible`
from SPEC-175. `provider_not_in_parent` is the admin write. The member
request keeps the SPEC-175 code so clients do not branch on two names for
the same refusal.
