# 14 — E2E test matrix

Parent: [README](README.md) · Prev: [13-implementation-plan](13-implementation-plan.md) ·
Next: [15-cross-ref](15-cross-ref.md)

## WHY

A case that is only described will regress. The name in the EC table is the
name in the test file. Playwright covers what a person sees. Rust covers
what a worker does, because the worker has no browser.

| EC | Kind | File | Name |
|----|------|------|------|
| EC-179-01 | Rust | `edgequake-api/tests/spec179_model_gate.rs` | `ingest_withdrawn_fails` |
| EC-179-02 | Rust | same | `rebuild_pair_rejected` |
| EC-179-03 | Rust | same | `reprocess_uses_vlm_role` |
| EC-179-04 | Rust | same | `keyword_does_not_swap` |
| EC-179-05 | Rust | same | `fleet_slug_resolves` |
| EC-179-06 | Rust | same | `env_connections_native` |
| EC-179-07 | Rust | `edgequake-api/tests/spec179_provider_lifecycle.rs` | `refresh_keeps_cache_on_error` |
| EC-179-08 | Rust | same | `embedding_dimension_required` |
| EC-179-09 | Rust | same | `duplicate_slug_no_orphan_secret` |
| EC-179-10 | Rust | same | `edit_clears_probe` |
| EC-179-11 | Rust | same | `url_userinfo_rejected` |
| EC-179-12 | Rust | `spec179_model_gate.rs` | `empty_offer_refuses_defaults` |
| EC-179-13 | Rust | `spec179_provider_lifecycle.rs` | `stale_revision` |
| EC-179-14 | Rust | same | `withdraw_requires_confirm` |
| EC-179-15 | Playwright | `e2e/spec179/choice.spec.ts` | `choice-does-not-search-inventory` |
| EC-179-16 | Playwright | `e2e/spec179/wizard.spec.ts` | `wizard-deployment-scope` |
| EC-179-17 | Playwright | `e2e/spec179/providers.spec.ts` | `offer-error-state` |
| EC-179-18 | Rust | `spec179_provider_lifecycle.rs` | `tenant_cannot_delete_fleet` |
| EC-179-19 | Rust | `spec179_model_gate.rs` | `constructor_guard` |
| EC-179-20 | Rust | same | `ingest_rejudge` |
| EC-179-21 | Playwright | `e2e/spec179/models.spec.ts` | `dimension-warned` |
| EC-179-22 | Rust | `spec179_provider_lifecycle.rs` | `refresh_id_collision` |
| EC-179-23 | Rust | same | `slug_rename_refused` |
| EC-179-24 | Rust | `spec179_model_gate.rs` | `deleted_connection_fails_closed` |
| EC-179-25 | Rust | same | `cross_tenant_offer` |
| EC-179-26 | Rust | `spec179_provider_lifecycle.rs` | `secret_down_fails_closed` |
| EC-179-27 | Rust | same | `bad_master_key` |
| EC-179-28 | Playwright | `e2e/spec179/choice.spec.ts` | `chip-slot-per-workspace` |
| EC-179-29 | Playwright | `e2e/spec179/models.spec.ts` | `viewer-readonly` |
| EC-179-30 | Rust | `spec179_model_gate.rs` | `operator_lock_bypasses_offer` |
| EC-179-31 | Rust | same | `operator_lock_refuses_request` |
| EC-179-32 | Rust | `spec179_provider_lifecycle.rs` | `refresh_timeout` |
| EC-179-33 | Rust | same | `stale_revision` (policy arm) |
| EC-179-34 | Rust | `spec179_model_gate.rs` | `mcp_judges_stored` |
| EC-179-35 | Rust | same | `parse_uses_gate` |

Playwright uses the `mock-api` project and the spec175 helpers. It stubs
`/models/eligible` and `/models/search` with different ids. A menu that
still searches fails EC-179-15.

Screenshots from `e2e/spec179/screens.spec.ts` land in
`specs/179-provider-and-model-choice-perfect/e2e/screenshots/`. Required
frames: connections, providers, org offer, workspace models, query chip,
upload vision, each at light 1440, dark 1440, and light 390.

Rust tests that need Postgres skip when `DATABASE_URL` is unset, matching
`spec177_provider_offers.rs`. Pure tests (`env_connections_native`,
`constructor_guard`, `url_userinfo_rejected`, `operator_lock_*`) always run.

## As built

Hermetic Playwright (`mock-api`, tag `@spec179`) in `edgequake_webui/e2e/spec179/`:

| File | Tests | Covers |
|------|-------|--------|
| `providers.spec.ts` | 4 | org and workspace offer editors, fail-closed error row |
| `deployment-providers.spec.ts` | 7 | provider cards, refresh diff, confirm-withdraw, fleet models, embedding dimension, failed probe |
| `connections.spec.ts` | 4 | saved connections, stored-in badge, delete with confirm |
| `offer-editor.spec.ts` | 3 | tenant and workspace grant copy, stale revision, retry |
| `wizard.spec.ts` | 7 | Create Workspace: stale default blocked, "Choose another", empty offer (admin link vs ask-admin), missing dimension, refused create returns to the models step |

Vitest (pure): `lib/settings/__tests__/provider-status.test.ts`,
`lib/onboarding/__tests__/defaults-eligibility.test.ts`, `wizard-picks.test.ts`,
`lib/api/edgequake/__tests__/eligible-path.test.ts`.

Rust: `spec179_model_gate.rs` (6), `spec179_provider_lifecycle.rs` (8).
Screenshots: `e2e/screenshots/round-01` to `round-09` (light 1440, dark 1440,
light 390). Round 08 holds the wizard frames.

Not yet wired to the eligibility hook: the First-run and Create Tenant wizards.
