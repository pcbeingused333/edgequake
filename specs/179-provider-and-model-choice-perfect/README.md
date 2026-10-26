# SPEC-179 — Provider and model choice, made true

> **Status:** Shipped in v0.38.1.
> **Product pin:** EdgeQuake v0.38.0. Migration file head before this pack is
> [223_spec177_provider_offers.sql](../../edgequake/migrations/223_spec177_provider_offers.sql).
> This pack reserves migration **224**.
> **Scope:** The filter already specified by SPEC-175, SPEC-177, and SPEC-178
> becomes the only way a model is listed, saved, or run. Deployment can add a
> provider with a secret, refresh its models, and the tenant and workspace
> filters are what every caller sees.
> **Inherits:** [SPEC-178](../178-provider-and-model-choice/README.md) choice law ·
> [SPEC-177](../177-provider-cascade/README.md) offer algebra ·
> [SPEC-176](../176-secret-manager/README.md) secret storage ·
> [SPEC-175](../175-perfect-settings/README.md) usages and policy rows ·
> [SPEC-163](../163-onboarding-provider-config/README.md) connections and SSRF.
> **Does not replace:** `eligible_models`, `resolve_usage`, the offer tables,
> or who may edit Deployment, Organization, and Workspace.

## Words

| Word | Meaning in this pack |
|------|----------------------|
| Gate | `ModelGate::resolve`. The only function that turns a scope and a usage into a client. |
| Inventory | `GET /models` and its siblings. Operator catalog. Not a choice list. |
| Choice | A pair `eligible_models` returns for one usage in one scope. |
| Refresh | A Platform Admin asks one provider for its live model list and stores the diff. |
| Broken | A stored assignment the gate now refuses. The task fails. Nothing else runs. |

## Start here

1. [00-why.md](00-why.md) — Five WHYs
2. [01-first-principles.md](01-first-principles.md) — LAW-179-1…12 + DRY/SOLID
3. [02-surfaces.md](02-surfaces.md) — Every caller, classified
4. [03-product-spec.md](03-product-spec.md) — Personas, stories, acceptance
5. [04-findings.md](04-findings.md) — F-179-* with file and symbol
6. [05-gate-law.md](05-gate-law.md) — Resolve, judge, fail closed
7. [06-provider-lifecycle.md](06-provider-lifecycle.md) — Add, secret, probe, refresh, withdraw
8. [07-data-and-db-contract.md](07-data-and-db-contract.md) — Migration 224
9. [08-api-contract.md](08-api-contract.md) — Routes, codes, ETag
10. [09-ux-ui-spec.md](09-ux-ui-spec.md) — Copy and states
11. [10-screens.md](10-screens.md) — ASCII wireframes
12. [11-frontend-architecture.md](11-frontend-architecture.md) — One hook, one menu
13. [12-edge-cases.md](12-edge-cases.md) — EC-179 register
14. [13-implementation-plan.md](13-implementation-plan.md) — Waves W0–W5
15. [14-e2e-test-matrix.md](14-e2e-test-matrix.md) — One gate per EC
16. [15-cross-ref.md](15-cross-ref.md) — Law, finding, EC, wave, test, lens
17. Lenses → [`lenses/`](lenses/)

## Locked decisions

1. **One judge.** List, save, and run call `ModelGate`. A second resolver is a bug. [LAW-179-1](01-first-principles.md).
2. **Fail closed.** A missing credential, a withdrawn provider, an ineligible stored pair, or an offer-load error does not call the process provider. An operator lock is the only bypass. [LAW-179-2](01-first-principles.md).
3. **The filter is the universe.** Ingest, reprocess, rebuild, vision, summary, keyword, embedding, parse, MCP, and titles use the same set the chip uses. [LAW-179-3](01-first-principles.md).
4. **A slug that is offered can run.** A fleet connection resolves from its slug. A pair is not a free string that happens to match a card. [LAW-179-4](01-first-principles.md).
5. **Secrets stay on Deployment.** Create, rotate, test, and delete happen there. The key is never returned. [LAW-179-5](01-first-principles.md).
6. **Refresh is per provider and durable.** Live ids are discovery until a Platform Admin registers them. [LAW-179-6](01-first-principles.md).
7. **A native provider with a key is offered.** `env_connections` lists every provider `models.toml` knows when its credential env is set. [LAW-179-7](01-first-principles.md).
8. **A write shows its blast radius.** Withdraw, narrow, and delete return who breaks. Confirm marks those assignments broken. It does not retarget them. [LAW-179-8](01-first-principles.md).
9. **The picker is a convenience.** It reads `/models/eligible` only. The server returns the same 422 to a raw client. [LAW-179-9](01-first-principles.md).
10. **One hook, one menu.** Choice components do not call the inventory. [LAW-179-10](01-first-principles.md).
11. **No new secret backend.** Vault and AWS stay out. Database and Infisical stay. [LAW-179-11](01-first-principles.md).
12. **Every EC-179 has a named test.** [LAW-179-12](01-first-principles.md).

## Job in one picture

```text
  Deployment                         Organization              Workspace
  +---------------------------+      +--------------------+    +------------------+
  | connection + secret       |      | subset of slugs    |    | subset of tenant |
  | probe, refresh, register  | ---> | model ceiling      | -> | model narrowing  |
  | global model ticks        |      |                    |    | llm_roles        |
  +---------------------------+      +--------------------+    +------------------+
                                                                    |
                                                                    v
                         +------------------------------------------+
                         | ModelGate.resolve(scope, usage, request) |
                         | eligible_models, then a client           |
                         +----+----------+----------+---------------+
                              |          |          |
                              v          v          v
                           picker      save        run
                           (list)      (422)    query, chat, ingest,
                                                vision, embed, MCP
```
