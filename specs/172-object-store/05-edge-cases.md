# 05 — Edge cases

Parent: [README](README.md) · Prev: [04 Architecture](04-architecture.md) · Next: [06 Implementation plan](06-implementation-plan.md)

Each case names the mitigation and the test in [07](07-e2e-test-matrix.md). Laws are [01](01-first-principles.md).

```text
┌────────────────────────────────────────────────────────────┐
│ failure window                                             │
│                                                            │
│  hash -> inflight -> PUT -> commit ref -> verify -> clear │
│           │           │         │            │        │    │
│           │           │         │            │        └ EC-17 disk
│           │           │         │            └ EC-03 mismatch
│           │           │         └ EC-02 both copies legal
│           │           └ EC-01 orphan object
│           └ EC-22 reaper waits one hour
└────────────────────────────────────────────────────────────┘
```

## Durability

**EC-172-01 — Crash after PUT, before the ref commit.**
Inflight was committed before the PUT, so the key is a row. The reaper deletes that object only after the inflight row is older than one hour, then deletes the row. It does not list the bucket. A retry of the same upload PUTs the same key. Mitigation: [LAW-172-3](01-first-principles.md). Test: T-172-01.

**EC-172-02 — Crash after the ref commit, before clear.**
Both the `BYTEA` and the ref exist. That state is legal. Download reads the object when `verified_at` is set, and the `BYTEA` if the ref is missing. Clear is still a separate command. Test: T-172-02.

**EC-172-03 — SHA-256 of the object does not match the ref or the source `BYTEA`.**
Copy does not set `verified_at`. Clear skips the row and exits non-zero. The `BYTEA` stays. The CLI names the owner id and the two hashes. ETag is not consulted. Test: T-172-03.

**EC-172-04 — Empty payload.**
`validate_original_data` already rejects an empty slice ([F-172-06](02-code-facts.md)). Object mode calls it before PUT. A zero-byte PUT is not a successful original. Test: T-172-04.

**EC-172-05 — 50 MiB boundary, and one cap.**
`len == MAX_UPLOAD_BYTES` is accepted. `len == MAX_UPLOAD_BYTES + 1` is rejected before any PUT and before any `BYTEA` insert. `validate_pdf_data` and `ResourceBudget` use one function ([F-172-21](02-code-facts.md)). An object at or under 100 MiB is one `put`. A multipart part other than the last is at least 5 MiB ([EC-172-38](05-edge-cases.md)). Test: T-172-05.

**EC-172-21 — Restore after clear.**
`originals restore` GETs the object, checks SHA-256, writes `BYTEA`, and leaves the ref in place until the operator switches mode. A mismatch aborts that owner and writes nothing. After a successful restore, `database` mode can boot. Test: T-172-21.

**EC-172-22 — Reaper and a live upload.**
An inflight row younger than one hour is not deleted, and its object is not deleted. An inflight row older than one hour is an abandoned upload: object deleted, inflight row deleted. Test: T-172-22.

## HTTP

**EC-172-06 — Range request that fits.**
`Range: bytes=0-99` on a larger PDF returns 206, `Content-Range`, and 100 bytes equal to the object slice. The route stays `pdf_byte_stream` ([F-172-12](02-code-facts.md)). Test: T-172-06.

**EC-172-07 — Range that does not fit.**
A start past `byte_length` returns 416. The body is empty. No full-object download is substituted. Test: T-172-07.

**EC-172-25 — Range bytes match the old substring.**
For the same PDF stored both ways in a test double, `get_pdf_bytes_range(start, end)` returns the same slice the Postgres `substring` returned. The object path goes through `inclusive_to_get_range`, so the last byte is present. Test: T-172-25.

**EC-172-31 — `get_pdf` still feeds the converter.**
Object mode uploads a PDF, `byte_location` is `object`, and `pdf_data` is null in the table. `get_pdf` returns a buffer that starts with `%PDF-` and matches the object. `fetch_pdf_row` does not decode SQL NULL into `Vec<u8>`. The PDF task and `download_document_original` both succeed ([F-172-17](02-code-facts.md), [F-172-18](02-code-facts.md)). Test: T-172-31.

**EC-172-32 — Same-row location flag.**
Migration 215 rejects `byte_location = 'object'` with a non-null `BYTEA`, and `byte_location = 'database'` with a null `BYTEA`. It does not create a `CHECK` that subqueries `original_object_refs`. Test: T-172-32.

**EC-172-33 — Workspace wipe enqueues object keys.**
`delete_relational_documents_for_workspace` removes the documents. The `BEFORE DELETE` trigger has inserted every key into `original_object_deletes`. No handler called `delete_pdf`. After the reaper, those objects are gone ([F-172-19](02-code-facts.md)). Test: T-172-33.

**EC-172-34 — Checksum conflict deletes the loser object.**
Two concurrent uploads of the same checksum: one commits, the other gets `23505`. The loser has no ref and no inflight row, and its object is already deleted. The winner's object remains. Test: T-172-34.

**EC-172-35 — Chainguard image has no curl and runs as 65532.**
Compose uses a named volume, no bind mount, and no `curl` healthcheck. The image user is 65532 ([specifications](https://images.chainguard.dev/directory/image/minio/specifications)). A one-shot `cgr.dev/chainguard/minio-client` creates the bucket. `EDGEQUAKE_ORIGINAL_STORE_CREATE_BUCKET=true` with an empty endpoint is a config error. Single-node MinIO is labeled a dev store (parity 0). Test: T-172-35.

**EC-172-36 — Maintenance role, and the detoast timeout.**
The CLI connects as `edgequake_originals_maint` (`BYPASSRLS`). The API role is unchanged. The BYTEA read runs with `statement_timeout` of 120s inside the claim transaction and restores the previous value after commit. A claim that sets `app.current_tenant_id` in autocommit and then selects is not the implementation ([F-172-20](02-code-facts.md)). Test: T-172-36.

**EC-172-37 — Plan names the free space `VACUUM FULL` needs.**
`plan` prints `pg_total_relation_size` for both BYTEA tables and a line that free space must cover that size before `VACUUM FULL`. Test: T-172-37.

**EC-172-38 — One put under 100 MiB, parts at least 5 MiB.**
A 50 MiB object is a single `put`. A multipart upload with a non-final part under 5 MiB is rejected by the client before the request, matching the S3 rule ([multipart limits](https://docs.aws.amazon.com/AmazonS3/latest/userguide/qfacts.html)). `validate_pdf_data(50 MiB + 1)` and the HTTP cap agree when `EDGEQUAKE_MAX_UPLOAD_BYTES` is unset. Test: T-172-38.

**EC-172-39 — Cloud credentials fail before a socket.**
`gs://` with no ADC and no `GOOGLE_APPLICATION_CREDENTIALS` is a `config` error and opens no connection. `az://` with no account is the same. `s3://` with a custom endpoint and `allow_http` false on an `http` URL is [EC-172-11](05-edge-cases.md). Test: T-172-39.

**EC-172-40 — The reaper does not list, and backups are one set.**
A store double that counts `list` calls stays at zero across a reaper pass. `plan` prints that a versioned bucket turns `DeleteObject` into a delete marker ([versioning](https://docs.aws.amazon.com/AmazonS3/latest/userguide/Versioning.html)) and that a Postgres PITR without the bucket is not a full restore. Test: T-172-40.

**EC-172-09 — Cross-workspace id.**
A caller in workspace B who knows a pdf id from workspace A receives 403 or 404, the same as today, and the object client is not called. RLS on `original_object_refs` denies a direct SQL session that is not the owner tenant. Test: T-172-09.

**EC-172-30 — The key prefix is not authorization.**
Two workspaces in one bucket are separated by the API check, not by IAM per workspace. A test that uses the storage trait with the wrong `workspace_id` does not return bytes even when the object exists. Test: T-172-30.

## Backend config

**EC-172-10 — Bucket missing or unreachable in object mode.**
Boot logs the endpoint host and the bucket name, not the secret. `/health` is 200 with `original_store_ready: false`. `/ready` is 503. `database` mode does not take this branch. Test: T-172-10.

**EC-172-11 — HTTP endpoint without allow-http.**
`http://localhost:9000` with `EDGEQUAKE_ORIGINAL_STORE_ALLOW_HTTP` unset or `false` fails config parse. No request is sent. This matches the crate error in [03](03-object-store-theory.md). Test: T-172-11.

**EC-172-12 — Virtual-hosted style against MinIO, path-style against AWS.**
With `EDGEQUAKE_ORIGINAL_STORE_ENDPOINT` set, the default is path-style. `PATH_STYLE=false` against a MinIO with no `MINIO_DOMAIN` fails the boot head and the error names path-style. With the endpoint unset, the default is virtual-hosted style for AWS ([Virtual hosting](https://docs.aws.amazon.com/AmazonS3/latest/userguide/VirtualHosting.html)). Test: T-172-12.

**EC-172-13 — Credentials expire mid-copy.**
The current owner is not marked verified. Already verified owners stay verified. The CLI exits with class `credentials` and the next command `edgequake originals copy`. A retry skips verified rows ([EC-172-28](05-edge-cases.md)). Test: T-172-13.

**EC-172-19 — SSE off by default.**
With `EDGEQUAKE_OBJECT_STORE_SSE` unset, the PUT does not add an SSE header. `AES256` adds the header the crate supports. Any other value fails config parse. Test: T-172-19.

**EC-172-20 — Replicas disagree.**
Documented as a deploy failure. The process does not discover peer env vars. The detectable case is local: `database` mode while `original_object_refs` is non-empty, or `object` mode while a cleared owner has a null `BYTEA` and the bucket head fails. Boot exits before serving. Test: T-172-20.

## Migration races

**EC-172-14 — Upload while the migrator runs.**
Server in `object` mode writes a ref and a null `BYTEA`. The claim is `BYTEA IS NOT NULL AND ref IS NULL`, so the new row is not copied again. Server in `database` mode can still insert `BYTEA`; the claim takes it. Test: T-172-14.

**EC-172-15 — Tool runs while the server is still in database mode.**
Copy is allowed: it only adds refs and objects, and it does not null `BYTEA`. The server keeps serving `BYTEA` until it is restarted in `object` mode. Clear refuses to run unless `EDGEQUAKE_ORIGINAL_STORE=object` is set in the tool's environment, so an operator cannot strip bytes the running server still reads. Test: T-172-15.

**EC-172-16 — Clear without verify.**
`clear` selects `verified_at IS NOT NULL` only. A ref with a null `verified_at` is reported as blocked. Exit code is non-zero if any blocked row remains. `migrate` does not call clear. Test: T-172-16.

**EC-172-17 — Disk after clear.**
The CLI prints the sum of cleared bytes, `pg_total_relation_size` of `pdf_document_blobs` and `document_originals`, and the sentence that Postgres will not return that space until `VACUUM FULL` or `pg_repack`. It also says `VACUUM FULL` needs about that much free space because it rewrites the table ([VACUUM](https://www.postgresql.org/docs/current/sql-vacuum.html)). The test asserts the sentence, the two table names, and that the columns are NULL. It does not require the relation size to shrink inside the test. Test: T-172-17.

**EC-172-28 — Resume.**
A second `copy` skips an owner whose ref `sha256` equals a fresh hash of the object and whose `byte_length` matches. It does not PUT again. Test: T-172-28.

**EC-172-29 — Two migrators.**
Two processes claim with `FOR UPDATE SKIP LOCKED`. Each owner is PUT once. The test uses a barrier and asserts one ref row per owner. Test: T-172-29.

## Identity and scope

**EC-172-18 — Unicode filename.**
A name with accents, spaces, and `../` is stored in `filename` and sent as `Content-Disposition`. The object key matches the UUID pattern and does not contain the name. Test: T-172-18.

**EC-172-08 — Object delete fails after the row commit.**
The document is gone from the API. `original_object_deletes` still has the key. A later reaper pass removes the object. The API does not return 500 to undo the delete. Test: T-172-08.

**EC-172-23 — Database mode never opens a bucket.**
Config with only `EDGEQUAKE_ORIGINAL_STORE=database` (or unset) builds no `ObjectStore`. Upload writes `BYTEA`. A test double that panics on client construction proves the path is not taken. Test: T-172-23.

**EC-172-24 — PDF dedup stays per workspace.**
The second upload of the same checksum in the same workspace returns the existing `pdf_id` and does not PUT. The same checksum in another workspace PUTs a different key. Test: T-172-24.

**EC-172-26 — Retrieval text stays.**
After copy and clear, `documents.content` and chunk rows are byte-identical to before the run. Test: T-172-26.

**EC-172-27 — Page images stay.**
`document_mm_assets.asset_data` length is unchanged by `originals migrate` and `clear`. The migrator SQL does not mention that table. Test: T-172-27.

## Case index

| Case | Mitigation in one line | Test |
|------|------------------------|------|
| EC-172-01 | Reaper after the inflight hour | T-172-01 |
| EC-172-02 | Both copies legal until clear | T-172-02 |
| EC-172-03 | No `verified_at`, BYTEA kept | T-172-03 |
| EC-172-04 | Reject empty before PUT | T-172-04 |
| EC-172-05 | 50 MiB inclusive cap | T-172-05 |
| EC-172-06 | 206 on the same route | T-172-06 |
| EC-172-07 | 416, no full-body fallback | T-172-07 |
| EC-172-08 | Delete queue, API stays deleted | T-172-08 |
| EC-172-09 | Workspace check before GET | T-172-09 |
| EC-172-10 | `/ready` 503, `/health` 200 | T-172-10 |
| EC-172-11 | Config refuses HTTP | T-172-11 |
| EC-172-12 | Default path-style | T-172-12 |
| EC-172-13 | Resume after credential failure | T-172-13 |
| EC-172-14 | Claim ignores new object-mode rows | T-172-14 |
| EC-172-15 | Clear requires object mode | T-172-15 |
| EC-172-16 | Clear requires `verified_at` | T-172-16 |
| EC-172-17 | CLI names VACUUM FULL | T-172-17 |
| EC-172-18 | Filename stays out of the key | T-172-18 |
| EC-172-19 | SSE header absent by default | T-172-19 |
| EC-172-20 | Boot fails on mode versus refs | T-172-20 |
| EC-172-21 | Restore checks hash then writes BYTEA | T-172-21 |
| EC-172-22 | Young inflight is kept | T-172-22 |
| EC-172-23 | Database mode builds no client | T-172-23 |
| EC-172-24 | Dedup is per workspace | T-172-24 |
| EC-172-25 | `inclusive_to_get_range` matches substring | T-172-25 |
| EC-172-26 | `documents.content` untouched | T-172-26 |
| EC-172-27 | `document_mm_assets` untouched | T-172-27 |
| EC-172-28 | Second copy skips a match | T-172-28 |
| EC-172-29 | `SKIP LOCKED` one PUT | T-172-29 |
| EC-172-30 | Wrong workspace reads nothing | T-172-30 |
| EC-172-31 | `get_pdf` fills bytes from the object | T-172-31 |
| EC-172-32 | Same-row `byte_location` check | T-172-32 |
| EC-172-33 | Wipe trigger fills the delete queue | T-172-33 |
| EC-172-34 | `23505` deletes the loser object | T-172-34 |
| EC-172-35 | No curl, UID 65532, bucket via minio-client | T-172-35 |
| EC-172-36 | `BYPASSRLS` role and statement timeout | T-172-36 |
| EC-172-37 | `plan` prints relation size | T-172-37 |
| EC-172-38 | Single put, 5 MiB part floor | T-172-38 |
| EC-172-39 | Missing cloud credentials fail closed | T-172-39 |
| EC-172-40 | Reaper list count is zero | T-172-40 |
