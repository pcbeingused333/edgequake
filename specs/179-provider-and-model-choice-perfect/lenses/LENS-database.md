# Lens — Database

Parent: [15-cross-ref](../15-cross-ref.md)

## WHY this lens

Refresh, revision, and audit are new facts. Everything else already has a
table. A second policy table would split the source of truth this pack is
trying to finish.

## Contract

[07-data-and-db-contract](../07-data-and-db-contract.md). Expand-only.
`compat_serve_max` moves to 224. No `DROP`. No secret column on the new
tables. A column-name test locks that.

## Integrity

- Secret write happens after the connection insert, and the insert is
  deleted if the secret write fails. The two are not one transaction across
  Infisical, so the compensation is explicit.
- `last_test_ok` is cleared in the same `UPDATE` as `base_url` or the key
  change.
- Revision increment is in the same transaction as the offer or policy
  replace.
- Fleet uniqueness for `tenant_id IS NULL` stays an application check.
  This pack adds the slug parse and the env-slug collision check. It does
  not pretend a UNIQUE index can include NULL the way we need without a
  partial unique index. Add `provider_connections_fleet_slug_uq` on
  `(slug) WHERE tenant_id IS NULL` in migration 224 so the race goes away.

## Isolation

New tables are global (slug-keyed) or append-only audit. They do not hold
tenant documents. RLS is not added to them. Queries that read tenant
metadata for impact run as the admin, and the route is Platform Admin or
the scope's manage role. The impact query filters `tenant_id`.

## What this lens rejected

A history table of every model assignment. The audit hash is enough to see
who changed a policy. The rows themselves are the current truth.
