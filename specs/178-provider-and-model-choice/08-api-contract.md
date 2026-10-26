# 08 — API contract

Parent: [README](README.md) · Prev: [07-data-and-db-contract](07-data-and-db-contract.md) ·
Next: [09-ux-ui-spec](09-ux-ui-spec.md)

W4 refreshes OpenAPI and the generated
[`schema.d.ts`](../../edgequake_webui/openapi/schema.d.ts). Until that
wave the shapes in this file are the contract the handlers implement.
Existing fields stay. New fields are additive.

## `GET /api/v1/models/eligible`

One route. Exactly one scope.

| Query | Scope | Auth |
|-------|-------|------|
| `usage` and `workspace_id` | workspace | member of that workspace. Today's `require_member`. |
| `usage` and `tenant_id` | tenant | tenant Manage, or permission to create a workspace in that tenant |
| `usage` and `scope=deployment` | deployment | `Role::Admin` |

`usage` is the SPEC-175 set the policy API already accepts:
`extract`, `query`, `summary`, `vlm`, `keyword`, `embedding`, `decision`.
`rerank` is `422 usage_not_configurable`, as today.

Two scope parameters, or none, is `400 scope_required`.
`scope=deployment` together with `workspace_id` or `tenant_id` is the
same 400. A workspace member who passes `tenant_id` or
`scope=deployment` and fails the grant is `403`, including when the id
is their own tenant. The body is empty. [EC-178-32](11-edge-cases.md) ·
[EC-178-39](11-edge-cases.md).

A `workspace_id` in another tenant is the current not-found behavior
for workspace reads. It is not that tenant's model list.

### Response

`EligibleModelsResponse` gains `scope` and `offer_mode`. Each model
gains the fields the chip and the embedding form read today from the
inventory, so those screens do not call `/models/llm`.

```json
{
  "usage": "query",
  "scope": "workspace",
  "policy_mode": "constrained",
  "offer_mode": "unconstrained",
  "models": [
    {
      "provider": "openai",
      "provider_display_name": "OpenAI",
      "model": "gpt-5-nano",
      "display_name": "GPT-5 nano",
      "supports_vision": false,
      "supports_streaming": true,
      "embedding_dimension": null
    }
  ]
}
```

| Field | Source |
|-------|--------|
| `policy_mode` | `constrained` when this usage has ceiling rows, else `unconstrained`. Unchanged meaning. |
| `offer_mode` | the scope's `provider_offer_constrained` flag, or `constrained` for deployment when the global offer is the explicit candidate set. Deployment has no unconstrained bit. The field is `constrained` there, and `parent_count` is omitted. |
| `supports_streaming` | the catalog capability. False when the card does not say. |
| `embedding_dimension` | the card dimension for embedding usage, otherwise null |

`supports_streaming` and `embedding_dimension` are optional in the sense
that old clients ignore them. New choice clients require them and do
not fall back to `/models/llm` when they are absent. [EC-178-33](11-edge-cases.md).

The workspace form without the new fields still returns `models` as
today, plus the new fields. [`usage-role-row.tsx`](../../edgequake_webui/src/components/settings/models/usage-role-row.tsx)
keeps working.

Family pickers call this route four times (`extract`, `query`,
`summary`, `keyword`) and render the intersection in
[05-choice-law](05-choice-law.md). There is no `usage=family` and no
second route. A hook may cache the four responses. The intersection is
a view. The save still judges each usage, so a hook bug cannot persist
a pair the ceiling removed from one of them. [DRY](01-first-principles.md).

## Save: workspace

`PUT /api/v1/workspaces/{id}` gains no new fields. Before
`apply_llm_config_update` and the `llm_roles` merge, the handler judges
each pair the body sets:

| Body fields | Set |
|-------------|-----|
| `llm_provider` and `llm_model` | chat-family intersection at workspace scope |
| `llm_roles.{usage}.provider` and `.model` | `eligible(workspace, usage)` |
| `vision_llm_provider` and `vision_llm_model` | `eligible(workspace, vlm)` |
| `embedding_provider` and `embedding_model` | `eligible(workspace, embedding)`, dimension from the card |
| `decision_model` | `eligible(workspace, decision)` |

Empty, `"none"`, or null on a field the writer already treats as clear
skips the verdict for that field. [EC-178-34](11-edge-cases.md).

A role object that sets `connection_id` without a model is still a role
assignment. The connection's slug must be in the workspace offer. The
model stored on that role, when present, is judged as the role pair.
W1 does not accept a connection id whose slug is outside the offer.

Codes:

| Code | When |
|------|------|
| `422 model_pair_incomplete` | one of provider or model is set and the other is empty, and the value is not a clear |
| `422 model_not_eligible` | pair not in the set |
| `422 capability_mismatch` | card exists, `supports(usage)` is false |
| `422 embedding_dimension_mismatch` | body dimension differs from the card |
| `422 policy_widens_ceiling` | unchanged, policy route only |

The first failing field is the error. The handler writes nothing.

## Save: tenant

`PUT` tenant and the setup create path use tenant scope and the same
codes for `default_llm_*` (chat-family intersection), `default_embedding_*`,
and `default_vision_llm_*`. A create that omits the defaults inherits
the deployment leaf and does not invent a pair. A create that sends a
pair is judged.

## Save: deployment

`PATCH /api/v1/settings/llm-defaults` judges each pair it sets at
deployment scope: chat fields against the chat-family intersection,
embedding fields against `embedding`, vision fields against `vlm`.
`reasoning_effort` and `priority_mode` are unchanged and are not models.

## Run

| Route | When the body has a pair | When the body has no pair |
|-------|--------------------------|---------------------------|
| `POST /api/v1/chat/completions` stream and blocking | both-or-neither, then `eligible(workspace, query)`, or `vlm` if the message has images | resolve, then the same verdict on the winning pair |
| `POST /api/v1/query` and `/query/stream` | unchanged pair rule, plus the resolved-pair verdict | resolved-pair verdict |
| PDF upload with a vision override | unchanged `ensure_pair_eligible` for `vlm` | resolved vlm pair |

Winning pair outside the set, usage not locked: `422 model_not_eligible`
or `422 capability_mismatch`.

Usage locked and the body names a pair: `422 usage_locked`.

Offer loader returns an error: `503 offer_unavailable`. No provider call.

Credential or probe failure on the winning pair: the SPEC-175 status for
that `broken_reason` (`422` for capability and eligibility, `503` for
credential and probe). The handler does not call
`execute_sota_query_with_auth_fallback`.

`rerank_model` in the body: `422 rerank_model_rejected`, unchanged.

## Effective

`GET /api/v1/config/effective?workspace_id=` is unchanged in shape.
Choice clients use `usages[].provider`, `model`, `source`, `locked`,
`eligible`, `broken_reason` for the inherit label. W1 makes `source`
follow [LAW-178-9](01-first-principles.md): a request is not part of
this GET, so a query role with `connection_id` reports
`source=workspace_role`.

## Inventory, unchanged

`GET /api/v1/models`, `/models/llm`, `/models/embedding` keep their
bodies and their filters. Choice clients stop calling them. The routes
stay for catalog clients. [LAW-178-8](01-first-principles.md).

## Errors, one table

```text
  400 scope_required              eligible called with zero or two scopes
  403                             wider scope than the grant
  422 model_pair_incomplete       one of provider, model
  422 model_not_eligible          pair outside the set
  422 capability_mismatch         card exists, wrong capability
  422 usage_locked                body pair while operator lock holds
  422 embedding_dimension_mismatch
  422 usage_not_configurable      rerank, unchanged
  422 rerank_model_rejected       unchanged
  503 offer_unavailable           offer query failed
  503                             credential or probe, SPEC-175 mapping
```

Machine codes stay snake_case. The WebUI maps them in
[09-ux-ui-spec](09-ux-ui-spec.md). The client does not treat 422 as a
signal to retry on the process model.
