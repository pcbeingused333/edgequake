# 00 — WHY (5-WHY)

Parent: [README](README.md) · Next: [01-first-principles](01-first-principles.md)

## The job to be done

EdgeQuake is a multi-tenant RAG product. A person asking a question should
see only providers and models that can actually run that question. An owner
should assign one model per job (extract, answer, embed, vision) and trust
that the pipeline uses that assignment. A tenant admin should be able to
say which models the organization allows. A platform operator should hold
the API keys.

Today those four jobs share one long page, and five different functions
pick the model that runs. The page can show a model the pipeline does not
use. That is why this pack exists.

The operation axis is already real: instance admin, tenant owner, workspace
owner, member. This pack does not replace it. It gives each owner a scope,
and one function that all of them read.

---

## 5-WHY chain A — Settings is a closet

| # | Question | Answer |
|---|----------|--------|
| 1 | Why does a member who wants dark mode also see API keys and quotas? | [`settings/page.tsx`](<../../edgequake_webui/src/app/(dashboard)/settings/page.tsx>) is one `ScrollArea`. [`SETTINGS_SECTIONS`](../../edgequake_webui/src/components/settings/settings-nav.tsx) are in-page anchors, not routes. |
| 2 | Why are anchors not enough? | General holds local theme. Models holds fleet defaults and workspace vision. Access holds document labels, connections, the role matrix, attribution, and quotas. |
| 3 | Why did that happen? | Each SPEC added a card to the same page (043, 109, 124, 163, 168, 174). |
| 4 | Why is a chip bar the wrong fix? | The user knows the setting's name, not which of five chips owns it. Owners are different people. |
| 5 | **Root cause** | **The page groups by when the card was built, not by who is allowed to change it.** |

```text
  Member
    |
    v
  /settings  (one scroll)
    |
    +-- theme, language, graph look     (this browser)
    +-- fleet LLM defaults              (instance admin)
    +-- workspace vision + PDF          (workspace owner)
    +-- connections + API keys          (instance admin)
    +-- document labels                 (workspace owner)
    +-- quotas                          (instance admin)
    +-- export local JSON               (this browser)
```

Cross-ref: [LAW-175-8](01-first-principles.md) ·
[F-175-01](04-findings.md) · [EC-175-34](11-edge-cases.md).

Market pattern this chain adopts: Vercel and GitHub put account, team, and
project in different owners
([settings layout survey](https://github.com/onetimesecret/onetimesecret/issues/2307)).
Linear puts search above the hierarchy
([The Settings Page Problem](https://brainy.ink/paper/the-settings-page-problem)).

---

## 5-WHY chain B — The screen and the run disagree

| # | Question | Answer |
|---|----------|--------|
| 1 | Why can the UI show a workspace extract model that ingest does not use? | `resolve_extract_role_llm` returns the env pin first ([`llm_roles.rs:353`](../../edgequake/crates/edgequake-core/src/llm_roles.rs)). |
| 2 | Why is that not `resolve_llm_choice`? | LAW-123-2 lives in [`model_resolution.rs:9`](../../edgequake/crates/edgequake-core/src/model_resolution.rs). Extract, keyword, the query connection branch, and VLM runtime do not call it. |
| 3 | Why does a query role skip the request? | `resolve_llm_provider_for_workspace` returns the connection client before it reads the request ([`resolver.rs:439`](../../edgequake/crates/edgequake-api/src/providers/resolver.rs)). |
| 4 | Why does a VLM role ignore the upload? | `resolve_workspace_vlm_config` returns `resolve_role_llm(Vlm)` when `llm_roles.vlm` exists ([`vlm_provider_resolver.rs:34`](../../edgequake/crates/edgequake-api/src/services/vlm_provider_resolver.rs)). |
| 5 | **Root cause** | **There is a documented ladder and several private ladders. The effective-config endpoint follows one of them.** |

```text
  LAW-123-2
  Request > Workspace > Tenant > Env > Default
       |                 |              |
       |                 |              +-- resolve_llm_choice
       |                 |
       |                 +-- extract/keyword: Env pin wins, tenant skipped
       |
       +-- query + connection_id: connection wins, request skipped
       +-- vlm role present: upload skipped
```

Cross-ref: [LAW-175-1](01-first-principles.md) ·
[LAW-175-2](01-first-principles.md) ·
[F-175-02](04-findings.md) · [F-175-03](04-findings.md) ·
[F-175-04](04-findings.md) · [F-175-05](04-findings.md) ·
[EC-175-01](11-edge-cases.md) · [EC-175-03](11-edge-cases.md) ·
[EC-175-22](11-edge-cases.md).

---

## 5-WHY chain C — Any string can be a model

| # | Question | Answer |
|---|----------|--------|
| 1 | Why can an owner type an embedding id into the extract row? | [`model-role-matrix-card.tsx`](../../edgequake_webui/src/components/settings/model-role-matrix-card.tsx) uses free-text inputs for provider and model (lines 143–165). |
| 2 | Why does the server accept it? | `RoleLlmConfig` is three optional strings. `llm_roles` is `unknown` in OpenAPI. |
| 3 | Why is there no capability check at resolve time? | `ModelCapabilities` exists on the catalog. Vision resolve does not read `supports_vision`. |
| 4 | Why does the member then see that id in chat? | The query store can send `provider` and `model` on the request. Nothing intersects an allowlist. |
| 5 | **Root cause** | **Usage, capability, and catalog are three ideas in the code and one text field in the UI.** |

Cross-ref: [LAW-175-3](01-first-principles.md) ·
[LAW-175-6](01-first-principles.md) ·
[F-175-07](04-findings.md) · [EC-175-06](11-edge-cases.md).

Market pattern: K2View discovers models, then a show/hide list decides what
appears in the picker
([Providers and Models](https://support.k2view.com/Academy/articles/04_fabric_studio/studio_ai/06_ai_configuration_and_settings.html)).
BYOK drives every control from a capability map
([model and reasoning controls](https://byok.pro/blog/design-model-reasoning-controls/)).

---

## 5-WHY chain D — A toggle that nothing reads

| # | Question | Answer |
|---|----------|--------|
| 1 | Why does "Enable Reranking" not change answers? | The switch writes `querySettings.enableRerank` in [`use-settings-store.ts:48`](../../edgequake_webui/src/stores/use-settings-store.ts). No query builder reads it. |
| 2 | Why does the API still have `enable_rerank`? | [`query_types.rs:166`](../../edgequake/crates/edgequake-api/src/handlers/query_types.rs) accepts it. The WebUI never sends the store value. |
| 3 | Why does gleaning on the same page not change ingest? | `enableGleaning`, `maxGleaning`, and `useLLMSummarization` are written only from [`settings/page.tsx`](<../../edgequake_webui/src/app/(dashboard)/settings/page.tsx>) (lines 476–526) into the same store. |
| 4 | Why is the rerank model field also a lie? | `rerank_model` is on the query body and set to `None` in [`query_context.rs:305`](../../edgequake/crates/edgequake-api/src/services/query_context.rs). The reranker is built once from env. |
| 5 | **Root cause** | **A setting was added because the pipeline has a concept, not because a writer and a reader share a field.** |

Cross-ref: [LAW-175-7](01-first-principles.md) ·
[F-175-08](04-findings.md) · [EC-175-09](11-edge-cases.md) ·
[EC-175-23](11-edge-cases.md).

---

## 5-WHY chain E — A tenant can run on the platform key

| # | Question | Answer |
|---|----------|--------|
| 1 | Why does a bad workspace key still return an answer? | `try_create_llm_provider` returns `None` when credentials are missing so the caller uses the server default ([`resolver.rs:498`](../../edgequake/crates/edgequake-api/src/providers/resolver.rs)). |
| 2 | Why does an auth failure on a request override also answer? | `execute_sota_query_with_auth_fallback` retries with no override ([`query_execution.rs:199`](../../edgequake/crates/edgequake-api/src/services/query_execution.rs)). |
| 3 | What is that default? | `ProviderFactory::from_env` in [`postgres.rs:134`](../../edgequake/crates/edgequake-api/src/state/postgres.rs). It is the process key, shared by every tenant. |
| 4 | Why is that unsafe on a SaaS deployment? | Tenant A's revoked key must not spend Tenant B's quota or the operator's key, and must not mix prompt cache prefixes. |
| 5 | **Root cause** | **Fallback-to-working was a single-tenant convenience. It is a cross-tenant credential.** |

```text
  Workspace key revoked
           |
           v
  +---------------------------+
  | try_create -> None        |
  +-------------+-------------+
                |
                v
  +---------------------------+
  | state.query.llm_provider  |
  | process OPENAI_API_KEY    |
  +---------------------------+
                |
                v
  Answer, billed to the platform
```

Cross-ref: [LAW-175-5](01-first-principles.md) ·
[F-175-06](04-findings.md) · [EC-175-04](11-edge-cases.md) ·
[EC-175-15](11-edge-cases.md).

---

## What this pack refuses to do

- It does not merge decision extraction into the chat catalog.
  Decision stays `EDGEQUAKE_DECISION_*` and `tev1`. `openai_logprobs`
  stays refused at boot.
- It does not put rerank model choice on the personal page.
- It does not auto-reprocess a corpus when the extract model changes.
- It does not store API keys on the policy table.
