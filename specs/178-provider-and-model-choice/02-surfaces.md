# 02 — Surfaces

Parent: [README](README.md) · Prev: [01-first-principles](01-first-principles.md) ·
Next: [03-product-spec](03-product-spec.md)

Every place a person sees a model name is one of four kinds. W2 moves
the Choice rows onto `useEligibleModels`. The Inventory row stays.
The Already-eligible rows stay, and they become the pattern the Choice
rows copy.

## Kinds

| Kind | List | Save | Run |
|------|------|------|-----|
| Choice | `eligible(scope, usage)` | pair must be in that set, or empty | same set |
| Already eligible | `GET /models/eligible?workspace_id` | policy rows, already checked | `resolve_usage` |
| Inventory | `GET /models/llm`, `GET /models/embedding` | none | none |
| Effective | `GET /config/effective` | none | the row is what runs |

[LAW-178-8](01-first-principles.md) · [10-frontend-architecture](10-frontend-architecture.md).

## Choice surfaces that still read the inventory

These import `useLlmModels` from
[`use-providers.ts`](../../edgequake_webui/src/hooks/use-providers.ts)
or a selector that does.

| Surface | File | Scope after W2 | Usage |
|---------|------|----------------|-------|
| Query chip | [`model-chip.tsx`](../../edgequake_webui/src/components/query/composer/model-chip.tsx) line 46 | workspace | `query` |
| Query settings sheet | [`query-settings-sheet.tsx`](../../edgequake_webui/src/components/query/query-settings-sheet.tsx) line 112, via [`provider-model-selector.tsx`](../../edgequake_webui/src/components/query/provider-model-selector.tsx) | workspace | `query` |
| Stream session sanitizer | [`use-query-stream-session.ts`](../../edgequake_webui/src/hooks/use-query-stream-session.ts) line 98 | workspace | `query` |
| Conversation lifecycle sanitizer | [`use-query-conversation-lifecycle.ts`](../../edgequake_webui/src/hooks/use-query-conversation-lifecycle.ts) line 77, calls [`sanitizeQueryModelSelection`](../../edgequake_webui/src/lib/query-model-selection.ts) | workspace | `query` |
| Document dropzone | [`document-dropzone.tsx`](../../edgequake_webui/src/components/documents/document-dropzone.tsx) line 353 | workspace | `vlm` when it picks a vision model; otherwise it stops offering a chat catalog |
| Workspace model grid | [`workspace-model-config-grid.tsx`](../../edgequake_webui/src/components/workspace/workspace-model-config-grid.tsx) line 179 | workspace | family `llm_*` is the chat-usage intersection; embedding is `embedding` |
| Workspace extended models | [`workspace-extended-model-config.tsx`](../../edgequake_webui/src/components/workspace/workspace-extended-model-config.tsx) | workspace | the usage of the row it edits |
| Create workspace | [`workspace-create-model-section.tsx`](../../edgequake_webui/src/components/workspace/workspace-create-model-section.tsx) | tenant | same split as the grid. No workspace id yet |
| Onboarding defaults | [`model-defaults-step.tsx`](../../edgequake_webui/src/components/onboarding/steps/model-defaults-step.tsx) line 65 | tenant | tenant `default_*` |
| Deployment fleet card | [`server-llm-config-card.tsx`](../../edgequake_webui/src/components/settings/server-llm-config-card.tsx) line 90 | deployment | one control per family: chat, embedding, vision |
| Vision card | [`vision-llm-settings-card.tsx`](../../edgequake_webui/src/components/settings/vision-llm-settings-card.tsx) | workspace | `vlm` |
| Shared chat picker | [`llm-model-selector.tsx`](../../edgequake_webui/src/components/workspace/llm-model-selector.tsx) line 49 | the scope its parent passes | the usage its parent passes |
| Shared embedding picker | [`embedding-model-selector.tsx`](../../edgequake_webui/src/components/workspace/embedding-model-selector.tsx) line 12, `useEmbeddingModels` | the scope its parent passes | `embedding` |
| Models barrel | [`model-selector.tsx`](../../edgequake_webui/src/components/models/model-selector.tsx) | delegates to the two selectors | same |

The two selectors are the duplication. W2 gives them a `scope` and a
`usage` and points them at `useEligibleModels`. Parents stop fetching
the inventory themselves. [LAW-178-2](01-first-principles.md).

```text
  model-chip  query-settings  dropzone  grids  onboarding  fleet card
       \            |            |        |         |          /
        \           |            |        |         |         /
         v          v            v        v         v        v
              LLMModelSelector / EmbeddingModelSelector
                               |
                               v
                    useLlmModels / useEmbeddingModels
                               |
                               v
                    GET /models/llm   GET /models/embedding
```

After W2 the bottom edge is `GET /models/eligible`.

## Already on the eligible set

| Surface | File | Notes |
|---------|------|-------|
| Workspace role row | [`usage-role-row.tsx`](../../edgequake_webui/src/components/settings/models/usage-role-row.tsx) line 65 | `useEligibleModels(workspaceId, usage)`. The pattern. |
| Workspace narrowing | [`workspace-model-narrowing-card.tsx`](../../edgequake_webui/src/components/settings/models/workspace-model-narrowing-card.tsx) | `useEligibleModelsByUsage`. Admin ticks, not a member picker. |
| Tenant ceiling | `tenant-model-ceiling-card.tsx` via `useTenantModelCatalog` | Catalog inside the global offer. Stays an admin editor. It is not a member choice list. |

The role row is workspace scope only. W2 generalizes the hook so the
create form and the fleet card can call it. The role row's call shape
stays valid: `workspaceId` + `usage` is the workspace-scope form.

## Inventory, kept

| Endpoint | Handler | Who calls it after W2 |
|----------|---------|------------------------|
| `GET /models` | [`list_models`](../../edgequake/crates/edgequake-api/src/handlers/models.rs) line 90 | Operators, catalog clients. No choice control. |
| `GET /models/llm` | `list_llm_models` line 131 | The one inventory hook. |
| `GET /models/embedding` | `list_embedding_models` | The one inventory hook. |
| `GET /models/health` | health handler | Sanitizer may still show a health hint. Membership stays the eligible payload. |
| `POST /models/discover/refresh` | discovery refresh | Still clears the discovery cache. Choice lists update because eligibility re-reads the catalog. |

[`hooks/index.ts`](../../edgequake_webui/src/hooks/index.ts) re-exports
`useLlmModels` from [`use-models.ts`](../../edgequake_webui/src/hooks/use-models.ts).
[`use-providers.ts`](../../edgequake_webui/src/hooks/use-providers.ts)
declares a second function with the same name. W2 keeps the `use-models`
export as the inventory hook and removes the choice callers of the
`use-providers` copy. [F-178-15](04-findings.md).

## Effective, kept

`GET /config/effective?workspace_id=` is the inherit label and the
broken-assignment line. The chip, the role row, and the query settings
sheet read the `query` usage from it when the local pair is empty.
[LAW-178-3](01-first-principles.md) · [F-178-14](04-findings.md).

## Browser store

[`use-settings-store.ts`](../../edgequake_webui/src/stores/use-settings-store.ts)
persists one `querySettings` object. `provider` and `model` are not keyed
by workspace. W2 stores the pair under the workspace id.
[F-178-19](04-findings.md) · [EC-178-05](11-edge-cases.md) ·
[EC-178-27](11-edge-cases.md).

## Server paths that run a choice

| Path | File | Check today | Check after W1 |
|------|------|-------------|----------------|
| Streaming chat | [`streaming.rs`](../../edgequake/crates/edgequake-api/src/handlers/chat/streaming.rs) ~356 | resolve only | request pair, then resolved pair |
| Non-streaming chat | [`completion.rs`](../../edgequake/crates/edgequake-api/src/handlers/chat/completion.rs) ~250 | after resolve, only if the request named a provider | both-or-neither, then resolved pair |
| `POST /query` | [`query_execute.rs`](../../edgequake/crates/edgequake-api/src/handlers/query/query_execute.rs) ~139 | request pair when both fields set | also the resolved pair when the request is empty |
| `POST /query/stream` | [`query_stream.rs`](../../edgequake/crates/edgequake-api/src/handlers/query/query_stream.rs) ~218 | same as `/query` | same as `/query` |
| PDF upload vision | [`pdf_upload/upload.rs`](../../edgequake/crates/edgequake-api/src/handlers/pdf_upload/upload.rs) ~577 | request pair | unchanged contract, still the vlm set |
| Workspace save | [`workspace_crud.rs`](../../edgequake/crates/edgequake-api/src/handlers/workspaces/workspace_crud.rs) | none | family intersection, role, embedding, vision, decision |
| Tenant save | [`tenants.rs`](../../edgequake/crates/edgequake-api/src/handlers/workspaces/tenants.rs) `update_tenant` ~451 | mock heal only | tenant-scope eligible |
| Deployment defaults | [`llm_defaults.rs`](../../edgequake/crates/edgequake-api/src/handlers/llm_defaults.rs) `update_llm_defaults` ~160 | none | deployment-scope eligible |

Image substitution when the override is empty:
[`completion.rs`](../../edgequake/crates/edgequake-api/src/handlers/chat/completion.rs)
lines 289–298. W1 resolves `vlm` instead.
[F-178-16](04-findings.md).

## Tests that freeze the old split

These stay as inventory or editor tests. They are not the coherence gate.
The coherence gate is [13-e2e-test-matrix](13-e2e-test-matrix.md).

| Test | What it locks |
|------|----------------|
| [`e2e/spec175/models-editor.spec.ts`](../../edgequake_webui/e2e/spec175/models-editor.spec.ts) | Role picker is eligible-only. Cited, not copied. |
| [`e2e/spec177/providers.spec.ts`](../../edgequake_webui/e2e/spec177/providers.spec.ts) | Offer editor. |
| [`e2e/spec032-query-model-selection.spec.ts`](../../edgequake_webui/e2e/spec032-query-model-selection.spec.ts) | Chip against `/models/llm`. W3 updates it or replaces the assertion with the spec178 spec. |
| [`src/lib/__tests__/query-model-selection.test.ts`](../../edgequake_webui/src/lib/__tests__/query-model-selection.test.ts) | Sanitizer against the inventory list. W2 changes the fixture to an eligible list. |

```text
  spec032 chip test  --asserts-->  inventory list
  spec175 editor     --asserts-->  eligible list
  spec178            --asserts-->  chip list = editor list = run 422
```
