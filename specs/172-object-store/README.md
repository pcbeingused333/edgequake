# SPEC-172 — Original documents in an object store

> **Status:** Specification. Implementation follows [06](06-implementation-plan.md).
> **Date:** 2026-10-23
> **Product pin:** EdgeQuake v0.37.0 (schema head is migration 218)
> **Inherits:** [SPEC-090](../090-performance/README.md) (PDF bytes left Postgres hot rows and stayed in `BYTEA`) · [SPEC-083](../083-improvements/README.md) (50 MiB cap, no raw multipart paths) · PDF trait in `edgequake-storage/src/pdf_storage.rs`
> **Peers:** [SPEC-150](../150-reliable-migration-system/README.md) (schema migrate stays separate from this data move) · [SPEC-154](../154-sec-hardening/README.md) (tenant bind) · migration 167 (RLS)

## What operators and implementers need to know

| Goal | Do this |
|------|---------|
| Understand why originals are still in Postgres | Read [00-why](00-why.md) |
| Know the laws | Read [01-first-principles](01-first-principles.md) |
| See the code that stores the bytes | Read [02-code-facts](02-code-facts.md) |
| See the backend contracts | Read [03-object-store-theory](03-object-store-theory.md) |
| See the target shape | Read [04-architecture](04-architecture.md) |
| Cover edge cases | Read [05-edge-cases](05-edge-cases.md) |
| Ship in waves | Follow [06-implementation-plan](06-implementation-plan.md) |
| Prove every edge case | Run gates in [07-e2e-test-matrix](07-e2e-test-matrix.md) |
| Run the move | Follow [08-migration-ux](08-migration-ux.md) |
| Check cross-refs | [09-cross-ref](09-cross-ref.md) and `python3 scripts/validate-cross-ref.py` |
| Read one lens | [`lenses/`](lenses/) |

## Start here

1. [00-why.md](00-why.md)
2. [01-first-principles.md](01-first-principles.md)
3. [02-code-facts.md](02-code-facts.md)
4. [03-object-store-theory.md](03-object-store-theory.md)
5. [04-architecture.md](04-architecture.md)
6. [05-edge-cases.md](05-edge-cases.md)
7. [06-implementation-plan.md](06-implementation-plan.md)
8. [07-e2e-test-matrix.md](07-e2e-test-matrix.md)
9. [08-migration-ux.md](08-migration-ux.md)
10. [09-cross-ref.md](09-cross-ref.md)
11. Lenses: [product](lenses/LENS-product-owner.md) · [full stack](lenses/LENS-full-stack.md) · [database](lenses/LENS-database.md) · [UX](lenses/LENS-ux.md) · [front](lenses/LENS-front.md) · [object store](lenses/LENS-object-store.md) · [security](lenses/LENS-security.md) · [system design](lenses/LENS-system-design.md)

## One-screen job

```text
┌────────────────────────────────────────────────────────────┐
│ today                                                      │
│ upload writes BYTEA                                        │
│ pdf_document_blobs.pdf_data                                │
│ document_originals.original_data                           │
└────────────────────────────┬───────────────────────────────┘
                             │                                
              ┌──────────────┴──────────────┐                 
              v                             v                 
┌──────────────────────────┐  ┌──────────────────────────────┐
│ database mode (default)  │  │ object mode                  │
│ same BYTEA path          │  │ PUT then pointer in Postgres │
│ no bucket required       │  │ download routes unchanged    │
└──────────────────────────┘  └──────────────────────────────┘
                             │                                
                             v                                
┌────────────────────────────────────────────────────────────┐
│ stays in Postgres                                          │
│ documents.content, chunks, embeddings, graph, markdown     │
└────────────────────────────────────────────────────────────┘
```

`EDGEQUAKE_ORIGINAL_STORE=database` is the default. `object` is opt-in. Search and the viewer markdown do not move.

## Locked decisions

| # | Decision | Law |
|---|----------|-----|
| 1 | Spec number is **172**. The next SQL file is migration **215**. Migration `172_request_path_lookup_indexes.sql` already exists and is unrelated. | — |
| 2 | Original bytes are `pdf_document_blobs.pdf_data` and `document_originals.original_data`. Retrieval text stays in `documents.content`. | LAW-172-1 |
| 3 | One client: the Apache `object_store` crate. S3, MinIO, GCS, and Azure Blob are configurations of that client. | LAW-172-2 |
| 4 | Local S3 is `cgr.dev/chainguard/minio:latest` only. Docker Hub and Quay `minio/minio` are forbidden. | LAW-172-2 |
| 5 | PUT the object, then commit the Postgres pointer. Inflight is committed before the PUT. The reaper deletes queued keys only. | LAW-172-3 |
| 6 | The migrator copies, verifies SHA-256, then clears `BYTEA` only when asked. `plan` prints the relation size and the free space `VACUUM FULL` needs. The CLI role is `BYPASSRLS` and is not the API role. | LAW-172-4 |
| 7 | Downloads stay on the existing API routes. `get_pdf` fills `pdf_data` from the object when the column is null, because the converter and the original download read that field. The bucket is private. This spec does not issue presigned URLs. | LAW-172-5 |
| 8 | The server and `edgequake originals` call one module for keys, put, ranged get, and verify. | LAW-172-6 |
| 9 | Every replica uses the same mode. Boot refuses a mismatch with the pointer rows it can see. `database` never opens a bucket. | LAW-172-7 |
| 10 | Object keys are ids. Filenames stay in Postgres and in `Content-Disposition`. | LAW-172-8 |
| 11 | This pack is the specification. Implementation follows [06](06-implementation-plan.md). | — |

## Non-goals

- Moving `documents.content`, `chunks`, embeddings, the graph, or `pdf_document_blobs.markdown_content`
- Moving `document_mm_assets.asset_data` (derived page PNGs, migration 084)
- Presigned browser uploads or downloads
- A fifth client beside `object_store`
- Changing `MAX_UPLOAD_BYTES` (50 MiB)
- Replacing `edgequake migrate` (schema). `edgequake originals` moves bytes only
- Public buckets, object ACLs, or treating the key prefix as authorization

## Recommendations

R1 through R8 are the wave map in [06](06-implementation-plan.md). R1 is the shared client. R2 is migration 215. R3 is the upload and download path. R4 is the CLI. R5 is compose, Helm, and health. R6 is the MinIO e2e. R7 is the reaper. R8 is restore.
