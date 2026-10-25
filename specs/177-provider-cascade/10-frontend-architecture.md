# 10 — Frontend architecture

Parent: [README](README.md) · Prev: [09-ux-ui-spec](09-ux-ui-spec.md) ·
Next: [11-edge-cases](11-edge-cases.md)

[LAW-177-6](01-first-principles.md) · [LAW-177-7](01-first-principles.md) ·
[LAW-177-9](01-first-principles.md).

## One editor

`ProviderOfferEditor` in
`edgequake_webui/src/components/settings/providers/provider-offer-editor.tsx`.

```text
  parent: "deployment" | "tenant" | "workspace"
  tenantId or workspaceId
  readOnly
```

| Parent | Reads | Writes | Model ticks |
|--------|-------|--------|-------------|
| `deployment` | `GET /deployment/providers` | withdraw PUT, connection-model PUT | `connection_models` |
| `tenant` | `GET /tenants/{id}/provider-offer` | tenant provider-offer PUT | existing tenant model-policy |
| `workspace` | `GET /workspaces/{id}/provider-offer` | workspace provider-offer PUT | existing workspace model-policy and `llm_roles` |

The deployment parent does not use `offer_mode`. It lists every candidate
and toggles `withdrawn`. Tenant and workspace use `offer_mode` and `slugs`.

The component does not import `models.toml`, does not call `eligible_models`,
and does not union catalogs in the browser. If the payload lacks a slug,
the slug is not rendered.

`TenantModelCeilingCard` and `WorkspaceModelNarrowingCard` stay the model
tables. They move under the expanded provider. They receive the provider
slug and render only that provider's models. They do not each grow a
fetch. The editor loads the catalog once and passes the slice.

## Registry

[`registry.ts`](../../edgequake_webui/src/lib/settings/registry.ts) gains:

| id | scope | segment | gate | keywords include |
|----|-------|---------|------|------------------|
| `providers` | `deployment` | `providers` | `instanceAdmin` | provider, offer, withdraw, model |
| `org-providers` | `organization` | `providers` | `tenantManage` | provider, ceiling, model, anthropic, openai |

Section ids must stay unique. Organization uses `org-providers` as the id
and `providers` as the segment, so the path is
`/settings/organization/providers` and the deployment path is
`/settings/deployment/providers`.

`people` keywords lose "model" if it was never there. The people lead
drops the model sentence. [`people-section.tsx`](../../edgequake_webui/src/components/settings/sections/people-section.tsx)
renders `MembersLinkCard` only.

[`sections/index.tsx`](../../edgequake_webui/src/components/settings/sections/index.tsx)
maps the two new ids. Workspace `models` keeps `WorkspaceModelsSection`,
which mounts `ProviderOfferEditor parent="workspace"` above
`WorkspaceRoleEditor`.

Search uses the existing index filter. A section the caller cannot open
is not a result. That is already `canOpenSection`. The new sections rely
on it. They do not add a second filter.

## Data hook

`use-provider-offer.ts` next to [`use-model-policy.ts`](../../edgequake_webui/src/hooks/use-model-policy.ts).

Query keys:

```text
["deployment-providers"]
["tenant-provider-offer", tenantId]
["workspace-provider-offer", workspaceId]
```

A successful tenant or workspace PUT invalidates that key, the model
policy key for the same scope, and `models/eligible` for the workspace.
A withdraw or a connection delete invalidates `deployment-providers` and
every tenant offer key. The hook does not cache the eligible list by hand.

Draft state uses `useServerBackedDraft`, the same hook as the ceiling
card. Dirty compares slugs plus `offer_mode` plus the model rows already
compared by `samePolicyRows`.

## Connections card

[`connections-card.tsx`](../../edgequake_webui/src/components/settings/connections-card.tsx)
keeps the create form. It gains Withdraw for `source === "env"` and the
Offered / Not offered badge from `offered` on the deployment providers
query, not from a local boolean.

Create sends no `X-Tenant-ID`. The API client today attaches the header
from the tenant store. The connections mutation must opt out of that
header. If it cannot opt out, the call will receive
`connection_scope_global` and the form will show that error. W4 makes the
opt-out explicit in the client rather than catching the 422 as the
success path.

The shape select reads `GET /connections/shapes`. The fallback list in
the card stays the three shapes. Unknown shapes are not added in the UI.

## Role editor

`WorkspaceRoleEditor` already lists eligible models. After W3 the eligible
endpoint applies the offer, so the menu shrinks without a new filter in
the component. The component must not keep a private copy of "all
providers."

## Tests in the webui

| Test | Holds |
|------|-------|
| `registry.test.ts` | workspace admin search for "provider" returns Models, not Organization |
| `provider-offer.test.ts` | pure helpers: unconstrained draft, empty constrained draft, omit models of a disabled provider |
| `e2e/spec177` | the gates in [13-e2e-test-matrix](13-e2e-test-matrix.md) |

Pure helpers live in `src/lib/settings/provider-offer.ts` with no React,
matching `model-usage.ts`.

## What not to build

- A second settings shell.
- A Zustand store for the offer. The server is the store. The draft hook is the only client copy.
- Three copies of the model table.
