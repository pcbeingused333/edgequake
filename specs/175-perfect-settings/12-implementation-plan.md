# 12 — Implementation plan

Parent: [README](README.md) · Prev: [11-edge-cases](11-edge-cases.md) ·
Next: [13-e2e-test-matrix](13-e2e-test-matrix.md)

W0 is this pack. W1–W6 are later changes. A wave does not start until the
previous wave's tests are green. No wave ships a surface that breaks a
[LAW-175](01-first-principles.md).

## Modules (DRY / SOLID)

| Piece | Crate or package | Depends on | Must not |
|-------|------------------|------------|----------|
| `resolve_usage`, `Usage`, `ResolvedUsage` | `edgequake-core` | catalog trait, no I/O | SQL, Axum, React, env reads inside the pure function (locks arrive in `ctx`) |
| Capability predicates | `edgequake-llm` | `models.toml`, discovery types | tenant policy |
| Policy repository | `edgequake-storage` | migration 221 | HTTP status codes |
| Handlers | `edgequake-api` | `resolve_usage` + repository | a second precedence match |
| Worker and query | existing call sites | `ResolvedUsage` | `ProviderFactory::from_env` as a recovery path |
| Picker and shell | `edgequake_webui` | eligible and effective endpoints | ladder math, `models.toml` |

Env is read once at the edge (`locks_from_env()`) and passed in. Tests
pass locks as data. That is how Acc pins stay deterministic without a
global mutex beyond what the process already does.

`LlmRole` remains. `Usage` wraps it and adds `embedding`, `decision`,
`rerank`. Do not fork a third enum in the WebUI.

## Waves

### W0 — This pack

Documents only. **DoD:** a reader can trace LAW → finding → EC → test
in [14-cross-ref](14-cross-ref.md).

### W1 — Pure resolver

Add `resolve_usage` beside `resolve_llm_choice`. Unit tests for
[EC-175-01](11-edge-cases.md), [EC-175-02](11-edge-cases.md),
[EC-175-12](11-edge-cases.md), [EC-175-13](11-edge-cases.md),
[EC-175-20](11-edge-cases.md), [EC-175-21](11-edge-cases.md),
[EC-175-22](11-edge-cases.md), [EC-175-54](11-edge-cases.md).

Switch call sites one usage at a time. Extract and query first, then
keyword, summary, vlm, embedding. Decision delegates to the existing
SPEC-160 function and copies the result into `ResolvedUsage`.

Delete the success path in `execute_sota_query_with_auth_fallback` and
the silent `None` fallthrough in `try_create_llm_provider`. Map
`broken_reason` to 503 at the handler. Tests:
[EC-175-03](11-edge-cases.md), [EC-175-04](11-edge-cases.md),
[EC-175-32](11-edge-cases.md).

Include the resolved model id in the answer-cache hash.
[EC-175-27](11-edge-cases.md).

**DoD:** `cargo test -p edgequake-core usage_resolution` green.
`cargo test -p edgequake-api` query and extract tests green. Acc doctor
still sees the extract pin win.

**Must not:** a new card, a migration, or a change to personal settings.

### W2 — Policy table and HTTP

Migration 221 as in [07-data-and-db-contract](07-data-and-db-contract.md).
Repository, `GET/PUT` policy, extended `GET /config/effective`,
`GET /models/eligible`, 422s in [08-api-contract](08-api-contract.md).
OpenAPI `RoleLlmConfig`. Regenerate `schema.d.ts`.

Tests: [EC-175-05](11-edge-cases.md), [EC-175-06](11-edge-cases.md),
[EC-175-15](11-edge-cases.md), [EC-175-19](11-edge-cases.md),
[EC-175-30](11-edge-cases.md), [EC-175-31](11-edge-cases.md),
[EC-175-09](11-edge-cases.md) (API half),
[EC-175-36](11-edge-cases.md), [EC-175-37](11-edge-cases.md),
[EC-175-38](11-edge-cases.md), [EC-175-41](11-edge-cases.md),
[EC-175-42](11-edge-cases.md), [EC-175-44](11-edge-cases.md),
[EC-175-45](11-edge-cases.md), [EC-175-47](11-edge-cases.md),
[EC-175-48](11-edge-cases.md), [EC-175-49](11-edge-cases.md),
[EC-175-50](11-edge-cases.md), [EC-175-51](11-edge-cases.md),
[EC-175-52](11-edge-cases.md), [EC-175-53](11-edge-cases.md).

Policy routes stay off `/api/v1/settings/` ([F-175-26](04-findings.md)).
No membership migration. Super Admin is a null `workspace_id`.

**DoD:** `cargo test -p edgequake-api --test` policy tests green.
Empty database still unconstrained.

**Must not:** the WebUI shell. Old `/settings` keeps working against the
old cards.

### W3 — Shell, same behavior

Routes in [10-frontend-architecture](10-frontend-architecture.md). Move
cards into sections without changing their payloads. Fix
`/w/[slug]/settings`. The nav reads membership scope
([15-roles](15-roles.md)): Organization only for a Super Admin or a
Platform Admin, Deployment only for a Platform Admin. A Workspace Admin
who switches to a workspace they do not administer sees the member
models page. The header chip follows the badge order in that file. One
health component fed by both current endpoints until W4 drops one call.

Tests: [EC-175-24](11-edge-cases.md), [EC-175-34](11-edge-cases.md),
[EC-175-29](11-edge-cases.md) (single region; data sources may still be
two until W4), [EC-175-39](11-edge-cases.md),
[EC-175-40](11-edge-cases.md), [EC-175-43](11-edge-cases.md),
[EC-175-46](11-edge-cases.md).

**DoD:** Playwright smoke: a Workspace Admin does not see Organization.
A Platform Admin chip does not say Super admin. Deeplink shows Models.
Existing settings e2e (`spec100`, `spec124`) retargeted to the new routes.

**Must not:** remove rerank yet. W4 does that so the move and the
deletion are reviewable apart.

### W4 — Typed catalog UI

`usage-row` and `catalog-editor` replace the free-text matrix.
Delete rerank and gleaning controls. Bump the zustand persist version
and drop those keys. Reject `rerank_model` already shipped in W2.
Connection shape becomes a select. Remove the duplicate status card's
`gpt-4o-mini` snippet.

Tests: [EC-175-07](11-edge-cases.md), [EC-175-09](11-edge-cases.md)
(UI half), [EC-175-11](11-edge-cases.md), [EC-175-16](11-edge-cases.md),
[EC-175-23](11-edge-cases.md), [EC-175-33](11-edge-cases.md).

**DoD:** no `<Input>` for a model id on workspace Models, except the
deployment custom-id field that probes first.

### W5 — Contextual bindings

Query chip and upload vision use `model-picker`. Header language uses
`setLanguage`. Header theme and You use `next-themes` only. Remove
`setTheme` from the settings store. Graph popover renamed. Workspace
overview cards link out. Sticky query model clears on workspace switch
when ineligible. Save of a new embedding dimension surfaces
`rebuild_required`.

Tests: [EC-175-17](11-edge-cases.md), [EC-175-18](11-edge-cases.md),
[EC-175-26](11-edge-cases.md), [EC-175-28](11-edge-cases.md),
[EC-175-35](11-edge-cases.md), [EC-175-08](11-edge-cases.md) (picker
does not list `tev1`), [EC-175-14](11-edge-cases.md) (no omit row).

**DoD:** one network call pattern for eligible models, shared by three
screens. `bun test` and the settings Playwright specs green.

### W6 — End-to-end gate

Playwright plus API: owner sets a ceiling, member picker hides the rest,
broken probe blocks a query, operator lock is visible. This is the demo
in [03-product-spec](03-product-spec.md).

**DoD:** every row in [13-e2e-test-matrix](13-e2e-test-matrix.md) has a
named test that exists in the tree. `cargo fmt`, `cargo clippy --workspace
--all-targets -- -D warnings` on the touched crates, `bun test` for the
touched UI.

## Order of risk

W1 changes runtime resolution. Ship it behind the same inputs: locks
match today's pins, and unconstrained policy is not in the binary until
W2. W2 without UI is safe because zero rows mean unconstrained. W3 is a
move. W4 is the first day a member sees fewer models, and only after an
admin opts into a ceiling.

Do not combine W1 and W4 in one pull request.
