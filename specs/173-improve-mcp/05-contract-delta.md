# 05 — Contract delta

Parent: [README](README.md) · Prev: [04 Architecture](04-architecture.md) · Next: [06 Edge cases](06-edge-cases.md)

EQ-MCP-1.3 changes the upload tools' arguments, the begin result, and the words the agent reads. It does not add a tool name. Schema files that mirror these tools stay under [SPEC-152](../152-new-mcp-contract/README.md), which [LAW-161-9](../161-improve-mcp/01-first-principles.md) already names as the schema home. 173 updates the gateway builder in `tools.rs` and the copy the stdio tests assert.

## `eq_upload_begin`

Required: `filename`, `byte_length`, `sha256`.

Optional: `media_type` (default `application/octet-stream`), `workspace_id` (still the `x-mcp-header` the gateway already maps).

`sha256` is 64 hex characters. Mixed case is accepted and compared case-insensitive, as `commit_upload_id` already does.

Success envelope (fields inside the existing SPEC-152 envelope, not a new top-level shape):

```json
{
  "upload_id": "upl_…",
  "transfer": "presign",
  "method": "PUT",
  "url": "https://…",
  "headers": {
    "content-type": "application/pdf",
    "x-amz-checksum-sha256": "…"
  },
  "expires_at": "2026-10-24T05:45:00Z",
  "sha256": "…",
  "byte_length": 4281932,
  "status": "open"
}
```

`transfer` is `relay` when [LAW-173-5](01-first-principles.md) applies. `headers` then still includes `content-type`. The checksum header name is omitted on the relay, because EdgeQuake checks the body hash itself. The HMAC query is part of `url` and is not also copied into `headers`.

Text content that accompanies the structured object:

```text
Upload is open. PUT the file bytes to url with exactly the headers shown.
Do not repeat the url. Do not paste the file. Then call eq_upload_commit
with this upload_id and poll eq_task_get until indexed.
```

Errors, all existing codes from [`ErrorCode`](../../edgequake/crates/edgequake-api/src/mcp/project/errors.rs):

| Condition | Code |
|-----------|------|
| `byte_length` over the cap | `eq/budget_exceeded` |
| `byte_length` < 1, or `sha256` missing or not 64 hex chars | `eq/invalid_id` |
| Query profile | existing control-profile rejection in `tool_validation.rs` |
| Grant lacks `edgequake:write` | `insufficient_scope` from `gateway/mod.rs` |
| Relay requested and no MAC key (default JWT secret, and no `EDGEQUAKE_MCP_UPLOAD_HMAC_KEY`) | `eq/not_implemented` |

Begin does not return a URL in any error.

## `eq_upload_write`

Stays for a small chunked payload. Schema required fields become `upload_id`, `offset`, `chunk_base64`. The handler accepts `data_base64` only when `chunk_base64` is absent, for one release ([LAW-173-9](01-first-principles.md)).

Description states that a PDF uses the PUT from begin, and that this tool is for a chunk that fits in the 1 MiB MCP body.

A write against a row whose `transfer` is `presign` or `relay` returns `eq/invalid_id` with the message that the bytes belong on the PUT. Mixing chunked writes and a signed PUT on one id is rejected.

## `eq_upload_commit`

Arguments stay `upload_id` plus optional `title` and `workspace_id`.

Success is the envelope `admit_bytes` already builds: `document_id`, `task_id`, `status` `pending` or `duplicate_processing`, `ready: false`, and `poll.tool = eq_task_get`.

A second commit of the same id returns that same envelope and does not admit again. `annotations.idempotentHint` on this tool changes from `false` to `true`.

| Condition | Code |
|-----------|------|
| Unknown, expired, or aborted id | `eq/not_found` |
| Other workspace or other user | `eq/forbidden` |
| Length or sha256 mismatch, or staging object missing | `eq/invalid_id` for a hash or length miss; `eq/not_found` when the bytes are absent (today's "upload bytes missing") |

## `eq_upload_abort`

Unchanged arguments. Success sets `status: aborted` and deletes staging. A second abort of an already aborted id is `eq/not_found`, matching today's missing-session behavior after the row is terminal. Idempotent hint stays `true` for the case where the delete is repeated by the reaper, not for a second tool call.

## Relay route

`PUT /api/v1/mcp/uploads/{upload_id}`

Query: `exp` (unix seconds), `sig` (hex HMAC-SHA256).

Headers the client must send: `content-type` equal to the row, `content-length` equal to `byte_length`.

Bearer mode also requires `Authorization` with the same token the host uses for `/mcp`.

Responses: `204` on success, `403` on a bad signature or a bad bearer, `404` on an unknown or expired id, `409` when the row is already committed or aborted, `413` when the body exceeds the declared length.

`GET` and `POST` on that path are `405`. There is no list route.

## Instructions and docs

[`profile.rs`](../../edgequake/crates/edgequake-api/src/mcp/project/profile.rs) gains one instruction, after the existing async-ingest sentence:

```text
A file such as a PDF uses eq_upload_begin, then an HTTP PUT of the raw bytes
to the returned url with the returned headers, then eq_upload_commit.
Do not place file bytes or the url in the chat. Poll eq_task_get until indexed.
```

[`docs/integrations/mcp.md`](../../docs/integrations/mcp.md) replaces the "Chunked upload" cell with the begin / PUT / commit / poll sequence and links this spec. The page's protocol header is already `2026-07-28`.

The stdio README in `mcp/` records the optional local-path helper as not part of the remote contract. Until that helper exists, a coding agent uses its own HTTP client.

## What does not change

- Tool names, including the legacy `edgequake_search` aliases
- `MCP-Protocol-Version`
- `eq_ingest`, delete, download, and graph tools
- Download routes and the absence of a presigned GET
- The 50 MiB cap and the 1 MiB MCP body
