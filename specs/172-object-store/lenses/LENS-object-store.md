# Lens — Object store

Parent: [README](../README.md) · Laws: [LAW-172-2](../01-first-principles.md), [LAW-172-8](../01-first-principles.md) · Theory: [03](../03-object-store-theory.md)

## Client

One crate: `object_store`, features `aws`, `gcp`, `azure`. Builders: `AmazonS3Builder`, `GoogleCloudStorageBuilder`, `MicrosoftAzureBuilder`.

MinIO is the `aws` builder with a custom endpoint:

- endpoint `http://localhost:9000` (or the in-cluster DNS name)
- `with_allow_http(true)` only for that HTTP endpoint
- `with_virtual_hosted_style_request(false)` because the endpoint is set
- region `us-east-1`
- static keys from `MINIO_ROOT_USER` and `MINIO_ROOT_PASSWORD` in local compose

Real AWS leaves the endpoint unset and uses virtual-hosted style. Path-style on AWS is an explicit override for a bucket whose name contains a dot.

Image: `cgr.dev/chainguard/minio:latest`. Command: `server /data --console-address ":9001"`. Ports 9000 and 9001. Named volume. No `MINIO_DOMAIN`. Do not pull `minio/minio` from Docker Hub or Quay. The image user is 65532 and the image has no `curl` and no `mc`. A one-shot `cgr.dev/chainguard/minio-client` creates the bucket. There is no in-container healthcheck. Readiness is `/ready`. Single-node parity 0 is a development store.

## Addressing

Path-style URL shape, used when an endpoint is set: `http://localhost:9000/{bucket}/{key}`. Virtual-hosted style against this MinIO fails ([EC-172-12](../05-edge-cases.md)). AWS with no endpoint uses virtual-hosted style.

## Integrity

SHA-256 is computed here, stored on the ref, and checked by GET. The ETag is recorded and ignored for equality. Multipart ETags are not MD5 of the body. `If-None-Match` is not part of the write protocol.

`put` for 50 MiB is one request. `put_multipart` is used only if the pinned crate requires it for that size. W1 writes the pinned version into the theory doc after reading that version's docs.

## Layout

Keys are UUID paths under the URL prefix. No filename, no content type in the key. Content type is object metadata and a Postgres column.

## Local compose health

The console on 9001 is for humans. Readiness is EdgeQuake `/ready`, which talks to the S3 API. The image has no shell, so compose does not run `curl` inside it.

## GCS and Azure

Same `put` / `get_opts` / `delete` calls. `gs://` and `az://` (or `abfs://`) select the builder. A container is the Azure bucket. Production does not create buckets. Local MinIO does, via `minio-client` or `EDGEQUAKE_ORIGINAL_STORE_CREATE_BUCKET` when an endpoint is set. The bucket stays private.
