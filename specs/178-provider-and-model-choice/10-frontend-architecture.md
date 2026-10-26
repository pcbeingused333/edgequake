# 10 — Frontend architecture

Parent: [README](README.md) · Prev: [09-ux-ui-spec](09-ux-ui-spec.md) ·
Next: [11-edge-cases](11-edge-cases.md)

One hook loads the set. One picker renders it. Parents pass scope and
usage. The ladder stays on the server. [DRY](01-first-principles.md) ·
[LAW-178-8](01-first-principles.md).

## Hook

`useEligibleModels` in
[`use-model-policy.ts`](../../edgequake_webui/src/hooks/use-model-policy.ts)
grows a scope argument. The workspace-id overload used by
[`usage-role-row.tsx`](../../edgequake_webui/src/components/settings/models/usage-role-row.tsx)
remains a wrapper so that file does not learn a new call shape in W1.

```text
  useEligibleModels({
    scope: "workspace" | "tenant" | "deployment",
    workspaceId?, tenantId?,
    usage,
  })
```

Query key: `["models", "eligible", scope, id, usage]`.

`enabled` is false until the id the scope needs is present. Deployment
scope needs no id. Stale time stays 15 seconds. `retry: false` stays.
The hook does not merge in `useLlmModels` results when the request fails.
The menu shows the load error sentence from [09-ux-ui-spec](09-ux-ui-spec.md).

`useChatFamilyModels(scope)` runs the four chat usages and returns the
intersection by `provider + model`. It is a view over four hook calls.
The save path does not trust the view. [08-api-contract](08-api-contract.md).

## Picker

[`llm-model-selector.tsx`](../../edgequake_webui/src/components/workspace/llm-model-selector.tsx)
and [`embedding-model-selector.tsx`](../../edgequake_webui/src/components/workspace/embedding-model-selector.tsx)
become thin wrappers over one `ModelChoiceMenu`.

```text
  ModelChoiceMenu
    scope, usage, value, onChange, allowInherit, disabled
    |
    +-- useEligibleModels
    +-- useEffectiveUsages   only when allowInherit and scope is workspace
    |
    v
  onChange(undefined)          inherit
  onChange({ provider, model, embeddingDimension? })
```

`LLMModelSelector` keeps its export so parents move one prop at a time.
New code calls `ModelChoiceMenu`. By the end of W2 the wrapper's only
job is to pass `usage` and `scope`. It does not import `useLlmModels`.

Embedding sets `usage="embedding"` and writes `embeddingDimension` from
the card into the selection. The parent puts that integer on the
workspace body. The parent does not call `resolveEmbeddingDimension`
to guess. That helper may remain for display of legacy rows. It is not
on the save path. [EC-178-14](11-edge-cases.md).

The query chip keeps its trigger and its stream switch. It stops calling
`buildModelMenu` on the inventory list. It groups the eligible payload
by `provider_display_name`. `modelSupportsStreaming` reads
`supports_streaming` on that payload. [EC-178-33](11-edge-cases.md).

## Call sites

W2 edits the Choice rows in [02-surfaces](02-surfaces.md). Each parent
passes the scope from that table. Parents that today call `useLlmModels`
only to feed a selector delete that call.

Inventory callers, if any remain, import `useLlmModels` from
[`use-models.ts`](../../edgequake_webui/src/hooks/use-models.ts) only.
The second declaration in
[`use-providers.ts`](../../edgequake_webui/src/hooks/use-providers.ts)
is removed after its callers move. One function, one query key.
[F-178-15](04-findings.md).

## Store

[`use-settings-store.ts`](../../edgequake_webui/src/stores/use-settings-store.ts)
gains `queryModelsByWorkspace: Record<string, { provider?: string; model?: string }>`.
`setQuerySettings` stops writing a global `provider` and `model` for the
chip. Mode and stream may stay global. They are not models.

Migration on rehydrate: if the old `querySettings.provider` and `model`
are set, copy them onto the current workspace id once, then delete the
global fields. The next sanitizer pass drops them if that workspace
rejects them. Workspaces the user has not opened get no pair, which is
Inherit. [EC-178-27](11-edge-cases.md).

## Sanitizer

[`sanitizeQueryModelSelection`](../../edgequake_webui/src/lib/query-model-selection.ts)
takes the eligible models for the workspace on screen. It drops the pair
when the id is absent. It does not consult provider health to keep a
pair the eligible list omitted. Health remains an operator signal on
the deployment screen.

While the eligible query is still loading, the sanitizer does not clear
the slot. Clearing on an empty list during load is the current footgun
in the other direction (lines 53–55 keep the pair when the list is
missing). W2 distinguishes "not loaded" from "loaded and empty."
Loaded and empty clears a non-empty slot. Not loaded leaves the slot
and does not send the request until the list has loaded, when a pair
is set. Inherit sends immediately. [EC-178-04](11-edge-cases.md).

## Invalidation

Mutations that already invalidate eligible keys
(workspace policy, tenant policy, provider offer, deployment withdraw)
also invalidate `["models", "eligible"]`. The chip uses that prefix, so
one invalidation refreshes every open scope. They do not need to
invalidate `["llm-models"]` for correctness of choice. Discovery refresh
still invalidates the inventory key and the eligible prefix, because a
new card can enter the catalog.

[`refreshDynamicModels`](../../edgequake_webui/src/hooks/use-providers.ts)
gains the eligible prefix. [EC-178-17](11-edge-cases.md) ·
[EC-178-28](11-edge-cases.md).

## What the client does not do

- Recompute `resolve_usage`.
- Hide `mock` a second time if the server already omitted it. A test
  fixture that injects `mock` is still filtered once, at the menu, so a
  stale server cannot show it. That filter is the existing
  `isUiVisibleProviderId` check, applied to the eligible payload, not a
  second catalog. [EC-178-22](11-edge-cases.md).
- Call `GET /models/llm` from `ModelChoiceMenu`, the chip, the settings
  sheet, the dropzone, the grids, onboarding, or the fleet card.

```text
  ModelChoiceMenu
        |
        v
  useEligibleModels -------> GET /models/eligible
        |
        v
  onChange(pair | inherit)
        |
        v
  PUT workspace | tenant | llm-defaults | chat body
        |
        v
  server PairVerdict
```
