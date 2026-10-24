# 02 — Code facts

Parent: [README](README.md) · Prev: [01 First principles](01-first-principles.md) · Next: [03 Protocol](03-protocol.md)

Facts below are what the tree does on 2026-10-24. Each one is a finding id. Defects say what 173 changes. Constraints say what 173 must not bypass.

## F-173-1 — The MCP body cap is 1 MiB

[`MCP_MAX_BODY_BYTES`](../../edgequake/crates/edgequake-api/src/mcp/gateway/body.rs) is `1024 * 1024`. [`routes.rs`](../../edgequake/crates/edgequake-api/src/routes.rs) applies it with `DefaultBodyLimit::max` on `POST /mcp`. SPEC-161 locked this as LAW-161-3. SPEC-162 repeated it as a non-goal.

A single `tools/call` cannot carry a PDF. 173 does not raise the constant.

## F-173-2 — The schema and the handler name different chunk fields

[`eq_upload_write_tool`](../../edgequake/crates/edgequake-api/src/mcp/gateway/tools.rs) requires `data_base64`. [`eq_upload_write`](../../edgequake/crates/edgequake-api/src/mcp/project/upload_session.rs) reads `chunk_base64` and returns `eq/invalid_id` when that field is not standard base64. An agent that follows the advertised schema sends a field the server ignores, then fails or writes an empty chunk.

The unit tests in the same file pass `chunk_base64`. They do not pass `data_base64`.

173 aligns the schema on `chunk_base64` and accepts `data_base64` as a one-release alias ([LAW-173-9](01-first-principles.md)).

## F-173-3 — The session is a process-local map and a temp file

`McpUploadStore` holds `Mutex<HashMap<String, Session>>`. `eq_upload_begin` writes an empty file at `{temp}/edgequake-mcp-uploads/{upload_id}`. `eq_upload_write` appends. Nothing in Postgres names the upload.

Begin on one replica and commit on another cannot see the map or the file. 173 adds `mcp_upload_sessions` ([LAW-173-7](01-first-principles.md)).

## F-173-4 — `user_id` is stored and never checked

`Session.user_id` is copied from `TenantContext` and marked `#[allow(dead_code)]` with the comment that it is for a future ownership check. `owner_matches` compares `workspace_id` and `tenant_id` only.

173 makes `user_id` part of the check when both sides have one ([LAW-173-7](01-first-principles.md), [EC-173-25](06-edge-cases.md)).

## F-173-5 — Commit already funnels into one admission function

`commit_upload_id` reads the whole temp file, checks optional `byte_length` and optional `sha256`, then calls `admit_bytes`. `admit_bytes` sends a PDF (name ends in `.pdf`, or media type `application/pdf`) to `admit_pdf_bytes`. Everything else goes through `resolve_upload_content` and `admit_document_for_processing`, then `persist_uploaded_original` when the postgres feature is on.

A committed session returns `idempotent_commit_envelope` without a second admit. Aborted and expired sessions return `eq/not_found`.

173 keeps this function as the only admission ([LAW-173-6](01-first-principles.md), [LAW-173-10](01-first-principles.md)). The read moves from the temp file to the staging object when the transfer is a presign.

## F-173-6 — Checksum and length are optional on begin

`eq_upload_begin` stores `sha256` and `byte_length` only when the caller sent them. The tool schema requires `filename` and `byte_length`. It does not require `sha256`. A signed URL cannot bind a checksum the caller did not declare.

173 requires `sha256` on every signed transfer ([LAW-173-2](01-first-principles.md), [EC-173-24](06-edge-cases.md)).

## F-173-7 — Upload TTL defaults to one hour and is not capped

[`upload_ttl_secs`](../../edgequake/crates/edgequake-api/src/mcp/project/blob.rs) reads `EDGEQUAKE_MCP_UPLOAD_TTL_SECS` and defaults to 3600. Any positive integer is accepted. Expiry is a process `Instant`, so two replicas do not share a clock for this value.

173 caps the URL at 900 seconds and stores `timestamptz` ([LAW-173-7](01-first-principles.md)).

## F-173-8 — The tree never calls `Signer`

`object_store` `0.14.2` is a dependency of `edgequake-storage` with features `aws`, `gcp`, and `azure`. [`originals/io.rs`](../../edgequake/crates/edgequake-storage/src/originals/io.rs) uses `put` and ranged `get`. No file in that crate calls `signed_url` or `signed_url_opts`.

SPEC-172 [LAW-172-5](../172-object-store/01-first-principles.md) says the bucket is private and that spec does not mint presigned URLs. The security lens of SPEC-172 leaves a later spec to add a short-lived URL after the workspace check. 173 is that later spec, and only for PUT.

`AmazonS3` implements `Signer`. GCS implements `signed_url_opts`. Azure keeps the default, which returns `NotSupported` when options are non-empty, so a checksum header cannot be dropped on the floor ([03](03-protocol.md)).

## F-173-9 — Object keys have no upload kind

[`OwnerKind`](../../edgequake/crates/edgequake-storage/src/originals/key.rs) is `pdf`, `original`, `asset`, or `markdown`. `object_key` refuses to put the filename in the key. A presign that wrote the canonical pdf key would make an uncommitted PUT look like a live original.

173 adds kind `upload` and never inserts that key into `original_object_refs` ([LAW-173-6](01-first-principles.md)).

## F-173-10 — Write scope and the query profile already gate these tools

[`required_scope_for_tool`](../../edgequake/crates/edgequake-api/src/oauth/scopes.rs) maps the four `eq_upload_*` tools to `MCP_SCOPE_WRITE`. [`gateway/mod.rs`](../../edgequake/crates/edgequake-api/src/mcp/gateway/mod.rs) returns `insufficient_scope` and a `WWW-Authenticate` challenge when the grant does not cover it. An empty grant is not allow-all.

[`tool_validation.rs`](../../edgequake/crates/edgequake-api/src/mcp/gateway/tool_validation.rs) rejects those four names when `mcp_profile()` does not advertise writes, with the existing message `tool requires control profile`. [`tools.rs`](../../edgequake/crates/edgequake-api/src/mcp/gateway/tools.rs) omits them from `tools/list` in that profile. `McpProfile::Memory` is an alias of control ([`profile.rs`](../../edgequake/crates/edgequake-api/src/mcp/project/profile.rs)).

173 does not add a fifth upload tool and does not weaken either gate ([LAW-173-8](01-first-principles.md)).

## F-173-11 — Instructions still describe a chunked upload

The begin tool description says to follow with `eq_upload_write` and `eq_upload_commit`. The server instructions in `profile.rs` say ingest and upload are async and that commit returns `pending`. They do not say that a PDF is a PUT, and they do not say to avoid repeating a URL.

[`docs/integrations/mcp.md`](../../docs/integrations/mcp.md) lists the four tools as "Chunked upload". 173 updates the description, the instructions, and that page ([05](05-contract-delta.md)).

## F-173-12 — The stdio bridge forwards arguments and does not read a file

[`callRemoteTool`](../../mcp/src/gateway-bridge.ts) posts `tools/call` with the arguments it was given. It does not open a local path and it does not PUT. A host that only has the bridge, and no shell, still cannot move a PDF until the bridge grows the helper in [LAW-173-10](01-first-principles.md).

## What already works and stays

| Behavior | Where |
|----------|--------|
| Sanitize the filename before it is stored | `sanitize_filename` in `file_validation.rs` |
| PDF vs other admission | `admit_bytes` |
| Idempotent commit envelope | `idempotent_commit_envelope` |
| Pending task and `eq_task_get` | envelope `poll` block |
| Private bucket, id keys, no presigned GET | SPEC-172 |
| 50 MiB cap | `originals/cap.rs` `MAX_UPLOAD_BYTES` |
