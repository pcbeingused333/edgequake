# 04 — Findings

Parent: [README](README.md) · Prev: [03-product-spec](03-product-spec.md) ·
Next: [05-cascade-law](05-cascade-law.md)

Each finding is a fact in the tree at v0.38.0. The law that closes it is
in [01-first-principles](01-first-principles.md).

## F-177-01 — The tenant catalog is the process catalog

`get_tenant_model_catalog` calls `load_catalog(&state)` and does not take
a parent slug set
([`model_policy.rs`](../../edgequake/crates/edgequake-api/src/handlers/model_policy.rs)).
`load_catalog` lists LLM and embedding models from the deployment catalog
([`model_policy_catalog.rs`](../../edgequake/crates/edgequake-api/src/services/model_policy_catalog.rs)).

Closes with [LAW-177-1](01-first-principles.md). Test [EC-177-40](11-edge-cases.md).

## F-177-02 — Empty policy means every registered model

`tenant_policy_mode` returns `Unconstrained` when the ceiling is empty.
`within_ceiling` returns true when the usage has no rows
([`eligibility.rs`](../../edgequake/crates/edgequake-core/src/model_policy/eligibility.rs)).
The card copy matches: "A job with no ticks can use any registered model"
([`tenant-model-ceiling-card.tsx`](../../edgequake_webui/src/components/settings/models/tenant-model-ceiling-card.tsx)).

Closes with [LAW-177-4](01-first-principles.md). Tests [EC-177-04](11-edge-cases.md),
[EC-177-11](11-edge-cases.md).

## F-177-03 — The editor is a model grid

`TenantModelCeilingCard` builds a table whose columns are usages and whose
rows are models. There is no provider group and no provider toggle.

Closes with [LAW-177-2](01-first-principles.md). Test [EC-177-17](11-edge-cases.md).

## F-177-04 — Create stores the header tenant

`create_connection` calls `upsert(&state, None, scoped_tenant(&tenant_ctx)?, body)`
([`connections.rs`](../../edgequake/crates/edgequake-api/src/handlers/connections.rs)).
`scoped_tenant` maps a present `X-Tenant-ID` to `Some(Uuid)`. The insert
binds that value to `tenant_id`. The table allows it
([`169_spec163_provider_connections.sql`](../../edgequake/migrations/169_spec163_provider_connections.sql)).

Closes with [LAW-177-3](01-first-principles.md). Test [EC-177-12](11-edge-cases.md).

## F-177-05 — A scoped list is tenant rows plus fleet rows

`connection_list_sql` documents why a tenant read unions
`WHERE tenant_id = $1` with `WHERE tenant_id IS NULL`
([`connections.rs`](../../edgequake/crates/edgequake-api/src/handlers/connections.rs)
`connection_list_sql`). Another tenant's rows are already excluded. A
provider can still be born below Global because of F-177-04.

Closes with [LAW-177-3](01-first-principles.md). Test [EC-177-08](11-edge-cases.md).

## F-177-06 — Env providers are not rows

`env_connections` pushes `openai`, `anthropic`, `ollama`, and the local
host slugs with `Uuid::nil`
([`connections.rs`](../../edgequake/crates/edgequake-api/src/handlers/connections.rs)).
Nothing in Postgres records withdrawal.

Closes with [LAW-177-1](01-first-principles.md). Tests [EC-177-01](11-edge-cases.md),
[EC-177-02](11-edge-cases.md), [EC-177-15](11-edge-cases.md).

## F-177-07 — The allowlist is an environment variable

`active_provider_names` reads `EDGEQUAKE_ALLOWED_PROVIDERS`
([`model_catalog.rs`](../../edgequake/crates/edgequake-api/src/model_catalog.rs)).
Empty or `*` means no filter. This is a process filter. It is not the
tenant offer and it is not a withdrawal row.

Closes with [LAW-177-1](01-first-principles.md). Test [EC-177-11](11-edge-cases.md).

## F-177-08 — The ceiling sits on People

[`people-section.tsx`](../../edgequake_webui/src/components/settings/sections/people-section.tsx)
renders `TenantModelCeilingCard`. [`registry.ts`](../../edgequake_webui/src/lib/settings/registry.ts)
has `people` and `secrets` under Organization, and `connections` under
Deployment. [`SECTION_COMPONENTS`](../../edgequake_webui/src/components/settings/sections/index.tsx)
has no providers entry.

Closes with [LAW-177-7](01-first-principles.md). Test [EC-177-29](11-edge-cases.md).

## F-177-09 — Eligibility ignores the parent

`eligible_models` filters `registered`, `supports(usage)`, probe, ceiling,
and narrowing. It has no offered-slug parameter
([`eligibility.rs`](../../edgequake/crates/edgequake-core/src/model_policy/eligibility.rs)).
The workspace page renders the role editor from that list
([`workspace-models-section.tsx`](../../edgequake_webui/src/components/settings/sections/workspace-models-section.tsx)).
`resolve_usage` does not receive an offer
([`usage_resolution.rs`](../../edgequake/crates/edgequake-core/src/usage_resolution.rs)).
`broken_reason` already understands `NotEligible`
([`effective_usages.rs`](../../edgequake/crates/edgequake-api/src/services/effective_usages.rs)).

Closes with [LAW-177-5](01-first-principles.md) and [LAW-177-6](01-first-principles.md).
Tests [EC-177-06](11-edge-cases.md), [EC-177-13](11-edge-cases.md),
[EC-177-21](11-edge-cases.md).

## F-177-10 — Probe failure is a catalog fact, not an offer fact

`eligible_models` drops `probe == Some(false)`. A custom connection can
be saved before a successful test, because `upsert` does not require
`last_test_ok`. The global offer in this pack requires `last_test_ok = true`
for a fleet row. The admin list still shows the row as not offered.

Closes with [LAW-177-3](01-first-principles.md). Tests [EC-177-03](11-edge-cases.md),
[EC-177-24](11-edge-cases.md).

## F-177-11 — The offer slug and the catalog provider must be the same string

Catalog entries use `item.provider` lowercased
([`from_llm`](../../edgequake/crates/edgequake-api/src/services/model_policy_catalog.rs)).
Env rows use slugs such as `openai` and `anthropic`. A custom connection
uses the operator's slug. A proxy saved as `acme` does not automatically
own the `openai` cards in `models.toml`. The offer key is that slug.
Models discovered or confirmed on the connection are catalog entries whose
provider equals the slug. This pack does not alias `acme` to `openai`.

Closes with [LAW-177-1](01-first-principles.md) and [LAW-177-9](01-first-principles.md).
Test [EC-177-44](11-edge-cases.md).

## F-177-12 — Secrets are per tenant, connections are the credential

SPEC-176 stores new keys as `secret_ref` and chooses Infisical per tenant
on Organization → Secrets. A fleet connection passes `tenant_id NULL` into
`SecretService::put` ([`connections.rs`](../../edgequake/crates/edgequake-api/src/handlers/connections.rs)
`upsert`). The offer table must not grow a ciphertext column. The
organization secret backend is not the place a provider is created.

Closes with [LAW-177-9](01-first-principles.md). Test [EC-177-26](11-edge-cases.md).
