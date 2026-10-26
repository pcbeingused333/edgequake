# 13 — E2E and test matrix

Parent: [README](README.md) · Prev: [12-implementation-plan](12-implementation-plan.md) ·
Next: [14-cross-ref](14-cross-ref.md)

One gate per [EC-178](11-edge-cases.md). The wave is the wave that
lands the gate. Playwright lives in
`edgequake_webui/e2e/spec178/choice-coherence.spec.ts` unless the row
names another file. Rust HTTP tests live in
`edgequake/crates/edgequake-api/tests/spec178_model_choice.rs` unless
the row names the enforcement unit module
(`model_policy_enforcement.rs` tests).

| EC | Gate | Wave |
|----|------|------|
| 01 | Playwright: chip ids equal the eligible stub and exclude the inventory-only id | W3 |
| 02 | `stream_and_query_reject_the_same_pair` | W1 |
| 03 | `put_workspace_unknown_pair_is_422` and `put_tenant_unknown_pair_is_422` | W1 |
| 04 | `sanitizeQueryModelSelection` drops a pair absent from the eligible fixture. Loading does not drop it | W2 |
| 05 | Playwright: two workspace routes show two slots | W3 |
| 06 | Playwright: inherit label is the effective stub, not `default_model` | W3 |
| 07 | `discovery_only_model_is_model_not_eligible` stays, plus the id is absent from `GET /models/eligible` | W1 |
| 08 | `unoffered_static_card_absent_from_eligible` | W1 |
| 09 | `unconstrained_eligible_omits_unoffered_card` | W1 |
| 10 | Playwright: locked chip does not open. Unit: effective row `locked` | W3, W1 |
| 11 | `request_pair_beats_query_connection` | W1 |
| 12 | `model_pair_incomplete_is_422` on chat and on PUT | W1 |
| 13 | `credential_miss_does_not_call_process_provider` | W1 |
| 14 | `embedding_save_copies_card_dimension` and `embedding_dimension_mismatch_is_422` | W1 |
| 15 | Playwright: create form requests `tenant_id` and renders that payload | W3 |
| 16 | `deployment_eligible_ignores_workspace_narrowing` | W1 |
| 17 | Hook test: offer mutation invalidates `["models", "eligible"]` | W2 |
| 18 | Playwright: empty eligible disables Send. `empty_resolved_pair_is_422` | W3, W1 |
| 19 | `probe_failed_omitted_from_eligible` | W1 |
| 20 | Playwright: withdrawn slug has no group | W3 |
| 21 | `legacy_tenant_connection_absent_from_eligible` | W1 |
| 22 | `mock_absent_from_eligible` | W1 |
| 23 | `omitted_request_stored_illegal_is_422` | W1 |
| 24 | `image_turn_resolves_vlm` | W1 |
| 25 | `offer_load_error_is_503` | W1 |
| 26 | Existing `usages_left_empty` test stays green | W1 |
| 27 | Store test: `queryModelsByWorkspace` round-trip and legacy rehydrate | W2 |
| 28 | Playwright: ceiling refetch clears the chip slot. In-flight pair still 422 via EC-02 | W3 |
| 29 | `family_save_rejects_pair_missing_from_extract` | W1 |
| 30 | Grep gate: chat and query handlers do not reference `with_auth_fallback` | W1 |
| 31 | `raw_post_query_outside_set_is_422` | W1 |
| 32 | `member_deployment_scope_is_403` and `member_tenant_scope_is_403` | W1 |
| 33 | Playwright: stream switch follows `supports_streaming` with `/models/llm` stubbed to the opposite bit | W3 |
| 34 | `clear_llm_model_is_not_422` | W1 |
| 35 | Existing widening 422 stays green | W1 |
| 36 | `embedding_only_model_is_capability_mismatch_for_query` | W1 |
| 37 | `request_pair_while_locked_is_usage_locked` | W1 |
| 38 | `embedding_save_does_not_enqueue_rebuild` | W1 |
| 39 | `other_tenant_workspace_eligible_is_not_found` | W1 |
| 40 | Source test: Choice files in [02-surfaces](02-surfaces.md) do not import `useLlmModels` or `useEmbeddingModels` | W2 |
| 41 | `spec178_adds_no_migration`: no file matches `migrations/*spec178*` | W1 |

## Playwright shape

The spec stubs, in one fixture:

```text
  GET /api/v1/models/llm          models [inventory-only, shared]
  GET /api/v1/models/eligible     models [shared]     usage=query
  GET /api/v1/config/effective    query row = shared, source = workspace_role
```

The chip must show `shared` and must not show `inventory-only`.
That single assertion is EC-178-01. Later tests in the file reuse the
fixture and vary one field (locked, empty, streaming bit, two workspaces).

A 422 test posts the chat stream with `inventory-only` and expects
`model_not_eligible` in the error body. The UI test only checks that
the composer renders the sentence. The status code is the Rust test
EC-178-02, so the browser test does not become the authority for the
code.

## What is cited and not duplicated

| Existing test | Role |
|---------------|------|
| [`e2e/spec175/models-editor.spec.ts`](../../edgequake_webui/e2e/spec175/models-editor.spec.ts) | Role row is eligible-only. Stays the editor gate. |
| [`e2e/spec177/providers.spec.ts`](../../edgequake_webui/e2e/spec177/providers.spec.ts) | Offer editor. Unchanged. |
| `model_policy/tests.rs` | Ceiling, narrowing, probe, rerank. Unchanged. |
| `discovery_only_model_is_model_not_eligible` | EC-178-07, extended with the list assertion. |

## Command

```text
  cargo test -p edgequake-api --test spec178_model_choice
  cargo test -p edgequake-api --lib model_policy_enforcement
  cd edgequake_webui && pnpm exec playwright test e2e/spec178/choice-coherence.spec.ts
```

W2 unit tests run with the webui test script the repo already uses for
`src/lib/__tests__`.
