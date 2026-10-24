# 07 — E2E test matrix

Parent: [README](README.md) · Prev: [06 Implementation plan](06-implementation-plan.md) · Next: [08 Migration UX](08-migration-ux.md)

Cases are defined in [05](05-edge-cases.md). A test may live in a unit file and still discharge an "e2e" row when the row says `InMemory`. The MinIO rows are the release gate ([06](06-implementation-plan.md) W6).

| Test | Case | Where | Store | Assert |
|------|------|-------|-------|--------|
| T-172-01 | EC-172-01 | `originals::io` plus reaper unit | InMemory | PUT, drop before ref, reaper removes the key after the hour (clock injected) |
| T-172-02 | EC-172-02 | CLI integration | InMemory | ref and BYTEA both present; download returns object bytes |
| T-172-03 | EC-172-03 | CLI | InMemory | flipped byte in the object; clear exit ≠ 0; BYTEA unchanged |
| T-172-04 | EC-172-04 | `originals` unit | none | empty slice errors; put not called |
| T-172-05 | EC-172-05 | upload unit | InMemory | 50 MiB ok; 50 MiB + 1 byte rejected with no object |
| T-172-06 | EC-172-06 | API e2e | MinIO | `Range: bytes=0-99` → 206 and the same 100 bytes |
| T-172-07 | EC-172-07 | API e2e | MinIO | start past end → 416 and empty body |
| T-172-08 | EC-172-08 | adapter unit | InMemory | delete of the object returns error; queue row remains; get_pdf is None |
| T-172-09 | EC-172-09 | API + SQL | MinIO | other workspace gets 404; RLS returns zero rows |
| T-172-10 | EC-172-10 | health unit | builder pointed at a closed port | `/health` 200, `original_store_ready` false, `/ready` 503 |
| T-172-11 | EC-172-11 | config unit | none | `http://` without allow-http is a config error |
| T-172-12 | EC-172-12 | config + MinIO | MinIO | path-style false fails the boot head; default true succeeds |
| T-172-13 | EC-172-13 | CLI | InMemory that fails on the second put | first owner verified; exit class credentials; second copy finishes |
| T-172-14 | EC-172-14 | CLI + adapter | InMemory | object-mode insert during claim is not selected |
| T-172-15 | EC-172-15 | CLI | InMemory | `clear` with mode database exits `mode`; BYTEA remains |
| T-172-16 | EC-172-16 | CLI | InMemory | `verified_at` null; clear does not null BYTEA |
| T-172-17 | EC-172-17 | CLI | InMemory | stdout contains `VACUUM FULL` and the table names |
| T-172-18 | EC-172-18 | key unit | none | filename `../café.pdf` absent from the key; key matches the UUID pattern |
| T-172-19 | EC-172-19 | io unit | InMemory | no SSE header by default; `AES256` sets it; `kms` is a config error |
| T-172-20 | EC-172-20 | boot unit | none | database mode plus a ref row exits before listen |
| T-172-21 | EC-172-21 | CLI | InMemory | restore writes BYTEA equal to the object; bad hash writes nothing |
| T-172-22 | EC-172-22 | reaper unit | InMemory | inflight age 1 minute kept; age 2 hours deleted |
| T-172-23 | EC-172-23 | adapter | panic client | database mode upload inserts BYTEA and does not construct the client |
| T-172-24 | EC-172-24 | adapter | InMemory | second checksum in the workspace does not PUT; other workspace does |
| T-172-25 | EC-172-25 | io unit | InMemory | range equals the source slice for three windows including the last byte |
| T-172-26 | EC-172-26 | CLI | Postgres fixture | `documents.content` hash unchanged after clear |
| T-172-27 | EC-172-27 | CLI | Postgres fixture | `octet_length(asset_data)` unchanged; migrator SQL text has no `document_mm_assets` |
| T-172-28 | EC-172-28 | CLI | InMemory | second copy reports skipped = 1 and put count stays 1 |
| T-172-29 | EC-172-29 | CLI | Postgres | two processes, ten rows, ten refs, ten puts |
| T-172-30 | EC-172-30 | adapter | InMemory | get with a different workspace id returns None |
| T-172-31 | EC-172-31 | API + PDF task | MinIO or InMemory | `get_pdf` bytes start with `%PDF-` while the column is null; original download matches |
| T-172-32 | EC-172-32 | migration | Postgres | `byte_location` check rejects the illegal pair; no subquery in `pg_get_constraintdef` |
| T-172-33 | EC-172-33 | wipe | Postgres + InMemory | workspace delete fills `original_object_deletes` for every key |
| T-172-34 | EC-172-34 | adapter | InMemory | second checksum returns Conflict and the loser key is gone |
| T-172-35 | EC-172-35 | compose review + config | none | compose file has no `curl`, uses a named volume, and the client image is `minio-client`; create-bucket with an empty endpoint errors |
| T-172-36 | EC-172-36 | CLI | Postgres | connection role has `BYPASSRLS`; `statement_timeout` during the read is 120s and is restored |
| T-172-37 | EC-172-37 | CLI | Postgres | stdout contains `pg_total_relation_size` and `free space` |
| T-172-38 | EC-172-38 | io unit | InMemory | 50 MiB uses `put` once; a 1 MiB multipart part is rejected in-process |
| T-172-39 | EC-172-39 | config unit | none | `gs://` and `az://` without credentials return `config` and do not build a client |
| T-172-40 | EC-172-40 | reaper unit | InMemory | list-call counter stays 0; delete queue and expired inflight are the only deletes |

## Release gate (W6)

One MinIO run covers T-172-06, T-172-07, T-172-09, T-172-12, and the narrative in [06](06-implementation-plan.md): upload PDF, upload non-PDF, range download, clear a seeded BYTEA row, download again, database-mode control.

`EDGEQUAKE_ORIGINAL_STORE_E2E` unset skips that binary with a message that names the variable. Default `cargo test -p edgequake-storage --lib` runs the InMemory rows and does not start Docker.

## Builder parse, not a second cloud

| Input | Expected builder | Test |
|-------|------------------|------|
| `s3://b/p` | `AmazonS3` | included in T-172-11's module |
| `gs://b/p` | `GoogleCloudStorage` | `config::tests::parses_gs` |
| `az://c/p` | `MicrosoftAzure` | `config::tests::parses_az` |
| `abfs://c@account.dfs.core.windows.net/p` | `MicrosoftAzure` | `config::tests::parses_abfs` |

Those three parses are part of W1. They do not call the network.

## Out of the matrix on purpose

- Live GCS and live Azure accounts
- Presigned URLs
- `document_mm_assets` relocation (EC-172-27 only asserts they are untouched)
- Raising `MAX_UPLOAD_BYTES`
