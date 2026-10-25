# 03 — Product spec

Parent: [README](README.md) · Prev: [02-surfaces](02-surfaces.md) ·
Next: [04-findings](04-findings.md)

## Personas

| Persona | Can | Cannot |
|---------|-----|--------|
| Member | Pick a query model inside the workspace set. Change theme, language, graph look. | Edit assignments, ceiling, connections, fleet. |
| Workspace Admin | Assign each usage in workspaces they administer. Narrow those workspaces. Edit their ingestion, labels, PDF, and vision policy. | Open Organization. Widen the tenant ceiling. Administer a workspace they were not granted. See keys. |
| Super Admin | Everything a Workspace Admin can do in every workspace of the tenant. Set org defaults. Turn on a ceiling. Invite people. Create a workspace. | Edit fleet keys. Delete the tenant unless they are the tenant Owner. Remove the last tenant Owner. |
| Platform Admin | Connections, fleet defaults, attribution, Langfuse status, SSO, server quotas. Create and delete tenants. Open a tenant for support. | Wear the Super admin chip. Skip the catalog. Spend a tenant's choice on the process key. |

The stored roles stay `owner` and `admin` on `memberships`. The chip is
the label. Detail: [15-roles](15-roles.md).

A self-hosted operator often holds Platform Admin and Super Admin. The
chip says Platform admin. The shell still labels You, Workspace,
Organization, and Deployment.

## Stories

### S1 — Member asks a question

The composer chip lists models the workspace enabled for `query`. Each row
shows the provider display name and the model id. Inherit is the first
row and the default. Choosing a model sends it on that request only when
it is eligible. A locked usage shows the lock and does not open.

Acceptance: [EC-175-16](11-edge-cases.md) · [EC-175-28](11-edge-cases.md).

### S2 — Workspace Admin assigns extract

The workspace Models page lists usages, not a grid of empty text fields.
Extract offers chat models that support JSON mode. The current row shows
the effective model and the layer (`inherited`, `this workspace`,
`locked by operator`, `broken`). Saving writes `llm_roles.extract`. The
next ingest uses that pair. Old documents stay as they are until Rebuild.

Acceptance: [EC-175-18](11-edge-cases.md) · [EC-175-01](11-edge-cases.md).

### S3 — Super Admin hides a provider

Organization → Catalog lists providers that have a healthy connection or
a healthy env probe. The admin turns models off. The confirm copy states
that this is the moment the tenant stops inheriting "everything healthy."
Workspace pickers drop those models. A workspace that still assigns a
removed model shows broken and refuses that usage.

Acceptance: [EC-175-05](11-edge-cases.md) · [EC-175-19](11-edge-cases.md) ·
[EC-175-30](11-edge-cases.md).

### S4 — Platform Admin adds a key

Deployment → Connections. Slug, API shape (a select of known shapes),
base URL, key, Test, Save. The key is write-only. Test must succeed
before the provider appears in the org catalog. Fleet defaults remain
the leaf under tenant and workspace.

Acceptance: [EC-175-04](11-edge-cases.md) · [EC-175-34](11-edge-cases.md).

### S5 — Acc pin

The benchmark sets `EDGEQUAKE_EXTRACT_LLM_MODEL`. The effective endpoint
reports `source: operator_lock` for extract. The workspace row is
disabled. Unset, the workspace assignment wins.

Acceptance: [EC-175-02](11-edge-cases.md).

### S6 — Two admins, two workspaces

A Workspace Admin of Legal sets extract to a JSON-capable model. A Super
Admin turns that model off in the catalog. Legal's row goes broken. The
Workspace Admin opens Finance, which they do not administer: the models
page has no Save, and Organization is not in the nav.

Acceptance: [EC-175-36](11-edge-cases.md) · [EC-175-37](11-edge-cases.md) ·
[EC-175-39](11-edge-cases.md).

## Copy (owner may edit words, not the fact)

| Moment | Sentence |
|--------|----------|
| Empty ceiling | "People in this organization can use every model that is healthy and fits the job." |
| First ceiling row | "From now on, only models you enable here appear in workspaces." |
| Broken assignment | "This job points at a model the organization no longer allows. Ingest and query for this job are paused." |
| Operator lock | "Locked by the server operator. This workspace cannot change it." |
| Fail closed | "This provider has no working key. Nothing was sent with the server key." |
| Rebuild | "New documents will use the new embedding model. Existing vectors stay until you rebuild." |

## Out of this product slice

- Billing, invoices, and plan checkout.
- Per-user model favorites beyond the query chip's inherit-or-pick.
- A gleaning slider. The pipeline's gleaning stage stays. The local toggle goes.
- Editing `openai_logprobs`. It stays refused.
- Renaming the five chat roles.

## Acceptance the owner can demo

1. Open settings as a member. Organization and Deployment are absent. The query chip lists two models, not the fleet.
2. As Workspace Admin of Legal, set extract to a JSON-capable model. Upload a text file. The task lineage matches that model. The effective panel matches it. Organization is still absent.
3. As Super Admin, disable that model. The usage row turns broken. A new upload fails with the broken sentence, not with a different model.
4. As Platform Admin, set the extract pin. The chip says Platform admin. The row shows the lock. Clear the pin. The workspace choice returns.
5. Toggle theme in the header and in You. Both stay in sync. Search "embedding" lands on the workspace usage, not on a second page.
6. As Workspace Admin of Legal, select Finance in the app header. Models have no Save. The settings switcher still lists only Legal.
