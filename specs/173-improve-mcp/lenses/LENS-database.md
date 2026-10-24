# Lens — Database

Parent: [README](../README.md) · Law: [LAW-173-7](../01-first-principles.md) · Plan wave W2: [07](../07-implementation-plan.md)

## Table

Migration **219** creates `mcp_upload_sessions`. Migration 173 is already `173_injection_list_index.sql` and is not this table.

The row is the upload. The temp file and the staging object are caches of bytes. A replica that can read the row can commit ([EC-173-21](../06-edge-cases.md)).

Suggested columns are listed in [07](../07-implementation-plan.md). `status` moves only from `open` to `committed`, `aborted`, or `expired`. The commit update is conditional on `status = 'open'` so two transactions cannot both admit ([EC-173-12](../06-edge-cases.md)).

`expires_at` is `timestamptz`. Comparisons use database time. Process `Instant` is how the bug in [F-173-7](../02-code-facts.md) works today.

## RLS

`ENABLE` and `FORCE` row level security, and grant `edgequake_tenant_access`, the way migration 215 does for `original_object_refs`. Do not copy that table’s predicate. It matches `workspace_id` and then looks up the tenant through `workspaces`. This table carries `tenant_id` on the row, so use the documents predicate from migration 167:

```text
tenant_id = public.current_tenant_id()
AND (public.current_workspace_id() IS NULL
     OR workspace_id = public.current_workspace_id())
```

Grant `SELECT`, `INSERT`, `UPDATE`, `DELETE` to `edgequake_tenant_access`. The API role does not get `BYPASSRLS`. A test that sets another tenant’s GUC must see zero rows ([EC-173-04](../06-edge-cases.md)).

`user_id` is a column the application checks when both sides have a value ([EC-173-25](../06-edge-cases.md)). It is not a substitute for the tenant predicate. RLS does not know the end-user id today, and this spec does not invent a new GUC for it.

## What stays out of this table

- File bytes. `BYTEA` stays on the SPEC-172 tables, written by `admit_bytes`.
- The canonical object key. `staging_key` is the only key this table stores, and the reaper deletes it. `original_object_refs` does not gain a foreign key to `upload_id`.

## Index

`(status, expires_at)` supports the reaper. The reaper’s `WHERE` is that index, not a bucket listing ([EC-173-20](../06-edge-cases.md)).
