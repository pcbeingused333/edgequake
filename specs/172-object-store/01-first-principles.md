# 01 — First principles

Parent: [README](README.md) · Prev: [00 WHY](00-why.md) · Next: [02 Code facts](02-code-facts.md)

These laws are the contract for [06 Implementation plan](06-implementation-plan.md). Each one answers a WHY in [00](00-why.md). Findings that violate them today are in [02](02-code-facts.md).

## LAW-172-1 — The original is the file bytes

The original document is the uploaded byte string. For a PDF that is `pdf_document_blobs.pdf_data`. For any other upload that is `document_originals.original_data`.

`documents.content`, chunk text, embeddings, the graph, and `markdown_content` are derived. `document_mm_assets.asset_data` is a derived page image. They stay in Postgres.

In `database` mode the original lives in `BYTEA`. In `object` mode the original lives in the bucket. After a successful clear, a readable original has bytes in one of those places, not both, and not neither.

The location is a column on the same row as the bytes: `byte_location` is `database` or `object`. A `CHECK` cannot look at `original_object_refs`. PostgreSQL forbids subqueries in `CHECK`, and a hidden subquery breaks `pg_restore` ([constraints](https://www.postgresql.org/docs/current/ddl-constraints.html)). `database` requires the `BYTEA` column to be non-null. `object` requires it to be null. The ref row is a separate table with real foreign keys.

WHY: [WHY-172-1](00-why.md), [WHY-172-2](00-why.md).

## LAW-172-2 — One client, four backends

All bucket IO goes through the Apache `object_store` crate (`ObjectStore`). AWS S3 and MinIO use `AmazonS3Builder`. GCS uses `GoogleCloudStorageBuilder`. Azure Blob uses `MicrosoftAzureBuilder`.

Local object storage is `cgr.dev/chainguard/minio:latest`. Images named `minio/minio` on Docker Hub or Quay are not a supported dependency.

Path-style (`with_virtual_hosted_style_request(false)`) is used only when `EDGEQUAKE_ORIGINAL_STORE_ENDPOINT` is set. That is the MinIO case. Real AWS, with no custom endpoint, uses virtual-hosted style. AWS still accepts path-style, and the 2020 cutoff was delayed, but the current guide says path-style will be discontinued ([Virtual hosting](https://docs.aws.amazon.com/AmazonS3/latest/userguide/VirtualHosting.html)). An `http://` endpoint is refused unless `with_allow_http(true)` is set on purpose for a local store.

A missing GCS or Azure credential fails config parse before a socket is opened. Objects at or under 100 MiB use one `put`. A multipart part, when multipart is used, is at least 5 MiB except the last part ([multipart limits](https://docs.aws.amazon.com/AmazonS3/latest/userguide/qfacts.html)). `validate_pdf_data` and the HTTP cap call one function. The hardcoded 50 MiB inside `validate_pdf_data` and the env override in `ResourceBudget` are the same number after W1.

WHY: [WHY-172-4](00-why.md). Contracts: [03](03-object-store-theory.md).

## LAW-172-3 — The pointer is written after the PUT

A new original in `object` mode is durable only when both of these are true: the object exists at the key, and Postgres has committed `original_object_refs` for that owner. The commit does not store `BYTEA`.

PUT of the same key with the same bytes is success (idempotent overwrite). PUT is not conditional on `If-None-Match`. MinIO is not required to honor that header for this design.

Inflight is committed before PUT, so every PUT has a row that names its key. If the process dies after PUT and before the ref commit, the reaper deletes that key after one hour. The reaper reads `original_object_inflight` and `original_object_deletes` only. It does not call `ListObjects`. A truncated list would delete live keys ([ListObjectsV2](https://docs.aws.amazon.com/AmazonS3/latest/API/API_ListObjectsV2.html) returns at most 1,000 keys per page). A ref whose object is missing is a failed verify, not a reaper delete.

A `23505` conflict after PUT deletes that new key and its inflight row before the handler returns `Conflict`. The reaper is the crash path, not the steady-state path.

User delete, workspace wipe, and a raw SQL delete all land in `original_object_deletes` through a `BEFORE DELETE` trigger on `original_object_refs`. The refs use two foreign keys (`pdf_id`, `document_id`), each `ON DELETE CASCADE`, with exactly one of them set. One `owner_id` cannot reference both parents ([foreign keys](https://www.postgresql.org/docs/current/ddl-constraints.html)). The API returns success after the Postgres commit. A failed object delete stays on the queue. It is not an HTTP 500 that puts the document back.

WHY: [WHY-172-2](00-why.md). Shape: [04](04-architecture.md).

## LAW-172-4 — Verify, then clear

The migrator never nulls `BYTEA` in the same step as the copy. `migrate` is plan, copy, and verify. `clear` is a separate verb. Clear refuses a row whose stored SHA-256 does not match a fresh hash of the object.

SHA-256 is computed by EdgeQuake. The S3 ETag is stored as a hint and is not the checksum. Multipart ETags are not MD5.

Nulling `BYTEA` does not return disk. `VACUUM FULL` rewrites the table into a new file and needs free space about the size of that table ([VACUUM](https://www.postgresql.org/docs/current/sql-vacuum.html)). `plan` prints `pg_total_relation_size` for `pdf_document_blobs` and `document_originals` and says clear needs that much free space afterwards. The CLI does not run `VACUUM FULL`.

The copy claim is one transaction on a maintenance role that has `BYPASSRLS`. The API role does not: `rls.rs` rejects a tenant role that bypasses row security. A fleet-wide copy cannot set `app.current_tenant_id` for one tenant and still see the others. `set_config('app.current_tenant_id', …, true)` outside a transaction is cleared before the next statement ([F-172-20](02-code-facts.md)). `--workspace` is a `WHERE` clause on that maintenance connection, not an RLS GUC.

The BYTEA read sets `statement_timeout` high enough for a 50 MiB detoast, then restores it ([statement_timeout](https://www.postgresql.org/docs/current/runtime-config-client.html)). `plan` prints both the maintenance count and, when the API role is used as a check, the RLS-scoped count. If the operator passed `--workspace` and those counts disagree, the tool exits `database` and does not clear.

Postgres backups and the bucket are one recovery set. On a versioned bucket, `DeleteObject` writes a delete marker and the bytes remain until lifecycle removes noncurrent versions ([versioning](https://docs.aws.amazon.com/AmazonS3/latest/userguide/Versioning.html)). A PITR of Postgres to a time before that delete restores a pointer whose object may already be a delete marker. Enable versioning and a lifecycle rule, or accept that object delete is not aligned with WAL.

`restore` copies a verified object back into `BYTEA` so an operator can leave object mode before clear is treated as final. Restore refuses a checksum mismatch.

WHY: [WHY-172-5](00-why.md). UX: [08](08-migration-ux.md).

## LAW-172-5 — Authorization stays in the API

Download routes do not change:

- `GET /api/v1/documents/pdf/{pdf_id}/download` returns 200, 206, or 416
- the non-PDF original download in `download.rs` returns the same headers it returns today

The handler checks the tenant and the workspace, then reads the object. The bucket policy is private. This spec does not mint presigned URLs. A key prefix is an operations boundary, not an access check.

WHY: [WHY-172-3](00-why.md).

## LAW-172-6 — One module for both processes

Key building, put, ranged get, SHA-256 verify, and delete live in one module inside `edgequake-storage`. `PostgresPdfStorage`, `PostgresOriginalStorage`, and `edgequake originals` call that module. They do not each speak HTTP to S3.

`PdfDocumentStorage::get_pdf_bytes_range` and `DocumentOriginalStorage::get_original` keep their signatures. HTTP handlers do not grow a second download implementation.

`get_pdf` is the path the converter uses. `pdf_processing` moves `pdf.pdf_data` into the pipeline ([F-172-17](02-code-facts.md)). `download_document_original` returns that same field for a PDF ([F-172-18](02-code-facts.md)). When `byte_location` is `object`, `fetch_pdf_row` does not select the `BYTEA`, and `get_pdf` fills `pdf_data` from `originals::io`. sqlx `Row::get::<Vec<u8>>` cannot decode SQL `NULL`.

`get_pdf_bytes_range` is an inclusive window `start..=end`. `object_store` `GetRange` is half-open. One function, `inclusive_to_get_range`, converts `start..=end` into `start..(end + 1)`. Callers do not pass the inclusive end through.

WHY: [WHY-172-4](00-why.md), [WHY-172-5](00-why.md).

## LAW-172-7 — The mode is fleet-wide

`EDGEQUAKE_ORIGINAL_STORE` is `database` or `object`. The default is `database`.

`database` does not construct a store and does not require credentials. `object` refuses to boot when the bucket cannot be reached.

If `original_object_refs` contains rows and the process is in `database` mode, boot fails. If the process is in `object` mode and a readable original has neither `BYTEA` nor a ref, boot fails the readiness check for that inconsistency rather than serving a hole. Replicas that disagree on the mode are a failed deploy, not a runtime negotiation.

WHY: [WHY-172-3](00-why.md).

## LAW-172-8 — The key is not the filename

Object keys are:

```text
{prefix}/t/{tenant_id}/w/{workspace_id}/pdf/{pdf_id}
{prefix}/t/{tenant_id}/w/{workspace_id}/original/{document_id}
```

`tenant_id`, `workspace_id`, `pdf_id`, and `document_id` are UUIDs. The filename, including Unicode and path separators, is metadata in Postgres and a `Content-Disposition` parameter. It is never interpolated into the key. This continues SPEC-083, which already refuses raw multipart path components.

WHY: [WHY-172-4](00-why.md).

## How the laws fit together

```text
┌────────────┐   ┌────────────┐   ┌────────────┐   ┌────────────┐
│ LAW-172-1  │   │ LAW-172-2  │   │ LAW-172-3  │   │ LAW-172-4  │
│ file bytes │   │ one client │   │ PUT then   │   │ verify     │
│ only       │   │ four clouds│   │ pointer    │   │ then clear │
└────────────┘   └────────────┘   └────────────┘   └────────────┘
      │                │                │                │       
      v                v                v                v       
┌───────────────────────────────────────────────────────────────┐
│ LAW-172-5 API authz   LAW-172-6 one module                    │
│ LAW-172-7 one mode    LAW-172-8 ids in the key                │
└───────────────────────────────────────────────────────────────┘
```

## DRY and SOLID, as laws

| Principle | Where it binds |
|-----------|----------------|
| SRP | Config parsing, the object client, the pointer transaction, and the CLI verbs each have one owner. |
| OCP | A new backend is a builder behind `ObjectStore`. PDF and original callers do not change. |
| LSP | `database` and `object` both honor `get_pdf_bytes_range` and `get_original`. Callers cannot tell which one filled the buffer, except by timing. |
| ISP | HTTP handlers depend on the existing storage traits. They do not depend on `AmazonS3`. |
| DIP | The CLI and the adapters depend on the shared module. The shared module depends on `Arc<dyn ObjectStore>`. |
| DRY | One key builder. One verify function. The migrator and the upload path call them. |

Edge cases are [05](05-edge-cases.md). Tests are [07](07-e2e-test-matrix.md). The matrix is [09](09-cross-ref.md).
