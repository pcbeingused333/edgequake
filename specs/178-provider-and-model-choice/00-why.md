# 00 — WHY (5-WHY)

Parent: [README](README.md) · Next: [01-first-principles](01-first-principles.md)

## The job to be done

Settings already say which providers a deployment offers, which of those a
tenant keeps, and which models a workspace may use. A workspace admin
assigning a model, and a member asking a question, should see that same
set. The model they pick should be the model that runs.

Today the workspace Models editor reads the eligible set. The query chip
and the older pickers read the deployment inventory. A save accepts any
string. The streaming path the composer uses does not check the set. The
screen and the run can name different models. That is why this pack exists.

SPEC-175 defined the ladder and the policy rows.
SPEC-177 defined the provider cascade.
This pack binds the choice to both.

---

## 5-WHY chain A — Two catalogs

| # | Question | Answer |
|---|----------|--------|
| 1 | Why can the query chip list a model the workspace Models editor hides? | The chip calls `useLlmModels` ([`model-chip.tsx:46`](../../edgequake_webui/src/components/query/composer/model-chip.tsx)). The editor calls `useEligibleModels` ([`usage-role-row.tsx:65`](../../edgequake_webui/src/components/settings/models/usage-role-row.tsx)). |
| 2 | Why do those hooks differ? | `useLlmModels` loads `GET /models/llm` ([`use-providers.ts:92`](../../edgequake_webui/src/hooks/use-providers.ts)). `useEligibleModels` loads `GET /models/eligible` ([`use-model-policy.ts:73`](../../edgequake_webui/src/hooks/use-model-policy.ts)). |
| 3 | Why is `/models/llm` so wide? | [`list_llm_models`](../../edgequake/crates/edgequake-api/src/handlers/models.rs) returns every enabled `models.toml` card plus live discovery, filtered only by `EDGEQUAKE_ALLOWED_PROVIDERS`. The comment at lines 134–135 says the list exists so every model is selectable. |
| 4 | Why was a second list added at all? | SPEC-175 added `eligible_models` for policy. The composer, created under SPEC-032, was left on the inventory. The client module already says member pickers read `/models/eligible` ([`model-catalog.ts:5`](../../edgequake_webui/src/lib/api/edgequake/model-catalog.ts)). The chip does not. |
| 5 | **Root cause** | **Inventory and choice are two endpoints, and the choice surfaces still call the inventory.** |

```text
  Workspace Models editor                Query chip
           |                                  |
           v                                  v
  GET /models/eligible               GET /models/llm
  offer + capability + probe         models.toml enabled
  + ceiling + narrowing              + live discovery
           |                                  |
           v                                  v
  short list                          deployment catalog
```

Cross-ref: [LAW-178-2](01-first-principles.md) · [LAW-178-8](01-first-principles.md) ·
[F-178-01](04-findings.md) · [F-178-02](04-findings.md) ·
[EC-178-01](11-edge-cases.md) · [EC-178-09](11-edge-cases.md).

---

## 5-WHY chain B — A save accepts any string

| # | Question | Answer |
|---|----------|--------|
| 1 | Why can a workspace store `provider=acme`, `model=not-in-catalog`? | `PUT /workspaces/{id}` copies `llm_provider`, `llm_model`, `embedding_*`, `vision_llm_*`, and `llm_roles` onto the workspace ([`workspace_crud.rs`](../../edgequake/crates/edgequake-api/src/handlers/workspaces/workspace_crud.rs)). |
| 2 | Why does that copy succeed? | [`apply_llm_config_update`](../../edgequake/crates/edgequake-core/src/workspace_model_update.rs) writes the strings into metadata. It does not call `eligible_models`. |
| 3 | Why was the check placed only on the query? | [`ensure_pair_eligible`](../../edgequake/crates/edgequake-api/src/services/model_policy_enforcement.rs) runs when a request names a pair. The comment on lines 1–8 calls the picker a convenience and the server the authority for requests. The assignment write was left out. |
| 4 | Why does a later query then run that assignment? | The same function returns success when either name is omitted (lines 91–93). An omitted pair means "use the stored assignment," and the stored assignment is never judged. |
| 5 | **Root cause** | **Eligibility is a request filter. The assignment is a free string, and the run treats that string as already chosen.** |

```text
  PUT /workspaces/{id}
        |
        v
  metadata.llm_model = any string
        |
        v
  query with no llm_provider / llm_model
        |
        v
  ensure_pair_eligible returns Ok
        |
        v
  resolver uses the stored string
```

Cross-ref: [LAW-178-3](01-first-principles.md) · [F-178-04](04-findings.md) ·
[F-178-06](04-findings.md) · [EC-178-03](11-edge-cases.md) ·
[EC-178-23](11-edge-cases.md).

---

## 5-WHY chain C — The stream skips the check

| # | Question | Answer |
|---|----------|--------|
| 1 | Why can a member send a model the tenant ceiling removed and still get an answer? | The composer streams. [`streaming.rs`](../../edgequake/crates/edgequake-api/src/handlers/chat/streaming.rs) resolves a provider around line 356 and does not call `ensure_pair_eligible`. |
| 2 | Why does `/query` refuse the same pair? | [`query_execute.rs`](../../edgequake/crates/edgequake-api/src/handlers/query/query_execute.rs) calls `ensure_pair_eligible` before the run. |
| 3 | Why is non-streaming chat only a partial check? | [`completion.rs`](../../edgequake/crates/edgequake-api/src/handlers/chat/completion.rs) calls the check after resolution, and only when `has_explicit_provider()` is true. A provider-only body is gap-filled, then checked as the filled pair. |
| 4 | Why does a failed key still answer? | `Ok(None)` from the resolver uses the process provider (streaming.rs around line 375, completion.rs around line 272). [`execute_sota_query_with_auth_fallback`](../../edgequake/crates/edgequake-api/src/services/query_execution.rs) retries the process provider after an auth error on the override. |
| 5 | **Root cause** | **The path the product uses to answer is not the path that enforces the eligible set, and failure falls through to a provider the settings may have withdrawn.** |

```text
  Composer  --stream-->  chat/streaming.rs
                              |
                              +-- resolve
                              +-- no ensure_pair_eligible
                              |
                              v
                         process provider, if resolve returns None
                         or the override fails auth
```

Cross-ref: [LAW-178-5](01-first-principles.md) · [F-178-07](04-findings.md) ·
[F-178-09](04-findings.md) · [F-178-10](04-findings.md) ·
[EC-178-02](11-edge-cases.md) · [EC-178-13](11-edge-cases.md) ·
[EC-178-30](11-edge-cases.md).

---

## 5-WHY chain D — Inherit shows the inventory default

| # | Question | Answer |
|---|----------|--------|
| 1 | Why does an empty chip say the process model, not the workspace model? | The chip labels inherit from `data.default_model` on the `/models/llm` payload ([`model-chip.tsx:57`](../../edgequake_webui/src/components/query/composer/model-chip.tsx)). |
| 2 | Why is that payload the process model? | `list_llm_models` sets `default_model` from `state.query.llm_provider`, the client built at startup ([`models.rs:149`](../../edgequake/crates/edgequake-api/src/handlers/models.rs)). |
| 3 | Why is the workspace model a different call? | `GET /config/effective` already returns `resolve_usage` for `query`, including `source` and `broken_reason`. The chip does not call it. |
| 4 | Why is a connection on the query role invisible there? | [`resolve_llm_provider_for_workspace`](../../edgequake/crates/edgequake-api/src/providers/resolver.rs) returns `try_llm_from_workspace_connection` before it reads the request (lines 439–446). The chip can show a selection the connection branch discards. |
| 5 | **Root cause** | **The empty choice is rendered from the inventory default, while the run is rendered from a private branch of the resolver.** |

```text
  Chip label (empty)          What runs
  state.query.llm_provider    1. llm_roles.query.connection_id
                              2. request, if no connection
                              3. workspace / tenant / env
```

Cross-ref: [LAW-178-3](01-first-principles.md) · [LAW-178-9](01-first-principles.md) ·
[F-178-11](04-findings.md) · [F-178-14](04-findings.md) ·
[EC-178-06](11-edge-cases.md) · [EC-178-11](11-edge-cases.md).

---

## 5-WHY chain E — The browser outlives the policy

| # | Question | Answer |
|---|----------|--------|
| 1 | Why does a model stay selected after an admin removes it from the ceiling? | The pair lives in `querySettings` on the persisted settings store ([`use-settings-store.ts`](../../edgequake_webui/src/stores/use-settings-store.ts)). One object, not one object per workspace. |
| 2 | Why does the sanitizer keep it? | [`sanitizeQueryModelSelection`](../../edgequake_webui/src/lib/query-model-selection.ts) drops a pair only when it is missing from the inventory list or the provider health check fails. A ceiling row does not remove the pair from `/models/llm`. |
| 3 | Why does switching workspace keep the same pair? | The store has a single `provider` and `model`. Workspace B inherits workspace A's last chip. |
| 4 | Why does a ceiling save not refresh the chip? | Eligible queries go stale in 15 seconds ([`use-model-policy.ts:78`](../../edgequake_webui/src/hooks/use-model-policy.ts)). The chip's hook is a different query key, `["llm-models"]`, stale at 30 seconds ([`use-providers.ts:97`](../../edgequake_webui/src/hooks/use-providers.ts)). A second `useLlmModels` in [`use-models.ts`](../../edgequake_webui/src/hooks/use-models.ts) uses yet another key and a 5-minute stale time. Invalidating eligible does not invalidate either inventory key. |
| 5 | **Root cause** | **The persisted choice is checked against the inventory, and it is not scoped to the workspace whose policy changed.** |

```text
  localStorage querySettings.provider / .model
        |
        +-- one slot for every workspace
        |
        v
  sanitize(inventory, health)   -- ceiling is invisible here
        |
        v
  next query sends the old pair
```

Cross-ref: [LAW-178-7](01-first-principles.md) · [F-178-13](04-findings.md) ·
[F-178-15](04-findings.md) · [F-178-19](04-findings.md) · [EC-178-04](11-edge-cases.md) ·
[EC-178-05](11-edge-cases.md) · [EC-178-17](11-edge-cases.md) ·
[EC-178-27](11-edge-cases.md) · [EC-178-28](11-edge-cases.md).

---

## What the five chains share

```text
  WHY A   the list is the inventory
  WHY B   the save skips the set
  WHY C   the run skips the set
  WHY D   inherit is labeled from the inventory
  WHY E   the browser remembers the inventory
```

One eligible set closes all five. The laws are in
[01-first-principles](01-first-principles.md). The code sites are in
[04-findings](04-findings.md).
