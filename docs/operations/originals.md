---
title: Originals migration
description: Move uploaded document bytes from Postgres into a private object store.
---

# Originals migration

`edgequake originals` copies PDF bytes, uploaded originals, page images, and extracted markdown into the bucket, then checks the hashes. It does not start the API. It does not run `VACUUM FULL`. It does not touch `documents.content` or chunk text. Those stay in Postgres.

The dashboard status line shows **Originals: Database** or **Originals: Object store**. That line is read-only. The move itself is this command.

The code default is `EDGEQUAKE_ORIGINAL_STORE=database`. Object writes happen only when that variable is `object`. `make dev` sets `object` and starts MinIO. `make dev-database` stays on Postgres and refuses `clear`.

## Before you start

1. Apply migrations through **218** (`edgequake migrate`). 215 adds the ref ledger. 216 adds the maintenance role. 217 adds image and markdown refs. 218 adds the PDF window digest column.
2. Point the same variables the server uses at a private bucket that already exists. The process does not create a bucket. `EDGEQUAKE_ORIGINAL_STORE_CREATE_BUCKET=true` is a configuration error.
3. Run the command as a role that can `SET ROLE edgequake_originals_maint`. The API role is not that role.

`make dev` writes the app key into `/tmp/edgequake-minio.env`. That user can read, write, and delete only under the configured prefix. It cannot create a bucket. Do not put the MinIO root key in the API process.

```bash
export DATABASE_URL=postgres://localhost/edgequake
export EDGEQUAKE_ORIGINAL_STORE=object
export EDGEQUAKE_ORIGINAL_STORE_URL=s3://eq-originals/dev
export EDGEQUAKE_ORIGINAL_STORE_ENDPOINT=http://127.0.0.1:19011
export EDGEQUAKE_ORIGINAL_STORE_ALLOW_HTTP=true
export EDGEQUAKE_ORIGINAL_STORE_ACCESS_KEY_ID=eq-originals-app
export EDGEQUAKE_ORIGINAL_STORE_SECRET_ACCESS_KEY=eq-originals-app-dev
```

The host port in the example is the current local MinIO mapping. Read the endpoint from `/tmp/edgequake-minio.env` if `make dev` chose another port. Passwords in `DATABASE_URL` are stripped from the output.

Keys look like `{prefix}/t/{tenant}/w/{workspace}/{kind}/{uuid}`. A replacement writes a new key. The previous key is deleted after the database row commits. A failed commit deletes the new key and leaves the old one.

## The safe order

```bash
edgequake originals              # plan: counts and the free space VACUUM FULL needs
edgequake originals status       # same counts, no disk estimate
edgequake originals copy --limit 10
edgequake originals copy
edgequake originals verify
edgequake originals clear --yes  # only after verify is clean, and only in object mode
```

`migrate` is copy, then verify. It clears only when you also pass `--clear --yes`. `clear` refuses to start without `--yes`. `--clear` on any other verb is a usage error (exit 2). A checksum mismatch prints the owner id and leaves the database copy in place.

`--kind` is `pdf`, `original`, `asset`, `markdown`, `both`, or `all`. `both` is PDF plus original. `all` includes page images and markdown. The default is `all`.

On a terminal, copy progress is one updating line. In a log, it prints every 100 owners. When `EDGEQUAKE_ORIGINALS_LOG` is unset, the command forces `RUST_LOG=error`. Set it to keep your own `RUST_LOG`.

`verify` checks the full SHA-256, then fills any missing PDF window digest. It does not upload again. Do not point `verify` or `clear` at a workspace you have not finished copying.

## If something fails

The last line is `originals_abort=<class>`.

| Class | What to do |
|-------|------------|
| `usage` | Read the usage block. Exit 2. |
| `config` | Fix `EDGEQUAKE_ORIGINAL_STORE*` and run `plan` again. |
| `credentials` | Refresh the key, then `edgequake originals copy`. |
| `checksum` | Do not clear. The owner id is in the line above. |
| `mode` | The server is not in object mode. Restart it that way, then clear. |
| `blocked` | Run `edgequake originals verify`. |
| `database` | Retry `edgequake originals copy`. |

`restore` copies verified objects back into Postgres. Switch `EDGEQUAKE_ORIGINAL_STORE=database` only after that command finishes and you have checked a download.

After `clear`, the table size does not shrink until you choose to run:

```bash
psql -c "VACUUM (FULL, ANALYZE) pdf_document_blobs, document_originals, document_mm_assets, pdf_documents"
```

That lock is yours. The tool will not take it.

## Cloud stores

| Store | URL | Body check |
|-------|-----|------------|
| MinIO or another S3-compatible endpoint | `s3://bucket/prefix` plus `EDGEQUAKE_ORIGINAL_STORE_ENDPOINT` | Read back after each put |
| Amazon S3, no custom endpoint | `s3://bucket/prefix` | The client sends `x-amz-checksum-sha256`. A successful put is the check |
| Google Cloud Storage | `gs://bucket/prefix` | Read back after each put |
| Azure Blob | `az://container/prefix` | Read back after each put |

SSE-S3 (`EDGEQUAKE_OBJECT_STORE_SSE=AES256`) is Amazon S3 only. GCS and Azure refuse it. A KMS key id is refused.

Amazon S3 can abort incomplete multipart uploads with the lifecycle document in `edgequake/docker/minio/aws-abort-multipart.json`. Apply that in the account. The EdgeQuake process does not call `PutBucketLifecycle`. MinIO already expires stale uploads after 24 hours and rejects that lifecycle field. GCS and Azure use the account lifecycle rules. Do not add an expire-object rule. That deletes real documents.

Objects up to 100 MiB are one put. Larger objects use 8 MiB multipart parts. The product upload cap defaults to 50 MiB. `EDGEQUAKE_MAX_UPLOAD_BYTES` may raise it from 1 MiB to 1 GiB.
