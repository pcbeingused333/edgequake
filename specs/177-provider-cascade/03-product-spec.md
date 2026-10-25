# 03 — Product spec

Parent: [README](README.md) · Prev: [02-surfaces](02-surfaces.md) ·
Next: [04-findings](04-findings.md)

Personas and role names stay [SPEC-175](../175-perfect-settings/15-roles.md).
This pack adds no role.

## Personas

| Persona | Can | Cannot |
|---------|-----|--------|
| Member | Pick a query model inside the workspace offer. | See a provider the workspace does not offer. Open Organization or Deployment. See a key. |
| Workspace Admin | Enable providers the tenant offers, then narrow models, then assign each usage. | Enable a provider the tenant did not offer. Open Organization. Type an API key. |
| Super Admin | Enable providers the deployment offers, then set the model ceiling inside that set. Invite people. | Create, rotate, or delete a provider key. See another tenant's legacy connection. |
| Platform Admin | Add, test, withdraw, and remove a fleet provider and its key. Set which models of an offered provider exist for the deployment. | Wear the Super admin chip. Spend a tenant's assignment on a provider the offer excluded, except an operator lock. |

A self-hosted operator often holds Platform Admin and Super Admin. The
screens stay on their scopes. Holding both roles does not move the key
form onto Organization.

## Stories

### S1 — Platform Admin adds a compatible provider

Deployment → Connections. The admin chooses OpenAI-compatible or
Anthropic-compatible, sets the display name, base URL, and key, and
presses Test. Test succeeds. Save stores a fleet row (`tenant_id` null)
and the provider appears as Offered. Deployment → Providers lists that
provider and the models the probe returned. A tenant that has not
constrained its offer can see the provider. A tenant that already saved
a provider list does not see it until that tenant enables it.

Acceptance: [EC-177-03](11-edge-cases.md) · [EC-177-04](11-edge-cases.md) ·
[EC-177-05](11-edge-cases.md) · [EC-177-12](11-edge-cases.md).

### S2 — Platform Admin withdraws an environment provider

OpenAI is present because `OPENAI_API_KEY` is set. The admin marks it
Withdrawn. The key stays in the environment. Tenant catalogs drop
`openai`. An extract pin that names that provider still shows Locked and
still runs. Other usages do not fall through to a different provider.

Acceptance: [EC-177-02](11-edge-cases.md) · [EC-177-10](11-edge-cases.md) ·
[EC-177-15](11-edge-cases.md) · [EC-177-27](11-edge-cases.md).

### S3 — Super Admin enables a subset

Organization → Providers lists only the global offer. The admin turns
Anthropic on and leaves a second provider off, then saves. The confirm
sentence says this organization will only use the providers enabled here.
Workspaces of this tenant lose the provider that was turned off. A
workspace assignment that still names it shows broken. Query and ingest
for that job refuse. They do not answer on the server key.

Acceptance: [EC-177-06](11-edge-cases.md) · [EC-177-16](11-edge-cases.md) ·
[EC-177-22](11-edge-cases.md).

### S4 — Workspace Admin cannot widen

The workspace Models page lists the tenant's providers. The admin enables
one of them and picks a JSON-capable model for extract. Saving a provider
the tenant turned off returns an error and the list does not gain a row.
The next ingest uses the assigned model. Old documents stay until Rebuild.

Acceptance: [EC-177-07](11-edge-cases.md) · [EC-177-09](11-edge-cases.md) ·
[EC-177-13](11-edge-cases.md) · [EC-177-42](11-edge-cases.md).

### S5 — Two tenants

Tenant A has a legacy connection created before this pack. Tenant B's
provider page does not list it. Tenant A's legacy panel lists it and says
new keys are added on Deployment. A fleet provider both tenants are
allowed to see appears for both. Neither tenant sees the other's legacy row.

Acceptance: [EC-177-08](11-edge-cases.md) · [EC-177-33](11-edge-cases.md).

### S6 — Member asks a question

The composer chip lists models whose provider the workspace offers and
whose capability is `query`. A withdrawn or ungranted provider is not a
disabled row in the chip. Inherit remains the first row.

Acceptance: [EC-177-13](11-edge-cases.md) · [EC-177-28](11-edge-cases.md).

### S7 — Empty deployment

No env key and no fleet connection. Organization → Providers says the
deployment has no providers. It does not list cards from `models.toml`.

Acceptance: [EC-177-11](11-edge-cases.md).

## Copy (owner may edit words, not the fact)

| Moment | Sentence |
|--------|----------|
| Empty global offer | "This deployment has no providers yet. Add one under Connections." |
| Empty tenant, parent has providers | "People in this organization can use every provider this deployment offers." |
| First tenant save | "From now on, only providers you enable here appear in workspaces." |
| Empty workspace, tenant has providers | "This workspace can use every provider the organization offers." |
| First workspace save | "From now on, only providers you enable here appear in this workspace." |
| Provider not in parent | "This provider is not offered to you. Ask the owner of the level above." |
| Probe failed | "The last test failed. Tenants cannot use this provider until a test succeeds." |
| Withdrawn env | "Hidden from organizations. The server key is unchanged." |
| Broken assignment | "This job points at a provider that is no longer offered. Ingest and query for this job are paused." |
| Operator lock | "Locked by the server operator. Hiding the provider does not change this job." |
| Legacy connection | "This key was saved on the organization. New keys are added on Deployment." |
| Fail closed | "Nothing was sent with another key." |

## Out of this product slice

- Billing and per-tenant markup on a shared key.
- A tenant bringing its own key (new BYOK). Legacy rows stay until a later pack.
- Per-user provider favorites.
- Editing `EDGEQUAKE_ALLOWED_PROVIDERS` in the UI.
- Decision and rerank as grantable chat providers.
- Renaming Deployment to Global in the nav.

## Acceptance the owner can demo

1. As Platform Admin, add an OpenAI-compatible provider, test it, and save. Organization → Providers shows it. The key is not on the page.
2. As Super Admin of a fresh tenant, leave the provider list unsaved. The workspace chip includes the new provider. Save with that provider off. The chip drops it. A query does not answer on the server key.
3. As Workspace Admin, the Organization item is absent. Enabling a provider the tenant turned off fails with the parent sentence.
4. As Platform Admin, withdraw the env provider. Tenants lose it. An extract pin still shows Locked and still runs.
5. As Super Admin of Tenant B, Tenant A's legacy connection is absent.
6. People no longer contains the model grid. Providers does.
