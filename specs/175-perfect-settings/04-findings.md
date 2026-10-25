# 04 — Findings (F-175)

Parent: [README](README.md) · Prev: [03-product-spec](03-product-spec.md) ·
Next: [05-resolution-law](05-resolution-law.md)

Each finding is a fact about the tree at migration file 220. The law that
closes it is in [01-first-principles](01-first-principles.md). This pack
does not fix them. W1–W6 do that.

## F-175-01 — One scroll, four owners

`SettingsPage` in
[`settings/page.tsx`](<../../edgequake_webui/src/app/(dashboard)/settings/page.tsx>)
renders General, Models, Access, Observability, and Data inside one
`ScrollArea`. `SETTINGS_SECTIONS` in
[`settings-nav.tsx:21`](../../edgequake_webui/src/components/settings/settings-nav.tsx)
are hash links. Theme (browser), fleet defaults (instance), workspace
vision, connections, and quotas share that scroll.

- **Law:** [LAW-175-8](01-first-principles.md)
- **EC:** [EC-175-34](11-edge-cases.md)

## F-175-02 — Five ladders

`resolve_llm_choice` in
[`model_resolution.rs:227`](../../edgequake/crates/edgequake-core/src/model_resolution.rs)
implements Request > Workspace > Tenant > Env > Default for the LLM pair
only, and only when the caller uses it. Extract uses
`resolve_extract_role_llm`. Query uses
`WorkspaceProviderResolver::resolve_llm_provider_for_workspace`. VLM
runtime uses `resolve_workspace_vlm_config`. Keyword uses
`resolve_keyword_llm`. Decision uses `DecisionSettings::from_env`.
Embedding uses `resolve_embedding_choice`, then a startup embedder when
that returns none.

- **Law:** [LAW-175-1](01-first-principles.md) · [LAW-175-2](01-first-principles.md)
- **EC:** [EC-175-01](11-edge-cases.md)

## F-175-03 — Extract env beats the workspace

`resolve_extract_role_llm`
([`llm_roles.rs:353`](../../edgequake/crates/edgequake-core/src/llm_roles.rs))
returns `env_extract_role_llm()` or else the workspace role. The comment
at line 342 states precedence **env → workspace `llm_roles.extract` →
workspace LLM**. Tenant is not in the chain. `connection_id` on the env
pin is `None` (`env_role_llm`, line 369).

- **Law:** [LAW-175-10](01-first-principles.md)
- **EC:** [EC-175-02](11-edge-cases.md) · [EC-175-20](11-edge-cases.md)

## F-175-04 — Query connection ignores the request

`resolve_llm_provider_for_workspace`
([`resolver.rs:439`](../../edgequake/crates/edgequake-api/src/providers/resolver.rs))
returns `try_llm_from_workspace_connection` for `LlmRole::Query` before
`resolve_llm_provider_with_workspace`. A request pair never runs when
`llm_roles.query.connection_id` parses.

- **Law:** [LAW-175-4](01-first-principles.md)
- **EC:** [EC-175-03](11-edge-cases.md)

## F-175-05 — VLM role ignores the upload

`resolve_workspace_vlm_config`
([`vlm_provider_resolver.rs:34`](../../edgequake/crates/edgequake-api/src/services/vlm_provider_resolver.rs))
returns `resolve_role_llm(Vlm)` when metadata `llm_roles.vlm` exists.
`resolve_vision_llm_choice` (upload > workspace vision > tenant > …) is
skipped. Manuscript env can replace the result later in
`pdf_processing.rs`.

- **Law:** [LAW-175-4](01-first-principles.md) · [LAW-175-10](01-first-principles.md)
- **EC:** [EC-175-21](11-edge-cases.md) · [EC-175-22](11-edge-cases.md)

## F-175-06 — Missing key falls through to the process provider

`try_create_llm_provider` documents a `None` fallthrough to the server
default ([`resolver.rs:498`](../../edgequake/crates/edgequake-api/src/providers/resolver.rs)).
`execute_sota_query_with_auth_fallback`
([`query_execution.rs:199`](../../edgequake/crates/edgequake-api/src/services/query_execution.rs))
retries with `llm_override: None` on auth failure. The process provider
is `ProviderFactory::from_env` ([`postgres.rs:134`](../../edgequake/crates/edgequake-api/src/state/postgres.rs)).

- **Law:** [LAW-175-5](01-first-principles.md)
- **EC:** [EC-175-04](11-edge-cases.md) · [EC-175-32](11-edge-cases.md)

## F-175-07 — Role matrix is free text

[`model-role-matrix-card.tsx:143`](../../edgequake_webui/src/components/settings/model-role-matrix-card.tsx)
binds provider and model to `<Input>`. The connection `<select>` is the
only typed control. Placeholder copy says "inherit / env" and "model id".
No capability filter.

- **Law:** [LAW-175-3](01-first-principles.md)
- **EC:** [EC-175-06](11-edge-cases.md)

## F-175-08 — Rerank and gleaning never leave the browser

`enableRerank` is written at
[`settings/page.tsx:430`](<../../edgequake_webui/src/app/(dashboard)/settings/page.tsx>)
and stored at [`use-settings-store.ts:48`](../../edgequake_webui/src/stores/use-settings-store.ts).
A repository search of `edgequake_webui/src` finds no other reader.
`enableGleaning`, `maxGleaning`, and `useLLMSummarization` are the same
pattern at `settings/page.tsx:476–526` and `use-settings-store.ts:57–59`.
The API field `enable_rerank` exists ([`query_types.rs:166`](../../edgequake/crates/edgequake-api/src/handlers/query_types.rs))
and the WebUI does not send the store value. `rerank_model` is forced to
`None` in [`query_context.rs:305`](../../edgequake/crates/edgequake-api/src/services/query_context.rs).

- **Law:** [LAW-175-7](01-first-principles.md)
- **EC:** [EC-175-09](11-edge-cases.md) · [EC-175-23](11-edge-cases.md)

## F-175-09 — Two provider status cards

`SettingsProviderHub` / `ProviderStatusHub` calls `GET /models/health`.
`ProviderStatusCard` calls `GET /settings/provider/status`. Both mount
under Models (`settings/page.tsx` around 537–539). Both are titled
"Provider Status".

- **Law:** [LAW-175-8](01-first-principles.md)
- **EC:** [EC-175-29](11-edge-cases.md)

## F-175-10 — `/w/{slug}/settings` is not settings

[`app/w/[slug]/settings/page.tsx:32`](../../edgequake_webui/src/app/w/[slug]/settings/page.tsx)
calls `router.push('/workspace')`. The comment cites SPEC-032 and says the
page only redirects. The fallback label still says "Workspace Settings".

- **Law:** [LAW-175-8](01-first-principles.md)
- **EC:** [EC-175-24](11-edge-cases.md)

## F-175-11 — Workspace overview looks editable

`workspace/page.tsx` passes `isEditing={false}` into the model, PDF,
chunking, and budget cards. The live exception is
`WorkspaceExtractionModeCard`. Edits otherwise go through
`ReconfigureWorkspaceWizard`.

- **Law:** [LAW-175-16](01-first-principles.md)
- **EC:** [EC-175-24](11-edge-cases.md)

## F-175-12 — Theme and language drift

The settings theme select and the header menu both call `next-themes`.
`useSettingsStore.setTheme` is a third writer the page does not use.
Language: the page calls `setLanguage` (zustand + i18next). The header
`LanguageSelector` calls `i18n.changeLanguage` only.

- **Law:** [LAW-175-7](01-first-principles.md)
- **EC:** [EC-175-17](11-edge-cases.md)

## F-175-13 — `llm_roles` is untyped in OpenAPI

Workspace create and update types in
`edgequake_webui/openapi/schema.d.ts` declare `llm_roles?: unknown`.
The Rust shape is `RoleLlmConfig` in
[`llm_roles.rs:70`](../../edgequake/crates/edgequake-core/src/llm_roles.rs):
`provider`, `model`, `reasoning_effort`, `connection_id`.

- **Law:** [LAW-175-3](01-first-principles.md)
- **EC:** [EC-175-06](11-edge-cases.md)

## F-175-14 — No allowlist store

Tenant and workspace model fields live in `metadata` JSONB
(`001_init_database.sql`). `provider_connections`
([`169_spec163_provider_connections.sql`](../../edgequake/migrations/169_spec163_provider_connections.sql))
has no model column. Nothing records "this tenant may use this model for
this usage."

- **Law:** [LAW-175-6](01-first-principles.md)
- **EC:** [EC-175-19](11-edge-cases.md)

## F-175-15 — Decision and rerank are not roles

`LlmRole::all` is extract, query, summary, vlm, keyword
([`llm_roles.rs:57`](../../edgequake/crates/edgequake-core/src/llm_roles.rs)).
Decision model is `DecisionSettings` plus workspace metadata
`decision_model`. Rerank is `edgequake-query` bootstrap from
`EDGEQUAKE_RERANKER*`. The role matrix still offers an `embedding` key
that the enum does not include; the server reads it only as a connection
id (`metadata_connection_id`).

- **Law:** [LAW-175-3](01-first-principles.md)
- **EC:** [EC-175-08](11-edge-cases.md)

## F-175-16 — Query override sticks in the browser

Default `provider` and `model` are `undefined`
([`use-settings-store.ts:50`](../../edgequake_webui/src/stores/use-settings-store.ts)),
so a fresh client inherits. After the user picks a model, the store sends
that pair on later chats. There is no server check that the pair is still
eligible.

- **Law:** [LAW-175-4](01-first-principles.md)
- **EC:** [EC-175-28](11-edge-cases.md)

## F-175-17 — Compiled defaults disagree

`resolve_llm_choice` fills from env then compiled defaults. Separate
tables name different OpenAI defaults: `models.toml` vision, the vision
helper, `Workspace::default_model_for_provider`, and the extract env
fallback `"gpt-5.4-nano"` inside `env_role_llm` (line 383). The UI can
show one of these while another path runs.

- **Law:** [LAW-175-2](01-first-principles.md) · [LAW-175-12](01-first-principles.md)
- **EC:** [EC-175-10](11-edge-cases.md)

## F-175-18 — Client gates are uneven

Present: admin quotas, SSO empty state, document labels `can_manage`,
server-LLM save when `editable` is false. Missing on the client:
connections, role matrix, attribution, PDF parser, vision model, rebuild.
Server checks for workspace writes exist when binding is on
(`can_manage_workspaces`). `/settings/*` is treated as a global resource
in `bind_request`, so the page must not assume workspace membership ran.

- **Law:** [LAW-175-13](01-first-principles.md)
- **EC:** [EC-175-31](11-edge-cases.md) · [EC-175-34](11-edge-cases.md)

## F-175-19 — `rerank_model` is accepted and ignored

[`query_types.rs:170`](../../edgequake/crates/edgequake-api/src/handlers/query_types.rs)
deserializes `rerank_model`. Call sites pass `None` into the engine
context. A client can believe it selected Cohere.

- **Law:** [LAW-175-7](01-first-principles.md)
- **EC:** [EC-175-09](11-edge-cases.md)

## F-175-20 — Embedding connection does not choose the model

When `llm_roles.embedding.connection_id` is set, ingest and query still
take the model name from `resolve_embedding_choice`. The connection
supplies URL and key. Decision-mode ingest does not look up that
connection (`workspace_pipeline_decision.rs`).

- **Law:** [LAW-175-11](01-first-principles.md)
- **EC:** [EC-175-12](11-edge-cases.md)

## F-175-21 — Workspace page and settings both edit PDF and vision

`PdfParserSettingsCard` and `VisionLLMSettingsCard` on `/settings` call
`updateWorkspace`. The same fields render on the workspace page and in
the reconfigure wizard. Three entry points, one metadata key, two of them
look disabled.

- **Law:** [LAW-175-16](01-first-principles.md)
- **EC:** [EC-175-24](11-edge-cases.md)

## F-175-22 — Stale model examples

`query_types.rs` documents `"gpt-4o-mini"` as the example model.
`ProviderStatusCard` copy-paste snippets still mention `gpt-4o-mini`.
Product guidance says prefer `gpt-5-nano`. The catalog, not the snippet,
must be the example.

- **Law:** [LAW-175-3](01-first-principles.md)
- **EC:** [EC-175-11](11-edge-cases.md)

## F-175-23 — Graph "Query Settings" is not query

`GraphSettingsPanel` is titled "Query Settings" and edits focus, depth,
and max nodes in `useGraphStore`. It does not edit `llm_roles.query`.

- **Law:** [LAW-175-16](01-first-principles.md)
- **EC:** [EC-175-35](11-edge-cases.md)

## F-175-24 — Identity providers are not on the settings page

`IdentityProvidersCard` mounts from the members SSO tab. The card is
read-only. Mutations are `PUT /admin/identity-providers/{slug}`. A
platform admin looking at `/settings` cannot find SSO.

- **Law:** [LAW-175-8](01-first-principles.md)
- **EC:** [EC-175-34](11-edge-cases.md)

## F-175-25 — Tenant role lookup drops workspace admins

`get_user_role` in
[`in_memory.rs`](../../edgequake/crates/edgequake-core/src/workspace_service/in_memory.rs)
(around line 1014) keeps memberships whose `workspace_id` is null, then
takes the highest `MembershipRole`. `require_tenant_access` with
`TenantAccess::Manage` calls that function and requires
`can_manage_users`
([`tenant_access.rs:74`](../../edgequake/crates/edgequake-api/src/handlers/workspaces/tenant_access.rs)).
A Workspace Admin, whose row has a workspace id, does not pass. That is
the correct tenant gate. The settings pack must not treat every admin as
a Super Admin.

- **Law:** [LAW-175-13](01-first-principles.md) · [LAW-175-17](01-first-principles.md)
- **EC:** [EC-175-36](11-edge-cases.md) · [EC-175-37](11-edge-cases.md)

## F-175-26 — `/settings/*` skips the membership check

`global_resource` in
[`request_authorization.rs`](../../edgequake/crates/edgequake-api/src/services/request_authorization.rs)
lists `/api/v1/settings/` and `/api/v1/tenants`. `bind_request` returns
before `membership_role` for both prefixes. A card on the settings page
is not a permission boundary. The ceiling handler calls
`require_tenant_access` itself. Assignment writes stay on
`/workspaces/{id}`, which is not a global prefix.

`PUT /workspaces/{id}` already uses the workspace check
(`can_manage_workspaces` on a membership that applies to that workspace),
not `require_tenant_access`.

- **Law:** [LAW-175-17](01-first-principles.md)
- **EC:** [EC-175-36](11-edge-cases.md) · [EC-175-39](11-edge-cases.md) · [EC-175-51](11-edge-cases.md)
