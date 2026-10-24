# 08 — API contract

Parent: [README](README.md) · Prev: [07-data-and-db-contract](07-data-and-db-contract.md) ·
Next: [09-ux-ui-spec](09-ux-ui-spec.md)

OpenAPI regeneration is W11 (`make codegen-openapi-refresh` and the
SPEC-027 contract test). This chapter is the contract those types must
match. Routes sit under the workspace the caller already bound. Operation
checks stay in `require_request_permission`. Object checks use
`VisibilityScope`.

## Errors

| Situation | Status | Body |
|-----------|--------|------|
| Hidden document, asset, PDF, lineage, task, retrieval id | 404 | Same shape as a missing id. No `hidden: true` |
| Query that matches nothing visible | 200 | Empty context |
| Missing `tags.manage` on a tag route | 403 | The route exists; this is not an existence oracle |
| Tag still assigned | 409 | Tag id only, no document ids the caller cannot see |
| Enable when the actor is not owner | 403 | |
| Label above the actor's clearance | 403 | Does not echo the document body |
| Break-glass read | 200 | Audit row written first. Response has no extra field |

404 comparison tests use the same JSON as today's not-found
([EC-174-13](12-edge-cases.md)).

## Settings and vocabulary

| Method | Path | Who |
|--------|------|-----|
| `GET` | `/api/v1/workspaces/{id}/clearance` | Any member. Returns `enforcement`, `policy_epoch`, and the caller's compiled rank and compartment ids. Does not return other people's roles |
| `POST` | `/api/v1/workspaces/{id}/clearance/enable` | Owner. Body: `{ "strict_start": false }` |
| `GET` | `/api/v1/workspaces/{id}/clearance/tags` | Steward sees all. Others see classification values at or below their rank, and compartments they hold |
| `POST` | `/api/v1/workspaces/{id}/clearance/tags` | `tags.manage`, and the new compartment is one they may hold, or they are steward |
| `DELETE` | `/api/v1/workspaces/{id}/clearance/tags/{tag_id}` | `tags.manage`. 409 if assigned. Classification seeds cannot be deleted |

## Roles and rules

| Method | Path | Who |
|--------|------|-----|
| `GET` | `/api/v1/workspaces/{id}/clearance/roles` | `tags.manage` |
| `POST` | `/api/v1/workspaces/{id}/clearance/roles` | `tags.manage`. Body: name, steward flag |
| `PUT` | `/api/v1/workspaces/{id}/clearance/roles/{role_id}/rules` | `tags.manage`. Replaces the rule list. Each rule: `effect`, `match_kind` (`classification_at_most`, `any_of`, `all_of`, `untagged`), `rank` or `tag_ids` |
| `PUT` | `/api/v1/workspaces/{id}/memberships/{membership_id}/clearance-roles` | `tags.manage`. Replaces assignments. Cannot remove the last steward |
| `POST` | `/api/v1/workspaces/{id}/clearance/simulate` | Steward. Body: `{ "role_ids": ["..."] }`. Response: `{ "count": n, "titles": ["..."] }` with at most 20 titles. Compiled without the caller's steward bit. Does not change later requests. No request header. Audit the call. A non-steward receives 403 |

The preview is only that response ([RB-174-05](17-roadblocks.md),
[US-174-6](03-product-spec.md)). It must not use the steward
short-circuit, or the titles would be the whole library.

## Documents

`PATCH /api/v1/documents/{id}/labels`

Body: `{ "classification": "confidential", "compartment_ids": ["..."] }`.

The actor needs `tags.manage` and [LAW-174-7](01-first-principles.md).
Success bumps `policy_epoch` and copies columns onto child rows.
Downgrade requires steward and writes `audit_logs` (`event_type =
Authorization`, action `relabel`, result `Success`).

`GET /api/v1/documents` and `GET /api/v1/documents/{id}` gain:

```json
{
  "classification": "confidential",
  "compartments": [{ "id": "...", "key": "department", "value": "hr" }]
}
```

Absent when enforcement is off, so current clients keep their shape.
Present when enforcement is on, and only with tags the caller may know.
A hidden id is 404 and omits this object.

Upload body may include the same fields. Values outside the actor's
clearance are 403. Omitted fields on an enforcing workspace store
`unverified` ([US-174-4](03-product-spec.md)).

`document_filter` on `POST /api/v1/query`, `/query/stream`, `/chat`, and
the context route is unchanged in JSON. The server intersects
([LAW-174-10](01-first-principles.md)). Clients do not send a clearance.

## Graph and query fields

Node and edge payloads gain no "hidden neighbor" count. `degree` is the
visible degree when enforcement is on. `description` is the visible
fragments. `source_document_ids` lists only visible documents. The
enforcement-off payload stays the current one.

## MCP

No new tool is required for the leak tests. Existing tools take the same
`VisibilityScope` as REST:

- `eq_document_list`, `eq_document_get`, `eq_document_download`, `eq_asset_get`
- `eq_search`, `eq_retrieve`, `eq_fetch`
- `eq_entity_search`, `eq_neighborhood`, `eq_graph_image`

Caller-supplied `document_ids` narrow. `eq_fetch` of another principal's
id, or of an id whose epoch or fingerprint no longer matches, is the MCP
not-found error, not the bundle ([EC-174-25](12-edge-cases.md)).

Write tools (`eq_ingest`, upload) apply the resulting-label check. A write
scope does not widen clearance ([EC-174-32](12-edge-cases.md)).

## Audit

| Action | When |
|--------|------|
| `clearance_enable` | Enable commits |
| `relabel` | Downgrade, and any rank or compartment change |
| `clearance_break_glass` | Any read while the flag is on, result `Warning` |
| `clearance_simulate` | Simulate |

The table is the existing `audit_logs` partition. `resource_type` is
`document` or `clearance_role`. Denied reads of hidden documents are not
audited as 403s, because the client received 404 and a deny row would be
a second store of the id. Steward relabel and break-glass are the rows
operators query.

## SDK

W11 updates the Rust SDK types and the generated TypeScript client. Until
that wave, the WebUI calls these routes through a single module
([10-frontend-architecture](10-frontend-architecture.md)) so a generated
client can replace it without a second filter implementation.

Next: [09-ux-ui-spec](09-ux-ui-spec.md).
