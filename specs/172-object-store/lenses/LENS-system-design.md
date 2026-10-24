# Lens — System design

Parent: [README](../README.md) · Laws: [LAW-172-3](../01-first-principles.md), [LAW-172-7](../01-first-principles.md)

## Two stores, one read path

Postgres remains the system of record for identity, tenancy, text, and the pointer. The bucket is the system of record for original bytes after clear. Neither store is allowed to be implied by the other: a ref without an object is a failed verify, an object without a ref is an orphan.

```text
┌────────────┐     pointer      ┌────────────┐
│ Postgres   │ ---------------> │ Bucket     │
│ who, hash  │                  │ bytes      │
└────────────┘                  └────────────┘
       ^                               │
       │     queue tables only         │
       └───────────────────────────────┘
```

The reaper reads `original_object_deletes` and expired inflight rows. A failed list is not a decision it makes, because it does not list.

## Fleet

Every replica uses the same `EDGEQUAKE_ORIGINAL_STORE`. The process cannot read its peers' env. It can see the table. So the boot rule is local and sufficient: database mode plus any ref row is fatal; object mode plus a bucket that fails the head means `/ready` 503 and `/health` 200 ([EC-172-10](../05-edge-cases.md), [EC-172-20](../05-edge-cases.md)).

New uploads in object mode do not write `BYTEA`, so they do not join the migrator's claim. That is how copy and live traffic share one database ([EC-172-14](../05-edge-cases.md)).

## Failure budget

| Event | User-visible result | Repair |
|-------|---------------------|--------|
| PUT ok, commit lost | upload returns 500, retry works | reaper after 1 hour |
| commit ok, object delete fails | delete returns success | `original_object_deletes` |
| hash mismatch | clear skips, download still serves BYTEA until clear | operator, class `checksum` |
| bucket down | ready fails, existing text search still runs if the process is up | restore the bucket; do not switch to database mode after clear |

Switching to database mode after clear without `restore` is a boot failure, not a graceful degrade. The bytes are not in Postgres anymore.

## Why the migrator is a separate process

The API binary's request timeout is the wrong clock for terabytes of `BYTEA`. The CLI can be stopped and resumed ([EC-172-28](../05-edge-cases.md)). It links the same library so the key and the hash cannot drift ([LAW-172-6](../01-first-principles.md)).

Two CLI processes are safe because of `SKIP LOCKED` ([EC-172-29](../05-edge-cases.md)). They are not required. Concurrency inside one process is the default (`--concurrency 4`).
