# 11 — Edge cases

Parent: [README](README.md) · Prev: [10-frontend-architecture](10-frontend-architecture.md) ·
Next: [12-implementation-plan](12-implementation-plan.md)

Each case has one gate in [13-e2e-test-matrix](13-e2e-test-matrix.md).
[LAW-177-10](01-first-principles.md).

| ID | Case | Law |
|----|------|-----|
| EC-177-01 | An env provider that is not withdrawn is in the tenant offer while the tenant is unconstrained. | LAW-177-1 |
| EC-177-02 | A withdrawn env provider is absent from the tenant payload while the process key is still set. | LAW-177-1 |
| EC-177-03 | A fleet connection with `last_test_ok` not true is on the deployment list as not offered and absent from the tenant payload. | LAW-177-3 |
| EC-177-04 | An unconstrained tenant sees a provider added to the global offer without a new grant row. | LAW-177-4 |
| EC-177-05 | A constrained tenant does not see that new provider until a save enables the slug. | LAW-177-4 |
| EC-177-06 | Deleting a fleet connection removes offer rows for the slug. The next resolve of a role that still names it is `not_eligible`. `ProviderFactory::from_env` is not called. | LAW-177-5 |
| EC-177-07 | Workspace PUT with a slug outside the tenant offer is `422 provider_not_in_parent`. The row is not inserted. | LAW-177-5 |
| EC-177-08 | Tenant B's connections and offer omit Tenant A's legacy row. | LAW-177-3 |
| EC-177-09 | Disabling every model of a provider leaves the provider in the offer. The usage save that would empty the eligible set is refused. | LAW-177-2 |
| EC-177-10 | An operator lock names a withdrawn provider. The lock still resolves when the process credential exists. The picker for that usage stays closed. | LAW-177-8 |
| EC-177-11 | A deployment with no env provider and no fleet connection returns an empty tenant provider list. `models.toml` cards are not added to fill it. | LAW-177-1 |
| EC-177-12 | `POST /connections` with a resolved `X-Tenant-ID` is `422 connection_scope_global` and inserts nothing. | LAW-177-3 |
| EC-177-13 | `GET /models/eligible` omits a provider outside the workspace offer. The chip has no row for it. | LAW-177-6 |
| EC-177-14 | A workspace admin receives 403 on the tenant offer and does not see Organization in the nav. | LAW-177-7 |
| EC-177-15 | Withdraw writes `deployment_provider_overrides` and does not change the process environment. | LAW-177-8 |
| EC-177-16 | The first tenant save sets `provider_offer_constrained`. The dialog uses the first-save sentence. Cancel does not write. | LAW-177-4 |
| EC-177-17 | Enabling a provider does not mark an embedding model as a chat model. The capability predicate still drops it from extract. | LAW-177-2 |
| EC-177-18 | Tenant PUT with a slug outside the global offer is `422 provider_not_in_parent`. | LAW-177-1 |
| EC-177-19 | A second tenant offer row for the same slug violates `provider_offers_tenant_uq`. | LAW-177-9 |
| EC-177-20 | A workspace row for a slug the constrained tenant omitted is rejected on write and ignored on read. | LAW-177-1 |
| EC-177-21 | A model-policy row whose provider is outside the offer is `422 provider_not_in_parent`. | LAW-177-6 |
| EC-177-22 | Two unconstrained tenants both see the same fleet slug and neither sees the other's legacy slug. | LAW-177-4 |
| EC-177-23 | Offer GET and deployment provider GET contain no `api_key`, ciphertext, or `secret_ref`. | LAW-177-3 |
| EC-177-24 | A key rotation whose test passes leaves the slug offered. A rotation whose test fails sets `last_test_ok` false and removes the slug from the global offer until a later test passes. | LAW-177-3 |
| EC-177-25 | Unknown API shape and SSRF rejection stay the SPEC-163 and SPEC-176 responses. This pack adds no new acceptance for those inputs. | LAW-177-3 |
| EC-177-26 | `provider_offers` and `deployment_provider_overrides` have no secret column. Infisical versus database remains on the connection's `secret_ref`. | LAW-177-9 |
| EC-177-27 | Effective config for a locked usage reports `source = operator_lock`, `locked = true`, and `broken_reason = none` when the credential exists, even if the tenant offer omitted the slug. | LAW-177-8 |
| EC-177-28 | Withdrawing a slug removes its chat, vision, and embedding models together. | LAW-177-2 |
| EC-177-29 | Organization → People renders members and does not render the ceiling card. | LAW-177-7 |
| EC-177-30 | Search for "provider" as a workspace admin returns Workspace → Models and does not return Organization → Providers. | LAW-177-7 |
| EC-177-31 | An unconstrained workspace offer equals the tenant offer, not the global offer. A slug the tenant disabled is absent. | LAW-177-4 |
| EC-177-32 | After the tenant drops a slug, a workspace save that still sends it is `422 provider_not_in_parent`. | LAW-177-5 |
| EC-177-33 | The owning tenant's connections payload marks the old row `legacy: true`. New role editors do not list it. Other tenants do not receive it. | LAW-177-3 |
| EC-177-34 | A tenant created after a fleet provider exists is unconstrained and therefore sees that provider. | LAW-177-4 |
| EC-177-35 | The editor omits model ticks of a provider whose checkbox is off. The PUT does not enable those models. | LAW-177-2 |
| EC-177-36 | A Super Admin who is not Platform Admin receives 403 on `POST /connections`. | LAW-177-3 |
| EC-177-37 | Organization → Providers has no key input. | LAW-177-3 |
| EC-177-38 | A workspace admin PUT that adds a slug the tenant did not offer is 422. The nav has no Organization item. | LAW-177-7 |
| EC-177-39 | An extract pin resolves with no `provider_offers` rows and no policy rows. `source` is `operator_lock`. | LAW-177-8 |
| EC-177-40 | `GET /tenants/{id}/model-catalog` omits providers outside the global offer. | LAW-177-6 |
| EC-177-41 | The web UI does not import `models.toml`. The offer page renders the endpoint payload. | LAW-177-6 |
| EC-177-42 | Deleting or withdrawing a provider does not enqueue a reprocess. Lineage columns stay. | LAW-177-5 |
| EC-177-43 | An offer change does not `DELETE` from `llm_cache` or `decision_cache`. | LAW-177-9 |
| EC-177-44 | A slug that is empty, longer than 64, or not lowercase `a-z`, digits, and hyphens is `422 slug_invalid`. A custom slug does not inherit another provider's catalog cards. | LAW-177-1 |
| EC-177-45 | PUT replaces the enabled set. A slug omitted from `slugs` is not kept from the previous row. | LAW-177-9 |
| EC-177-46 | A withdrawn provider remains on `GET /deployment/providers` with `withdrawn: true` and `offered: false`. | LAW-177-1 |
| EC-177-47 | The connections create mutation does not send `X-Tenant-ID`. A header that still arrives is EC-177-12. | LAW-177-3 |

## Mitigations that are the law, not a workaround

- Upgrade does not darken a tenant. The new flag defaults false, so the tenant keeps the global offer. The shrink is the loss of catalog cards that have no credential. [EC-177-11](11-edge-cases.md) is that shrink, and it is intended.
- "Offer nothing" is the flag true and an empty slug list. It is not the same as unconstrained. [EC-177-16](11-edge-cases.md).
- A lock and a grant disagree in one direction only. The lock wins. The grant wins for every usage that is not locked. [EC-177-10](11-edge-cases.md).
- Legacy BYOK is not migrated and not shared. [EC-177-33](11-edge-cases.md).
