# 00 — WHY (5-WHY)

Parent: [README](README.md) · Next: [01-first-principles](01-first-principles.md)

## The job to be done

A platform operator adds an OpenAI-compatible or Anthropic-compatible
endpoint and a key once, on the deployment. A tenant admin then decides
which of those providers the organization may use, and which models of
each provider. A workspace admin decides which of the organization's
providers that workspace may use, and which model runs each job.

If the deployment never offered a provider, the tenant screen does not
list it. If the tenant never offered a provider, the workspace screen
does not list it.

SPEC-175 already chooses the model that runs, and SPEC-176 already stores
the key. Neither one is a parent/child offer. That is why this pack exists.

---

## 5-WHY chain A — The tenant sees the world

| # | Question | Answer |
|---|----------|--------|
| 1 | Why can a tenant admin tick a model the deployment never connected? | `get_tenant_model_catalog` loads `load_catalog`, which is the deployment `/models` list ([`model_policy.rs`](../../edgequake/crates/edgequake-api/src/handlers/model_policy.rs) `get_tenant_model_catalog`, [`model_policy_catalog.rs`](../../edgequake/crates/edgequake-api/src/services/model_policy_catalog.rs) `load_catalog`). |
| 2 | Why is that list so wide? | `active_provider_names` filters with `EDGEQUAKE_ALLOWED_PROVIDERS` or returns every enabled card in `models.toml` ([`model_catalog.rs`](../../edgequake/crates/edgequake-api/src/model_catalog.rs)). A card does not require a connection row. |
| 3 | Why does an empty ceiling not shrink it? | `within_ceiling` returns true when the usage has no rows ([`eligibility.rs`](../../edgequake/crates/edgequake-core/src/model_policy/eligibility.rs)). The card says a job with no ticks can use any registered model ([`tenant-model-ceiling-card.tsx`](../../edgequake_webui/src/components/settings/models/tenant-model-ceiling-card.tsx)). |
| 4 | Why is "registered" not "the parent offered it"? | SPEC-175's universe is the catalog. The catalog is the process, not a grant from Deployment to Organization. |
| 5 | **Root cause** | **Unconstrained means every model the process knows. It does not mean every provider the parent enabled.** |

```text
  models.toml + discovery + env
              |
              v
  +---------------------------+
  | load_catalog              |
  | no parent slug filter     |
  +-------------+-------------+
                |
                v
  +---------------------------+
  | tenant ceiling editor     |
  | empty usage = all of that |
  +---------------------------+
```

Cross-ref: [LAW-177-1](01-first-principles.md) ·
[LAW-177-4](01-first-principles.md) ·
[F-177-01](04-findings.md) · [F-177-02](04-findings.md) ·
[EC-177-01](11-edge-cases.md) · [EC-177-11](11-edge-cases.md).

---

## 5-WHY chain B — Models before providers

| # | Question | Answer |
|---|----------|--------|
| 1 | Why is the organization screen a grid of models by job? | [`tenant-model-ceiling-card.tsx`](../../edgequake_webui/src/components/settings/models/tenant-model-ceiling-card.tsx) renders one row per model and one column per usage. |
| 2 | Why is there no provider switch above that grid? | `model_catalog_policy` stores `(provider, model, usage, enabled)`. There is no `provider_offers` table. Turning a provider off means ticking every model off, per usage. |
| 3 | Why does that miss the operator's question? | The operator asks "may this organization call Anthropic?" The grid answers "may this job use this model id?" |
| 4 | Why did the grid land on People? | [`people-section.tsx`](../../edgequake_webui/src/components/settings/sections/people-section.tsx) mounts `TenantModelCeilingCard` next to the members link. [`registry.ts`](../../edgequake_webui/src/lib/settings/registry.ts) has no Organization → Providers section. |
| 5 | **Root cause** | **The policy row is a model pair. The decision the admin makes first is a provider.** |

Cross-ref: [LAW-177-2](01-first-principles.md) ·
[F-177-03](04-findings.md) · [F-177-08](04-findings.md) ·
[EC-177-17](11-edge-cases.md) · [EC-177-29](11-edge-cases.md).

---

## 5-WHY chain C — A key can be born on a tenant

| # | Question | Answer |
|---|----------|--------|
| 1 | Why can `POST /api/v1/connections` store a tenant id? | `create_connection` passes `scoped_tenant` into `upsert` ([`connections.rs`](../../edgequake/crates/edgequake-api/src/handlers/connections.rs)). `scoped_tenant` reads `X-Tenant-ID`. |
| 2 | Why does the schema allow it? | [`169_spec163_provider_connections.sql`](../../edgequake/migrations/169_spec163_provider_connections.sql) declares `tenant_id UUID` without `NOT NULL`, and `UNIQUE (tenant_id, slug)`. |
| 3 | Why does a tenant list then mix two owners? | `connection_list_sql` unions `tenant_id = $1` with `tenant_id IS NULL`. |
| 4 | Why is that the wrong birthplace for a shared provider? | A key created under Tenant A is invisible to Tenant B, and it never passed through a deployment offer. The deployment admin did not decide that this provider exists. |
| 5 | **Root cause** | **The connection table is both the fleet credential store and a per-tenant BYOK store. The product asked for one birthplace.** |

```text
  Platform Admin, X-Tenant-ID set
              |
              v
  +---------------------------+
  | provider_connections      |
  | tenant_id = Tenant A      |
  +-------------+-------------+
                |
                +-- visible to Tenant A
                +-- absent from Tenant B
                +-- absent from the global offer
```

Cross-ref: [LAW-177-3](01-first-principles.md) ·
[F-177-04](04-findings.md) · [F-177-05](04-findings.md) ·
[EC-177-08](11-edge-cases.md) · [EC-177-12](11-edge-cases.md) ·
[EC-177-33](11-edge-cases.md).

---

## 5-WHY chain D — An env provider cannot be hidden

| # | Question | Answer |
|---|----------|--------|
| 1 | Why does OpenAI appear when `OPENAI_API_KEY` is set? | `env_connections` pushes slug `openai` with shape `openai_chat` ([`connections.rs`](../../edgequake/crates/edgequake-api/src/handlers/connections.rs) `env_connections`). The id is `Uuid::nil`. There is no row to delete. |
| 2 | Why does withdrawing it matter? | LAW-177-1 says a provider the deployment does not offer is absent at the tenant. An env key is a real credential. The operator may still want tenants not to see it. |
| 3 | Why is `EDGEQUAKE_ALLOWED_PROVIDERS` not that switch? | It is a process allowlist ([`active_provider_names`](../../edgequake/crates/edgequake-api/src/model_catalog.rs)). It is not a row an admin can toggle per provider in the UI, and it is not stored per deployment in Postgres. |
| 4 | Why must the key stay in the environment? | Operator locks and the bootstrap leaf still read the process credential ([LAW-175-2](../175-perfect-settings/01-first-principles.md), [LAW-175-10](../175-perfect-settings/01-first-principles.md)). Hiding a provider from tenants is not the same as unsetting the key. |
| 5 | **Root cause** | **Env providers are computed. Nothing persists "this deployment withdrew this slug."** |

Cross-ref: [LAW-177-1](01-first-principles.md) ·
[LAW-177-8](01-first-principles.md) ·
[F-177-06](04-findings.md) · [F-177-07](04-findings.md) ·
[EC-177-02](11-edge-cases.md) · [EC-177-10](11-edge-cases.md) ·
[EC-177-15](11-edge-cases.md).

---

## 5-WHY chain E — The screen and the run can name a stranger

| # | Question | Answer |
|---|----------|--------|
| 1 | Why can a workspace assignment name a provider the organization never granted? | There is no grant check in `eligible_models`. The function filters capability, registration, probe, ceiling, and narrowing ([`eligibility.rs`](../../edgequake/crates/edgequake-core/src/model_policy/eligibility.rs) `eligible_models`). |
| 2 | Why does the workspace page still show that model? | [`workspace-models-section.tsx`](../../edgequake_webui/src/components/settings/sections/workspace-models-section.tsx) mounts the role editor and the narrowing card on the eligible list. The eligible list is the catalog after policy, and the catalog is global. |
| 3 | Why would a deleted connection still be dangerous? | SPEC-175 fail-closed is specified. A provider that was never in the parent offer is a different bug: the pair can look eligible because the catalog card is healthy via the process key. |
| 4 | Why must one function own this? | `resolve_usage` ([`usage_resolution.rs`](../../edgequake/crates/edgequake-core/src/usage_resolution.rs)) is already the run. `broken_reason` already maps `PairVerdict::NotEligible` ([`effective_usages.rs`](../../edgequake/crates/edgequake-api/src/services/effective_usages.rs)). A second cascade in React would drift. |
| 5 | **Root cause** | **Eligibility knows catalog, capability, and model policy. It does not know the parent offer.** |

Cross-ref: [LAW-177-5](01-first-principles.md) ·
[LAW-177-6](01-first-principles.md) ·
[F-177-09](04-findings.md) ·
[EC-177-06](11-edge-cases.md) · [EC-177-07](11-edge-cases.md) ·
[EC-177-13](11-edge-cases.md).

---

## What this pack refuses to do

- It does not add a scope called Global. Deployment remains the scope.
- It does not move API keys onto the tenant or the workspace.
- It does not put a second model matrix next to `model_catalog_policy`.
- It does not auto-reprocess a corpus when a provider is withdrawn.
- It does not rewrite legacy tenant-scoped ciphertext. Those rows stay
  readable by their owner and are retired in a later pack.
- It does not change decision extraction (`EDGEQUAKE_DECISION_*`, `tev1`)
  or the rerank process flag.
- It does not replace `EDGEQUAKE_ALLOWED_PROVIDERS`. That env filter still
  runs before the offer. A slug it excludes is not offered.
