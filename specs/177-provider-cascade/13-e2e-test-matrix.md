# 13 — E2E and test matrix

Parent: [README](README.md) · Prev: [12-implementation-plan](12-implementation-plan.md) ·
Next: [14-cross-ref](14-cross-ref.md)

One row per [EC-177](11-edge-cases.md). The command is the gate. A case
is done when that command fails if the case regresses.

| EC | Wave | Gate |
|----|------|------|
| 01 | W1 | `provider_offer::tests::env_slug_is_in_the_global_offer` |
| 02 | W1 | `provider_offer::tests::withdrawn_env_slug_is_omitted` |
| 03 | W2 | `spec177_provider_offers::failed_probe_is_not_offered` |
| 04 | W1 | `provider_offer::tests::unconstrained_child_sees_new_parent_slug` |
| 05 | W1 | `provider_offer::tests::constrained_child_hides_new_parent_slug` |
| 06 | W2, W3 | `spec177_provider_offers::delete_connection_removes_offers_and_refuses_role` |
| 07 | W2 | `spec177_provider_offers::workspace_slug_outside_tenant_is_422` |
| 08 | W2 | `spec177_provider_offers::other_tenant_legacy_is_hidden` |
| 09 | W1 | `provider_offer::tests::provider_stays_when_models_are_empty` plus existing `usages_left_empty` |
| 10 | W3 | `spec177_provider_offers::lock_ignores_withdrawn_grant` |
| 11 | W1 | `provider_offer::tests::empty_global_offer_drops_catalog_cards` |
| 12 | W2 | `spec177_provider_offers::tenant_header_on_create_is_422` |
| 13 | W3, W4 | `spec177_provider_offers::eligible_omits_slug` and `e2e/spec177/chip.spec.ts` |
| 14 | W4 | `e2e/spec177/nav.spec.ts` workspace admin |
| 15 | W2 | `spec177_provider_offers::withdraw_does_not_touch_env` |
| 16 | W4 | `e2e/spec177/tenant-confirm.spec.ts` |
| 17 | W1 | `provider_offer::tests::embedding_card_fails_chat_predicate` |
| 18 | W2 | `spec177_provider_offers::tenant_slug_outside_global_is_422` |
| 19 | W2 | `spec177_provider_offers::duplicate_tenant_slug_rejected` |
| 20 | W1 | `provider_offer::tests::workspace_cannot_widen_tenant` |
| 21 | W2 | `spec177_provider_offers::model_policy_outside_offer_is_422` |
| 22 | W2 | `spec177_provider_offers::two_tenants_share_fleet_not_legacy` |
| 23 | W2 | `spec177_provider_offers::offer_json_has_no_secret` |
| 24 | W2 | `spec177_provider_offers::failed_rotation_leaves_the_offer` |
| 25 | — | Covered by `connection_shapes` and SPEC-163 SSRF tests. No new spec177 test. |
| 26 | W2 | `spec177_provider_offers::offer_tables_have_no_secret_column` |
| 27 | W3 | `spec177_provider_offers::effective_lock_stays_eligible` |
| 28 | W1 | `provider_offer::tests::withdraw_drops_embed_and_chat` |
| 29 | W4 | `e2e/spec177/people.spec.ts` |
| 30 | W4 | `registry.test.ts` provider search, and `e2e/spec177/nav.spec.ts` |
| 31 | W1 | `provider_offer::tests::workspace_unconstrained_follows_tenant` |
| 32 | W2 | `spec177_provider_offers::stale_workspace_slug_is_422` |
| 33 | W2, W5 | `spec177_provider_offers::legacy_flag_on_owner_only` and `e2e/spec177/legacy.spec.ts` |
| 34 | W2 | `spec177_provider_offers::new_tenant_is_unconstrained` |
| 35 | W4 | `provider-offer.test.ts` omits ticks, and `e2e/spec177/tenant-confirm.spec.ts` |
| 36 | W2 | `spec177_provider_offers::non_platform_admin_cannot_create` |
| 37 | W4 | `e2e/spec177/tenant-confirm.spec.ts` no key field |
| 38 | W4 | `e2e/spec177/nav.spec.ts` |
| 39 | W3 | `spec177_provider_offers::extract_pin_without_offer_rows` |
| 40 | W2 | `spec177_provider_offers::catalog_omits_unoffered_provider` |
| 41 | W4 | `e2e/spec177` fixture has no `models.toml` import; `rg models.toml edgequake_webui/src/components/settings/providers` is empty |
| 42 | W3 | `spec177_provider_offers::delete_does_not_reprocess` |
| 43 | W3 | `spec177_provider_offers::offer_write_does_not_delete_cache` |
| 44 | W2 | `spec177_provider_offers::slug_invalid` and `provider_offer::tests::custom_slug_does_not_match_openai` |
| 45 | W2 | `spec177_provider_offers::replace_drops_omitted_slug` |
| 46 | W2 | `spec177_provider_offers::withdrawn_stays_on_deployment_list` |
| 47 | W4 | `e2e/spec177/connections.spec.ts` asserts the create request has no `X-Tenant-ID` |

## Commands

```text
cargo test -p edgequake-core --lib provider_offer
cargo test -p edgequake-api --test spec177_provider_offers
cd edgequake_webui && npx playwright test --project=mock-api e2e/spec177
```

Playwright stays on `mock-api`. A live stack is not the gate for this pack.

## Fixture shape for the browser

The mock returns two providers, `openai` and `acme`. `acme` is fleet,
offered, no key in the JSON. A third object `withdrawn-env` is on the
deployment list only. The tenant unconstrained payload includes `openai`
and `acme`. The constrained payload includes `openai` only. The workspace
admin token gets 403 on `/tenants/{id}/provider-offer`.
