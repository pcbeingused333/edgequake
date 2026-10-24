# Lens — Full Stack

Parent: [README](../README.md) · Plan: [13-implementation-plan](../13-implementation-plan.md) ·
Surfaces: [02-surfaces](../02-surfaces.md)

## What this lens owns

[LAW-174-5](../01-first-principles.md) and [LAW-174-15](../01-first-principles.md).
One PDP (`allows` in `edgequake-core`). One PEP input (`VisibilityScope`
on the request). One test name per edge case.

## Request order

Authentication, then `require_request_permission`, then compile, then
`SET LOCAL` GUCs, then the handler. Readonly is rejected at step two even
if step three would allow the label ([EC-174-31](../12-edge-cases.md)).

## Insertion points (do not invent new ones)

| Need | Existing symbol to extend |
|------|---------------------------|
| Who is asking | `bind_request` |
| Which documents | `VisibilityScope` consumed by `relational_workspace_doc_ids` and `execute_query` |
| Query modes | `allowed_document_ids` already threaded through five modes |
| SQL backstop | `with_rls_transaction` / `set_tenant_context` |
| MCP | `mcp/gateway/dispatch.rs`, same extension |

`resolve_document_filter` stays the client-filter parser. It loses the
right to return all-pass when enforcement is on. The handler intersects.

## DRY failures to reject in review

- A handler that SQL-loads `security_role_rules` itself.
- A new `Permission::DocumentReadHr` variant.
- A React function that reimplements subset-of-compartments.
- A second cache of visible ids that ignores `policy_epoch`.

## Wave discipline

W1 has no database. W2 does not flip the flag. W5 does not ship graph
fragments. W6 does not change the answer-cache key. The matrix in
[14-e2e-test-matrix](../14-e2e-test-matrix.md) is the checklist.
