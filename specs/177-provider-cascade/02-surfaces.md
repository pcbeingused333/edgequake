# 02 — Surfaces (as they are)

Parent: [README](README.md) · Prev: [01-first-principles](01-first-principles.md) ·
Next: [03-product-spec](03-product-spec.md)

Facts at product pin v0.38.0, after SPEC-175 and SPEC-176 landed in the
tree. Line numbers refer to that tree. This file records the gap. It does
not describe the target screens. Those are [06-screens](06-screens.md).

## How the three levels are named in the product

[`SETTINGS_SCOPES`](../../edgequake_webui/src/lib/settings/registry.ts)
is the map from this pack's words to routes.

| This pack | Scope id | Label in the nav | Gate | Default section |
|-----------|----------|------------------|------|-----------------|
| Global | `deployment` | Deployment | `instanceAdmin` | `connections` |
| Tenant | `organization` | Organization | `tenantManage` | `people` |
| Workspace | `workspace` | Workspace | `everyone` | `models` |

[`sections/index.tsx`](../../edgequake_webui/src/components/settings/sections/index.tsx)
is the only map from section id to a component. There is no `providers`
section.

## Global — Deployment

### Connections

[`connections-section.tsx`](../../edgequake_webui/src/components/settings/sections/connections-section.tsx)
renders [`connections-card.tsx`](../../edgequake_webui/src/components/settings/connections-card.tsx).

The card loads `GET /connections` and `GET /connections/shapes`. The
fallback shapes, used when the shapes call fails, are `openai_chat`,
`anthropic_messages`, and `ollama`. The server list is
[`CONNECTION_SHAPES`](../../edgequake/crates/edgequake-api/src/providers/connection_shapes.rs).
The form posts a slug, display name, shape, base URL, and write-only
`api_key`. Keys already go to `secret_ref` ([SPEC-176](../176-secret-manager/README.md)).

`create_connection` stores `scoped_tenant(&tenant_ctx)`
([`connections.rs`](../../edgequake/crates/edgequake-api/src/handlers/connections.rs)).
A request that carries `X-Tenant-ID` writes a tenant-owned row. A request
with no tenant header writes `tenant_id NULL`, which is the fleet row
this pack wants as the only new birthplace.

`env_connections` adds virtual rows (`id` nil, `source` would be `env`
on the view) for `OPENAI_API_KEY`, `ANTHROPIC_API_KEY` or
`ANTHROPIC_BASE_URL`, `OLLAMA_HOST`, and the local-host variables
(`OPENAI_COMPATIBLE_BASE_URL`, `OMLX_HOST`, `LMSTUDIO_HOST`, and the
others in that function). Those rows cannot be deleted.

`connection_list_sql(scoped = true)` returns that tenant's rows union
fleet rows (`tenant_id IS NULL`). It does not return another tenant's rows.

There is no Offered / Withdrawn control.

### Fleet

[`fleet-section.tsx`](../../edgequake_webui/src/components/settings/sections/fleet-section.tsx)
holds server LLM defaults and provider health. It does not grant a
provider to a tenant.

### What Global does not have

A page that lists providers and then their models as the parent set.
Withdrawal of an env slug. A guarantee that new connections are fleet-scoped.

## Tenant — Organization

### People

[`people-section.tsx`](../../edgequake_webui/src/components/settings/sections/people-section.tsx)
renders `MembersLinkCard` and, when `access.tenantManage && tenantId`,
[`TenantModelCeilingCard`](../../edgequake_webui/src/components/settings/models/tenant-model-ceiling-card.tsx).

The card calls `GET /tenants/{id}/model-catalog` and
`GET|PUT /tenants/{id}/model-policy`. The catalog handler loads every
deployment model ([`get_tenant_model_catalog`](../../edgequake/crates/edgequake-api/src/handlers/model_policy.rs)).
The description on the card says a job with no ticks can use any
registered model.

The table is models by usage. There is no provider master switch.

### Secrets

[`secrets-section.tsx`](../../edgequake_webui/src/components/settings/sections/secrets-section.tsx)
chooses the tenant secret backend (database or Infisical). It does not
create a provider. This pack leaves that page in place. New provider keys
are deployment credentials and use the deployment secret service with
`tenant_id NULL` ([LAW-177-3](01-first-principles.md)).

### What the tenant catalog is made of

[`load_catalog`](../../edgequake/crates/edgequake-api/src/services/model_policy_catalog.rs)
lists LLM and embedding models from `state.query.model_catalog` filtered
by [`active_provider_names`](../../edgequake/crates/edgequake-api/src/model_catalog.rs).
That filter is `EDGEQUAKE_ALLOWED_PROVIDERS` or "every enabled provider."
It is not a parent offer.

[`eligible_models`](../../edgequake/crates/edgequake-core/src/model_policy/eligibility.rs)
then applies capability, `registered`, probe, ceiling, and narrowing.
`within_ceiling` passes every candidate when the usage has no policy rows.

## Workspace

[`workspace-models-section.tsx`](../../edgequake_webui/src/components/settings/sections/workspace-models-section.tsx)
renders, in order:

| Component | Writes | Who |
|-----------|--------|-----|
| `WorkspaceRoleEditor` | `llm_roles` | editor; members read |
| `WorkspaceModelNarrowingCard` | `model_catalog_policy` workspace rows | editor |
| `RebuildEmbeddingsButton` | rebuild action | editor |
| `QueryDefaultsGroup` | local query defaults | the signed-in browser |

The narrowing card cannot widen the tenant ceiling
([`widening_rows`](../../edgequake/crates/edgequake-core/src/model_policy/eligibility.rs)).
It can still list a model whose provider the deployment never connected,
because the ceiling's universe is the catalog.

Members pick query models from `GET /models/eligible`. That handler uses
the same `eligible_models` function.

## Duplicate writers this pack must not add

| Fact today | Writers | SPEC-177 rule |
|------------|---------|---------------|
| Credential | `upsert` in `connections.rs` | keep this writer; force `tenant_id NULL` on new rows |
| Model enablement | `PUT` tenant and workspace model-policy | keep; feed it a smaller catalog |
| Assignment | workspace `llm_roles` | keep |
| Run | `resolve_usage` | keep; pass the offer into eligibility |
| Visibility of a slug | none | new `provider_offers` only |

## Auth gates already in force

[LAW-175-13](../175-perfect-settings/01-first-principles.md) is the role
check. This pack adds sections. It does not add a role.

| Action today | Check |
|--------------|-------|
| Connections | `ApiRequireAdmin` (Platform Admin) |
| Tenant model policy | `TenantAccess::Manage` |
| Workspace model policy | workspace manage, or tenant manage, or platform admin |
| Member eligible list | membership of the workspace |

A Workspace Admin who is not a tenant admin does not see Organization in
[`registry.ts`](../../edgequake_webui/src/lib/settings/registry.ts) tests
(`a workspace admin still does not see Organization or Deployment`).
