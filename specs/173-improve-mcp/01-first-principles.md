# 01 — First principles

Parent: [README](README.md) · Prev: [00 WHY](00-why.md) · Next: [02 Code facts](02-code-facts.md)

These laws are the contract for [07 Implementation plan](07-implementation-plan.md). Each one answers a WHY in [00](00-why.md). The code that violates them today is in [02](02-code-facts.md). The protocol sources are in [03](03-protocol.md).

## LAW-173-1 — Bytes stay out of the model context

`MCP_MAX_BODY_BYTES` stays `1024 * 1024`. A PDF, or any other binary the agent did not type, does not travel inside `tools/call`.

The remote server rejects a caller-supplied `file://` URI and any URL the caller invented. MCP roots describe a local filesystem the remote server cannot see. They are not authorization and they are not a transfer.

The stdio package may later read a path the user approved and perform the PUT itself ([LAW-173-10](#law-173-10--one-admission-function)). That read happens in the bridge, on the machine that has the file. The gateway still receives only `upload_id`.

WHY: [WHY-173-1](00-why.md).

## LAW-173-2 — Metadata, then the target, then the bytes, then finalize

The order is fixed:

1. `eq_upload_begin` with `filename`, `byte_length`, and, for a signed transfer, `sha256` and `media_type`.
2. The tool result carries `upload_id` and one PUT target.
3. The host PUTs the raw bytes.
4. `eq_upload_commit` checks length and sha256, then admits.
5. The agent polls `eq_task_get` until `indexed`, `failed`, or `cancelled`.

`byte_length` above `max_document_size` (the SPEC-083 cap, 50 MiB unless `EDGEQUAKE_MAX_UPLOAD_BYTES` lowers it) returns `eq/budget_exceeded` and no URL. `byte_length` below 1 is `eq/invalid_id` and no URL.

Commit does not wait for the pipeline. A second commit of a session that already admitted returns the same `document_id` and `task_id`.

WHY: [WHY-173-3](00-why.md).

## LAW-173-3 — One result shape, and the URL is PUT-only

`eq_upload_begin` returns one object for every transfer mode:

- `upload_id`
- `method` = `PUT`
- `url`
- `headers` (the host must send these exactly)
- `expires_at`
- `sha256`
- `byte_length`
- `transfer` = `presign` or `relay`

The human-readable text of the tool result names `upload_id` and tells the model not to repeat the URL and not to paste the file. The URL is still present, because a coding agent host has to read it in order to PUT. Logs omit it ([EC-173-14](06-edge-cases.md)).

The signature is for `PUT` only. This spec does not mint a presigned GET. Downloads stay on the existing API routes. [LAW-172-5](../172-object-store/01-first-principles.md) still holds for reads. SPEC-172 left presigned URLs to a later spec. This is that spec, and it is upload-only.

WHY: [WHY-173-2](00-why.md).

## LAW-173-4 — S3 and GCS bind the checksum into the signature

When `EDGEQUAKE_ORIGINAL_STORE=object` and the backend implements `Signer::signed_url_opts`, begin returns a presigned PUT.

The client is the Apache `object_store` crate already in the workspace (`0.14.2`). `AmazonS3` and Google Cloud Storage implement signed headers. Azure SAS does not. A backend that returns `Error::NotSupported` does not silently drop the headers. It takes the relay in [LAW-173-5](#law-173-5--relay-when-a-bucket-url-cannot-carry-the-constraint).

Signed headers:

- `content-type`, equal to the declared media type
- the checksum header the backend verifies (`x-amz-checksum-sha256` on S3 and MinIO; the GCS equivalent the `Signer` impl documents)

The checksum value is the base64 of the raw SHA-256 bytes, which is what [S3 object integrity](https://docs.aws.amazon.com/AmazonS3/latest/userguide/checking-object-integrity.html) checks. A leaked URL can be used only to write that content type and that hash.

The call lives next to `put_bytes` in `edgequake-storage` `originals`. The MCP handler does not name `AmazonS3`.

`EDGEQUAKE_MCP_UPLOAD_BIND=bearer` skips this branch even on S3. See [LAW-173-5](#law-173-5--relay-when-a-bucket-url-cannot-carry-the-constraint).

WHY: [WHY-173-2](00-why.md). Contract: [03](03-protocol.md). Object lens: [lenses/LENS-object-store.md](lenses/LENS-object-store.md).

## LAW-173-5 — Relay when a bucket URL cannot carry the constraint

Begin returns a same-origin HMAC URL, not a bucket URL, when any of these is true:

- `EDGEQUAKE_ORIGINAL_STORE=database` (the default)
- the backend is Azure, or `signed_url_opts` returns `NotSupported`
- `EDGEQUAKE_MCP_UPLOAD_BIND=bearer`

The URL is `{EDGEQUAKE_PUBLIC_URL}/api/v1/mcp/uploads/{upload_id}` with `exp` and `sig` in the query. `sig` is HMAC-SHA256 over `upload_id`, `exp`, `sha256`, `content-type`, and `byte_length`. The MAC key is `EDGEQUAKE_MCP_UPLOAD_HMAC_KEY` when set, otherwise the configured JWT secret. The default insecure JWT secret cannot sign an upload URL. The handler does not log the query string.

In bearer mode the same route also requires the MCP bearer. The URL alone is then not a capability. Presign mode cannot add that requirement, because the bucket does not know the EdgeQuake token. Enterprise installs that must keep the bearer inside the host set `EDGEQUAKE_MCP_UPLOAD_BIND=bearer` and accept the relay.

The handler streams the body into the staging temp file (database mode) or into the staging object through `put_bytes` (object mode). It checks `Content-Type` and `Content-Length` before it writes.

WHY: [WHY-173-2](00-why.md), [WHY-173-5](00-why.md).

## LAW-173-6 — Staging key, then the existing admission

The object the client writes is a staging key:

```text
{prefix}/t/{tenant}/w/{workspace}/upload/{uuid}
```

`prefix`, tenant, and workspace use the same formatter as [`object_key`](../../edgequake/crates/edgequake-storage/src/originals/key.rs). The new kind is `upload`. The last segment is a hyphenated uuid. The filename is not in the key ([LAW-172-8](../172-object-store/01-first-principles.md)). The client does not choose the key.

`original_object_refs` never points at a staging key. Commit:

1. Reads the staging bytes (object GET, or the temp file in database mode).
2. Rejects a length or sha256 mismatch with `eq/invalid_id`.
3. Calls the existing `admit_bytes`.
4. Deletes the staging object or the temp file after admission succeeds.

Reading up to 50 MiB into `admit_bytes` matches today's temp-file commit. A later wave may stream. It must still call `admit_bytes` or the function `admit_bytes` calls. It must not grow a second PDF pipeline.

Abort deletes the staging object and marks the row aborted. A later PUT that recreates the object is ignored, because commit of an aborted row is `eq/not_found`.

WHY: [WHY-173-4](00-why.md), [WHY-173-5](00-why.md).

## LAW-173-7 — The session row is the shared lock

Upload sessions live in Postgres table `mcp_upload_sessions` (migration 219). Row level security uses the tenant predicate already applied to `documents` in migration 167: `tenant_id = current_tenant_id()` and, when a workspace is set, `workspace_id = current_workspace_id()`. The table enables and forces RLS, the same way `original_object_refs` does in migration 215.

Any replica can commit. Expiry is `timestamptz` compared with database time, not a process `Instant`.

`user_id`, when both the row and the caller have one, must match. A missing user on either side does not widen the check. Tenant and workspace still have to match. The key prefix is not an access check.

The URL lifetime is `min(EDGEQUAKE_MCP_UPLOAD_TTL_SECS, 900)` seconds. The env may only lower the 900 second cap. There is no refresh. A new file is a new begin.

The in-memory `HashMap` remains only until the chunked path writes the same row. After that wave, the map is gone.

WHY: [WHY-173-5](00-why.md). Database lens: [lenses/LENS-database.md](lenses/LENS-database.md).

## LAW-173-8 — The protocol surface does not grow

EdgeQuake does not implement `files/authorizeUpload`, `files/authorizeDownload`, or the experimental extension's RPC. The protocol header stays `MCP-Protocol-Version: 2026-07-28`.

Tools stay the four that exist: `eq_upload_begin`, `eq_upload_write`, `eq_upload_commit`, `eq_upload_abort`. The relay is an HTTP route, not a tool.

`required_scope_for_tool` keeps those four on `edgequake:write`. `EDGEQUAKE_MCP_PROFILE=query` keeps omitting them and keeps rejecting a direct call. `memory` stays an alias of `control`.

WHY: [WHY-173-6](00-why.md). MCP lens: [lenses/LENS-mcp.md](lenses/LENS-mcp.md).

## LAW-173-9 — The advertised schema matches the handler

The handler accepts `chunk_base64`. For one release it also accepts `data_base64` when `chunk_base64` is absent. The tool schema advertises `chunk_base64` and stops advertising `data_base64` as the only field.

Chunked write stays legal for a payload that fits in the 1 MiB body after base64. It is the wrong path for a PDF. The tool description says so. Chunked write stays single-process until it persists the same session row. It is not a second admission implementation: commit still ends in `admit_bytes`.

WHY: [WHY-173-1](00-why.md). Finding: [F-173-2](02-code-facts.md).

## LAW-173-10 — One admission function

`admit_bytes` remains the only function that turns upload bytes into a document and a task. Presign commit, relay commit, and chunked commit all call it.

Out of this spec:

- the Web UI multipart and the document upload routes
- changing `MAX_UPLOAD_BYTES`
- a public bucket, an object ACL, or a presigned GET
- S3 multipart upload IDs and per-part URLs (`Signer` documents that multipart orchestration is out of its scope)
- a fifth object-store client

The stdio bridge in `mcp/` may gain a local-path helper that calls begin, PUTs, and commits. The helper refuses a path outside the user-approved root. The remote server still never opens that path.

WHY: [WHY-173-4](00-why.md).
