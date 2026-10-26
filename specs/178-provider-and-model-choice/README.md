# SPEC-178 — One choice set

> **Status:** Shipped in v0.38.1.
> **Product pin:** EdgeQuake v0.38.0. Migration file head is
> [223_spec177_provider_offers.sql](../../edgequake/migrations/223_spec177_provider_offers.sql).
> This pack reserves **no** migration.
> **Scope:** Which model a person may see and select, at workspace
> configuration and at query time, once settings have already decided the
> providers and the models. The list, the save, and the run are one set.
> **Inherits:** [SPEC-175](../175-perfect-settings/README.md) resolution,
> usages, and `model_catalog_policy` ·
> [SPEC-177](../177-provider-cascade/README.md) offer algebra ·
> [SPEC-123](../123-env-config-priority/) ladder ·
> [SPEC-043](../043-update-edgequake-llm/000-index.md) catalog cards ·
> [SPEC-032](../032-provider-integration/) the composer chip this pack rebinds.
> **Does not replace:** `resolve_usage`, the usage enum, the offer tables,
> the settings shell, or who may edit Deployment, Organization, and Workspace.

## Words

| Word | Meaning in this pack |
|------|----------------------|
| Inventory | `GET /models/llm` and `GET /models/embedding`. Every enabled card plus live discovery. An operator catalog. |
| Choice | A model a person may assign or override for one usage in one scope. |
| Eligible set | The result of `eligible_models` for that scope and usage. |
| Inherit | An empty choice. The run uses `resolve_usage` with no request pair. |
| Scope | `deployment`, `tenant`, or `workspace`. The scope picks the offer and the policy rows. It does not pick a different function. |

Global, tenant, and workspace keep the SPEC-177 names: Deployment,
Organization, and Workspace ([LAW-177-7](../177-provider-cascade/01-first-principles.md)).

## Start here

1. [00-why.md](00-why.md) — Five WHYs
2. [01-first-principles.md](01-first-principles.md) — LAW-178-1…10 + DRY/SOLID
3. [02-surfaces.md](02-surfaces.md) — Every picker, classified
4. [03-product-spec.md](03-product-spec.md) — Personas, stories, acceptance
5. [04-findings.md](04-findings.md) — F-178-* with file and symbol
6. [05-choice-law.md](05-choice-law.md) — Scope algebra
7. [06-screens.md](06-screens.md) — ASCII wireframes
8. [07-data-and-db-contract.md](07-data-and-db-contract.md) — No new relation
9. [08-api-contract.md](08-api-contract.md) — Eligible scopes, 422s
10. [09-ux-ui-spec.md](09-ux-ui-spec.md) — Copy and states
11. [10-frontend-architecture.md](10-frontend-architecture.md) — One hook, one picker
12. [11-edge-cases.md](11-edge-cases.md) — EC-178 register
13. [12-implementation-plan.md](12-implementation-plan.md) — Waves W0–W4
14. [13-e2e-test-matrix.md](13-e2e-test-matrix.md) — One gate per EC
15. [14-cross-ref.md](14-cross-ref.md) — Law, finding, EC, wave, test, lens
16. Lenses → [`lenses/`](lenses/)
    - [Product Owner](lenses/LENS-product-owner.md)
    - [Full Stack](lenses/LENS-full-stack.md)
    - [Database](lenses/LENS-database.md)
    - [UX / UI](lenses/LENS-ux-ui.md)
    - [Front](lenses/LENS-front.md)
    - [AI Engineer](lenses/LENS-ai-engineer.md)

## Locked decisions (Wave 0)

1. **Choice sits inside the universe settings already built.** Deployment
   offers providers. The tenant intersects and may ceiling models. The
   workspace intersects and may narrow. This pack owns the selection a
   person then makes. [LAW-178-1](01-first-principles.md).
2. **One function, three scopes.** `eligible_models` stays the filter.
   The scope changes which offer and which policy rows the caller passes.
   [LAW-178-2](01-first-principles.md).
3. **The list, the save, and the run are the same set.** A non-empty pair
   outside the set is `422 model_not_eligible` on write and on run. Empty
   means inherit. [LAW-178-3](01-first-principles.md).
4. **No new table.** Choice is not a seventh fact. Migration head stays
   223. [LAW-178-4](01-first-principles.md).
5. **Fail closed.** A missing credential, a withdrawn provider, an
   ineligible stored assignment, or an offer-load failure does not call
   the process provider. The operator lock still bypasses the offer.
   [LAW-178-5](01-first-principles.md).
6. **Query override is query-only.** The chip lists `usage=query`. Upload
   vision lists `usage=vlm`. Embedding dimension comes from the card.
   [LAW-178-6](01-first-principles.md).
7. **The server is the authority.** The picker is a convenience. A raw
   client receives the same 422. [LAW-178-7](01-first-principles.md).
8. **Inventory is not a choice list.** `GET /models/llm` stays. No choice
   control calls it. [LAW-178-8](01-first-principles.md).
9. **A request pair is kept.** For query, a non-empty eligible request
   runs. `llm_roles.query.connection_id` applies when the request is empty.
   [LAW-178-9](01-first-principles.md).
10. **Every EC-178 has a named test.** [LAW-178-10](01-first-principles.md).

## Job in one picture

```text
  Settings (already specified)
  +------------------+     +------------------+     +------------------+
  | Global offer     | --> | Tenant offer     | --> | Workspace offer  |
  | SPEC-177         |     | + model ceiling  |     | + narrowing      |
  +------------------+     | SPEC-175         |     | + llm_roles      |
                           +------------------+     +------------------+
                                                              |
                                                              v
  +-----------------------------------------------------------+
  | eligible_models(scope, usage)          one function       |
  +---------------------------+-------------------------------+
                              |
          +-------------------+-------------------+
          |                   |                   |
          v                   v                   v
  +---------------+   +---------------+   +---------------+
  | Picker        |   | Save          |   | Run           |
  | shows the set |   | 422 outside   |   | 422 outside   |
  +---------------+   +---------------+   +---------------+

  GET /models/llm is the inventory beside this picture.
  It does not feed the three boxes.
```

Who chooses what:

```text
+----------------+---------------------------+----------------------+
| Scope          | Choice                    | Who                  |
+----------------+---------------------------+----------------------+
| Deployment     | server llm_defaults       | Platform Admin       |
| Tenant         | tenant default_*          | tenant owner / admin |
|                | and the create-workspace  | or a person who may  |
|                | form, before a workspace  | create a workspace   |
|                | id exists                 |                      |
+----------------+---------------------------+----------------------+
| Workspace      | llm_roles, family keys,   | workspace owner /    |
|                | embedding, vision         | admin                |
| Workspace      | query chip, this turn     | any member who may   |
|                |                           | query                |
+----------------+---------------------------+----------------------+
```
