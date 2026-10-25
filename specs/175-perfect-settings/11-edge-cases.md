# 11 — Edge cases (EC-175)

Parent: [README](README.md) · Prev: [10-frontend-architecture](10-frontend-architecture.md) ·
Next: [12-implementation-plan](12-implementation-plan.md)

Each case has one primary gate in [13-e2e-test-matrix](13-e2e-test-matrix.md)
([LAW-175-15](01-first-principles.md)) and a **Mitigation** line. The
mitigation is the gate that makes the bad outcome unreachable. "Unit"
means `resolve_usage` with no I/O. "API" means a Postgres-backed HTTP
test. "Playwright" means the WebUI.

## Register

### EC-175-01 — Lock beats a workspace role

**Given** `EDGEQUAKE_EXTRACT_LLM_MODEL=gpt-5-nano` and
`llm_roles.extract.model=gemma3`. **When** ingest runs.
**Then** the model is `gpt-5-nano`, `source` is `operator_lock`, and the
usage row is disabled.
**Mitigation:** step 1 of the ladder. [LAW-175-10](01-first-principles.md).

### EC-175-02 — Acc pin with empty workspace metadata

**Given** the extract pin set and no `llm_model` metadata key.
**Then** extract uses the pin. The effective payload says `operator_lock`.
Unsetting the pin uses the next layer, not a cached pin.
**Mitigation:** lock is read from the process env on each resolve.
[LAW-175-9](01-first-principles.md).

### EC-175-03 — Request and a query connection

**Given** `llm_roles.query.connection_id` set and the body sends an
eligible pair. **Then** the request pair runs. The connection is used
only when it belongs to that provider; otherwise the provider's own
credential is used. An ineligible pair is 422 and does not run the
connection's model.
**Mitigation:** request is step 2 and does not fall through.
[LAW-175-4](01-first-principles.md). Closes [F-175-04](04-findings.md).

### EC-175-04 — Revoked key

**Given** the winning provider's probe failed or the key was deleted.
**When** a member queries. **Then** 503 `credential_unavailable`. The
process `OPENAI_API_KEY` is not called.
**Mitigation:** [LAW-175-5](01-first-principles.md). No auth-fallback retry.

### EC-175-05 — Ceiling removes an assigned model

**Given** a workspace assigns `llava` for vlm and the tenant then disables
that pair. **Then** the row shows the broken panel. A new upload that
needs vlm returns 503. Other usages still run.
**Mitigation:** `broken_reason` does not fall through.
[LAW-175-5](01-first-principles.md).

### EC-175-06 — Embedding id on extract

**Given** a client puts an embedding model in `llm_roles.extract`.
**Then** save returns 422 `capability_mismatch`. The previous assignment
stays.
**Mitigation:** predicate in [06-catalog](06-catalog.md).

### EC-175-07 — Custom model

**Given** a provider with `allows_custom_model`. **When** the admin saves
an id whose probe failed. **Then** the id is not in member pickers.
**When** the probe succeeded. **Then** the id is a normal policy row.
**Mitigation:** [06-catalog](06-catalog.md).

### EC-175-08 — Decision is not a chat model

**Given** the chat catalog. **Then** `tev1:0.8b` is absent from the query
picker. `openai_logprobs` still fails boot. Decision effective source
follows SPEC-160 overrides inside `resolve_usage`.
**Mitigation:** [LAW-175-3](01-first-principles.md).

### EC-175-09 — Rerank model rejected

**Given** `POST /query` with `rerank_model` set. **Then** 422
`rerank_model_rejected`. The You page has no rerank switch.
**Mitigation:** [LAW-175-7](01-first-principles.md).

### EC-175-10 — Discovery off

**Given** `EDGEQUAKE_DYNAMIC_MODELS=0`. **Then** eligible models are the
static cards that match the predicate. Live `/api/tags` is not required.
**Mitigation:** Static cards are the catalog when the flag is off. Live `/api/tags` is not a
fallback list. [06-catalog](06-catalog.md).

### EC-175-11 — OpenAI ids without cards

**Given** discovery on and an OpenAI id with no `models.toml` card.
**Then** the id is hidden. A card's `supports_json_mode` is what extract
uses, not the live list.
**Mitigation:** an id with no static card stays out of `eligible_models`.
Extract reads `supports_json_mode` from the card, not from the live list.
Connection help copies an id from that list, so `gpt-4o-mini` cannot
reappear as a hardcoded example. [F-175-22](04-findings.md).

### EC-175-12 — Hybrid embed is the leaf

**Given** no workspace or tenant embedding metadata and
`EDGEQUAKE_EMBEDDING_PROVIDER=ollama`. **Then** source is `env_bootstrap`.
A workspace `embedding_model` key beats it.
**Mitigation:** [LAW-175-2](01-first-principles.md).

### EC-175-13 — Priority mode swaps the leaf only

**Given** `EDGEQUAKE_CONFIG_PRIORITY=env`, a tenant default, and a
different env model. **Then** the tenant default runs. **Given** no
tenant and no workspace metadata. **Then** env beats `server_config`.
**Mitigation:** `EDGEQUAKE_CONFIG_PRIORITY` swaps steps 6 and 7 only. A tenant or workspace
value still beats env. [05-resolution-law](05-resolution-law.md).

### EC-175-14 — Transport flags are not rows

**Given** `EDGEQUAKE_LLM_OMIT_TEMPERATURE=1`. **Then** the settings shell
has no temperature-omit control. The wire body omits temperature.
Reasoning omit behaves the same.
**Mitigation:** The flags are read by the provider HTTP client. `SETTINGS_INDEX` has no row
for them, so search cannot surface one.

### EC-175-15 — Other tenant's connection

**Given** workspace in tenant A and `connection_id` of tenant B.
**Then** 404 `connection_not_found`. The composite foreign key in
migration 221 makes a mismatched policy row unrepresentable. Connection
ids are checked in the loader because they already exist on
`llm_roles`.
**Mitigation:** Migration 221's composite foreign key cannot store a workspace from another
tenant. The loader returns 404 and does not decrypt the other tenant's key.

### EC-175-16 — Member picker

**Given** a member. **Then** the query chip lists the narrowed query set.
`PUT` assignment or policy returns 403. Organization and Deployment are
absent from the nav.
**Mitigation:** `eligible_models` applies the ceiling before the response. The write uses
`can_manage_workspaces`. Hidden nav is not the check. [LAW-175-13](01-first-
principles.md).

### EC-175-17 — Theme and language agree

**Given** the header language set to French. **Then** the You select
shows French after navigation. Theme light, dark, and system agree the
same way. `useSettingsStore.theme` is not written.
**Mitigation:** Theme has one writer, `next-themes`. Language has one writer, `setLanguage`.
`useSettingsStore.setTheme` is deleted. [10-frontend-
architecture](10-frontend-architecture.md).

### EC-175-18 — No silent reprocess

**Given** a completed document and a new extract model. **Then** the
document status stays completed. A new upload uses the new model.
Rebuild runs only after the explicit action.
**Mitigation:** A model change does not enqueue reprocess and does not rewrite lineage.
[LAW-175-14](01-first-principles.md).

### EC-175-19 — Empty policy

**Given** zero `model_catalog_policy` rows. **Then** `policy_mode` is
`unconstrained` and every healthy, capable model is eligible. The first
ceiling PUT switches the mode and the response says `constrained`.
**Mitigation:** Zero rows is unconstrained. The first non-empty ceiling PUT flips the mode
and the UI shows the confirm sentence. A later PUT of an empty array deletes
the rows and returns to unconstrained. The replace does not merge, so a
stale client cannot leave one old enable behind.

### EC-175-20 — Keyword pin

**Given** `EDGEQUAKE_KEYWORD_LLM_MODEL` set and a keyword role.
**Then** the pin wins, `locked` is true, and the role connection is not
loaded.
**Mitigation:** The keyword pin is step 1 of the same ladder as extract. While `locked` is
true the role connection is not loaded.

### EC-175-21 — Manuscript lock

**Given** manuscript vision env set and a workspace vlm role.
**When** the page is manuscript-class. **Then** the env pair runs and
source is `operator_lock`. **When** the page is print. **Then** the
workspace role runs.
**Mitigation:** The manuscript env pair is an operator lock only when the page is
manuscript-class. Print pages use the workspace vlm ladder.

### EC-175-22 — Upload vision is the request layer

**Given** `llm_roles.vlm` set and the upload form sends a different
eligible vision pair. **Then** the upload pair is stored and used for
that document. **Given** a workspace whose vision Option fields were
painted from env and whose metadata has no `vision_llm_*` keys.
**Then** source is not `workspace_role` or `workspace_default`.
**Mitigation:** closes [F-175-05](04-findings.md) and the vision half of
LAW-123-8.

### EC-175-23 — Gleaning control is gone

**Given** the You and Workspace pages. **Then** no control writes
`enableGleaning`. A persisted old zustand blob does not send gleaning on
upload.
**Mitigation:** The controls are removed and the zustand persist version drops those keys,
so an old browser blob cannot post them.

### EC-175-24 — Deeplink opens the shell

**Given** `/w/{slug}/settings`. **Then** the shell shows that workspace's
Models section. It does not redirect to `/workspace`.
**Mitigation:** The slug route selects that workspace and renders the shell. It does not
call `router.push('/workspace')`.

### EC-175-25 — Fleet connection

**Given** `provider_connections.tenant_id` null and a workspace assignment
that names it. **Then** the connection loads. A non-admin member does not
see the key. They see the connection display name on the usage row only
when they can edit; members see the effective provider name.
**Mitigation:** A null `tenant_id` connection is fleet-visible. The key never leaves the
server. Members see the provider name from `ResolvedUsage`, not the
connection secret.

### EC-175-26 — Embedding dimension

**Given** a workspace changes `embedding_model` to a different dimension.
**Then** the save response includes `rebuild_required: true`. Query does
not mix the two dimensions. Vectors stay until rebuild.
**Mitigation:** The save response sets `rebuild_required` when the dimension changes. Query
keeps using the old vectors until that action runs.

### EC-175-27 — Answer cache does not cross models

**Given** an answer cached under model A. **When** the query model is B
and the prompt text matches. **Then** the cache misses and B runs.
**Mitigation:** The answer-cache hash input gains the resolved model id. A miss is the safe
result. The cache is not wiped on a model change.

### EC-175-28 — Inherit is the default

**Given** a fresh browser profile. **Then** query requests omit
`llm_provider` and `llm_model`. After a pick, the chip shows `request`.
After a workspace switch to a workspace where that id is not eligible,
the chip returns to inherit.
**Mitigation:** The persisted query provider defaults to absent. On workspace change the
client drops a pair that is not in the new eligible list.

### EC-175-29 — One health view

**Given** Deployment → Fleet. **Then** one health region lists providers.
The strings "Provider Status" do not appear twice.
**Mitigation:** One `provider-health` component replaces the two cards. W4 removes the
second fetch.

### EC-175-30 — Narrowing cannot widen

**Given** a constrained tenant that did not enable `gpt-5.4` for query.
**When** the workspace PUT enables it. **Then** 422
`policy_widens_ceiling`.
**Mitigation:** The repository rejects the write before insert when a workspace
`enabled=true` row is outside the ceiling. The previous narrowing stays.

### EC-175-31 — Readonly membership

**Given** a readonly member. **Then** usage radios are absent.
`PUT /workspaces/{id}` with `llm_roles` returns 403 when binding is on.
**Mitigation:** Readonly fails `can_manage_workspaces`, so the handler returns 403 and the
radios are not rendered. Binding off does not open the tenant ceiling
([EC-175-54](11-edge-cases.md)).

### EC-175-32 — Broken assignment does not use the compiled default

**Given** `broken_reason = probe` on extract. **Then** ingest fails.
The worker does not call the compiled default model.
**Mitigation:** `broken_reason` stops provider construction. The compiled default runs only
when it is the winning layer, never as a recovery model.
[LAW-175-5](01-first-principles.md).

### EC-175-33 — Search

**Given** the query "embedding". **Then** the nav shows the workspace
Models entry and activating it scrolls to the embedding row.
**Mitigation:** Search runs on `SETTINGS_INDEX` after the role filter. A scope the caller
cannot open cannot be the result.

### EC-175-34 — Deployment is admin-only

**Given** a tenant admin who is not instance admin. **Then** the
Deployment group is absent and `/settings/deployment` redirects to You.
Connections and quotas are not on You or Workspace.
**Mitigation:** The layout treats Deployment as `Role::Admin` only. A Super Admin who is not
a Platform Admin is redirected from `/settings/deployment` to You.

### EC-175-35 — Graph window is not query

**Given** the graph popover. **Then** its title is "Graph window." It
has no provider or model field.
**Mitigation:** The popover writes `graphSettings` only. It does not mount `model-picker`.

### EC-175-36 — Workspace Admin cannot write the ceiling

**Given** a membership with `workspace_id` set and role admin.
**When** that user `PUT`s `/tenants/{id}/model-policy`. **Then** 403
`tenant_access_denied`. Organization is absent from their nav.
**Mitigation:** `get_user_role` keeps tenant-wide rows and requires `can_manage_users`. The
handler calls `require_tenant_access` itself, because `/api/v1/tenants` is
`global_resource` and `bind_request` returns before the membership check.
The same flag hides the nav item, filters search, and redirects
`/settings/organization` to You. GET of the ceiling uses that gate too. The
broken sentence comes from `GET /config/effective`.

### EC-175-37 — Super Admin assigns any workspace in the tenant

**Given** a membership with `workspace_id` null and role admin.
**When** they `PUT` `/workspaces/{id}` for Finance and for Legal.
**Then** both succeed. They did not need a second membership row.
**Mitigation:** A null `workspace_id` passes `applies_to` for every workspace in the tenant.
A second membership row is not created and is not required.

### EC-175-38 — Last active tenant Owner cannot be removed

**Given** one active tenant-wide Owner. **When** that Owner, another
Super Admin, or a Platform Admin demotes or removes them. **Then** 409
`last_tenant_owner`.
**Given** two active tenant-wide Owners. **Then** demoting one succeeds.
**Given** the only Owner row is inactive. **Then** it is not counted. A
Platform Admin may appoint a new tenant-wide Owner. They still cannot
remove the last active one while it exists.
**Mitigation:** the count is active rows with `workspace_id` null and
role `owner`. An inactive Owner does not brick the tenant. There is no
matching lock on the last Workspace Admin
([EC-175-50](11-edge-cases.md)).

### EC-175-39 — Other workspace is the member page

**Given** a Workspace Admin of Legal and a member-or-absent grant on
Finance. **When** they open Finance models. **Then** radios and Save are
absent. The chip stays Workspace admin.
**Mitigation:** The app header still lists every workspace this person can access, so
Finance stays reachable. The settings switcher lists only workspaces they
administer. Opening Finance by the header or by `/w/finance/settings`
renders the member page and returns 403 on PUT. The chip stays Workspace
admin. Organization stays absent.

### EC-175-40 — Platform admin chip is not Super admin

**Given** `Role::Admin` opening a tenant's Organization. **Then** the
chip reads "Platform admin". A disabled catalog model stays disabled for
their support session.
**Mitigation:** The chip reads the instance role first. Support writes still go through
`resolve_usage` and the ceiling. `Role::Admin` does not set `policy_mode` to
unconstrained.

### EC-175-41 — SSO max_role cannot mint a tenant Owner

**Given** an IdP map whose `max_role` is admin. **When** the claim would
write a tenant-wide Owner. **Then** the membership stays at or below
admin. A Super Admin can still pin a role with `pin_manual_role`.
**Mitigation:** `max_role` caps the mapped role before insert. `pin_manual_role` sets
`metadata.source` to `manual`, and the next sign-in skips rows that are not
`sso`. A Workspace Admin receives 403 if they try to pin a tenant-wide
membership. Only a Super Admin or a Platform Admin can pin.

### EC-175-42 — A Workspace Admin cannot write another workspace

**Given** a Workspace Admin of Legal. **When** they `PUT`
`/workspaces/{finance}` or `/workspaces/{finance}/model-policy`.
**Then** 403. `/w/finance/settings` renders the member page
([EC-175-39](11-edge-cases.md)).
**Mitigation:** `membership_role` takes the highest role that
`applies_to` Finance. A Legal row does not apply. The page hide is not
the check.

### EC-175-43 — Organization cannot be opened by URL or search

**Given** a Workspace Admin. **When** they open `/settings/organization`
or search "catalog" or "people". **Then** the route redirects to You
and search has no Organization hit.
**Mitigation:** the layout flag is `get_user_role` plus
`can_manage_users`, the same predicate as the tenant policy handler.
`SETTINGS_INDEX` is filtered before match. [10-frontend-architecture](10-frontend-architecture.md).

### EC-175-44 — Invites and new workspaces stay tenant-wide

**Given** a Workspace Admin. **When** they create a workspace or invite
a member. **Then** 403 `tenant_access_denied`. A Super Admin's same
calls succeed.
**Mitigation:** `create_workspace` already calls
`require_tenant_access(Manage)`. Invites use `get_user_role`, which
drops workspace-scoped rows. [F-175-25](04-findings.md).

### EC-175-45 — Another tenant's id is a 404

**Given** a Super Admin of Acme. **When** they `PUT` a tenant policy or
a workspace policy whose id is in a different tenant. **Then** 404.
**Given** an inactive tenant. **Then** 403 `tenant_access_denied`.
**Mitigation:** cross-tenant workspace checks return 404, matching
`verify_workspace_tenant_access`, so the caller does not learn that the
id exists. `require_tenant_access` rejects an inactive tenant before
the role lookup.

### EC-175-46 — A tenant-wide member is not a Super Admin

**Given** a null `workspace_id` membership with role `member`, plus an
admin membership on Legal. **Then** the chip says Workspace admin.
Organization is absent. Legal is editable. Finance is the member page.
**Mitigation:** Super Admin requires `can_manage_users` on the
tenant-wide row. A null `workspace_id` alone is only "member of every
workspace." [LAW-175-17](01-first-principles.md).

### EC-175-47 — An inactive membership grants nothing

**Given** `is_active = false` on an owner row, workspace-scoped or
tenant-wide. **Then** the chip does not use it, `PUT` returns 403, and
`can_access_workspace` is false.
**Mitigation:** `applies_to` and `get_user_role` already skip inactive
rows. New policy handlers call those functions. They do not read a raw
membership and ignore the flag.

### EC-175-48 — Ceiling replace does not merge

**Given** two Super Admins save the ceiling, and the older request
arrives last with one model omitted. **Then** the omitted model is
disabled. It is not re-enabled by a key the stale client left out.
**Given** `PUT` body `[]`. **Then** `policy_mode` returns to
`unconstrained`.
**Mitigation:** the write is a replace inside one transaction: delete
that scope's rows, insert the body, commit. A unique-index conflict
rolls back, and the previous ceiling stays. [08-api-contract](08-api-contract.md).

### EC-175-49 — Reading the ceiling is the same gate as writing it

**Given** a Workspace Admin. **When** they `GET /tenants/{id}/model-policy`.
**Then** 403 `tenant_access_denied`. The usage row still shows
`broken_reason` from `GET /config/effective`, which a member may call.
**Mitigation:** GET and PUT share `require_tenant_access(Manage)`. The
effective route stays a workspace-member read and does not list disabled
ceiling rows.

### EC-175-50 — The last Workspace Admin may be removed

**Given** Legal has one Workspace Admin and the tenant has a Super
Admin. **When** the Super Admin removes that Workspace Admin.
**Then** the call succeeds. Legal stays editable by the Super Admin.
**Mitigation:** 409 `last_tenant_owner` is the only last-role lock
([EC-175-38](11-edge-cases.md)). Copying it onto a workspace would
strand a workspace the tenant Super Admin already covers.

### EC-175-51 — Binding off does not open the ceiling

**Given** membership binding is off (no SSO and strict tenant bind
unset). **When** a Workspace Admin `PUT`s the tenant ceiling.
**Then** 403 `tenant_access_denied` unless they also hold a tenant-wide
owner or admin row.
**Given** the same flag and `PUT /workspaces/{id}` for a workspace they
administer. **Then** the write follows today's `bind_request`: binding
off allows the authenticated user, the same as workspace update today.
**Mitigation:** `/api/v1/tenants` is `global_resource`, so the ceiling
handler calls `require_tenant_access` and does not rely on
`bind_request`. It does not treat binding off as "everyone is Super
Admin." [F-175-26](04-findings.md).

### EC-175-52 — The master API key is not a Super Admin

**Given** a request authenticated as `master-api-key`. **When** it
`PUT`s the tenant ceiling. **Then** 403 `tenant_access_denied`.
`require_tenant_access` parses a user UUID, and this caller has none.
**When** it `PUT`s `/workspaces/{id}` the way workspace update works
today. **Then** `bind_request` returns before the membership check, the
existing audit line is emitted, and the handler runs.
**Mitigation:** do not insert a Super Admin membership for this key,
and do not skip `require_tenant_access` on the ceiling to make the key
succeed. The ceiling stays on the same function as `PUT /tenants/{id}`.

### EC-175-53 — Catalog filters run before policy

**Given** `EDGEQUAKE_ALLOWED_PROVIDERS` omits a provider that a ceiling
row names. **Then** that provider is absent from eligible models.
**Given** `last_test_ok` is true and `last_test_at` is older than 24
hours, and env health does not report the provider up. **Then** member
pickers omit it and an assignment that names it is
`broken_reason = probe`.
**Mitigation:** the allow-list and the probe window in
[06-catalog](06-catalog.md) filter candidates before the ceiling is
applied. A ceiling row cannot resurrect a provider the deployment hid
or a probe that expired.

### EC-175-54 — Request fields do not steer extract

**Given** a client sends `llm_provider` and `llm_model` on a path that
resolves extract, summary, keyword, embedding, or decision.
**Then** those fields are ignored. The ladder continues at the
workspace role. The response is not 422 for the extra fields.
**Given** the same pair on query or upload vision, and the pair is not
eligible. **Then** 422 `model_not_eligible` and the connection's model
does not run.
**Mitigation:** step 2 of the ladder runs only for `query` and `vlm`.
[05-resolution-law](05-resolution-law.md). Ignoring the extra fields
keeps an old client that echoes them from breaking ingest.
