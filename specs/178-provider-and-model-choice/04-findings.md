# 04 — Findings

Parent: [README](README.md) · Prev: [03-product-spec](03-product-spec.md) ·
Next: [05-choice-law](05-choice-law.md)

Each finding is a behavior in the tree at SPEC-178 W0. The law that
closes it is named. Line numbers are the tree this pack was read against.

## F-178-01 — Choice surfaces call the inventory hook

[`model-chip.tsx:46`](../../edgequake_webui/src/components/query/composer/model-chip.tsx)
calls `useLlmModels` from
[`use-providers.ts:92`](../../edgequake_webui/src/hooks/use-providers.ts).
The same hook, or `LLMModelSelector` which calls it at
[`llm-model-selector.tsx:49`](../../edgequake_webui/src/components/workspace/llm-model-selector.tsx),
feeds the query settings sheet, the document dropzone, the workspace
grids, workspace create, onboarding, the deployment fleet card, and the
vision card. The full table is [02-surfaces](02-surfaces.md).

The workspace role editor already calls `useEligibleModels`
([`usage-role-row.tsx:65`](../../edgequake_webui/src/components/settings/models/usage-role-row.tsx)).
[`model-catalog.ts:5`](../../edgequake_webui/src/lib/api/edgequake/model-catalog.ts)
says member pickers read `/models/eligible`.

- **Closes with:** [LAW-178-2](01-first-principles.md) · [LAW-178-8](01-first-principles.md)
- **EC:** [EC-178-01](11-edge-cases.md) · [EC-178-40](11-edge-cases.md)

## F-178-02 — The inventory handler describes itself as the choice list

[`list_llm_models`](../../edgequake/crates/edgequake-api/src/handlers/models.rs)
lines 134–135: show every enabled `models.toml` provider so every model
is selectable. The filter is `active_provider_names`
([`model_catalog.rs`](../../edgequake/crates/edgequake-api/src/model_catalog.rs)).
There is no offer, no ceiling, no narrowing, no capability-per-usage.

`list_models` lines 93–95 says the same for workspace and tenant create.

- **Closes with:** [LAW-178-8](01-first-principles.md)
- **EC:** [EC-178-09](11-edge-cases.md) · [EC-178-15](11-edge-cases.md)

## F-178-03 — Static cards are marked available with no credential

[`model_to_response`](../../edgequake/crates/edgequake-api/src/model_catalog.rs)
sets `available: Some(true)` at line 138 for every static card.
[`from_llm`](../../edgequake/crates/edgequake-api/src/services/model_policy_catalog.rs)
copies `available` into `probe`. A static card therefore passes the
probe filter inside `eligible_models` whenever its provider is offered.

The offer is what keeps an unconfigured provider out of a choice list.
The inventory does not apply the offer, so the card is visible there
with or without a key.

- **Closes with:** [LAW-178-8](01-first-principles.md)
- **EC:** [EC-178-08](11-edge-cases.md) · [EC-178-19](11-edge-cases.md)

## F-178-04 — Workspace assignment is a free string

[`workspace_crud.rs`](../../edgequake/crates/edgequake-api/src/handlers/workspaces/workspace_crud.rs)
copies `llm_roles` and the family fields onto the update. 
[`apply_llm_config_update`](../../edgequake/crates/edgequake-core/src/workspace_model_update.rs)
lines 82–93 insert `llm_model` and `llm_provider` into metadata with no
catalog call. Embedding update in the same file stores the model and,
when the client omits a dimension, does not read the card.

`UpdateWorkspaceRequest.llm_roles` is `serde_json::Value`
([`requests.rs`](../../edgequake/crates/edgequake-api/src/handlers/workspaces_types/requests.rs)).
The Rust shape `RoleLlmConfig` is not the wire type. W1 validates the
object before the merge. It does not require a new OpenAPI component
beyond documenting the fields the checker reads: `provider`, `model`,
and the existing optional `connection_id` and `reasoning_effort`.

- **Closes with:** [LAW-178-3](01-first-principles.md) · [LAW-178-4](01-first-principles.md)
- **EC:** [EC-178-03](11-edge-cases.md) · [EC-178-14](11-edge-cases.md) · [EC-178-29](11-edge-cases.md)

## F-178-05 — Tenant defaults and setup are free strings

[`update_tenant`](../../edgequake/crates/edgequake-api/src/handlers/workspaces/tenants.rs)
around line 451 assigns `default_llm_provider` from the trimmed body.
Create, around line 64, runs `heal_optional_mock_provider` and otherwise
stores the body. [`setup.rs`](../../edgequake/crates/edgequake-api/src/handlers/setup.rs)
writes the same fields at tenant birth. None of these call
`eligible_models`.

- **Closes with:** [LAW-178-3](01-first-principles.md)
- **EC:** [EC-178-03](11-edge-cases.md) · [EC-178-15](11-edge-cases.md)

## F-178-06 — An omitted request pair skips the check

[`ensure_pair_eligible`](../../edgequake/crates/edgequake-api/src/services/model_policy_enforcement.rs)
lines 91–93 return `Ok(())` when provider or model is missing. The
comment on lines 7–8 says requests without a pair are untouched so the
workspace default applies. The default itself is not judged.

A stored pair that the ceiling later removes still runs on this path.

- **Closes with:** [LAW-178-3](01-first-principles.md) · [LAW-178-5](01-first-principles.md)
- **EC:** [EC-178-23](11-edge-cases.md)

## F-178-07 — Streaming chat does not call the check

[`streaming.rs`](../../edgequake/crates/edgequake-api/src/handlers/chat/streaming.rs)
around line 356 calls `resolve_llm_provider_for_workspace` and branches
on `Ok(Some)`, `Ok(None)`, and `Err`. `ensure_pair_eligible` is not in
the file. The composer streams by default
(`chat_default_stream` on the chat request type).

`/query` does call the check
([`query_execute.rs`](../../edgequake/crates/edgequake-api/src/handlers/query/query_execute.rs)
around line 139). The two doors disagree.

- **Closes with:** [LAW-178-5](01-first-principles.md)
- **EC:** [EC-178-02](11-edge-cases.md)

## F-178-08 — Non-streaming chat checks a filled pair

[`completion.rs`](../../edgequake/crates/edgequake-api/src/handlers/chat/completion.rs)
lines 250–258 call `ensure_pair_eligible` only when
`llm_request.has_explicit_provider()` is true, and they pass the
resolved names. A body with a provider and no model is filled from a
lower layer first ([`fill_missing_provider_model`](../../edgequake/crates/edgequake-core/src/model_resolution.rs)),
then checked. The filled model may belong to a different provider than
the one the client named. `/query` rejects that shape earlier
(`validate_llm_override_pair`).

- **Closes with:** [LAW-178-6](01-first-principles.md)
- **EC:** [EC-178-12](11-edge-cases.md)

## F-178-09 — Auth failure retries the process provider

[`execute_sota_query_with_auth_fallback`](../../edgequake/crates/edgequake-api/src/services/query_execution.rs)
lines 199–214, and the stream twin below it, catch an auth-shaped error
on an override and call the engine again with `llm_override = None`.
Chat completion line 342, chat streaming line 504, and both query
handlers use these functions.

SPEC-175 already named this as a path to delete
([05-resolution-law](../175-perfect-settings/05-resolution-law.md) "Fail closed").
The functions are still the success path. This pack deletes their use
on the choice handlers. It does not keep a second retry policy.

- **Closes with:** [LAW-178-5](01-first-principles.md)
- **EC:** [EC-178-30](11-edge-cases.md)

## F-178-10 — `Ok(None)` selects the process provider

Streaming around line 375 and completion around line 272 treat
`Ok(None)` as "use the server default." The resolver returns `None`
when `try_create_llm_provider` finds no credential
([`resolver.rs`](../../edgequake/crates/edgequake-api/src/providers/resolver.rs)
`try_create_llm_provider`). The process provider is then used even when
the named provider was the one settings withdrew, if the name was empty
by the time `refuse_unoffered` ran.

- **Closes with:** [LAW-178-5](01-first-principles.md)
- **EC:** [EC-178-13](11-edge-cases.md)

## F-178-11 — A query-role connection discards the request

[`resolve_llm_provider_for_workspace`](../../edgequake/crates/edgequake-api/src/providers/resolver.rs)
lines 439–446: if `try_llm_from_workspace_connection` returns a client
for `LlmRole::Query`, the function returns it and does not read the
request pair. `try_llm_from_workspace_connection` (line 505) keys off
`connection_id` on the role.

SPEC-175 puts the request at step 2 and the role at step 3. The code
puts the connection first. A chip selection is ignored whenever the
role has a connection id.

- **Closes with:** [LAW-178-9](01-first-principles.md)
- **EC:** [EC-178-11](11-edge-cases.md)

## F-178-12 — Offer load failure is fail-open

[`refuse_unoffered`](../../edgequake/crates/edgequake-api/src/providers/resolver.rs)
lines 488–490 log "provider offer unavailable; query was not refused"
and return `Ok(())`. A database error while loading the offer lets the
named provider run.

- **Closes with:** [LAW-178-5](01-first-principles.md)
- **EC:** [EC-178-25](11-edge-cases.md)

## F-178-13 — The sanitizer judges the inventory

[`sanitizeQueryModelSelection`](../../edgequake_webui/src/lib/query-model-selection.ts)
lines 37–66 keep a pair when it appears in the model list passed in and
the provider health entry is enabled and available. Callers pass the
`/models/llm` list
([`use-query-conversation-lifecycle.ts`](../../edgequake_webui/src/hooks/use-query-conversation-lifecycle.ts)).
A ceiling disable does not remove the pair from that list, so the
sanitizer keeps it. When the lists are empty the function also keeps
the pair (lines 53–55), so a loading state does not clear a stale id.

- **Closes with:** [LAW-178-7](01-first-principles.md)
- **EC:** [EC-178-04](11-edge-cases.md)

## F-178-14 — Inherit is labeled with the process model

[`model-chip.tsx`](../../edgequake_webui/src/components/query/composer/model-chip.tsx)
lines 57–61 and 79–81 take `default_provider` and `default_model` from
the inventory response. Those fields are
`state.query.llm_provider` ([`models.rs:149`](../../edgequake/crates/edgequake-api/src/handlers/models.rs)).
`GET /config/effective` is the workspace resolution and is unused by the
chip.

- **Closes with:** [LAW-178-3](01-first-principles.md)
- **EC:** [EC-178-06](11-edge-cases.md)

## F-178-15 — Two inventory hooks, three clocks

| Hook | File | Query key | Stale |
|------|------|-----------|-------|
| `useLlmModels` | [`use-providers.ts:92`](../../edgequake_webui/src/hooks/use-providers.ts) | `["llm-models"]` | 30s |
| `useLlmModels` | [`use-models.ts:61`](../../edgequake_webui/src/hooks/use-models.ts) | `modelsQueryKeys.llm` | 5 min |
| `useEligibleModels` | [`use-model-policy.ts:73`](../../edgequake_webui/src/hooks/use-model-policy.ts) | eligible key | 15s |

The barrel in [`hooks/index.ts`](../../edgequake_webui/src/hooks/index.ts)
exports the `use-models` copy. Choice files import the `use-providers`
copy. Invalidating `["eligible-models"]` does not touch either inventory
key. `refreshDynamicModels` in `use-providers.ts` invalidates
`["llm-models"]` only.

- **Closes with:** [LAW-178-7](01-first-principles.md)
- **EC:** [EC-178-17](11-edge-cases.md) · [EC-178-28](11-edge-cases.md)

## F-178-16 — Images swap in the process vision provider

[`completion.rs`](../../edgequake/crates/edgequake-api/src/handlers/chat/completion.rs)
lines 285–304: when the resolved override is `None` and the request has
images, the handler substitutes `state.query.vision_llm_provider`.
That client is the process vision model. It is not
`resolve_usage(vlm)`. Streaming builds image parts earlier in the same
handler family and then resolves the chat provider, so an image turn
can run a chat model that drops images, or the process vision model,
depending on the door.

- **Closes with:** [LAW-178-6](01-first-principles.md)
- **EC:** [EC-178-24](11-edge-cases.md)

## F-178-17 — Discovery-only ids are listed and not eligible

[`registered`](../../edgequake/crates/edgequake-api/src/services/model_policy_catalog.rs)
lines 23–26 return false when `discovery_source` is `dynamic_api`.
`eligible_models` drops `registered = false`. `list_llm_models` still
appends those rows. A chip built on the inventory shows an id that
`/query` rejects with `model_not_eligible`. Streaming does not reject
it ([F-178-07](#f-178-07--streaming-chat-does-not-call-the-check)).

This pack leaves discovery-only ids on the inventory and off every
choice list. It does not promote them to registered cards.

- **Closes with:** [LAW-178-8](01-first-principles.md)
- **EC:** [EC-178-07](11-edge-cases.md)

## F-178-18 — The enforcement comment names a different call

[`model_policy_enforcement.rs`](../../edgequake/crates/edgequake-api/src/services/model_policy_enforcement.rs)
lines 3–4 say the pair must be in `eligible_models(usage, None)`.
`None` skips the offer filter ([`eligibility.rs`](../../edgequake/crates/edgequake-core/src/model_policy/eligibility.rs)
lines 43–44). The function body passes `Some(&offer.slugs)` (line 118).
The comment is the bug. W4 corrects the comment so the next reader does
not "fix" the call back to `None`.

- **Closes with:** [LAW-178-2](01-first-principles.md)
- **EC:** [EC-178-09](11-edge-cases.md)

## F-178-19 — One persisted pair for every workspace

[`use-settings-store.ts`](../../edgequake_webui/src/stores/use-settings-store.ts)
holds a single `querySettings`. `setQuerySettings` merges provider and
model into that object (around line 159). Switching workspace does not
swap the pair. The pair is also what the next stream sends.

- **Closes with:** [LAW-178-7](01-first-principles.md)
- **EC:** [EC-178-05](11-edge-cases.md) · [EC-178-27](11-edge-cases.md)

## Finding to surface

| Finding | Surface |
|---------|---------|
| F-178-01 | Choice components in [02-surfaces](02-surfaces.md) |
| F-178-02 | `list_models`, `list_llm_models` |
| F-178-03 | `model_to_response` `available: Some(true)` |
| F-178-04 | `workspace_crud`, `apply_llm_config_update` |
| F-178-05 | `update_tenant`, `setup.rs` |
| F-178-06 | `ensure_pair_eligible` early return |
| F-178-07 | `chat/streaming.rs` |
| F-178-08 | `chat/completion.rs` explicit-provider gate |
| F-178-09 | `execute_sota_query_*_with_auth_fallback` |
| F-178-10 | `Ok(None)` branches in both chat handlers |
| F-178-11 | `resolve_llm_provider_for_workspace` connection branch |
| F-178-12 | `refuse_unoffered` error arm |
| F-178-13 | `sanitizeQueryModelSelection` |
| F-178-14 | `model-chip.tsx` `default_model` |
| F-178-15 | the two `useLlmModels` hooks |
| F-178-16 | `completion.rs` vision substitution |
| F-178-17 | `registered` vs `list_llm_models` |
| F-178-18 | module comment on `model_policy_enforcement.rs` |
| F-178-19 | `use-settings-store.ts` `querySettings` |
