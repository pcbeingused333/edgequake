# 01 — First principles (LAW-177)

Parent: [README](README.md) · Prev: [00-why](00-why.md) · Next: [02-surfaces](02-surfaces.md)

These laws are non-negotiable for SPEC-177. A wave may defer a screen.
A wave must not ship a screen that violates a law. SPEC-175 laws stay in
force except where a law below names the amendment.

## DRY / SOLID application

| Principle | Application in SPEC-177 |
|-----------|-------------------------|
| **DRY** | One offer algebra in `edgequake-core`. One `eligible_models`, extended with the parent offer. One `ProviderOfferEditor` with a parent argument. Connection create stays `upsert` in `connections.rs`. |
| **SRP** | The offer function returns slugs. SQL for `provider_offers` lives in the API storage module. HTTP maps 403 and 422. React renders the payload. Capability flags stay in `edgequake-llm`. |
| **OCP** | A new API shape is a `ConnectionShape` plus the existing accept list. It is not a new settings page. A new usage stays a SPEC-175 enum arm. |
| **LSP** | REST, the worker, and MCP see the same eligible set. A caller cannot tell a slug was removed by withdrawal, by a tenant grant, or by a workspace grant except by reading `broken_reason` and the offer mode. |
| **ISP** | The member picker asks for `usage=query`. It does not receive keys, withdrawn slugs, or the deployment editor. |
| **DIP** | Handlers depend on `resolve_usage` and the offer function. Pages depend on the offer endpoint. They do not import `models.toml` and they do not recompute "all of parent." |

---

## LAW-177-1 — The parent is the universe

The tenant catalog, the tenant grant editor, and every tenant picker
contain only providers in the **global offer**. The workspace catalog and
the workspace editor contain only providers in the **tenant offer**.

A slug outside the parent set is omitted from the JSON. The child does
not receive it as `enabled: false` with a control that can turn it on.

The global offer is:

- an `env_connections` slug that has no withdrawn override, or
- a `provider_connections` row with `tenant_id IS NULL`, `last_test_ok = true`,
  and no withdrawn override.

`EDGEQUAKE_ALLOWED_PROVIDERS`, when set to a list, still removes slugs
before this offer is built ([`active_provider_names`](../../edgequake/crates/edgequake-api/src/model_catalog.rs)).
`models.toml` cards whose provider has neither an env row nor a fleet
connection are not in the offer.

Detail: [05-cascade-law](05-cascade-law.md).

- **Closes:** [WHY A](00-why.md) · [WHY D](00-why.md) · [F-177-01](04-findings.md) · [F-177-06](04-findings.md)
- **Tests:** [EC-177-01](11-edge-cases.md) · [EC-177-02](11-edge-cases.md) · [EC-177-11](11-edge-cases.md)

## LAW-177-2 — Provider, then models

Each scope edits in this order:

1. Providers the parent offers.
2. Models of the enabled providers, per usage, in `model_catalog_policy`.

Disabling a provider removes its models from the child universe. The
model rows for that provider at that scope and below are not a way to
keep the provider alive.

Enabling a provider does not mark every model enabled. Model ticks stay
on the SPEC-175 rules: empty model policy for a usage means every model
of the **offered** providers that matches the capability. The first model
tick for a usage constrains that usage.

Chat, embedding, and vision share the provider switch. A slug off for
chat is off for embeddings. The capability predicate still decides which
usages a model may fill ([LAW-175-3](../175-perfect-settings/01-first-principles.md)).

- **Closes:** [WHY B](00-why.md) · [F-177-03](04-findings.md)
- **Tests:** [EC-177-09](11-edge-cases.md) · [EC-177-17](11-edge-cases.md) · [EC-177-28](11-edge-cases.md)

## LAW-177-3 — Keys are born at Global

Create, rotate, test, and delete of a provider credential happen on
Deployment → Connections, stored as `tenant_id IS NULL`.

Shapes stay the SPEC-176 set: `openai_chat`, `anthropic_messages`,
`ollama` ([`CONNECTION_SHAPES`](../../edgequake/crates/edgequake-api/src/providers/connection_shapes.rs)).
A key is required for the two cloud shapes. Ollama keeps an optional key.
`POST /providers/test` must succeed before `last_test_ok` is true and the
slug enters the global offer.

`POST /api/v1/connections` and `PUT /api/v1/connections/{id}` reject a
resolved `X-Tenant-ID` with `422 connection_scope_global`. Organization
and Workspace forms have no key field.

Existing rows with a non-null `tenant_id` stay readable by that tenant
only. They are legacy. They are not copied, and they are not part of the
global offer. [07-data-and-db-contract](07-data-and-db-contract.md).

- **Closes:** [WHY C](00-why.md) · [F-177-04](04-findings.md) · [F-177-05](04-findings.md)
- **Tests:** [EC-177-03](11-edge-cases.md) · [EC-177-08](11-edge-cases.md) · [EC-177-12](11-edge-cases.md) · [EC-177-23](11-edge-cases.md) · [EC-177-33](11-edge-cases.md)

## LAW-177-4 — Empty means all of the parent

This amends the universe of [LAW-175-9](../175-perfect-settings/01-first-principles.md).
Unconstrained mode remains. "All" means all of the parent offer.

| Scope | `provider_offer_constrained` false | `provider_offer_constrained` true |
|-------|-------------------------------------|------------------------------------|
| Tenant | every global-offer slug | the enabled rows, which may be empty |
| Workspace | every tenant-offer slug | the enabled rows, which may be empty |

The first successful save sets `provider_offer_constrained` and writes
the enabled slugs. An empty slug list is a real choice: the scope offers
nothing. The UI states that before the write. Returning to "all of the
parent" is a different save, `offer_mode: unconstrained`, which clears
the rows and clears the flag. An empty list does not mean that.

A provider added to the global offer after a tenant became constrained
does not appear in that tenant until the tenant enables it. A tenant that
is still unconstrained gains it, because the parent grew.

No backfill. Existing tenants stay unconstrained and therefore keep every
provider the deployment actually offers. They lose `models.toml` cards
that have no credential. That is the intended shrink, not a migration of
rows.

- **Closes:** [WHY A](00-why.md) · [F-177-02](04-findings.md)
- **Tests:** [EC-177-04](11-edge-cases.md) · [EC-177-05](11-edge-cases.md) · [EC-177-16](11-edge-cases.md) · [EC-177-31](11-edge-cases.md) · [EC-177-34](11-edge-cases.md)

## LAW-177-5 — Fail closed, no sibling keys

When the winning layer names a provider outside the offer that applies to
that call, `broken_reason` is `not_eligible`. The handler does not call
`ProviderFactory::from_env` and does not load another tenant's
`provider_connections` row.

Deleting a fleet connection deletes the secret ref the way SPEC-176
already does, and descendant offer rows for that slug are removed by the
application in the same transaction as the delete (the slug is not a
foreign key, because env slugs have no connection row). Workspace
assignments that still name the slug stay in `llm_roles` and read as
broken until an editor clears them. Documents are not reprocessed.

A workspace write that names a slug outside the tenant offer is `422
provider_not_in_parent`. The server does not drop the slug silently.

- **Closes:** [WHY E](00-why.md) · [F-177-09](04-findings.md)
- **Tests:** [EC-177-06](11-edge-cases.md) · [EC-177-07](11-edge-cases.md) · [EC-177-13](11-edge-cases.md) · [EC-177-42](11-edge-cases.md)

## LAW-177-6 — One eligibility function

`eligible_models` in [`eligibility.rs`](../../edgequake/crates/edgequake-core/src/model_policy/eligibility.rs)
gains the offered provider slugs as an argument. A catalog entry whose
provider is not in that set is dropped before ceiling and narrowing.

`resolve_usage` stays the function that picks the pair
([`usage_resolution.rs`](../../edgequake/crates/edgequake-core/src/usage_resolution.rs)).
The API passes the eligible set in. React, the worker, and MCP do not
implement a second cascade.

`GET /config/effective` returns the same `broken_reason` the run uses
([`effective_usages.rs`](../../edgequake/crates/edgequake-api/src/services/effective_usages.rs)).

- **Closes:** [WHY E](00-why.md) · [F-177-09](04-findings.md)
- **Tests:** [EC-177-13](11-edge-cases.md) · [EC-177-21](11-edge-cases.md) · [EC-177-40](11-edge-cases.md) · [EC-177-41](11-edge-cases.md)

## LAW-177-7 — Do not rename the shell

Routes stay:

| Scope | Route | Gate already in `registry.ts` |
|-------|-------|-------------------------------|
| You | `/settings/you` | `everyone` |
| Workspace | `/settings/workspace`, `/w/{slug}/settings` | `everyone` reads, `workspaceEditor` writes |
| Organization | `/settings/organization` | `tenantManage` |
| Deployment | `/settings/deployment` | `instanceAdmin` |

New sections are `deployment/providers` and `organization/providers`.
Workspace keeps the `models` section and gains the provider list on that
page. People loses the ceiling card.

Copy may say "this deployment" when the English sentence needs a noun for
Global. The nav label stays Deployment.

- **Closes:** [F-177-08](04-findings.md)
- **Tests:** [EC-177-14](11-edge-cases.md) · [EC-177-29](11-edge-cases.md) · [EC-177-30](11-edge-cases.md)

## LAW-177-8 — An operator lock bypasses the grant

[LAW-175-2](../175-perfect-settings/01-first-principles.md) step 1 still
wins. When `operator_lock` names a provider and a model, `resolve_usage`
returns that pair even if the tenant or workspace offer omitted the slug,
as long as the process credential or the fleet connection for that
provider exists.

The grant cannot hide the pin. The picker for that usage stays disabled
and the row says the usage is locked. Other usages of the same provider
follow the offer.

If the lock's provider has no credential, the result is the SPEC-175
credential failure. The resolver does not substitute a granted provider.

Withdrawing an env slug does not unset the environment variable.

- **Tests:** [EC-177-10](11-edge-cases.md) · [EC-177-15](11-edge-cases.md) · [EC-177-27](11-edge-cases.md) · [EC-177-39](11-edge-cases.md)

## LAW-177-9 — One writer per fact

| Fact | Writer | Must not |
|------|--------|----------|
| URL, shape, secret ref, probe | `provider_connections` via `upsert` | store a model id or an offer bit |
| Withdrawn | `deployment_provider_overrides` | store a key |
| Which child may use the slug | `provider_offers` | store a key or a model id |
| Which model and usage | `model_catalog_policy` | store a secret or a slug grant |
| Which pair a job runs | `llm_roles` and tenant `default_*` | store a key |
| What runs now | `resolve_usage` | be recomputed in React |

Legacy tenant-scoped connections keep their existing writer until the
follow-up that retires them. This pack only refuses new tenant-scoped
creates.

- **Tests:** [EC-177-23](11-edge-cases.md) · [EC-177-26](11-edge-cases.md) · [EC-177-35](11-edge-cases.md) · [EC-177-43](11-edge-cases.md)

## LAW-177-10 — Every EC-177 has a named test

[13-e2e-test-matrix](13-e2e-test-matrix.md) names the test for each case
in [11-edge-cases](11-edge-cases.md). A wave that adds a case adds the
row in the same change. A case without a test is not done.

- **Tests:** the matrix itself
