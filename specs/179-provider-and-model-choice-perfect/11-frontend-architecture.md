# 11 — Frontend architecture

Parent: [README](README.md) · Prev: [10-screens](10-screens.md) ·
Next: [12-edge-cases](12-edge-cases.md)

## WHY

Two hooks and a search panel each learned a slightly different definition of
"the models". The fix is to delete the extra definitions, not to add a third
filter in the component.

## Modules

```text
  lib/api/edgequake/model-catalog.ts     eligible fetch, types
  hooks/use-model-policy.ts              useScopedEligibleModels
  components/models/model-choice-menu.tsx
  hooks/use-dirty-guard.ts
  lib/api/edgequake/provider-lifecycle.ts  refresh, impact, delete, rotate
  components/settings/providers/provider-offer-editor.tsx
  components/settings/connections-card.tsx
```

| Module | Owns | Does not own |
|--------|------|----------------|
| `useScopedEligibleModels` | query key `["models","eligible",scope,id,usage]`, one fetch | inventory, health |
| `ModelChoiceMenu` | listbox, search, inherit, chips | saving a role |
| `useDirtyGuard` | beforeunload, router block | the form fields |
| `provider-lifecycle.ts` | refresh, impact, confirm delete | offer algebra |
| role editor | `llm_roles` patch | vision's private copy |

## Deletions

- `useLlmModels` and `useEmbeddingModels` in `hooks/use-providers.ts` once no caller remains. The copies in `hooks/use-models.ts` stay only if `useModelsConfig` still needs capability lookup. After eligible carries `context_length`, `supports_tools`, and costs, `use-llm-capability-models.ts` reads the eligible payload for the active workspace and the inventory hook goes too.
- Remote search inside `ModelPickerPanel` for every choice caller. The refresh review uses `GET /deployment/providers/{slug}` cache rows, not `/models/search`.
- The vision settings card stops writing provider and model. It links to the `vlm` row. Parser controls stay.

## Wizard scopes

| Wizard | Scope argument |
|--------|----------------|
| First-run, create tenant | `deployment` |
| Create workspace | `tenant` (the parent offer and ceiling) |
| Reconfigure | `workspace` |

`model-defaults-step.tsx` takes the scope as a prop. It does not read
`selectedTenantId` and hope a tenant exists.

## Invalidation

Refresh, offer PUT, policy PUT, connection test, and connection delete
invalidate `["models","eligible"]`. They do not invalidate a second
inventory key, because choice does not read one.

## Server defaults card

`use-server-model-defaults.ts` stops calling `GET /models`. The label
"Using server defaults" reads deployment eligible for the default pair, or
`GET /config/effective` when a workspace id exists.

## i18n

New keys under `settings.providers` and `settings.models` in the existing
locale files. Components in the table above call `useTranslation`. A unit
test fails if those files contain the literal `"Unsaved changes"`.
