# 12 — Implementation plan

Parent: [README](README.md) · Prev: [11-edge-cases](11-edge-cases.md) ·
Next: [13-e2e-test-matrix](13-e2e-test-matrix.md)

W0 is this pack. W1 through W4 are product work. A wave lands its tests
in the same change. No wave adds a table.
[LAW-178-4](01-first-principles.md).

```text
  W0  spec
  W1  server: one verdict on list, save, and run
  W2  client: one hook, one menu, per-workspace slot
  W3  browser gate: chip, editor, and 422
  W4  OpenAPI, generated types, the stale comment
```

## W0 — Spec

Documents in this directory. No Rust, TypeScript, migration, or
Playwright edits. Done when [14-cross-ref](14-cross-ref.md) links every
law to a finding, an EC, a wave, and a test.

## W1 — Server

The loader changes. `eligible_models` does not gain a scope parameter.
[SRP](01-first-principles.md) · [OCP](01-first-principles.md).

| File | Change |
|------|--------|
| [`model_policy.rs`](../../edgequake/crates/edgequake-api/src/handlers/model_policy.rs) `get_eligible_models` | Accept `tenant_id` or `scope=deployment` beside `workspace_id`. Reject zero or two scopes with `400 scope_required`. Apply the grants in [08-api-contract](08-api-contract.md). |
| New loader next to [`provider_offer_rows.rs`](../../edgequake/crates/edgequake-api/src/services/provider_offer_rows.rs) and [`model_policy_rows.rs`](../../edgequake/crates/edgequake-api/src/services/model_policy_rows.rs) | `load_choice_inputs(scope) -> (offer, ceiling, narrowing)`. Deployment: global offer, empty rows. Tenant: tenant offer, ceiling, empty narrowing. Workspace: today's load. |
| [`model_policy_types.rs`](../../edgequake/crates/edgequake-api/src/handlers/model_policy_types.rs) | Add `scope`, `offer_mode`, `supports_streaming`, `embedding_dimension` to the DTOs. Fill them from the catalog item. Do not add a route. |
| [`model_policy_enforcement.rs`](../../edgequake/crates/edgequake-api/src/services/model_policy_enforcement.rs) | `ensure_resolved_pair` judges the pair that will run, including a stored assignment. `ensure_pair_complete` rejects one-sided bodies. `usage_locked` when a body pair meets a lock. Offer-load error becomes `503 offer_unavailable` instead of the fail-open arm. |
| [`chat/streaming.rs`](../../edgequake/crates/edgequake-api/src/handlers/chat/streaming.rs) | Call the two ensures. Delete the `Ok(None)` process-provider branch. Do not call `execute_sota_query_stream_with_auth_fallback`. |
| [`chat/completion.rs`](../../edgequake/crates/edgequake-api/src/handlers/chat/completion.rs) | Same. Delete the vision substitution at lines 289–304. Image turns pass `Usage::Vlm`. |
| [`query_execute.rs`](../../edgequake/crates/edgequake-api/src/handlers/query/query_execute.rs), [`query_stream.rs`](../../edgequake/crates/edgequake-api/src/handlers/query/query_stream.rs) | After the existing request check, judge the resolved pair. Stop calling the auth-fallback wrappers. |
| [`resolver.rs`](../../edgequake/crates/edgequake-api/src/providers/resolver.rs) `resolve_llm_provider_for_workspace` | If the request has both fields, do not return `try_llm_from_workspace_connection` first. Judge the request. Connection remains the empty-request path. `refuse_unoffered` returns the load error. |
| [`workspace_crud.rs`](../../edgequake/crates/edgequake-api/src/handlers/workspaces/workspace_crud.rs) | Judge family, roles, vision, embedding, decision before `apply_*`. Write nothing on the first failure. Copy embedding dimension from the card. |
| [`tenants.rs`](../../edgequake/crates/edgequake-api/src/handlers/workspaces/tenants.rs), [`setup.rs`](../../edgequake/crates/edgequake-api/src/handlers/setup.rs) | Tenant-scope verdict on `default_*` pairs. Omitted defaults stay omitted. |
| [`llm_defaults.rs`](../../edgequake/crates/edgequake-api/src/handlers/llm_defaults.rs) | Deployment-scope verdict on each pair the patch sets. |
| [`query_execution.rs`](../../edgequake/crates/edgequake-api/src/services/query_execution.rs) | Leave the fallback functions in the file only if a non-choice caller remains. Choice handlers must not reference them. A grep test locks that. |

`apply_llm_config_update` stays a string writer. The handler refuses
before the call. That keeps core free of the HTTP codes. [SRP](01-first-principles.md).

Family intersection is four calls to `eligible_models`, then a set
intersection in the handler. No new usage enum arm.

Tests in W1, beside the code:

- `edgequake-api/src/services/model_policy_enforcement.rs` unit tests for
  complete-pair, resolved-pair, lock, and offer-load error.
- `edgequake-api/tests/spec178_model_choice.rs` for the HTTP doors in
  the matrix rows marked W1.
- The existing `discovery_only_model_is_model_not_eligible` test stays
  green.

Do not add `edgequake/migrations/224_spec178_*.sql`.

## W2 — Client

| File | Change |
|------|--------|
| [`use-model-policy.ts`](../../edgequake_webui/src/hooks/use-model-policy.ts) | Scope argument. Query key `["models", "eligible", scope, id, usage]`. `useChatFamilyModels` intersects four calls. |
| [`model-catalog.ts`](../../edgequake_webui/src/lib/api/edgequake/model-catalog.ts) | Fetch helpers for the three query shapes. Types for the new fields. |
| New `model-choice-menu.tsx` next to the selectors | The menu in [10-frontend-architecture](10-frontend-architecture.md). |
| [`llm-model-selector.tsx`](../../edgequake_webui/src/components/workspace/llm-model-selector.tsx), [`embedding-model-selector.tsx`](../../edgequake_webui/src/components/workspace/embedding-model-selector.tsx) | Wrappers. No `useLlmModels`. |
| Choice parents in [02-surfaces](02-surfaces.md) | Pass scope and usage. Delete their inventory fetches. |
| [`model-chip.tsx`](../../edgequake_webui/src/components/query/composer/model-chip.tsx) | Eligible groups. Inherit from `useEffectiveUsages`. Stream bit from the payload. |
| [`query-model-selection.ts`](../../edgequake_webui/src/lib/query-model-selection.ts) | Eligible list. Distinguish loading from empty. |
| [`use-settings-store.ts`](../../edgequake_webui/src/stores/use-settings-store.ts) | `queryModelsByWorkspace`. One-time rehydrate of the global pair. |
| [`use-providers.ts`](../../edgequake_webui/src/hooks/use-providers.ts) | Remove the duplicate `useLlmModels` / `useEmbeddingModels` once callers are gone. `refreshDynamicModels` invalidates the eligible prefix. |
| Policy and offer mutation hooks | Invalidate `["models", "eligible"]`. |

The role row keeps `useEligibleModels(workspaceId, usage)` as a wrapper.
It does not grow a second request.

Tests in W2:

- Update [`query-model-selection.test.ts`](../../edgequake_webui/src/lib/__tests__/query-model-selection.test.ts).
- A test that the Choice paths do not import `useLlmModels` or
  `useEmbeddingModels` ([EC-178-40](11-edge-cases.md)).
- Store rehydrate test next to
  [`settings-store-migration.test.ts`](../../edgequake_webui/src/stores/__tests__/settings-store-migration.test.ts).

## W3 — Browser gate

New [`edgequake_webui/e2e/spec178/choice-coherence.spec.ts`](../../edgequake_webui/e2e/spec178/choice-coherence.spec.ts).
Stub `GET /models/eligible` and `GET /models/llm` with different ids so
a chip that still reads the inventory fails. Walk the demo in
[03-product-spec](03-product-spec.md): chip, workspace row, ceiling
change, 422 body.

Cite [`e2e/spec175/models-editor.spec.ts`](../../edgequake_webui/e2e/spec175/models-editor.spec.ts).
Do not copy its cases. Update
[`spec032-query-model-selection.spec.ts`](../../edgequake_webui/e2e/spec032-query-model-selection.spec.ts)
where it asserts the inventory is the chip list. The assertion becomes
"the chip matches eligible."

Playwright runs against the dev server the makefile already starts.
The spec stubs the API. It does not require a live model provider.

## W4 — Contract text

- `make codegen-openapi-refresh` so `schema.d.ts` grows `scope`,
  `offer_mode`, `supports_streaming`, `embedding_dimension`, and the
  new 422 codes.
- Replace the comment on
  [`model_policy_enforcement.rs`](../../edgequake/crates/edgequake-api/src/services/model_policy_enforcement.rs)
  lines 3–4. It must say the production call passes `Some(offer)`.
  [F-178-18](04-findings.md).
- `cargo test -p edgequake-api --test spec027_api_contract` if that
  suite snapshots the eligible schema.

## Order inside a wave

```text
  loader and DTO
       |
       v
  verdict helpers, with unit tests
       |
       v
  wire streaming, completion, query, saves
       |
       v
  HTTP tests named in the matrix
```

The client wave starts when the workspace form of `GET /models/eligible`
returns `supports_streaming`. It does not wait for OpenAPI codegen.
Types in `model-catalog.ts` are written by hand in W2 and reconciled
in W4.

## Done

- A model absent from `eligible(workspace, query)` is absent from the
  chip and refused by the stream, by `/query`, and by a workspace PUT.
- Inherit on the chip names the effective query row.
- Deployment defaults ignore workspace narrowing.
- No file under `edgequake/migrations` is added for this spec.
- [13-e2e-test-matrix](13-e2e-test-matrix.md) is green for every row.
