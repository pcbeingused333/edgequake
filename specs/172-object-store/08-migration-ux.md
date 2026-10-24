# 08 — Migration UX

Parent: [README](README.md) · Prev: [07 E2E test matrix](07-e2e-test-matrix.md) · Next: [09 Cross-ref](09-cross-ref.md)

The tool is `edgequake originals`. It does not open the HTTP listener. It uses a `BYPASSRLS` maintenance role and the same `EDGEQUAKE_ORIGINAL_STORE_*` variables as the server. The API role is not that role. Laws: [LAW-172-4](01-first-principles.md), [LAW-172-6](01-first-principles.md).

## Verbs

| Verb | Writes | When to use it |
|------|--------|----------------|
| `plan` | no | Default. Counts rows, bytes, and what clear would remove. |
| `copy` | objects and refs | PUT each `BYTEA` that has no verified ref. |
| `verify` | `verified_at` | Hash every ref against the object. |
| `migrate` | copy + verify | The usual command. Does not clear. |
| `clear` | nulls `BYTEA` | Only verified rows, only when mode is `object`. |
| `restore` | writes `BYTEA` from objects | Leave object mode. |
| `status` | no | Counts by state. |

Unknown verbs print a usage block and exit `usage`, the same way [`MIGRATE_USAGE`](../../edgequake/src/main.rs) does for `migrate`.

`plan` is what you get if you pass no verb: `edgequake originals` equals `edgequake originals plan`.

## Flags

| Flag | Default | Effect |
|------|---------|--------|
| `--concurrency` | 4 | Parallel owners. Each owner is one task. |
| `--limit` | none | Stop after N owners. For a first look at a large database. |
| `--kind` | both | `pdf` or `original`. |
| `--workspace` | all | One workspace UUID. |
| `--clear` | off | Only on `migrate`. Runs clear after verify if verify's failed count is 0. |

There is no `--yes` on `plan`. `clear` and `migrate --clear` require `--yes` because they are the step that drops the database copy.

## What the operator sees

```text
edgequake originals plan

originals  database=postgres://localhost/edgequake  store=s3://eq-originals/prod
mode       object
role       edgequake_originals_maint
pdf        1200 rows   4.2 GiB   already verified 0
original   80 rows     310 MiB   already verified 0
relations  pdf_document_blobs 4.4 GiB   document_originals 320 MiB
space      VACUUM FULL needs about 4.7 GiB free after clear
blocked    0
next       edgequake originals migrate
```

```text
edgequake originals migrate --concurrency 4

copy    1280/1280  verified 1280  skipped 0  failed 0
bytes   4.5 GiB
time    3m 10s
kept    BYTEA still in pdf_document_blobs and document_originals
next    edgequake originals clear --yes
note    clear does not shrink the disk. After clear, run VACUUM FULL
        or pg_repack on pdf_document_blobs and document_originals.
```

```text
edgequake originals clear --yes

cleared pdf 1200  original 80  bytes 4.5 GiB
disk    unchanged until VACUUM FULL or pg_repack
        tables: pdf_document_blobs, document_originals
        free space required: about 4.7 GiB (the rewrite is a second copy)
next    psql -c "VACUUM (FULL, ANALYZE) pdf_document_blobs, document_originals"
```

Progress is a single updating line in a terminal, and one line per 100 owners when stdout is not a terminal, so logs stay readable. Secrets are not printed. The URL is printed with the userinfo stripped.

## Exit classes

Modeled on `MigrateAbortClass` in [`migrate_console.rs`](../../edgequake/src/migrate_console.rs). The class is the last line: `originals_abort=<class>`.

| Class | Meaning | Next command |
|-------|---------|--------------|
| `usage` | Unknown verb or flag | the usage block |
| `config` | Mode, URL, path-style, or allow-http is wrong | fix env, `originals plan` |
| `credentials` | The store rejected the key mid-run | refresh the key, `originals copy` |
| `checksum` | A hash mismatched | do not clear; inspect the owner id printed above |
| `mode` | `clear` while `EDGEQUAKE_ORIGINAL_STORE` is not `object` | restart the server in object mode, then clear |
| `blocked` | Clear saw unverified refs | `originals verify` |
| `database` | Postgres error, including lock timeout | retry `originals copy` |
| `empty` | Nothing to do | none |

Exit code 0 is `empty` or a finished verb with failed = 0. Any failed owner is exit code 1. `usage` is exit code 2.

## Failure transcript

```text
edgequake originals copy

copy    40/1280  verified 39  skipped 0  failed 1
failed  kind=pdf owner=8f1c... sha256 source=ab12… object=ab13…
originals_abort=checksum
next    fix or delete that object, then: edgequake originals copy
kept    BYTEA for 8f1c… was not cleared
```

The owner id is enough to find the row. The filename is printed only as metadata, never as a path to retry.

## States an operator can explain

```text
┌────────────┐   copy    ┌────────────┐   verify   ┌────────────┐
│ BYTEA only │ ------->  │ ref + BYTEA│ -------->  │ verified   │
└────────────┘           └────────────┘            └─────┬──────┘
                                                        │ clear
                                                        v
                                                  ┌────────────┐
                                                  │ ref only   │
                                                  └────────────┘
```

`plan` counts each box. `status` prints the same counts without estimating a copy.

## What this tool will not do

- It will not `VACUUM FULL` by itself. That lock is an operator decision.
- It will not delete the bucket.
- It will not move `documents.content` or `document_mm_assets`.
- It will not start if `EDGEQUAKE_ORIGINAL_STORE_URL` is missing when the verb writes to the store. `plan` in `database` mode still runs and tells the operator the server is not in object mode, so clear would be refused ([EC-172-15](05-edge-cases.md)).
