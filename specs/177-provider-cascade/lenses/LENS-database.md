# Lens — Database

Parent: [README](../README.md) · Contract: [07-data-and-db-contract](../07-data-and-db-contract.md)

## What this lens owns

[LAW-177-4](../01-first-principles.md) and [LAW-177-9](../01-first-principles.md).
The mode is a boolean because an empty `provider_offers` set cannot mean
both "all of the parent" and "nothing." The slug list is rows. The secret
stays on `provider_connections.secret_ref` and `managed_secrets`.

## Tables migration 223 adds

| Table or column | Why it is not a column on the other table |
|-----------------|-------------------------------------------|
| `deployment_provider_overrides` | Env providers have no `provider_connections` row to hang `withdrawn` on. Fleet rows use the same table so withdrawal has one reader. |
| `provider_offers` | Visibility is not a model tick and not a secret. |
| `tenants.provider_offer_constrained` | Distinguishes unconstrained from "constrained and empty." |
| `workspaces.provider_offer_constrained` | Same, one level down. |
| `connection_models` | A custom slug's models are not `models.toml` cards. FK to the connection deletes them with the connection. |

Partial unique indexes follow migration 221. A single `UNIQUE` that
includes `workspace_id` would allow two ceiling rows because nulls are
distinct. [EC-177-19](../11-edge-cases.md).

The composite foreign key `(tenant_id, workspace_id)` uses
`workspaces_tenant_id_workspace_id_uq` from migration 221. A workspace
id from another tenant cannot be inserted. [EC-177-08](../11-edge-cases.md)
is the legacy case; the FK is the offer case.

## What this migration does not do

- No backfill of offers. The boolean defaults false.
- No `UPDATE` of `tenant_id` on existing connections.
- No secret column, no probe column, no model id on `provider_offers`.
- No RLS policy. Callers are checked in the handlers, as SPEC-175 did for
  `model_catalog_policy`.
- No cache delete. [EC-177-43](../11-edge-cases.md).

## Replace transaction

One transaction: update the boolean, delete the scope's rows, insert the
enabled slugs. A failed insert rolls the boolean back. Two concurrent
PUTs: the later commit wins the whole set. There is no per-slug merge.
[EC-177-45](../11-edge-cases.md).

Connection delete uses the same transaction as the `secret_ref` delete
and the `DELETE FROM provider_offers WHERE provider_slug = $1`. The slug
is not a foreign key, because an env slug has no connection. The
application issues that delete. `connection_models` cascade through the
connection FK.

## Out of this lens

Which sentence the dialog shows, and which usage a model supports.
