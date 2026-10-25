# 12 — Implementation plan

Parent: [README](README.md) · Prev: [11-edge-cases](11-edge-cases.md) ·
Next: [13-e2e-test-matrix](13-e2e-test-matrix.md)

W0 is this pack. W1–W5 are product work. Each wave leaves `resolve_usage`
as the only function that picks a pair, and leaves `upsert` as the only
function that writes a connection.

## W0 — Specification

Documents in this directory. No migration, no UI.

Exit: [14-cross-ref](14-cross-ref.md) cites every law, finding, and EC.

## W1 — Offer algebra, no SQL

Crate: `edgequake-core`. New module `provider_offer` next to `model_policy`.

Pure functions:

| Function | Returns |
|----------|---------|
| `global_offer` | slugs after allowlist, withdrawal, and `last_test_ok` |
| `child_offer` | parent slugs, or the intersection with enabled slugs when constrained |
| `filter_catalog` | catalog entries whose provider is in the child offer |

`eligible_models` takes the offered slugs and calls `filter_catalog`
before the ceiling. Existing callers pass `None` meaning "no offer filter"
only inside tests that have not moved. Production call sites in W3 pass
the set. W1 adds the parameter with the filter defaulting to "keep all"
so W1 does not change runtime. The default is deleted in W3. A comment
on the parameter says so, so the default cannot become a second policy.

Unit tests cover EC-177-01, 02, 04, 05, 09, 11, 17, 20, 28, 31, 44
(the alias half: slug `acme` does not match provider `openai`).

No `sqlx`. No Axum.

Exit: `cargo test -p edgequake-core --lib provider_offer`.

## W2 — Migration 223 and HTTP

- Add `edgequake/migrations/223_spec177_provider_offers.sql` as specified in [07-data-and-db-contract](07-data-and-db-contract.md).
- Storage module `provider_offer_rows.rs` beside `model_policy_rows.rs`. Replace-rows is one transaction: set the boolean, delete the scope's rows, insert the enabled slugs.
- Handlers and utoipa paths from [08-api-contract](08-api-contract.md).
- `create_connection` returns `connection_scope_global` when `scoped_tenant` is `Some`. Update of a fleet row with a tenant header is the same 422. Update of a legacy row stays scoped to that tenant so the owner can still delete it.
- Connection delete deletes `provider_offers` for that slug in the same transaction.
- OpenAPI refresh follows the repo's codegen target. The contract test is `edgequake-api --test spec177_provider_offers`.

Tests: EC-177-03, 06, 07, 08, 12, 15, 18, 19, 21, 22, 23, 24, 26, 32, 33, 34, 36, 40, 45, 46.

`spec177` uses the same postgres test harness as `spec176_secrets` and
`e2e_spec175_model_policy`. It does not boot a live provider. Probe flags
are written as columns.

Exit: `cargo test -p edgequake-api --test spec177_provider_offers`.

## W3 — The run

- `load_catalog` / tenant catalog / eligible models pass `child_offer` into `eligible_models`.
- Remove the W1 "keep all" default at production call sites.
- `effective_usages` sets `not_eligible` when the pair's provider is outside the workspace offer, unless `source` is `operator_lock` and the credential exists.
- Ingest and query use that verdict. They do not add a fallback client.
- Legacy slug: if the role names a legacy connection of this tenant, resolution still loads that connection. The slug is not added to the offer list. New writes cannot select it (the role editor's options come from eligible models, which exclude it).

Tests: EC-177-06 (runtime half), 10, 13, 27, 39, 42, 43.

EC-177-42 and EC-177-43 are assertions that the delete/withdraw handler
does not call the reprocess service and does not issue a cache delete.
They live next to the handler test, not in a browser.

Exit: `cargo test -p edgequake-api --test spec177_provider_offers` and the
core tests still pass.

## W4 — One editor

- `src/lib/settings/provider-offer.ts` and its unit test.
- `use-provider-offer.ts`.
- `ProviderOfferEditor`.
- Registry sections `providers` and `org-providers`. People drops the ceiling card.
- Workspace models page mounts the editor above the role editor.
- Connections card: Withdraw, offered badge, create without `X-Tenant-ID`.
- Playwright `edgequake_webui/e2e/spec177` on project `mock-api`.

Tests: EC-177-14, 16, 29, 30, 35, 37, 38, 41, 47, plus the chip half of
EC-177-13.

The mock API returns the offer payload. The spec does not start Ollama.

Exit: `cd edgequake_webui && npx playwright test --project=mock-api e2e/spec177`.

## W5 — Legacy panel

- Connections JSON `legacy` flag rendered on Organization → Providers.
- Copy from [03-product-spec](03-product-spec.md).
- No enable checkbox. No key field.

Test: the UI half of EC-177-33.

## Dependency order

```text
  W0 spec
   |
   v
  W1 pure functions
   |
   v
  W2 tables and HTTP ----+
   |                     |
   v                     v
  W3 resolve_usage      W4 editor
                         |
                         v
                        W5 legacy panel
```

W4 can start against the W2 endpoints with fixture JSON before W3 merges.
The chip assertion in W4 waits until W3, because the eligible route is
what the chip reads. Split the Playwright file if the branches land apart:
`offer-admin.spec.ts` in W4, `offer-chip.spec.ts` after W3.

## DRY rules for the implementer

- Do not copy `within_ceiling` into the offer module. Call it after the slug filter.
- Do not add `provider_offers` columns to `model_catalog_policy`.
- Do not store the offer in `tenants.settings` or workspace metadata JSON.
- Do not build a second `resolve_usage`.
- Do not let the React draft become the source of `offer_mode`. The GET body is.

## Out of these waves

- Retiring legacy ciphertext.
- A UI for `EDGEQUAKE_ALLOWED_PROVIDERS`.
- A 24-hour probe expiry. The gate stays `last_test_ok`.
- Decision and rerank grants.
