# SPEC-177 — Provider cascade

> **Status:** Shipped in v0.38.1.
> **Product pin:** EdgeQuake v0.38.0. Migration file head is
> [222_spec176_secret_manager.sql](../../edgequake/migrations/222_spec176_secret_manager.sql).
> This pack reserves migration **223**.
> **Scope:** Which providers a scope may see, then which models of those
> providers. Global offers. The tenant takes a subset. The workspace takes
> a subset of the tenant. Keys are created only on the deployment.
> **Inherits:** [SPEC-175](../175-perfect-settings/) resolution, usages, and
> `model_catalog_policy` · [SPEC-176](../176-secret-manager/README.md)
> secret storage and API shapes · [SPEC-163](../163-onboarding-provider-config/)
> connections, probe, and SSRF · [SPEC-123](../123-env-config-priority/)
> model ladder · [SPEC-043](../043-update-edgequake-llm/000-index.md) catalog.
> **Does not replace:** `resolve_usage`, the usage enum, the settings shell,
> or the four scopes in [LAW-175-8](../175-perfect-settings/01-first-principles.md).

## Words

| Word in this pack | Product surface |
|-------------------|-----------------|
| Global | Deployment. Route `/settings/deployment`. Platform Admin. |
| Tenant | Organization. Route `/settings/organization`. Tenant owner or admin. |
| Workspace | Workspace. `/settings/workspace` and `/w/{slug}/settings`. |

Global is not a fifth scope. The shell labels stay You, Workspace,
Organization, and Deployment ([LAW-177-7](01-first-principles.md)).

## Start here

1. [00-why.md](00-why.md) — Five WHYs
2. [01-first-principles.md](01-first-principles.md) — LAW-177-1…10 + DRY/SOLID
3. [02-surfaces.md](02-surfaces.md) — Global, tenant, and workspace as they are
4. [03-product-spec.md](03-product-spec.md) — Personas, stories, acceptance
5. [04-findings.md](04-findings.md) — F-177-* with file and symbol
6. [05-cascade-law.md](05-cascade-law.md) — Offer algebra
7. [06-screens.md](06-screens.md) — ASCII wireframes
8. [07-data-and-db-contract.md](07-data-and-db-contract.md) — Migration 223
9. [08-api-contract.md](08-api-contract.md) — Offer endpoints and 422s
10. [09-ux-ui-spec.md](09-ux-ui-spec.md) — Copy and states
11. [10-frontend-architecture.md](10-frontend-architecture.md) — One editor
12. [11-edge-cases.md](11-edge-cases.md) — EC-177 register
13. [12-implementation-plan.md](12-implementation-plan.md) — Waves W0–W5
14. [13-e2e-test-matrix.md](13-e2e-test-matrix.md) — One gate per EC
15. [14-cross-ref.md](14-cross-ref.md) — Law, finding, EC, wave, test, lens
16. Lenses → [`lenses/`](lenses/)
    - [Product Owner](lenses/LENS-product-owner.md)
    - [Full Stack](lenses/LENS-full-stack.md)
    - [Database](lenses/LENS-database.md)
    - [UX / UI](lenses/LENS-ux-ui.md)
    - [Front](lenses/LENS-front.md)
    - [AI Engineer](lenses/LENS-ai-engineer.md)
    - [SaaS](lenses/LENS-saas.md)

## Locked decisions (Wave 0)

1. **The parent is the universe.** A provider absent from the global offer
   is absent from the tenant payload. A provider absent from the tenant
   offer is absent from the workspace payload. It is not a disabled row
   the child can turn on. [LAW-177-1](01-first-principles.md).
2. **Provider, then models.** Each scope enables providers from its parent,
   then models of those providers. Model ticks stay
   `model_catalog_policy`. They do not gain a second writer.
   [LAW-177-2](01-first-principles.md).
3. **Keys are born at Global.** OpenAI-compatible
   (`openai_chat`), Anthropic-compatible (`anthropic_messages`), and
   Ollama are created, tested, rotated, and deleted on Deployment →
   Connections. Organization and Workspace never collect an API key.
   [LAW-177-3](01-first-principles.md).
4. **Empty means all of the parent.** `provider_offer_constrained` defaults
   to false. While it is false, the tenant may use every globally offered
   provider. The first save sets the flag. A save with no slugs offers
   nothing. `offer_mode: unconstrained` is the only way back. The same
   rule runs from tenant to workspace. [LAW-177-4](01-first-principles.md).
   This amends the universe in [LAW-175-9](../175-perfect-settings/01-first-principles.md).
   It does not delete unconstrained mode.
5. **Fail closed.** A deleted provider or a revoked grant sets
   `broken_reason = not_eligible`. The call does not use
   `ProviderFactory::from_env` or another tenant's connection.
   [LAW-177-5](01-first-principles.md).
6. **One eligibility function.** `eligible_models` receives the parent
   offer. HTTP, the worker, MCP, and `GET /config/effective` keep calling
   `resolve_usage`. [LAW-177-6](01-first-principles.md).
7. **The shell keeps its names.** [LAW-177-7](01-first-principles.md).
8. **An operator lock bypasses the grant.** A pin still wins when the
   process credential exists. The grant cannot hide it.
   [LAW-177-8](01-first-principles.md).
9. **One writer per fact.** Connection for the secret, override for
   withdrawal, offer row for visibility, `model_catalog_policy` for model
   ticks, `llm_roles` for the assignment, `resolve_usage` for the run.
   [LAW-177-9](01-first-principles.md).
10. **Every EC-177 has a named test.** [LAW-177-10](01-first-principles.md).

Chat, embedding, and vision share one provider offer. One connection is
one credential.

## Job in one picture

```text
+-------------------------------+
| Global offer                  |
| env provider, or a fleet      |
| connection whose probe passed |
| and that nobody withdrew      |
+---------------+---------------+
                |
                v
+-------------------------------+
| Tenant offer                  |
| no rows: every global slug    |
| rows: the enabled slugs only  |
+---------------+---------------+
                |
                v
+-------------------------------+
| Workspace offer               |
| no rows: every tenant slug    |
| rows: the enabled slugs only  |
+---------------+---------------+
                |
                v
+-------------------------------+
| model ticks, then llm_roles   |
| resolve_usage still decides   |
| which pair runs               |
+-------------------------------+
```

Who edits what:

```text
+----------------+---------------------------+----------------------+
| Scope          | Writes                    | Who                  |
+----------------+---------------------------+----------------------+
| Global         | connection, key, withdraw | Platform Admin       |
|                | global model ticks        |                      |
+----------------+---------------------------+----------------------+
| Tenant         | provider_offers           | tenant owner / admin |
|                | ceiling inside the offer  |                      |
+----------------+---------------------------+----------------------+
| Workspace      | provider_offers           | workspace owner /    |
|                | narrowing, llm_roles      | admin                |
+----------------+---------------------------+----------------------+
```
