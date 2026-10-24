# 03 — Protocol and the October 2026 practice

Parent: [README](README.md) · Prev: [02 Code facts](02-code-facts.md) · Next: [04 Architecture](04-architecture.md)

This page records what EdgeQuake adopts from the published MCP revision and from the October 2026 file-upload work, and what it refuses. Laws that follow from it are in [01](01-first-principles.md).

## Published protocol

The pin is [MCP 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/changelog).

Adopted, because the server already speaks it:

- Tools are model-controlled. The host should still be able to deny a call. Begin and commit stay non-read-only in `annotations` ([tools](https://modelcontextprotocol.io/specification/2026-07-28/server/tools)).
- `tools/list` is deterministic for a given profile and grant, so a prompt cache can hit. 173 does not add or rename a tool ([LAW-173-8](01-first-principles.md)).
- Results that finish carry `resultType: "complete"`. EdgeQuake's envelope stays the SPEC-152 object inside the tool result. 173 does not switch the gateway to multi-round-trip `input_required`.
- Cross-call state is an explicit server-minted handle in a tool argument (changelog item on SEP-2567). `upload_id` is that handle. There is no `Mcp-Session-Id`.
- Resources and `BlobResourceContents` remain the way a small result comes back. A PDF download stays paged under the SPEC-162 65536 byte cap. A large original is not stuffed into a resource ([resources](https://modelcontextprotocol.io/specification/2026-07-28/server/resources)).

Refused, because it is not the upload problem:

- URL-mode elicitation, as specified for 2025-11-25 and then folded into multi-round-trip requests. Opening a browser is not how a coding agent uploads a file it already has.
- The experimental tasks extension. `eq_task_get` already polls the pipeline task.

## The file-shaped hole

[Discussion #1197](https://github.com/modelcontextprotocol/modelcontextprotocol/discussions/1197) (updated July 2026) states the production pattern this spec implements:

```text
  user or agent has the file
           |
           v
  host uploads bytes to storage the server minted
           |
           v
  tools/call carries an opaque handle
           |
           v
  server checks the bytes and returns a small result
```

The same note lists the properties 173 treats as laws:

| Property in #1197 | Where 173 puts it |
|-------------------|-------------------|
| Bind the handle to the user, the server, and the intended tool | [LAW-173-7](01-first-principles.md) tenant, workspace, user; the only tool that finalizes is `eq_upload_commit` |
| Short expiry, preferably one redemption | [LAW-173-7](01-first-principles.md) 900 second cap; commit and abort retire the row |
| Check length, media type, digest, and filename on the server | [LAW-173-2](01-first-principles.md), [LAW-173-6](01-first-principles.md), `sanitize_filename` |
| Server-minted URL, never a URL the model supplies | [LAW-173-1](01-first-principles.md), [EC-173-23](06-edge-cases.md) |
| Small result, not the bytes | commit returns ids and `pending` |

## SEP-2631 and the working group

[SEP-2631](https://github.com/modelcontextprotocol/modelcontextprotocol/pull/2631) proposed `files/authorizeUpload` and `files/authorizeDownload`, with file inputs as URI strings. It was closed on 2026-10-14. The closer pointed at the experimental extension and recorded the October 2 decision: the server returns the upload target from the tool call, so it sees the full call before any byte moves.

[Discussion #3412](https://github.com/modelcontextprotocol/modelcontextprotocol/discussions/3412) (2 Oct 2026) is the source for two decisions in this pack:

1. Metadata first, then the upload target, then the bytes, then finalize. That is [LAW-173-2](01-first-principles.md). SEP-2631 moved bytes before metadata. The group called that the flaw.
2. The data plane must allow both a bearer token and a pre-signed URL. Enterprise notes in that meeting called a pre-authorized URL a non-starter when the token must not reach a third-party store. The group kept the spec generic. 173 uses presign on S3 and GCS ([LAW-173-4](01-first-principles.md)) and same-origin bearer-capable relay everywhere else, including an explicit `EDGEQUAKE_MCP_UPLOAD_BIND=bearer` switch ([LAW-173-5](01-first-principles.md)).

The [experimental extension](https://github.com/modelcontextprotocol/experimental-ext-file-uploads/pull/1) is a draft. 173 does not implement it. The result fields `method`, `url`, `headers`, and `sha256` are ordinary JSON so a later adapter can map them. They are not an extension capability flag.

[PR #3446](https://github.com/modelcontextprotocol/modelcontextprotocol/pull/3446) (21 Oct 2026) adds large downloads to that working group's charter. 173 does not. Downloads stay on the API ([LAW-173-3](01-first-principles.md)).

## Signer, as the crate actually behaves

`object_store` 0.14.2, already in [`edgequake-storage/Cargo.toml`](../../edgequake/crates/edgequake-storage/Cargo.toml):

- [`Signer::signed_url_opts`](https://docs.rs/object_store/0.14.2/object_store/signer/trait.Signer.html) folds [`SignedUrlOptions`](https://docs.rs/object_store/0.14.2/object_store/signer/struct.SignedUrlOptions.html) into the signature. The recipient must send those headers with those values.
- The default implementation returns `NotSupported` when options are non-empty. It must not drop them. Azure documents that SAS cannot sign arbitrary headers, so Azure stays on the default.
- S3 (SigV4) and GCS (GOOG4) implement the method. The crate change is [arrow-rs-object-store #771](https://github.com/apache/arrow-rs-object-store/pull/771).
- The same trait docs say presigning one request is in scope and orchestrating a multipart upload is not. 173 does not presign parts ([LAW-173-10](01-first-principles.md)).

S3 checks `x-amz-checksum-sha256` when the client sends it ([checking object integrity](https://docs.aws.amazon.com/AmazonS3/latest/userguide/checking-object-integrity.html)). Binding that header into the presign means a different body fails at the bucket, before commit. Commit still hashes the bytes EdgeQuake reads. The ETag is not the checksum. SPEC-172 already says that, and it stays true for the staging object.

MinIO is the local S3. SPEC-172 pins `cgr.dev/chainguard/minio:latest`. 173 uses that image for the presign e2e. It does not add a second MinIO distribution.
