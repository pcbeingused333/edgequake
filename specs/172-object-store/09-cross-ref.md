# 09 — Cross-ref matrix

Parent: [README](README.md) · Prev: [08 Migration UX](08-migration-ux.md)

Validate with `python3 scripts/validate-cross-ref.py`.

## Law, WHY, finding, edge case, wave, test, lens

| Law | WHY | Findings | Edge cases | Wave | Tests | Lens |
|-----|-----|----------|------------|------|-------|------|
| LAW-172-1 | WHY-172-1, WHY-172-2 | F-172-01, F-172-05, F-172-08, F-172-10, F-172-11 | EC-172-04, EC-172-05, EC-172-24, EC-172-26, EC-172-27, EC-172-32 | W2, W3 | T-172-04, T-172-05, T-172-24, T-172-26, T-172-27, T-172-32 | product, database |
| LAW-172-2 | WHY-172-4 | F-172-09, F-172-21 | EC-172-11, EC-172-12, EC-172-13, EC-172-19, EC-172-35, EC-172-38, EC-172-39 | W1, W5 | T-172-11, T-172-12, T-172-13, T-172-19, T-172-35, T-172-38, T-172-39 | object-store |
| LAW-172-3 | WHY-172-2 | F-172-02, F-172-04, F-172-19 | EC-172-01, EC-172-02, EC-172-08, EC-172-22, EC-172-33, EC-172-34, EC-172-40 | W2, W3 | T-172-01, T-172-02, T-172-08, T-172-22, T-172-33, T-172-34, T-172-40 | system-design, database |
| LAW-172-4 | WHY-172-5 | F-172-15, F-172-20 | EC-172-03, EC-172-16, EC-172-17, EC-172-21, EC-172-28, EC-172-36, EC-172-37 | W4 | T-172-03, T-172-16, T-172-17, T-172-21, T-172-28, T-172-36, T-172-37 | ux, database |
| LAW-172-5 | WHY-172-3 | F-172-07, F-172-12 | EC-172-06, EC-172-07, EC-172-09, EC-172-25, EC-172-30 | W3 | T-172-06, T-172-07, T-172-09, T-172-25, T-172-30 | security, front |
| LAW-172-6 | WHY-172-4, WHY-172-5 | F-172-03, F-172-06, F-172-13, F-172-17, F-172-18 | EC-172-14, EC-172-29, EC-172-31 | W1, W3, W4 | T-172-14, T-172-29, T-172-31 | full-stack |
| LAW-172-7 | WHY-172-3 | F-172-14, F-172-16 | EC-172-10, EC-172-15, EC-172-20, EC-172-23 | W3, W5 | T-172-10, T-172-15, T-172-20, T-172-23 | system-design, product |
| LAW-172-8 | WHY-172-4 | F-172-07 | EC-172-18 | W1 | T-172-18 | security, object-store |

## Finding to symbol

| Finding | Symbol |
|---------|--------|
| F-172-01 | `pdf_document_blobs.pdf_data` |
| F-172-02 | `create_pdf` BYTEA insert |
| F-172-03 | `get_pdf_bytes_range` |
| F-172-04 | `octet_length(pdf_data)` |
| F-172-05 | `document_originals.original_data` |
| F-172-06 | `DocumentOriginal.original_data` |
| F-172-07 | `download.rs` original handler |
| F-172-08 | `MAX_UPLOAD_BYTES` |
| F-172-09 | no `object_store` crate |
| F-172-10 | `documents.content` list guard |
| F-172-11 | `document_mm_assets.asset_data` |
| F-172-12 | `pdf_byte_stream` |
| F-172-13 | `MIGRATE_USAGE` |
| F-172-14 | migration 214 versus spec 172 |
| F-172-15 | `sha256_checksum` |
| F-172-16 | `HealthResponse.storage_mode` |
| F-172-17 | `pdf_processing` reads `pdf.pdf_data` |
| F-172-18 | `download_document_original` uses `get_pdf` |
| F-172-19 | workspace wipe set delete |
| F-172-20 | `app.current_tenant_id` is transaction-local |
| F-172-21 | `validate_pdf_data` ignores the env cap |

## WHY to question

| WHY | Question |
|-----|----------|
| WHY-172-1 | Why do original documents sit in the database? |
| WHY-172-2 | Why is that the wrong place for the bytes? |
| WHY-172-3 | Why a config mode, not a forced cutover? |
| WHY-172-4 | Why one crate for S3, GCS, Azure, and MinIO? |
| WHY-172-5 | Why an independent migration tool? |

## Wave to recommendation

| Wave | Recommendations | Edge-case map in [05](05-edge-cases.md) |
|------|-----------------|------------------------------------------|
| W0 | R0 this pack | — |
| W1 | R1 client | EC-172-04, EC-172-05, EC-172-11, EC-172-18, EC-172-19, EC-172-23, EC-172-25, EC-172-38, EC-172-39 |
| W2 | R2 schema | EC-172-09, EC-172-20, EC-172-32, EC-172-33 |
| W3 | R3 server, R7 reaper | EC-172-01, EC-172-06, EC-172-07, EC-172-08, EC-172-10, EC-172-12, EC-172-14, EC-172-22, EC-172-24, EC-172-30, EC-172-31, EC-172-34, EC-172-40 |
| W4 | R4 CLI, R8 restore | EC-172-02, EC-172-03, EC-172-13, EC-172-15, EC-172-16, EC-172-17, EC-172-21, EC-172-26, EC-172-27, EC-172-28, EC-172-29, EC-172-36, EC-172-37 |
| W5 | R5 deploy | EC-172-12, EC-172-35 |
| W6 | R6 MinIO e2e | EC-172-06, EC-172-07, EC-172-09 |

## Prior specs

| Spec | What this pack takes from it |
|------|------------------------------|
| [SPEC-090](../090-performance/README.md) | PDF bytes already left the hot row and stayed in `BYTEA`. This pack moves that `BYTEA`, not the list query. |
| [SPEC-083](../083-improvements/README.md) | 50 MiB cap and no raw multipart paths. The object key continues that rule. |
| [SPEC-150](../150-reliable-migration-system/README.md) | Schema migrate stays `edgequake migrate`. Byte move is `edgequake originals`. |
| [SPEC-154](../154-sec-hardening/README.md) | Tenant bind stays. The ref table joins the migration 167 RLS list. |
| [SPEC-171](../171-dataperf/README.md) | Do not hold a row lock across a network call. The migrator commits the claim before PUT. |
