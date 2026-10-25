# 01 — First principles (LAW-175)

Parent: [README](README.md) · Prev: [00-why](00-why.md) · Next: [02-surfaces](02-surfaces.md)

These laws are non-negotiable for SPEC-175. A wave may defer a surface.
A wave must not ship a surface that violates a law. An empty
`model_catalog_policy` is the pre-feature state, not a violation
([LAW-175-9](#law-175-9--empty-policy-matches-today)).

## DRY / SOLID application

| Principle | Application in SPEC-175 |
|-----------|-------------------------|
| **DRY** | One `resolve_usage`. One `eligible_models`. One picker component. One settings shell. Query, extract, embed, vision, keyword, summary, decision, and `GET /config/effective` consume the resolver. |
| **SRP** | The resolver in `edgequake-core` returns a value. SQL for policy lives in storage. HTTP maps that value to status codes. React renders the value. Capability flags stay in `edgequake-llm`. |
| **OCP** | A new usage is a new enum arm plus a capability predicate. It is not a new card and a new precedence function. |
| **LSP** | REST, the worker, and MCP receive the same `ResolvedUsage`. A caller cannot tell which layer won except by reading `source`. |
| **ISP** | The query picker asks for `usage=query`. It does not receive connection secrets or the fleet editor. |
| **DIP** | Handlers depend on `resolve_usage`. They do not read `llm_roles` JSON themselves. Pages depend on the eligible-models endpoint. They do not import `models.toml`. |

---

## LAW-175-1 — One resolver

`resolve_usage(usage, ctx) -> ResolvedUsage` is a pure function in
`edgequake-core`. It chooses provider, model, connection id, and reasoning
effort, and it names the layer that won. It does not open SQL or HTTP.

Every runtime path calls it: extract, query, summary, vlm, keyword,
embedding, decision. `GET /config/effective` calls it and returns that
value. The UI does not recompute the ladder.

Closes [WHY B](00-why.md). Replaces the private ladders in
`resolve_extract_role_llm`, `resolve_llm_provider_for_workspace`, and
`resolve_workspace_vlm_config`.

- **Closes:** [F-175-02](04-findings.md) · [F-175-03](04-findings.md) · [F-175-04](04-findings.md) · [F-175-05](04-findings.md)
- **Tests:** [EC-175-01](11-edge-cases.md) · [EC-175-03](11-edge-cases.md) · [EC-175-22](11-edge-cases.md)

## LAW-175-2 — The ladder is visible

For every usage, the first layer that sets a value wins:

```text
  operator_lock
       > request          (only query, and vision on that upload)
       > workspace_role
       > workspace_default
       > tenant_default
       > server_config
       > env_bootstrap
       > compiled_default
```

A blank layer is skipped. Provenance is the winning layer's name.
`EDGEQUAKE_LLM_PROVIDER` and `EDGEQUAKE_EMBEDDING_PROVIDER` are
`env_bootstrap`. `EDGEQUAKE_CONFIG_PRIORITY` swaps `server_config` and
`env_bootstrap` only. It does not jump over workspace or tenant.

`ModelResolutionSource` grows `OperatorLock` and `ServerConfig`. Existing
`Request | Workspace | Tenant | Env | Default` map onto the layers above.
`Env` in LAW-123-2 splits into lock, server, and bootstrap so a pin is
not the same fact as a default.

- **Closes:** [F-175-02](04-findings.md) · [F-175-17](04-findings.md)
- **Tests:** [EC-175-12](11-edge-cases.md) · [EC-175-13](11-edge-cases.md)
- **Detail:** [05-resolution-law](05-resolution-law.md)

## LAW-175-3 — Usage is a type

Usages:

| Usage | Capability required | Request override |
|-------|---------------------|------------------|
| `extract` | chat + json mode | no |
| `query` | chat | yes, inside the eligible set |
| `summary` | chat | no |
| `vlm` | vision | yes, on that upload only |
| `keyword` | chat | no |
| `embedding` | embedding, fixed dimension | no |
| `decision` | decision catalog (`tev1` today) | no |
| `rerank` | rerank provider | no |

`LlmRole` stays the five chat roles. `embedding`, `decision`, and `rerank`
are usages in the same resolver and are not chat rows in the picker.
Eligibility reads `ModelCapabilities` and `ProviderFeatures` from
`edgequake-llm`. Reasoning controls render only when the effort table or
live Ollama `thinking` says the model supports them
([SPEC-109](../109-configurable-reasoning-effort/), [SPEC-131](../131-env-config-temp-openai-support/)).

- **Closes:** [F-175-07](04-findings.md) · [F-175-15](04-findings.md)
- **Tests:** [EC-175-06](11-edge-cases.md) · [EC-175-08](11-edge-cases.md)

## LAW-175-4 — Request is narrow

Only `query` and per-document `vlm` accept a request pair. Both fields
must be set, or both absent (`validate_llm_override_pair` stays). The id
must be in `eligible_models` for that usage and workspace. Otherwise 422
`model_not_eligible`. Extract, summary, keyword, embedding, decision, and
rerank ignore a request pair.

The query UI default stays inherit: `provider` and `model` are `undefined`
in [`use-settings-store.ts:50`](../../edgequake_webui/src/stores/use-settings-store.ts).
Setting them is an explicit per-browser override, still inside the eligible
set, and the chip shows `request` as the source.

- **Closes:** [F-175-16](04-findings.md)
- **Tests:** [EC-175-03](11-edge-cases.md) · [EC-175-28](11-edge-cases.md)

## LAW-175-5 — Fail closed

If the winning layer names a provider with no credential, a deleted
connection, a failed probe, or a capability mismatch, the call returns a
typed error. It does not call `ProviderFactory::from_env` and it does not
retry on the process key.

`execute_sota_query_with_auth_fallback` stops retrying onto the server
default. `try_create_llm_provider` returning `None` becomes
`credential_unavailable` (503) at the handler, not a silent success.

A broken workspace assignment does not fall through to the compiled
default. The effective payload sets `broken_reason`. Ingest and query for
that usage refuse until the assignment is fixed or cleared.

- **Closes:** [WHY E](00-why.md) · [F-175-06](04-findings.md)
- **Tests:** [EC-175-04](11-edge-cases.md) · [EC-175-05](11-edge-cases.md) · [EC-175-32](11-edge-cases.md)

## LAW-175-6 — Policy ceiling, then assignment

Three stores, three jobs:

| Store | Job | Must not |
|-------|-----|----------|
| `provider_connections` | URL, auth, timeout | store a model id |
| `model_catalog_policy` | which provider+model+usage is enabled | store a secret |
| `llm_roles` and `default_*` | which enabled pair a usage runs | store a key |

Instance admin creates connections. Tenant owner or admin writes ceiling
rows (`workspace_id` null). Workspace owner or admin may add narrowing
rows and writes assignments. A workspace row cannot enable a pair the
ceiling omitted. Members read the narrowed query set. They do not write
policy.

- **Closes:** [F-175-14](04-findings.md) · [F-175-18](04-findings.md)
- **Tests:** [EC-175-16](11-edge-cases.md) · [EC-175-30](11-edge-cases.md) · [EC-175-31](11-edge-cases.md)
- **Detail:** [06-catalog](06-catalog.md) · [07-data-and-db-contract](07-data-and-db-contract.md)

## LAW-175-7 — A control writes its reader

A control in the product either writes the field the pipeline reads, or
it is removed. Personal `enableRerank`, `rerankTopK`, `enableGleaning`,
`maxGleaning`, and `useLLMSummarization` have no server reader. They leave
the You scope.

Rerank stays a deployment fact (`EDGEQUAKE_RERANKER*`). `rerank_model` on
the query body is rejected until a later spec wires it. This pack does
not add a gleaning column.

Theme and language have one writer each. The header and the You page call
that writer.

- **Closes:** [WHY D](00-why.md) · [F-175-08](04-findings.md) · [F-175-12](04-findings.md)
- **Tests:** [EC-175-09](11-edge-cases.md) · [EC-175-17](11-edge-cases.md) · [EC-175-23](11-edge-cases.md)

## LAW-175-8 — Scope is the navigation

One shell. Four scopes. Search matches section titles and control labels.

| Scope | Route | Who sees it |
|-------|-------|-------------|
| You | `/settings/you` | every signed-in user |
| Workspace | `/settings/workspace` and `/w/{slug}/settings` | members of that workspace; editors as in LAW-175-13 |
| Organization | `/settings/organization` | tenant manage |
| Deployment | `/settings/deployment` | instance admin |

`/settings` redirects to `/settings/you`. `/w/{slug}/settings` renders the
workspace scope for that slug. It does not redirect to `/workspace`.

Contextual sheets (query, graph overlay, upload) stay beside the work.
They do not grow a second model store. Notion's rule: the setting sits
next to the object. Vercel's rule: the owner is labeled.

- **Closes:** [WHY A](00-why.md) · [F-175-01](04-findings.md) · [F-175-10](04-findings.md)
- **Tests:** [EC-175-24](11-edge-cases.md) · [EC-175-33](11-edge-cases.md) · [EC-175-34](11-edge-cases.md)
- **Detail:** [09-ux-ui-spec](09-ux-ui-spec.md)

## LAW-175-9 — Empty policy matches today

Zero rows in `model_catalog_policy` for a tenant means every healthy,
capability-matched model is eligible. Existing workspaces keep their
metadata assignments. Acc with no policy rows behaves as it does once the
pins are read as locks.

The first ceiling row switches that tenant from unconstrained to
constrained. The UI says so before the write.

- **Tests:** [EC-175-19](11-edge-cases.md) · [EC-175-02](11-edge-cases.md)

## LAW-175-10 — Pins are locks

These environment variables are `operator_lock` for the named usage when
either the model or the provider variable is non-empty:

| Usage | Variables |
|-------|-----------|
| `extract` | `EDGEQUAKE_EXTRACT_LLM_MODEL`, `EDGEQUAKE_EXTRACT_LLM_PROVIDER` |
| `keyword` | `EDGEQUAKE_KEYWORD_LLM_MODEL`, `EDGEQUAKE_KEYWORD_LLM_PROVIDER` |
| `vlm` on manuscript-class pages | `EDGEQUAKE_VISION_PROVIDER_MANUSCRIPT`, `EDGEQUAKE_VISION_MODEL_MANUSCRIPT` |

When the lock is set, the picker for that usage is disabled and the row
shows "Locked by operator". `connection_id` on that role is ignored while
the lock is set, matching today's `connection_id: None` in
`env_role_llm`. Unset variables do not beat a workspace choice.

- **Closes:** [F-175-03](04-findings.md)
- **Tests:** [EC-175-01](11-edge-cases.md) · [EC-175-02](11-edge-cases.md) · [EC-175-20](11-edge-cases.md) · [EC-175-21](11-edge-cases.md)

## LAW-175-11 — Credential and model stay apart

A connection row holds base URL, auth, timeout, locality, and probe
result. The model id lives on the usage assignment and is passed into
`load_connection_spec` as it is today
([`resolver.rs:470`](../../edgequake/crates/edgequake-api/src/providers/resolver.rs)).
A workspace may bind only a connection in its tenant, or a fleet
connection (`tenant_id` null) that deployment has marked available.
Another tenant's connection is 404.

- **Tests:** [EC-175-15](11-edge-cases.md) · [EC-175-25](11-edge-cases.md)

## LAW-175-12 — Effective config is computed

Nothing stores "the model that will run" except the lineage columns
already written after a run (`messages.llm_model`,
`pdf_documents.vision_model`). `GET /config/effective` is a pure read of
`resolve_usage`. Clients that need the badge call that endpoint. They do
not cache it across a workspace switch.

- **Tests:** [EC-175-01](11-edge-cases.md)

## LAW-175-13 — The server is the authority

Three checks, three people. Names are in [15-roles](15-roles.md).

- Workspace assignment, narrowing, ingestion, and labels: a Workspace
  Admin of that workspace, a Super Admin of the tenant, or a Platform
  Admin. The check is `can_manage_workspaces` on a membership that
  `applies_to` that workspace.
- Tenant defaults, the catalog ceiling, invites, and creating a workspace:
  a Super Admin (`TenantAccess::Manage` via `get_user_role`, which keeps
  only tenant-wide rows) or a Platform Admin. A Workspace Admin receives
  403.
- Connections, fleet defaults, attribution, and identity providers:
  Platform Admin (`Role::Admin`).

The client hides the control. A hidden control is not the check. Delete
tenant stays tenant-wide Owner only. The last tenant Owner cannot be
removed.

- **Closes:** [F-175-18](04-findings.md) · [F-175-25](04-findings.md)
- **Tests:** [EC-175-16](11-edge-cases.md) · [EC-175-31](11-edge-cases.md) · [EC-175-36](11-edge-cases.md) · [EC-175-38](11-edge-cases.md)

## LAW-175-14 — A model change is not a migration

Changing extract or embedding does not reprocess documents. The rebuild
button stays an explicit workspace action. New chunks use the new model.
Keyword cache hashes already include the model. Answer cache must not
return a completion produced under another model: the hash input includes
the resolved model id. Embedding dimension changes still require the
existing rebuild.

- **Tests:** [EC-175-18](11-edge-cases.md) · [EC-175-26](11-edge-cases.md) · [EC-175-27](11-edge-cases.md)

## LAW-175-15 — Every EC has a named test

Each [EC-175](11-edge-cases.md) row has one primary gate in
[13-e2e-test-matrix](13-e2e-test-matrix.md). Unit tests cover the pure
resolver. API tests cover 422 and 503. Playwright covers the shell, the
ceiling, and the disabled lock.

## LAW-175-16 — Contextual UI binds to the resolver

The query chip, the upload vision fields, and the workspace usage matrix
are three views of `eligible_models` + `ResolvedUsage`. Graph layout,
node size, and color stay in `edgequake-settings`. They are display
preferences, not model policy. The graph popover titled "Query Settings"
is renamed so it is not a second query model form.

- **Closes:** [F-175-11](04-findings.md) · [F-175-23](04-findings.md) · [F-175-24](04-findings.md)
- **Tests:** [EC-175-28](11-edge-cases.md) · [EC-175-35](11-edge-cases.md)

## LAW-175-17 — Scope is the membership row

Super Admin and Workspace Admin are not two strings in a new enum. They
are the same `MembershipRole` values (`owner`, `admin`) with a different
`workspace_id`. Null means the tenant. A set id means that workspace.

Do not mount catalog or assignment writes on `/api/v1/settings/`. That
prefix is `global_resource` and skips the membership check. Tenant routes
use `require_tenant_access`. Workspace routes use `membership_role` for
the target workspace.

The shell chip is the highest grant: Platform admin, else Super admin,
else Workspace admin, else Member. Platform admin never displays as Super
admin.

- **Closes:** [F-175-26](04-findings.md)
- **Tests:** [EC-175-37](11-edge-cases.md) · [EC-175-39](11-edge-cases.md) · [EC-175-40](11-edge-cases.md) · [EC-175-41](11-edge-cases.md)
- **Detail:** [15-roles](15-roles.md)
