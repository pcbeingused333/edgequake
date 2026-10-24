# 02 — Code facts

Parent: [README](README.md) · Prev: [01 First principles](01-first-principles.md) · Next: [03 Object store theory](03-object-store-theory.md)

Facts below are from the tree at spec time (2026-10-23, product pin v0.36.0). Line numbers name the current functions. A later edit can move a line; the symbol is the stable id.

## Where the bytes are

**F-172-01 — PDF originals are `BYTEA` in a side table.**
[`edgequake/migrations/103_pdf_document_blobs_side_table.sql`](../../edgequake/migrations/103_pdf_document_blobs_side_table.sql) creates `pdf_document_blobs(pdf_id, pdf_data BYTEA NOT NULL, …)`. Migration 105 makes that table the PDF byte SSOT and drops `pdf_documents.pdf_data`. The blob row cascades on `pdf_documents` delete (`ON DELETE CASCADE`).

**F-172-02 — PDF create writes the bytes in the same transaction as the metadata.**
[`pdf_storage_impl.rs`](../../edgequake/crates/edgequake-storage/src/adapters/postgres/pdf_storage_impl.rs) `create_pdf` inserts `pdf_documents` (including `sha256_checksum`, around the bind at line 284) and then `INSERT INTO pdf_document_blobs (pdf_id, pdf_data)` before `tx.commit()`. A unique violation `23505` becomes `StorageError::Conflict`.

**F-172-03 — Range reads slice `BYTEA` inside Postgres.**
`get_pdf_bytes_range` in the same file runs `substring(pdf_data FROM $2 FOR $3)`. The comment states the window is 1-based so the whole blob does not cross the wire. The trait default in [`pdf_storage.rs`](../../edgequake/crates/edgequake-storage/src/pdf_storage.rs) `get_pdf_bytes_range` (around line 390) slices a full `get_pdf` instead. Object mode must override the Postgres method and keep the trait signature.

**F-172-04 — Size for authorization still detoasts.**
`get_pdf_blob_info` selects `octet_length(b.pdf_data)`. `pdf_documents.file_size_bytes` already exists from migration 022. Object mode must use the stored length (and the ref), not `octet_length` of a null blob.

**F-172-05 — Non-PDF originals are `BYTEA` and have no checksum.**
[`082_add_document_originals.sql`](../../edgequake/migrations/082_add_document_originals.sql) defines `original_data BYTEA NOT NULL` plus filename, content type, and `file_size_bytes`. There is no `sha256` column. [`original_storage_impl.rs`](../../edgequake/crates/edgequake-storage/src/adapters/postgres/original_storage_impl.rs) `store_original` binds the full `Vec<u8>`. `delete_original` is a SQL `DELETE` scoped by `document_id` and `workspace_id`. The cascade on `documents(id)` removes the row when the document goes away, which will not delete a bucket object.

**F-172-06 — The original trait returns the whole buffer.**
[`original_storage.rs`](../../edgequake/crates/edgequake-storage/src/original_storage.rs) `DocumentOriginal` always carries `original_data: Vec<u8>`. `validate_original_data` rejects an empty slice. Object mode still fills that `Vec` for this spec (the cap is 50 MiB). A streaming original response is a later change. The trait methods stay `store_original`, `get_original`, `delete_original`.

**F-172-07 — Non-PDF download is an authenticated handler, not a bucket URL.**
[`download.rs`](../../edgequake/crates/edgequake-api/src/handlers/documents/query/download.rs) loads `get_original(&workspace_id, &document_uuid)` and responds with `CONTENT_TYPE`, `CONTENT_DISPOSITION` (the filename), and the bytes. The log line is `Document original download via document_originals`. The memory-storage branch returns `pdf.pdf_data` from the PDF row instead.

**F-172-08 — The upload cap is 50 MiB.**
[`budget.rs`](../../edgequake/crates/edgequake-core/src/resource/budget.rs) line 21: `MAX_UPLOAD_BYTES = 50 * 1024 * 1024`. [`pdf_storage.rs`](../../edgequake/crates/edgequake-storage/src/pdf_storage.rs) documents the same cap as BR0702. This spec does not raise it. Multipart part sizing in the object client must still accept a 50 MiB object.

**F-172-09 — No object-store dependency exists.**
[`edgequake-storage/Cargo.toml`](../../edgequake/crates/edgequake-storage/Cargo.toml) depends on `sqlx` and `async-trait`. A workspace search for `object_store`, `aws-sdk-s3`, and `opendal` returns no Rust dependency. W1 adds `object_store` with features `aws`, `gcp`, and `azure` only.

## What must not move

**F-172-10 — `documents.content` is the retrieval text, and list already avoids it.**
[`document_read_model.rs`](../../edgequake/crates/edgequake-api/src/document_read_model.rs) (GH-400 comment near line 135) forbids detoasting `documents.content` on the list path. [`finalize.rs`](../../edgequake/crates/edgequake-api/src/processor/text_insert/finalize.rs) treats `documents.content` as the detail SSOT. Moving it would change query and FTS. Out of scope ([LAW-172-1](01-first-principles.md)).

**F-172-11 — Page images are a second `BYTEA` family.**
[`084_add_document_mm_assets.sql`](../../edgequake/migrations/084_add_document_mm_assets.sql) stores `asset_data BYTEA NOT NULL` for `page_full` and `page_chart_crop`. [`document_mm_asset_persist.rs`](../../edgequake/crates/edgequake-api/src/services/document_mm_asset_persist.rs) calls that table the durable copy. These bytes are derived. This spec does not relocate them. A follow-on can reuse the same client; it must not be folded into the originals migrator.

**F-172-12 — PDF download already streams ranges through the trait.**
[`content.rs`](../../edgequake/crates/edgequake-api/src/handlers/pdf_upload/content.rs) `pdf_byte_stream` calls `get_pdf_bytes_range` per window and `build_pdf_response` emits 200, 206, or 416. Object mode replaces the body of `get_pdf_bytes_range`. It does not add a route.

## Operator surface today

**F-172-13 — Schema migrate is a subcommand, not a library the HTTP server runs ad hoc.**
[`main.rs`](../../edgequake/src/main.rs) owns `MIGRATE_USAGE` (around line 770) and rejects unknown migrate verbs. [`migrate_console.rs`](../../edgequake/src/migrate_console.rs) classifies abort reasons and prints the next human step. `edgequake originals` follows that shape and does not overload `migrate`.

**F-172-14 — Schema head is 214. Spec 172 is not a migration number.**
The newest files are `213_spec169_tenant_invitations.sql` and `214_spec169_user_access_links.sql`. `172_request_path_lookup_indexes.sql` is an index migration. The pointer table is migration **215**.

**F-172-15 — PDF dedup is a per-workspace SHA-256, already computed before insert.**
Migration 022 defines `sha256_checksum VARCHAR(64) NOT NULL` with a hex check (around line 86 and line 158). `create_pdf` calls `find_pdf_by_checksum` before insert. Object mode keeps that check. The object key stays per `pdf_id`, not per hash, so delete does not need a reference count. Two different workspaces never share an object.

**F-172-16 — Health reports database storage, not an original-byte location.**
[`health_types.rs`](../../edgequake/crates/edgequake-api/src/handlers/health_types.rs) `HealthResponse.storage_mode` is `"memory"` or `"postgresql"`. `ComponentHealth` has `kv_storage`, `vector_storage`, `graph_storage`, and `llm_provider`. It has no original-store field. W3 adds `original_store` (`database` or `object`) and `original_store_ready` without renaming `storage_mode`. `HealthResponse` derives `ToSchema`, so the OpenAPI snapshot is regenerated in that same change.

**F-172-17 — The PDF pipeline reads `pdf.pdf_data` from `get_pdf`.**
[`pdf_processing.rs`](../../edgequake/crates/edgequake-api/src/processor/pdf_processing.rs) loads `get_pdf` (around line 560) and then moves `pdf.pdf_data` into the converter (around line 609). Page-count healing also scans `pdf.pdf_data` (around line 588). A `get_pdf` that returns an empty buffer fails `%PDF-` validation before pdfium runs.

**F-172-18 — Original download of a PDF also uses `get_pdf`.**
[`download.rs`](../../edgequake/crates/edgequake-api/src/handlers/documents/query/download.rs) `download_document_original` (around line 123) calls `get_pdf` and responds with `pdf.pdf_data` (around line 151). The range route in `content.rs` is a second reader, not the only one.

**F-172-19 — Workspace wipe deletes rows in SQL and never calls `delete_pdf`.**
[`delete_relational_documents_for_workspace`](../../edgequake/crates/edgequake-api/src/document_read_model.rs) (around line 1444) is the set delete used by [`workspace_document_wipe.rs`](../../edgequake/crates/edgequake-api/src/services/workspace_document_wipe.rs) (around line 237). `document_originals` and `pdf_document_blobs` disappear by foreign key. A bucket object does not.

**F-172-20 — The tenant GUC dies outside the transaction that set it.**
[`rls.rs`](../../edgequake/crates/edgequake-storage/src/adapters/postgres/rls.rs) (around line 337) documents that `set_config(..., is_local = true)` clears `app.current_tenant_id` when the statement ends, unless it runs inside `BEGIN`. The GUC names are `app.current_tenant_id` and `app.current_workspace_id` (migration 001). The same file rejects a tenant role that has `BYPASSRLS`.

**F-172-21 — The PDF validator ignores the env cap.**
`validate_pdf_data` hardcodes `50 * 1024 * 1024` ([`pdf_storage.rs`](../../edgequake/crates/edgequake-storage/src/pdf_storage.rs) around line 627). `ResourceBudget::from_env` raises `max_upload_bytes` from `EDGEQUAKE_MAX_UPLOAD_BYTES` with a floor of 1 MiB and no ceiling ([`budget.rs`](../../edgequake/crates/edgequake-core/src/resource/budget.rs) around line 144).

## RLS fact the new table must join

Migration [`167_tenant_access_rls.sql`](../../edgequake/migrations/167_tenant_access_rls.sql) enables forced RLS for `document_originals` and `pdf_documents` with a workspace predicate plus a tenant check through `workspaces`. `pdf_document_blobs` is not in that array; access today is through `pdf_documents`. `original_object_refs` must be in the workspace-scoped list in migration 215, with `tenant_id` and `workspace_id` columns, so a ref cannot be read with only a guessed UUID.

## Finding index

| Finding | Symbol | Law it pressures |
|---------|--------|------------------|
| F-172-01 | `pdf_document_blobs.pdf_data` | LAW-172-1 |
| F-172-02 | `create_pdf` BYTEA insert | LAW-172-3 |
| F-172-03 | `get_pdf_bytes_range` | LAW-172-6 |
| F-172-04 | `octet_length(pdf_data)` | LAW-172-3 |
| F-172-05 | `document_originals.original_data` | LAW-172-1 |
| F-172-06 | `DocumentOriginal.original_data` | LAW-172-6 |
| F-172-07 | `download.rs` original handler | LAW-172-5 |
| F-172-08 | `MAX_UPLOAD_BYTES` | LAW-172-1 |
| F-172-09 | no `object_store` crate | LAW-172-2 |
| F-172-10 | `documents.content` list guard | LAW-172-1 |
| F-172-11 | `document_mm_assets.asset_data` | LAW-172-1 |
| F-172-12 | `pdf_byte_stream` | LAW-172-5 |
| F-172-13 | `MIGRATE_USAGE` | LAW-172-6 |
| F-172-14 | migration 214 vs spec 172 | LAW-172-7 |
| F-172-15 | `sha256_checksum` | LAW-172-4 |
| F-172-16 | `HealthResponse.storage_mode` | LAW-172-7 |
| F-172-17 | `pdf_processing` `pdf.pdf_data` | LAW-172-6 |
| F-172-18 | `download_document_original` `get_pdf` | LAW-172-6 |
| F-172-19 | workspace wipe set delete | LAW-172-3 |
| F-172-20 | `app.current_tenant_id` is local | LAW-172-4 |
| F-172-21 | `validate_pdf_data` vs env cap | LAW-172-2 |
