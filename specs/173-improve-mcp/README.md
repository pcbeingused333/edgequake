# SPEC-173 — Signed upload for agent memory

> **Status:** Specification. Implementation follows [07](07-implementation-plan.md).
> **Date:** 2026-10-24
> **Document control:** EQ-MCP-1.3 (delta on EQ-MCP-1.2 / SPEC-162)
> **Protocol:** MCP `2026-07-28`. No new RPC. [SEP-2631](https://github.com/modelcontextprotocol/modelcontextprotocol/pull/2631) is closed and is not implemented.
> **Product pin:** EdgeQuake v0.37.0 (schema head is migration 218). The SQL file for the session row, when implementation starts, is migration **219**. Migration `173_injection_list_index.sql` already exists and is unrelated.
> **Inherits:** [SPEC-161](../161-improve-mcp/README.md) (control tools, 1 MiB body) · [SPEC-162](../162-improve-mcp/README.md) (handles, poll until indexed) · [SPEC-172](../172-object-store/README.md) (private bucket, id keys) · [SPEC-152](../152-new-mcp-contract/README.md) · [SPEC-154](../154-sec-hardening/README.md) (scopes)
> **Peers:** [SPEC-028 MCP](../028-edgequake-query-service/mcp/000-index.md)

## What operators and implementers need to know

| Goal | Do this |
|------|---------|
| Understand why base64 cannot carry a PDF | Read [00-why](00-why.md) |
| Know the laws | Read [01-first-principles](01-first-principles.md) |
| See the code that stores the bytes today | Read [02-code-facts](02-code-facts.md) |
| See what the October 2026 protocol does and does not say | Read [03-protocol](03-protocol.md) |
| See the target shape | Read [04-architecture](04-architecture.md) |
| See the tool delta | Read [05-contract-delta](05-contract-delta.md) |
| Cover edge cases | Read [06-edge-cases](06-edge-cases.md) |
| Ship in waves | Follow [07-implementation-plan](07-implementation-plan.md) |
| Prove every edge case | Run gates in [08-e2e-test-matrix](08-e2e-test-matrix.md) |
| Check cross-refs | [09-cross-ref](09-cross-ref.md) and `python3 scripts/validate-cross-ref.py` |
| Read one lens | [`lenses/`](lenses/) |

## Start here

1. [00-why.md](00-why.md)
2. [01-first-principles.md](01-first-principles.md)
3. [02-code-facts.md](02-code-facts.md)
4. [03-protocol.md](03-protocol.md)
5. [04-architecture.md](04-architecture.md)
6. [05-contract-delta.md](05-contract-delta.md)
7. [06-edge-cases.md](06-edge-cases.md)
8. [07-implementation-plan.md](07-implementation-plan.md)
9. [08-e2e-test-matrix.md](08-e2e-test-matrix.md)
10. [09-cross-ref.md](09-cross-ref.md)
11. Lenses: [product](lenses/LENS-product-owner.md) · [full stack](lenses/LENS-full-stack.md) · [database](lenses/LENS-database.md) · [UX](lenses/LENS-ux-ui.md) · [front](lenses/LENS-front.md) · [AI engineer](lenses/LENS-ai-engineer.md) · [MCP](lenses/LENS-mcp.md) · [security](lenses/LENS-security.md) · [object store](lenses/LENS-object-store.md)

## One-screen job

```text
  Model calls eq_upload_begin
  filename, byte_length, sha256
           |
           v
  Tool result: upload_id + PUT url + signed headers
  Model keeps upload_id. Host performs the PUT.
           |
           +-- S3 / GCS object mode --> presigned PUT to staging key
           |
           +-- database, Azure, or bearer bind --> HMAC PUT on this server
           |
           v
  eq_upload_commit(upload_id)
  verify length + sha256, then existing admit_bytes
           |
           v
  pending + task_id ---- eq_task_get ---- indexed
```

Memory is the indexed document. The model context is not a disk. The URL is a short-lived capability for one object of one hash.

## Locked decisions

| # | Decision | Law |
|---|----------|-----|
| 1 | Spec number is **173**. The next SQL file is migration **219**. | — |
| 2 | `MCP_MAX_BODY_BYTES` stays 1 MiB. A PDF does not travel inside `tools/call`. | LAW-173-1 |
| 3 | Order is metadata, upload target, bytes, finalize, then poll. | LAW-173-2 |
| 4 | One result shape. PUT only. No presigned GET. Downloads stay on the API. | LAW-173-3 |
| 5 | S3 and GCS presign through `Signer::signed_url_opts` with content-type and checksum bound into the signature. | LAW-173-4 |
| 6 | Database mode, Azure, and bearer bind use a same-origin HMAC PUT. | LAW-173-5 |
| 7 | The client writes a staging key. Commit calls the existing `admit_bytes`. | LAW-173-6 |
| 8 | The session row is in Postgres under tenant RLS. URL TTL is at most 900 seconds. | LAW-173-7 |
| 9 | No `files/authorizeUpload`. Protocol stays `2026-07-28`. Write scope and the query profile stay. | LAW-173-8 |
| 10 | The schema advertises `chunk_base64`. `data_base64` is a one-release alias. Chunked write is not the PDF path. | LAW-173-9 |
| 11 | One admission function. The Web UI upload, the 50 MiB cap, public buckets, and multipart part URLs stay out. | LAW-173-10 |

## Non-goals

- Raising [`MCP_MAX_BODY_BYTES`](../../edgequake/crates/edgequake-api/src/mcp/gateway/body.rs) or [`MAX_UPLOAD_BYTES`](../../edgequake/crates/edgequake-storage/src/originals/cap.rs) (50 MiB)
- A presigned GET, a public bucket, or a browser download that skips the API ([LAW-172-5](../172-object-store/01-first-principles.md))
- Implementing the closed SEP-2631 RPCs or the unpublished file-uploads extension
- Multipart part URLs (one PUT is enough under the 50 MiB cap)
- A new Web UI upload screen
- Replacing `admit_pdf_bytes` or `admit_document_for_processing`
- Letting the model, or the remote server, read an arbitrary `file://` path

## Official references

- [MCP tools (2026-07-28)](https://modelcontextprotocol.io/specification/2026-07-28/server/tools)
- [MCP resources (2026-07-28)](https://modelcontextprotocol.io/specification/2026-07-28/server/resources)
- [MCP changelog (2026-07-28)](https://modelcontextprotocol.io/specification/2026-07-28/changelog)
- [MCP schema (2026-07-28)](https://modelcontextprotocol.io/specification/2026-07-28/schema)
- [SEP-2631 (closed 2026-10-14)](https://github.com/modelcontextprotocol/modelcontextprotocol/pull/2631)
- [File uploads working group, 2 Oct 2026](https://github.com/modelcontextprotocol/modelcontextprotocol/discussions/3412)
- [Passing files into a tool](https://github.com/modelcontextprotocol/modelcontextprotocol/discussions/1197)
- [Experimental file-uploads extension (draft)](https://github.com/modelcontextprotocol/experimental-ext-file-uploads/pull/1)
- [`object_store` Signer](https://docs.rs/object_store/0.14.2/object_store/signer/trait.Signer.html)
- [`SignedUrlOptions`](https://docs.rs/object_store/0.14.2/object_store/signer/struct.SignedUrlOptions.html)
- [S3 presigned URLs](https://docs.aws.amazon.com/AmazonS3/latest/userguide/using-presigned-url.html)
- [S3 object integrity](https://docs.aws.amazon.com/AmazonS3/latest/userguide/checking-object-integrity.html)
