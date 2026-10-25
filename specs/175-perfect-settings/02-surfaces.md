# 02 — Surfaces (inventory)

Parent: [README](README.md) · Prev: [01-first-principles](01-first-principles.md) ·
Next: [03-product-spec](03-product-spec.md)

Facts about the tree at product pin v0.38.0. Line numbers refer to that
tree. Later waves move cards. They do not invent a second writer for the
same field.

## How you get there

| Entry | File | Route |
|-------|------|-------|
| Sidebar, command palette, `g` then `s` | `edgequake_webui/src/lib/keyboard/destinations.ts` | `/settings` |
| Workspace overview | same destinations module | `/workspace` |
| Members | same | `/members` |
| Tenant switcher | `components/layout/header.tsx`, `components/layout/sidebar.tsx` | persists `edgequake-tenant` |
| Section chips | `components/settings/settings-nav.tsx` `SETTINGS_SECTIONS` | `#settings-{id}` on the same page |
| Deeplink named settings | `app/w/[slug]/settings/page.tsx:32` | `router.push('/workspace')` |

The chips are General, Models & providers, Access, Observability, Data
([`settings-nav.tsx:21`](../../edgequake_webui/src/components/settings/settings-nav.tsx)).

## `/settings` in page order

Route: `edgequake_webui/src/app/(dashboard)/settings/page.tsx`, export
`SettingsPage`. One `ScrollArea`. Nav at the top of that scroll.

### General (`SettingsSection` 180–531)

| Lines | Control | Writes | Scope | Pipeline reads it |
|-------|---------|--------|-------|-------------------|
| 182–250 | Theme, language | `next-themes`; `setLanguage` | this browser | n/a |
| 252–350 | Graph labels, node size, layout | `setGraphSettings` | this browser | graph canvas, also written there |
| 352–459 | Query mode, stream, rerank, rerank top K | `setQuerySettings` | this browser | mode and stream yes; rerank no |
| 461–530 | Gleaning, max passes, LLM summarization | `setIngestionSettings` | this browser | no |

### Models (`SettingsSection` 533–552)

| Component | Writes | Scope | Gate |
|-----------|--------|-------|------|
| `ServerLlmConfigCard` | `PATCH /settings/llm-defaults` | instance `server_config` | save disabled when API says `editable: false` |
| `ProviderStatusHub` | none | `GET /models/health` | none |
| `ProviderStatusCard` | none | `GET /settings/provider/status` | none |
| `ConfigExplainabilityPanel` | none | `GET /config/effective` | none |
| `PdfParserSettingsCard` | `updateWorkspace({ pdf_parser_backend })` | workspace | hidden without a workspace; no role check |
| `VisionLLMSettingsCard` | `updateWorkspace({ vision_llm_* })` | workspace | same |
| `RebuildEmbeddingsButton` | rebuild + reprocess | workspace action | none in the card |

### Access (554–567)

| Component | Writes | Scope | Gate |
|-----------|--------|-------|------|
| `DocumentLabelsCard` | `/workspaces/{id}/clearance` | workspace | `can_manage` or the card returns null |
| `ConnectionsCard` | `POST /connections`, `POST /providers/test` | instance (encrypted keys) | none on the client |
| `ModelRoleMatrixCard` | `updateWorkspace({ llm_roles })` | workspace | none |
| `AppAttributionSettingsCard` | `PATCH /settings/app-attribution` | instance | none |
| `AdminQuotaSection` | `PATCH /admin/config/defaults`, tenant quota | instance + tenant | platform `role === 'admin'` |
| `MembersLinkCard` | none | link to `/members` | none |

### Observability (569–572)

`LangfuseObservabilityCard`. `GET /settings/langfuse`. Read-only. Secrets
stay in env. No admin gate on the client.

### Data (574–683)

Export and import of the zustand JSON, clear query history, reset local
settings. Does not touch server config.

## Local store

`useSettingsStore` in
[`use-settings-store.ts`](../../edgequake_webui/src/stores/use-settings-store.ts),
key `edgequake-settings`, version 1. Types in
[`types/settings.ts`](../../edgequake_webui/src/types/settings.ts).

Persists theme, language, `graphSettings`, `querySettings`,
`ingestionSettings`, `sidebarCollapsed`.

The settings page does not render every field the store holds: graph
`edgeThickness`, `colorBy`, drag, neighbor highlight, hide unselected
edges; query `topK`, `temperature`, `maxTokens`, `provider`, `model`,
`reasoningEffort`, `systemPrompt`, document filter, `fullChunkContent`.
Those live on the query sheet and the graph canvas.

`setTheme` exists on the store. The page and the header use `next-themes`
(`providers/theme-provider.tsx`). The stored `theme` field can drift.
The header language selector calls `i18n.changeLanguage` and does not
call `setLanguage`.

## Other configuration surfaces

| Surface | Mount | What it sets | Persistence |
|---------|-------|--------------|-------------|
| `QuerySettingsSheet` | query page | provider, model, filters, stream, top K, effort, temperature, tokens, system prompt | `setQuerySettings` |
| `ModelChip` | query composer | model + stream | same store |
| `ModeMenu` | query composer | mode | same field as the settings page |
| Header theme | `header.tsx` | light / dark / system | `next-themes` |
| Header language | `header.tsx` | en / zh / fr | `edgequake-language` only |
| `GraphControls` | graph viewer overlay | layout, size, color, labels | `setGraphSettings` |
| `LayoutControl` | graph viewer | layout only | same |
| `GraphSettingsPanel` | graph viewer | focus, max nodes, depth | `useGraphStore` plus local keys. Title is "Query Settings" |
| Upload dropzone | documents | parser, vision modalities, prompts, effort | multipart body |
| `VisionExtractControls` | upload and onboarding | images, charts, figures, prompts | draft, then workspace or this upload |
| Workspace page | `/workspace` | LLM, embedding, vision, PDF, chunking, budget, entity types shown with `isEditing={false}` | read-only except `WorkspaceExtractionModeCard` |
| `ReconfigureWorkspaceWizard` | workspace header | the real editor for those cards | `PUT` workspace |
| `/w/{slug}/workspace` | deeplink | same cards | same |
| `/w/{slug}/settings` | deeplink | nothing | redirects to `/workspace` |
| `/members` | members page | invites, roles, SSO tab | tenant |
| `IdentityProvidersCard` | SSO tab only | read-only IdP table | instance |
| First-run wizard | onboarding | admin, tenant, workspace, models | create tenant and workspace |

Workspace page card order: stats, model grid (LLM, embedding, effort),
extended model config (vision, PDF), extraction language, chunking,
extract budget, extraction mode (live save), entity types, relation types
(read-only), rebuild, delete workspace.

## Duplicate writers (the DRY list)

1. Query mode and stream: settings page, query sheet, `ModeMenu`, `ModelChip`.
2. Reasoning effort: server card, query sheet, workspace grid, upload vision, shared widget `reasoning-effort-select.tsx`.
3. LLM / embedding / vision: server card, workspace grid, reconfigure wizard, vision card, query picker, role matrix free text.
4. PDF parser: settings card, workspace extended card, upload dropzone, onboarding. Shared field `pdf-parser-backend-field.tsx`.
5. Vision policy: upload popover and wizard. The settings vision card picks the model only.
6. Graph look: settings card, `GraphControls`, `LayoutControl`. A fourth popover uses another store.
7. Provider health: `ProviderStatusHub` (`GET /models/health`) and `ProviderStatusCard` (`GET /settings/provider/status`). Both titled "Provider Status".
8. Theme and language: settings page and header, two stores.
9. People: `MembersLinkCard` plus the whole `/members` page. Document labels are a second access model ([SPEC-174](../174-document-security/)).
10. "Workspace settings": `/w/{slug}/settings` lands on `/workspace`, which is a third model surface, not `/settings`.

## Where each usage is chosen today

| Usage | UI that pretends to set it | Function that actually runs |
|-------|----------------------------|-----------------------------|
| extract | role matrix, workspace LLM grid | `resolve_extract_role_llm` |
| query | role matrix, query chip, server card | `resolve_llm_provider_for_workspace` |
| summary | role matrix | `resolve_summary_llm_or_fallback` |
| vlm | vision card, upload form, role matrix | `resolve_workspace_vlm_config`, then manuscript env |
| keyword | role matrix | `resolve_keyword_llm` |
| embedding | workspace grid, server card | `resolve_embedding_choice` |
| decision | `WorkspaceExtractionModeCard` | `DecisionSettings::from_env` + workspace overrides |
| rerank | settings page toggle | env factory, once per process |
| reasoning | several selects | `resolve_role_reasoning_effort` |

Target owner for each row is in [09-ux-ui-spec](09-ux-ui-spec.md). The
coverage table there maps every surface on this page to a wire id or to
"removed". The single function is in [05-resolution-law](05-resolution-law.md).
