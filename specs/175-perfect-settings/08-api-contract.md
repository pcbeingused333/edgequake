# 08 — API contract

Parent: [README](README.md) · Prev: [07-data-and-db-contract](07-data-and-db-contract.md) ·
Next: [09-ux-ui-spec](09-ux-ui-spec.md)

All routes stay under `/api/v1` behind `protected_api_auth`. Handlers call
`resolve_usage` or the policy repository. They do not reimplement the
ladder.

## Implementation status (2026-10-25)

| Endpoint | Status |
| --- | --- |
| `GET /api/v1/me/settings-access` | Implemented. Returns the caller's own grants: `instance_admin`, `tenant_role`, `tenant_manage`, `workspace_id`, `workspace_manage`, `administered_workspace_ids`. Fails closed for inactive tenants. Global-resource prefix `/api/v1/me/` in request authorization. |
| `GET /models/eligible?workspace_id=&usage=` | Implemented (W2). Member-only. Returns `policy_mode` per usage (`constrained` when the tenant ceiling has a row for that usage). Discovery-only models (`dynamic_api`) are excluded. Rerank and unknown usages are 422. |
| `GET/PUT /tenants/{id}/model-policy` | Implemented (W2). Tenant manage only (`require_tenant_access`, since `/api/v1/tenants` is a global resource). PUT is a full replace in one transaction. An empty array returns to unconstrained. |
| `GET/PUT /workspaces/{id}/model-policy` | Implemented (W2). Read for members, write for workspace owner/admin or tenant manager. A narrowing row that widens the ceiling is 422 `policy_widens_ceiling`. |
| 422 `policy_empty_usage` | Implemented (W2, added decision). A write that leaves a written usage with no selectable catalog model is refused, so members are never locked out. Not in the original list. |
| `GET /tenants/{id}/model-catalog` | Implemented. Manage-only. Registered models with `health` (`healthy` / `probe_failed` / `unknown`) and the usages each can serve. Feeds the organization ceiling grid. |
| `GET /config/effective?workspace_id=` usages and `policy_mode` | Implemented (W1/W2). `workspace_id` is required (400 when missing). Member-only (403 otherwise, 404 for an unknown workspace). Returns one `ResolvedUsage` per usage in `usages` (seven: `extract`, `query`, `summary`, `vlm`, `keyword`, `embedding`, `decision`; `rerank` is a deployment fact, not a row). Each row carries `eligible` and `broken_reason` (`none` / `capability` / `not_eligible`). `credential`, `probe` and `cross_tenant` are not computed yet. Legacy `llm`/`embedding`/`vision` fields are kept. Resolution is `edgequake_core::usage_layers` over `resolve_usage`. |
| 422 `model_not_eligible` and `capability_mismatch` on `POST /query`, `POST /query/stream`, `POST /chat/completions`, and `POST /documents/pdf` (and batch) | Implemented (W1). A request-supplied pair must be in `eligible_models(usage)` for the workspace (`query` for query and chat, `vlm` for upload vision). Requests without a pair are untouched, and workspace defaults are not re-checked on upload. Partial pairs keep their existing rules. |
| 422 `rerank_model_rejected` | Implemented on `POST /query` and `POST /query/stream`. |
| 422 `usage_locked` | Deferred. No request path can reach it yet: only `extract` and `keyword` locks exist, and no request pair is accepted for those usages. |

## Types

`RoleLlmConfig` replaces `llm_roles?: unknown` on workspace create,
update, and response.

```text
  RoleLlmConfig
    provider           string, optional
    model              string, optional
    reasoning_effort   string, optional
    connection_id      uuid, optional

  LlmRoles
    extract, query, summary, vlm, keyword, embedding
    each RoleLlmConfig, optional
```

`ResolvedUsage` (effective and error bodies):

```text
  usage, provider, model, connection_id
  reasoning_effort, reasoning_source, reasoning_clamped
  source          operator_lock | request | workspace_role
                  | workspace_default | tenant_default
                  | server_config | env_bootstrap | compiled_default
  locked          bool
  eligible        bool
  broken_reason   none | not_eligible | capability | credential
                  | probe | cross_tenant
  policy_mode     unconstrained | constrained
```

## Reads

| Method | Path | Who | Body |
|--------|------|-----|------|
| GET | `/config/effective?workspace_id=` | member of the workspace | `{ usages: ResolvedUsage[] }` for all eight usages |
| GET | `/models/eligible?workspace_id=&usage=` | member | `{ models: [{ provider, model, capabilities }], policy_mode }` |
| GET | `/tenants/{id}/model-policy` | Super Admin or Platform Admin | ceiling rows + `policy_mode` |
| GET | `/workspaces/{id}/model-policy` | member read, owner write | narrowing rows |

`GET /config/effective` already exists (`handlers/settings.rs`). W2
extends it. Old fields for the single LLM pair stay until W5 removes
the duplicate panel. New clients read `usages`.

`GET /models`, `/models/llm`, `/models/embedding`, `/models/health` stay
the deployment catalog. Member pickers call `/models/eligible`, not
`/models`.

## Writes

| Method | Path | Who | Effect |
|--------|------|-----|--------|
| PUT | `/tenants/{id}/model-policy` | Super Admin or Platform Admin | replace ceiling rows |
| PUT | `/workspaces/{id}/model-policy` | Workspace Admin of that id, Super Admin, or Platform Admin | replace narrowing rows |
| PUT | `/workspaces/{id}` | Workspace Admin of that id, Super Admin, or Platform Admin | assignments, unchanged fields |
| PUT | `/tenants/{id}` | Super Admin or Platform Admin | `default_*`, unchanged |
| PATCH | `/settings/llm-defaults` | instance admin | server layer |
| POST | `/connections` | instance admin | unchanged |
| POST | `/providers/test` | instance admin | unchanged; response gains capability bits |

Policy PUT is a full replace of that scope's rows inside one
transaction: delete that scope, insert the body, commit. A unique-index
conflict rolls back and the previous rows stay. Empty array on the
tenant route returns to unconstrained and deletes ceiling rows. The
server rejects a workspace row that widens the ceiling with 422
`policy_widens_ceiling`. [EC-175-48](11-edge-cases.md).

## Query and upload

`POST /query` and `POST /query/stream`:

- both `llm_provider` and `llm_model` set, or both absent
- if set, the pair must be in `eligible_models(query)` or 422
  `model_not_eligible`
- `rerank_model` if present and non-null: 422 `rerank_model_rejected`
- `enable_rerank` stays accepted and still only gates the process
  reranker. The WebUI stops sending it from a personal toggle. API
  clients may still pass it.

Upload vision fields `vision_provider` and `vision_model` follow the same
pair rule against `eligible_models(vlm)`.

Chat completions `provider` + `model` use the same check.

## Status codes

| Condition | HTTP | Code |
|-----------|------|------|
| Pair not in the eligible set | 422 | `model_not_eligible` |
| Capability predicate fails | 422 | `capability_mismatch` |
| Usage locked and the client sent a different pair | 422 | `usage_locked` |
| Workspace policy widens the ceiling | 422 | `policy_widens_ceiling` |
| Rerank model sent | 422 | `rerank_model_rejected` |
| Rerank policy write | 422 | `usage_not_configurable` |
| Winning provider has no key or probe failed | 503 | `credential_unavailable` |
| Connection belongs to another tenant | 404 | `connection_not_found` |
| Member or Workspace Admin reads or writes tenant policy | 403 | `tenant_access_denied` |
| Workspace Admin writes a workspace they do not administer | 403 | existing permission error |
| Member writes a workspace assignment | 403 | existing permission error |
| Remove or demote the last active tenant Owner | 409 | `last_tenant_owner` |
| Workspace or tenant id belongs to another tenant | 404 | not found |

503 is deliberate. A 200 that used the platform key is the bug in
[F-175-06](04-findings.md). The body includes `ResolvedUsage` so the UI
can show the broken sentence without a second request.

## Permission map

| Action | Who | Existing symbol |
|--------|-----|-----------------|
| Workspace assignment and narrowing | Workspace Admin of that workspace, Super Admin, or Platform Admin | `membership_role` + `can_manage_workspaces` |
| Tenant defaults, ceiling, create workspace | Super Admin or Platform Admin | `TenantAccess::Manage` via `get_user_role` |
| Delete tenant | tenant-wide Owner, or Platform Admin | `can_delete_tenant`, `require_platform_admin` |
| Last tenant Owner removal | nobody | new 409 `last_tenant_owner` |
| Fleet, connections, attribution, IdP | Platform Admin | `ApiRequireAdmin` |
| Eligible read, effective read | workspace member | `bind_request` |

Names are defined in [15-roles](15-roles.md). These routes are not under
`/api/v1/settings/`. `/api/v1/tenants` is also `global_resource`, so the
ceiling handler calls `require_tenant_access` itself
([F-175-26](04-findings.md), [EC-175-51](11-edge-cases.md)).

`/config/effective` today sits on the settings router, which
`bind_request` treats as a global resource ([F-175-18](04-findings.md)).
W2 requires a workspace id and runs the workspace membership check. A
missing id is 400. An id outside the caller's tenant is 404.

## Compatibility

- Missing `llm_roles` key: inherit. Same as today.
- `llm_roles.embedding` with only `connection_id`: still valid.
- No policy rows: `policy_mode = unconstrained`.
- `EDGEQUAKE_EXTRACT_LLM_MODEL` set: effective extract `source` is
  `operator_lock` even if the client never called the new policy routes.
- SPEC-131 env flags: no new fields.
