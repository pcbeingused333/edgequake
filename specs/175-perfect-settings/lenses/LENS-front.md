# Lens — Front

Parent: [README](../README.md) · Architecture: [10-frontend-architecture](../10-frontend-architecture.md)

## What this lens owns

[LAW-175-16](../01-first-principles.md). One `model-picker`. One shell.
The client does not encode provider quirks.

## Components

| Component | Renders | Does not |
|-----------|---------|----------|
| `settings-shell.tsx` | nav from `SETTINGS_INDEX` after the role filter | fetch models, or show Organization to a Workspace Admin ([EC-175-43](../11-edge-cases.md)) |
| `usage-row.tsx` | one usage, radios, effort if present | know OpenAI vs Ollama |
| `model-picker.tsx` | `EligibleModel[]` | call `resolve_usage` |
| `catalog-editor.tsx` | ceiling or narrowing | store keys |
| `connection-form.tsx` | shape select, write-only key | list models |

React Query keys: `['models','eligible', workspaceId, usage]` and
`['config','effective', workspaceId]`. Workspace switch invalidates both.
A sticky query provider that is absent from the new list clears to
inherit ([EC-175-28](../11-edge-cases.md)).

## Persistence

Theme: `next-themes` only. Delete `useSettingsStore.setTheme`.
Language: `setLanguage` only. The header stops calling
`i18n.changeLanguage` by itself.
Zustand persist version bumps and drops `enableRerank`, `rerankTopK`,
`enableGleaning`, `maxGleaning`, `useLLMSummarization`.

## Routes

`/settings` redirects to `/settings/you`. `/w/[slug]/settings` renders
the shell for that slug ([EC-175-24](../11-edge-cases.md)). It does not
`router.push('/workspace')`.
