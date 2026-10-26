# 08 — API contract

Parent: [README](README.md) · Prev: [07-data-and-db-contract](07-data-and-db-contract.md) ·
Next: [09-ux-ui-spec](09-ux-ui-spec.md)

## WHY

The routes already exist for offers, policy, and connections. This page
names what they must return after SPEC-179, and the three routes that are
new. Paths are under `/api/v1`.

## New and changed routes

| Method and path | Who | Body | Result |
|-----------------|-----|------|--------|
| `POST /deployment/providers/{slug}/refresh` | Platform Admin | empty | `{ added, removed, unchanged, refreshed_at, status, error }` |
| `GET /deployment/providers/{slug}/impact` | Platform Admin | — | `{ assignments: [...] }` |
| `DELETE /connections/{id}?confirm=true` | Platform Admin, no tenant header | — | 204, or 409 `confirm_required` with the impact list |
| `PUT /deployment/providers/{slug}/models` | Platform Admin | items plus `embedding_dimension` | 422 `embedding_dimension_required` when missing |
| `POST /models/discover/refresh` | Platform Admin | — | same as a refresh of every refreshable env provider. Was ungated |
| `GET /models/eligible` | member of the scope | — | adds `context_length`, `supports_tools`, `supports_vision`, `input_cost_per_million`, `output_cost_per_million` when the card has them |

Offer and policy PUT require `If-Match`. Missing header on a constrained
scope that already has a revision greater than 1 is `428 precondition_required`
only when the client library sends writes; a missing header is accepted as
revision 0 for the first save and rejected after that. Simpler rule used by
the tests: if `If-Match` is present it must match; if it is absent the write
proceeds and returns the new `ETag`. Clients that opt in cannot lose a race.
The response always sets `ETag`.

## Error codes

| Code | Status | When |
|------|--------|------|
| `model_not_eligible` | 422 | pair outside the set, including a stored pair at run time |
| `capability_mismatch` | 422 | card does not support the usage |
| `model_pair_incomplete` | 422 | one of provider, model |
| `usage_locked` | 422 | request pair against an operator lock |
| `credential_missing` | 422 | eligible pair, no secret |
| `offer_unavailable` | 503 | offer query failed |
| `embedding_dimension_required` | 422 | embedding model without a dimension |
| `slug_invalid` | 422 | `parse_slug` failed |
| `slug_rename` | 422 | rename while referenced |
| `connection_scope_global` | 422 | tenant header on a fleet mutation |
| `stale_revision` | 409 | `If-Match` mismatch |
| `confirm_required` | 409 | delete or withdraw with assignments and no confirm |
| `url_rejected` | 422 | userinfo, query string, or SSRF |

`provider_not_offered` is not emitted. The offer miss is `model_not_eligible`.

## Eligible query

Unchanged: exactly one of `workspace_id`, `tenant_id`, `scope=deployment`,
plus `usage`. Zero or two scopes is `400 scope_required`.

## Secrets

No response body in this pack contains `api_key`, `ciphertext`, or
`client_secret`. Tests grep JSON responses from the connection and refresh
handlers.

## OpenAPI

`make codegen-openapi-refresh` updates the snapshot and `schema.d.ts` for
the new fields and routes. `spec027_api_contract` keeps passing.
