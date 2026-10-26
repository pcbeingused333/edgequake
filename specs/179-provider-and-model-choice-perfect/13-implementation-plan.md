# 13 — Implementation plan

Parent: [README](README.md) · Prev: [12-edge-cases](12-edge-cases.md) ·
Next: [14-e2e-test-matrix](14-e2e-test-matrix.md)

## WHY

The gate has to exist before the UI promises it. The migration has to exist
before refresh persists. Screenshots come last, because they record the UI
the tests already drive.

```text
  W0  spec pack
  W1  ModelGate and every runtime caller
  W2  migration 224, connection lifecycle, refresh, impact, ETag
  W3  one hook, one menu, provider screens
  W4  Playwright and screenshot rounds
  W5  fmt, clippy, tests, OpenAPI, task log
```

## W0 — Spec

This directory. Done when [15-cross-ref](15-cross-ref.md) links every law.

## W1 — Gate

New `edgequake-api/src/services/model_gate.rs`.

- `resolve(state, workspace, usage, request_provider, request_model)` returns a client or `GateError`.
- Callers in [02-surfaces](02-surfaces.md) switch to it.
- `ensure_deployment_choice` refuses an empty offer.
- `refuse_unoffered` is no longer the HTTP decision; the gate's 422/503 is.
- Delete unused `*_with_auth_fallback` if no caller remains.
- Tests: `edgequake-api/tests/spec179_model_gate.rs` for the rows marked Rust in the matrix.
- `constructor_guard` allow-lists boot, the gate, and probe.

`apply_llm_config_update` stays a string writer. Rebuild handlers call
`ensure_workspace_choice` first.

## W2 — Lifecycle and database

- `224_spec179_provider_refresh.sql`, manifest entry, `compat_serve_max = 224`.
- Connection create order, slug check, userinfo rejection, probe reset, fleet scope on delete and test.
- `env_connections` grows mistral, azure, xai, openrouter, gemini, and any other `models.toml` provider whose key env is set. The map lives next to `env_connections` as data, not as a copied function.
- `POST /deployment/providers/{slug}/refresh`, impact GET, confirm on delete and withdraw.
- `If-Match` on offer and policy PUT.
- Audit insert with hashes, no secrets.
- `make codegen-openapi-refresh`.

## W3 — Client

[11-frontend-architecture](11-frontend-architecture.md). Screens follow
[10-screens](10-screens.md). i18n keys. `useDirtyGuard`. 390px stacked matrix.

## W4 — Browser

`edgequake_webui/e2e/spec179/` on the `mock-api` project. `shot()` writes
`specs/179-provider-and-model-choice-perfect/e2e/screenshots/round-NN/`.
Inspect every PNG. Fix and recapture while a row is clipped, a state is
missing, or contrast fails.

## W5 — Gates

`cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings` on the
crates this pack touches, the new tests, Playwright, web unit tests,
`.env.example` if a variable was added (none is planned), task log under
`/logs`.

## Order inside W1

```text
  model_gate.rs with unit tests on the verdict
        |
        v
  query, chat, MCP            (already close; switch the helper)
        |
        v
  ingest, vision, reprocess, rebuild, parse, keyword, embedding
        |
        v
  constructor guard
```

A caller moves only when its test is in the same change.
