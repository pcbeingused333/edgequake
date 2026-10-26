# Lens — Front

Parent: [README](../README.md) · Architecture: [10-frontend-architecture](../10-frontend-architecture.md) ·
Surfaces: [02-surfaces](../02-surfaces.md)

## What this lens owns

One hook, one menu, one query-key prefix. Choice files do not import
the inventory hooks. The persisted pair is keyed by workspace id.
[LAW-178-7](../01-first-principles.md) · [LAW-178-8](../01-first-principles.md).

## Decisions this lens will not reopen

- `useEligibleModels({ scope, workspaceId, tenantId, usage })` is the
  loader. The workspace-id overload stays so
  [`usage-role-row.tsx`](../../../edgequake_webui/src/components/settings/models/usage-role-row.tsx)
  does not change call shape for no reason.
- Query key `["models", "eligible", scope, id, usage]`. Invalidation
  uses the `["models", "eligible"]` prefix. Offer, policy, withdraw,
  and discovery refresh all hit that prefix.
- `useChatFamilyModels` is four hook calls and an intersection. It is
  not a fifth endpoint.
- `ModelChoiceMenu` is the component. `LLMModelSelector` and
  `EmbeddingModelSelector` become wrappers with a scope and a usage.
  Their public exports survive W2 so parents move one prop at a time.
- The chip keeps its trigger. It drops `buildModelMenu` on the
  inventory array. Groups come from `provider_display_name`.
- `sanitizeQueryModelSelection` learns "not loaded" versus "loaded and
  empty." It stops taking provider health as a reason to keep a pair.
- `useLlmModels` in [`use-providers.ts`](../../../edgequake_webui/src/hooks/use-providers.ts)
  is removed when its callers are gone. The `use-models.ts` export
  remains the inventory hook. Two functions with one name is
  [F-178-15](../04-findings.md).
- Rehydrate copies a legacy global pair onto the current workspace
  once, then the sanitizer may drop it.

## What this lens will not let the others break

- A choice component that unions the eligible list with `/models/llm`
  so the menu "feels" as full as before. [EC-178-01](../11-edge-cases.md)
  is the test that fails if that happens: the inventory-only id must
  not render.
- Caching the chip on `["llm-models"]` "as well." One key.
  [EC-178-17](../11-edge-cases.md).
- Sending a one-sided `{ provider }` from the menu. The menu emits both
  or `undefined`. [EC-178-12](../11-edge-cases.md).
- Guessing embedding dimension in `resolveEmbeddingDimension` on the
  save path. The card field is the integer. The helper may label a
  legacy row. [EC-178-14](../11-edge-cases.md).
- A source test that is skipped. [EC-178-40](../11-edge-cases.md) is the
  ratchet.

## Out of this lens

The 422 body and the offer SQL. The menu renders the payload and the
error sentence. It does not decide membership.
