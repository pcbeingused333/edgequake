# 10 — Frontend architecture

Parent: [README](README.md) · Prev: [09-ux-ui-spec](09-ux-ui-spec.md) ·
Next: [11-edge-cases](11-edge-cases.md)

[LAW-175-8](01-first-principles.md) · [LAW-175-16](01-first-principles.md).
React renders server values. It does not reimplement `resolve_usage`.

## Routes

| Path | Page |
|------|------|
| `/settings` | redirect to `/settings/you` |
| `/settings/you` | Appearance, graph display, browser data. Wires [W-03](09-ux-ui-spec.md#w-03-you-appearance), [W-04](09-ux-ui-spec.md#w-04-you-graph-display), [W-05](09-ux-ui-spec.md#w-05-you-browser-data) |
| `/settings/workspace` | current workspace from the header store |
| `/settings/organization` | current tenant, manage only |
| `/settings/deployment` | instance admin only |
| `/w/[slug]/settings` | resolve the slug, select that workspace, render workspace scope |
| `/w/[slug]/settings/ingestion` | same shell, ingestion section |
| `/w/[slug]/settings/access` | same shell, labels section |

`/workspace` remains the overview. Its model cards become links to
`/settings/workspace`. `ReconfigureWorkspaceWizard` calls the same
section components. It does not keep a private form.

## Modules

```text
  edgequake_webui/src/app/(dashboard)/settings/
      layout.tsx                 shell, search, scope gate
      you/page.tsx
      workspace/page.tsx
      organization/page.tsx
      deployment/page.tsx

  src/components/settings/
      settings-shell.tsx         nav + outlet
      settings-search.tsx        filters SETTINGS_INDEX
      usage-row.tsx              one row, all chat and embed usages
      model-picker.tsx           pure list from eligible_models
      catalog-editor.tsx
      connection-form.tsx        moved from connections-card
      provider-health.tsx        one health view

  src/lib/settings/
      index.ts                   section id, label, scope, keywords
      eligible.ts                types for ResolvedUsage, no ladder math
```

Deleted or reduced to re-exports during W4:

- free-text fields in `model-role-matrix-card.tsx`
- `provider-status-card.tsx` once `provider-health.tsx` covers both GETs
- rerank and gleaning blocks in `settings/page.tsx`

Kept and moved, not rewritten:

- `document-labels-card.tsx`
- `pdf-parser-backend-field.tsx`
- `reasoning-effort-select.tsx`
- `vision-extract-controls.tsx` (upload and ingestion section share it)
- `langfuse-observability-card.tsx`

## One picker

`model-picker.tsx` props:

```text
  usage: Usage
  models: EligibleModel[]      from GET /models/eligible
  value: { provider, model } | "inherit"
  locked: boolean
  onChange
```

It does not fetch. The parent uses one React Query key
`['models','eligible', workspaceId, usage]`. The query chip, the usage
row, and the upload vision select all render this component. Provider
icons stay in `components/providers/provider-icon.tsx`.

`eligible.ts` is types and a `groupByProvider` helper. If a function
starts to encode "OpenAI cannot do X," that function is in the wrong
package. The server sent the list.

## Store split

| Fact | Store | Writer |
|------|-------|--------|
| Theme | `next-themes` only | header and You call `setTheme` from `next-themes` |
| Language | `useSettingsStore.setLanguage` | header and You. Header stops calling `i18n` directly |
| Graph look | `useSettingsStore.graphSettings` | You and `GraphControls` |
| Query mode, stream, top K, temperature, prompt, filters | `useSettingsStore.querySettings` | query sheet and composer only |
| Query provider, model | same, default `undefined` | `model-picker` in the composer. Cleared on workspace change |
| Rerank, gleaning | removed from the persisted type | migration of the zustand version drops the keys |
| Assignments, policy | React Query, server | usage row, catalog editor |

`useSettingsStore.setTheme` is removed so it cannot drift from
`next-themes`. [F-175-12](04-findings.md).

Workspace change clears a sticky query provider if that pair is not in
the new workspace's eligible list. The chip falls back to inherit.
[EC-175-28](11-edge-cases.md).

## Search index

`SETTINGS_INDEX` is a constant: `{ id, scope, label, keywords }`. Search
is client-side. Keyboard `g` then `s` still opens settings. Command
palette entries deep-link to `/settings/workspace#extract` and the other
section ids. The palette does not duplicate the forms.

## Gates

The layout reads the same role flags the API uses:

- workspace editors: owner or admin on a membership that applies to this workspace
- organization: tenant-wide owner or admin (`get_user_role`), or instance admin
- deployment: instance `admin`

`SETTINGS_INDEX` drops entries whose scope the flags deny, before search
matches. Hidden nav is not the security check. Mutations still go
through the hooks that surface 403.

A Workspace Admin who opens `/settings/organization` or
`/settings/deployment` is redirected to You
([EC-175-43](11-edge-cases.md), [EC-175-34](11-edge-cases.md)).
The settings switcher lists administered workspaces. The app header
lists every accessible workspace. Selecting one they do not administer
and opening settings shows the member models page
([EC-175-39](11-edge-cases.md)).

## Data the pages must not invent

- Model ids typed by hand, except the deployment custom-model field that
  posts to `/providers/test` first.
- A local copy of the ladder ("if no workspace model, show tenant").
  The inherit subtitle comes from `GET /config/effective`.
- Env snippets that name `gpt-4o-mini`. Examples come from the eligible
  list. [F-175-22](04-findings.md).

## Tests on the client

| Test | Asserts |
|------|---------|
| `usage-row` unit | embedding usage does not render a chat model passed by mistake; locked disables the select |
| `model-picker` unit | inherit calls onChange with inherit, not an empty string |
| settings search unit | "embedding" returns the workspace Models entry |
| Playwright | [13-e2e-test-matrix](13-e2e-test-matrix.md) W6 rows |
