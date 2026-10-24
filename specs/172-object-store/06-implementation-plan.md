# 06 — Implementation plan

Parent: [README](README.md) · Prev: [05 Edge cases](05-edge-cases.md) · Next: [07 E2E test matrix](07-e2e-test-matrix.md)

Waves are sequential. A wave merges only when its tests in [07](07-e2e-test-matrix.md) pass. This document is the plan. It does not change Rust by itself.

## Principles that bind every wave

- DRY: put, ranged get, SHA-256, and key format exist once, in `edgequake-storage/src/originals/`.
- SOLID: the map in [04](04-architecture.md). Handlers stay on `PdfDocumentStorage` and `DocumentOriginalStorage`.
- First principles: [01](01-first-principles.md). A wave that stores a third copy, or that clears `BYTEA` before verify, is wrong even if tests are green.
- Schema numbers: migration **215**. Do not add `172_*.sql` for this spec ([F-172-14](02-code-facts.md)).
- Default stays `database`. CI without MinIO still passes the database-mode tests.

## W0 — This pack

Recommendation R0. Documents under `specs/172-object-store/` and `scripts/validate-cross-ref.py`.

Done when `python3 specs/172-object-store/scripts/validate-cross-ref.py` exits 0.

No product code.

## W1 — Client module

Recommendation R1. Files:

- `edgequake/crates/edgequake-storage/Cargo.toml` — `object_store` with features `aws`, `gcp`, `azure`. Record the crates.io version in [03](03-object-store-theory.md).
- `edgequake/crates/edgequake-storage/src/originals/mod.rs`
- `originals/config.rs` — env table in [04](04-architecture.md)
- `originals/key.rs` — [LAW-172-8](01-first-principles.md)
- `originals/io.rs` — `put`, `get_range`, `verify_sha256`, `delete`
- `edgequake/crates/edgequake-storage/src/lib.rs` — export the module

Tests, no network: `InMemory` store. Cover EC-172-04, EC-172-05, EC-172-11, EC-172-18, EC-172-19, EC-172-23, EC-172-25, EC-172-38, EC-172-39. `inclusive_to_get_range` is in this wave. `validate_pdf_data` and `ResourceBudget` share one cap function.

Probe `cgr.dev/chainguard/minio:latest` and record the user (expected 65532) and that `curl` is absent. Compose does not wait on that probe to choose TCP-from-outside: the image has no shell, so an in-container `CMD` healthcheck is not the design. Write the probe result into [03](03-object-store-theory.md).

Re-fetch the pinned docs.rs pages. If `with_virtual_hosted_style_request` or the URL schemes moved, update [03](03-object-store-theory.md) in the same change.

## W2 — Schema

Recommendation R2. File:

- `edgequake/migrations/215_spec172_original_object_refs.sql`

Contents specified in [04](04-architecture.md): `original_object_refs` with `pdf_id` and `document_id` foreign keys (exactly one set), `byte_location` on the two BYTEA tables, nullable `pdf_data` and `original_data`, same-row checks only, `original_object_inflight`, `original_object_deletes`, a `BEFORE DELETE` trigger from the ref into the delete queue, and RLS in the migration 167 style. No `CHECK` subquery. Role `edgequake_originals_maint` with `BYPASSRLS`, not granted to the API user.

A down migration is not required if the repo's migrate engine is forward-only (SPEC-150). The SQL comment says how to restore nullability only when the ref table is empty.

Tests: migration applies on a database that already has migration 214; existing `BYTEA` rows stay `byte_location = database`; the cross-table check is absent (EC-172-32); the wipe trigger fires (EC-172-33). RLS hides a ref from another tenant. This is the home for the SQL half of EC-172-09 and EC-172-20.

## W3 — Server path

Recommendation R3 and R7. Files:

- `pdf_storage_impl.rs` — object branch in `create_pdf` (inflight commit before PUT, `23505` deletes the loser key), `fetch_pdf_row` (no NULL `BYTEA` decode), `get_pdf` (fill `pdf_data` from the object when `byte_location = object`), `get_pdf_blob_info` (use `file_size_bytes` or the ref, not `octet_length` of a null blob), `get_pdf_bytes_range` via `inclusive_to_get_range`, `delete_pdf`
- `original_storage_impl.rs` — object branch in `store_original`, `get_original`, `delete_original`
- `edgequake-api` health types — `original_store`, `original_store_ready`, and the OpenAPI snapshot
- reaper tick from the existing background loop, or a dedicated spawn next to other maintenance tasks. One function, called from startup and from the CLI. It selects queue rows. It does not call `ListObjects`.

`database` mode paths stay the current SQL. Do not wrap them in a trait object that changes the transaction boundary of the BYTEA insert.

Tests: EC-172-01, EC-172-06, EC-172-07, EC-172-08, EC-172-10, EC-172-12, EC-172-14, EC-172-22, EC-172-24, EC-172-30, EC-172-31, EC-172-34, EC-172-40. MinIO or `InMemory` as the matrix says. The PDF task test in EC-172-31 is mandatory before W3 is called done.

## W4 — CLI

Recommendation R4 and R8. Files:

- `edgequake/src/originals_cmd.rs`
- `edgequake/src/main.rs` — verb `originals`, unknown verb message in the same style as `MIGRATE_USAGE` ([F-172-13](02-code-facts.md))

Verbs and exit classes are [08](08-migration-ux.md). The CLI links `edgequake-storage` and calls `originals::io`. It does not duplicate hashing.

Tests: EC-172-02, EC-172-03, EC-172-13, EC-172-15, EC-172-16, EC-172-17, EC-172-21, EC-172-26, EC-172-27, EC-172-28, EC-172-29, EC-172-36, EC-172-37. The CLI connection is the maintenance role.

## W5 — Deploy surface

Recommendation R5. Files:

- `edgequake/docker/docker-compose.yml` — optional `minio` profile. Image `cgr.dev/chainguard/minio:latest`. Named volume only. Ports 9000 and 9001. Command `server /data --console-address ":9001"`. No `MINIO_DOMAIN`. No `curl` healthcheck and no shell entrypoint. A one-shot `cgr.dev/chainguard/minio-client` runs `mc mb` against the bucket. `EDGEQUAKE_ORIGINAL_STORE_CREATE_BUCKET` stays off unless the endpoint is the compose host. The service comment says single-node parity 0 is a dev store.
- `.env.example` — the variables in [04](04-architecture.md)
- `deploy/kubernetes/helm/edgequake/values.yaml` — `originalStore.mode`, URL, pathStyle, allowHttp. Secrets come from an existing secret key, not from values committed with a password.
- `AGENTS.md` environment table — one row for `EDGEQUAKE_ORIGINAL_STORE`

No `docker.io/minio/minio` and no `quay.io/minio/minio`.

## W6 — End-to-end

Recommendation R6. One test binary `e2e_spec172_originals` under `edgequake/crates/edgequake-storage/tests/` or `edgequake/crates/edgequake-api/tests/`, gated on `EDGEQUAKE_ORIGINAL_STORE_E2E=1` so default `cargo test` stays hermetic.

The test starts against MinIO (compose profile or testcontainers-equivalent already used in this repo; if none exists, the test skips unless `EDGEQUAKE_ORIGINAL_STORE_ENDPOINT` is set). Flow:

1. Boot API in `object` mode, path-style, allow-http.
2. Upload a PDF and a non-PDF original.
3. Range download 206 and a full original download. Bytes match.
4. Assert `documents.content` still holds the text and `pdf_data` is null.
5. Delete the document. Row is gone. Object is gone after the reaper, or immediately if delete succeeded.
6. Seed `BYTEA` rows, run `originals plan`, `copy`, `verify`, `clear`.
7. Download again. Bytes match. `BYTEA` is null.
8. Repeat a database-mode upload in the same file, behind a config flag, and assert a `BYTEA` write and zero PUTs.

GCS and Azure are not a second implementation. A unit test parses `gs://` and `az://` into the right builder type. Live cloud accounts are out of the merge gate.

## Wave to recommendation

| Wave | Recommendation | What "done" means |
|------|----------------|-------------------|
| W0 | R0 | Validator exits 0 |
| W1 | R1 | InMemory tests, crate pin written into [03](03-object-store-theory.md) |
| W2 | R2 | Migration 215 applies after 214 |
| W3 | R3, R7 | Upload and range download in object mode |
| W4 | R4, R8 | CLI plan/copy/verify/clear/restore |
| W5 | R5 | Compose profile uses Chainguard only |
| W6 | R6 | MinIO e2e of the flow above |

## Order

```text
W0 spec -> W1 client -> W2 SQL -> W3 server -> W4 CLI -> W5 deploy -> W6 e2e
```

W4 can start once W1 and W2 are merged. It does not wait for the health field in W3. W6 waits for W3, W4, and W5.
