# Lens — Database

Parent: [README](../README.md) · Contract: [07-data-and-db-contract](../07-data-and-db-contract.md)

## What this lens owns

[LAW-178-4](../01-first-principles.md). Choice is a function of rows
that already have a writer. This pack adds no table, no column, no
index, no RLS policy, and no migration. Head stays 223.
[EC-178-41](../11-edge-cases.md).

## Decisions this lens will not reopen

- No eligible-set cache table. A cache row would diverge from
  `eligible_models` the moment a ceiling write commits.
- No new JSON key for "the model the user picked at query time." That
  pair lives in the browser, keyed by workspace id, and the server
  re-checks it.
- `workspaces.settings` and `tenants.settings` stay empty of model
  policy. Assignments stay on the metadata keys the readers already use.
- `model_catalog_policy` stays the only tick table. A choice save does
  not insert a tick to remember a selection.
- Embedding dimension stays the integer already stored with the model.
  The card is the source on write. Vector tables and the rebuild route
  are unchanged.
- Deletes keep the SPEC-177 behavior: offer rows and `connection_models`
  go away, `llm_roles` stays, the next run reports `not_eligible`.

## Concurrency

A ceiling write and an assignment write are two transactions. This pack
does not wrap them. The run after both commits is the authority.
[EC-178-28](../11-edge-cases.md).

## Grants

Workspace read stays member. Tenant-scope eligible stays tenant Manage
or workspace-create. Deployment-scope eligible stays `Role::Admin`.
The checks live in the handlers that already guard those routes. No
policy is added to migration 221's RLS.

## What this lens will not let the others break

- A "small" column on `workspaces` for the chip selection.
- Writing the resolved pair back into metadata when the stored pair is
  broken. That is a second writer for the assignment.
- Guessing dimension 1536 inside the UPDATE. The refusal happens before
  the write. [EC-178-14](../11-edge-cases.md).
- Treating "zero policy rows" as "no filter." The offer is still a
  predicate. [EC-178-09](../11-edge-cases.md).

## Out of this lens

Picker layout and the order of the resolution ladder. The ladder reads
these rows. It does not get a new one.
