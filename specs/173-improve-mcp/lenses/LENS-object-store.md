# Lens — Object store

Parent: [README](../README.md) · Laws: [LAW-173-4](../01-first-principles.md), [LAW-173-6](../01-first-principles.md) · Facts: [F-173-8](../02-code-facts.md), [F-173-9](../02-code-facts.md)

## One client

Signing uses the `object_store` `0.14.2` `Signer` the workspace already depends on. There is no second S3 SDK. `presign_put` calls `signed_url_opts` and no other signer entry point ([EC-173-19](../06-edge-cases.md)).

| Backend | Behavior |
|---------|----------|
| `AmazonS3`, including MinIO | Presign PUT with `content-type` and `x-amz-checksum-sha256` |
| GCS | Presign PUT with the signed headers that impl accepts |
| Azure | `NotSupported` when headers are set. Begin uses relay ([EC-173-17](../06-edge-cases.md)) |
| `InMemory` | Same `NotSupported` path. Tests do not pretend it signed |

The checksum header value is standard base64 of the raw 32-byte digest, matching [S3 checksums](https://docs.aws.amazon.com/AmazonS3/latest/userguide/checking-object-integrity.html). Hex in the tool argument stays hex. The presign function converts. Commit hashes the bytes it reads and compares hex. The ETag is not a checksum, which SPEC-172 already states.

## Keys

`OwnerKind::Upload` extends [`object_key`](../../../edgequake/crates/edgequake-storage/src/originals/key.rs). The key is `{prefix}/t/{tenant}/w/{workspace}/upload/{uuid}`. The filename stays in Postgres ([LAW-172-8](../../172-object-store/01-first-principles.md)).

That key is staging. `original_object_refs` points at `pdf`, `original`, `asset`, or `markdown` only. Commit’s call to `admit_bytes` is what writes the canonical object, through the SPEC-172 put path. Then the staging key is deleted ([EC-173-20](../06-edge-cases.md)).

A crash between admit and delete leaves a row the reaper can see. The reaper does not `ListObjects`. SPEC-172 [LAW-172-3](../../172-object-store/01-first-principles.md) gives the reason: a truncated list can miss or, worse, invite a bug that deletes keys the table does not own.

## Local and production URLs

MinIO stays `cgr.dev/chainguard/minio:latest`, path-style, `ALLOW_HTTP` only when the endpoint is set for local. A presign without that flag is `https` ([EC-173-28](../06-edge-cases.md)). Production buckets stay private. This spec does not add a bucket policy statement for `s3:GetObject` to `*`.

## What this lens refuses

- Multipart upload IDs and per-part presigns. The `Signer` docs put multipart orchestration out of scope, and the cap is 50 MiB ([LAW-173-10](../01-first-principles.md)).
- Presign of the canonical pdf key. An abandoned PUT would look like a live original ([F-173-9](../02-code-facts.md)).
- Skipping `put_bytes` inside admission and writing a second put implementation for the canonical key.
