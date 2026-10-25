# Lens — Full stack

Parent: [README](../README.md) · Plan: [12-implementation-plan](../12-implementation-plan.md)

## What this lens owns

[LAW-177-5](../01-first-principles.md), [LAW-177-6](../01-first-principles.md),
and [LAW-177-9](../01-first-principles.md). One offer function, one
eligibility function, one connection writer, one editor. The HTTP status
in [08-api-contract](../08-api-contract.md) is the same fact the worker
uses when a job is refused.

## Where the code moves

| Wave | Crate or app | Add | Leave alone |
|------|--------------|-----|-------------|
| W1 | `edgequake-core` | `provider_offer` | `resolve_usage` body |
| W2 | `edgequake-api` | handlers, `provider_offer_rows.rs`, migration 223 | `SecretService` |
| W3 | `edgequake-api` | pass the offer into `eligible_models` and effective config | a second ladder |
| W4 | `edgequake_webui` | `ProviderOfferEditor`, two registry sections | a second settings shell |
| W5 | `edgequake_webui` | legacy block | a key field on that block |

`create_connection` stays the create path. It gains a guard when
`scoped_tenant` is `Some`. It does not grow a sibling `create_fleet_connection`.

## Contracts that must match

- `broken_reason = not_eligible` on effective config and on the ingest
  or query refusal. [EC-177-06](../11-edge-cases.md).
- `422 provider_not_in_parent` on admin writes.
  `422 model_not_eligible` stays the member request code.
  [08-api-contract](../08-api-contract.md).
- The React draft is not sent as `offer_mode` unless the user saved.
  Cancel on the confirm dialog writes nothing. [EC-177-16](../11-edge-cases.md).

## Out of this lens

Nav labels, badge color, and the confirm sentence wording. Those are the
UX lens. The column types are the database lens.
