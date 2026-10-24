# 06 — Edge cases

Parent: [README](README.md) · Prev: [05 Contract delta](05-contract-delta.md) · Next: [07 Implementation plan](07-implementation-plan.md)

Every case has a mitigation in the laws and a test id in [08](08-e2e-test-matrix.md). The test id is the same number: EC-173-01 is proved by T-173-01.

## Transfer integrity

**EC-173-01 — Checksum mismatch.**
The host PUTs a body whose SHA-256 is not the hex declared at begin. On S3 and GCS the signed checksum header fails the PUT. Commit also hashes the stored bytes and returns `eq/invalid_id` if they differ. A leaked URL cannot replace the file with different bytes ([LAW-173-4](01-first-principles.md)).

**EC-173-02 — Wrong content-type.**
The PUT sends a content type other than the one signed or stored on the row. S3 rejects a signed-header miss. The relay rejects before writing. Commit does not admit.

**EC-173-03 — Expired URL.**
`exp` or the row `expires_at` is in the past, compared with database time. Commit and the relay return `eq/not_found` / `404`. No URL refresh exists ([LAW-173-7](01-first-principles.md)).

**EC-173-04 — Cross-workspace commit.**
The caller’s tenant or workspace differs from the row. Response is `eq/forbidden`. The staging object is not read into the other workspace.

**EC-173-05 — Replay after success.**
A second `eq_upload_commit` on a committed id returns the stored `document_id` and `task_id` and does not call `admit_bytes` again.

**EC-173-06 — Abort, then PUT, then commit.**
Abort marks the row aborted and deletes staging. A later PUT may recreate an object at a key the reaper will drop. Commit of an aborted id is `eq/not_found` and does not admit.

**EC-173-07 — Oversize declaration.**
`byte_length` above `max_document_size` returns `eq/budget_exceeded` and the response has no `url`.

**EC-173-08 — Empty file.**
`byte_length` of 0 or a missing length returns `eq/invalid_id` and no URL. A PUT of zero bytes against a positive declaration fails the length check.

**EC-173-09 — Path in the filename.**
`../café.pdf`, a backslash path, and a null-byte name are stored only after `sanitize_filename`. The object key’s last segment stays a uuid. The filename is not a storage path ([F-173-9](02-code-facts.md)).

**EC-173-10 — Duplicate content hash.**
The bytes match a document already in the workspace. `admit_bytes` returns `duplicate_processing` as it does today. The session is marked committed with that status. A second document row is not created.

**EC-173-11 — Nothing stored at commit.**
Commit runs before the PUT, or the object was deleted. The response is `eq/not_found` with the existing meaning of missing bytes. The row stays open until expiry or abort, so a late PUT can still be committed once.

**EC-173-12 — Two commits at once.**
Two replicas commit the same open row. One transaction admits. The other sees `committed` and returns the same ids. The admission function runs once. The row update is `UPDATE … WHERE status = 'open'`.

## Capability URL

**EC-173-13 — Clock skew.**
A URL whose `exp` is more than 30 seconds behind database `now()` is rejected. A URL minted with `exp` more than 900 seconds ahead is not produced. The 30 second grace does not stack on top of a longer TTL. Skew the other way (a slow signer) still cannot exceed the cap, because the cap is applied at mint time.

**EC-173-14 — URL in logs.**
Access and tracing lines for begin, the relay, and commit may contain `upload_id`. They must not contain the presigned query, the HMAC `sig`, or the `Authorization` header. The relay router skips the query string in its span.

**EC-173-15 — Query profile.**
`EDGEQUAKE_MCP_PROFILE=query` omits the four upload tools from `tools/list`. A direct `tools/call` hits the existing control-profile rejection in `tool_validation.rs`. `memory` still advertises them.

**EC-173-16 — MCP body still 1 MiB.**
A `tools/call` larger than `MCP_MAX_BODY_BYTES` is rejected by the existing body limit, including a `chunk_base64` that would have fit the document cap. The PUT route is the one that accepts up to the declared length.

**EC-173-17 — Signer cannot bind headers.**
Azure, or any backend whose `signed_url_opts` returns `NotSupported`, does not return a half-signed URL. Begin returns `transfer: relay` ([LAW-173-5](01-first-principles.md)).

**EC-173-18 — Bearer bind.**
`EDGEQUAKE_MCP_UPLOAD_BIND=bearer` forces relay even when S3 could presign. A PUT with a valid HMAC and no bearer is `403`. A PUT with the MCP bearer and a valid HMAC is `204`.

**EC-173-19 — No presigned GET.**
The presign function takes `Method::PUT` only. There is no call with `GET`. A client that changes the method on a copied URL fails at the bucket. Download routes are unchanged.

**EC-173-20 — Staging left behind.**
After commit, abort, or expiry the staging object and the temp file are gone. If the process dies after admit and before delete, the reaper deletes the key from the row and does not list the bucket ([04](04-architecture.md)).

**EC-173-21 — Other replica commits.**
Begin is served by replica A. The PUT lands in the shared bucket or on the relay, which writes the staging key. Commit is served by replica B and succeeds. This is the case the in-memory map fails today ([F-173-3](02-code-facts.md)).

## Contract and identity

**EC-173-22 — Schema alias.**
A write that sends only `data_base64` is accepted for one release. A write that sends `chunk_base64` uses that value. The advertised schema names `chunk_base64`. After the alias release, a later spec may drop `data_base64`. 173 does not drop it.

**EC-173-23 — Caller-supplied location.**
`file://`, `https://evil.example/`, and an extra `url` argument on begin or commit are ignored if unknown and rejected if the tool tried to treat them as the bytes. Commit reads only the staging key stored on the row. The server does not HTTP GET a model-supplied URL.

**EC-173-24 — Signed begin without a hash.**
Missing, short, or non-hex `sha256` returns `eq/invalid_id` and no URL. The chunked path is not a way around this for a new presign or relay row. Chunked rows are a separate `transfer = chunk` and still cannot be given a bucket URL.

**EC-173-25 — User mismatch.**
The row has `user_id` U1 and the caller has `user_id` U2, same workspace. Commit and abort return `eq/forbidden`. If either side has no user id, the tenant and workspace check still applies and the user check does not invent a match.

**EC-173-26 — Length mismatch at commit.**
The stored object’s length differs from `byte_length`. Commit returns `eq/invalid_id` and does not admit. The relay stops reading once the declared length is exceeded and returns `413` without leaving a full extra object.

**EC-173-27 — Missing write scope.**
A token whose scopes do not cover `edgequake:write` receives `insufficient_scope` and the existing `WWW-Authenticate` challenge. An empty scope list does not allow the tool ([F-173-10](02-code-facts.md)).

**EC-173-28 — Plain HTTP presign.**
With `EDGEQUAKE_ORIGINAL_STORE_ALLOW_HTTP` unset, a presigned URL is `https`. Local MinIO sets that flag, as SPEC-172 already requires, and the e2e may see `http://localhost`. Production config that would emit `http` without the flag fails at the existing store builder before a URL is minted.

**EC-173-29 — PDF and non-PDF admission.**
A `.pdf` or `application/pdf` body goes to `admit_pdf_bytes`. A `.txt` body goes to `admit_document_for_processing`. Both return `pending` and a `task_id`. 173 does not add a third branch.

**EC-173-30 — HMAC covers a different hash than the body.**
The relay signature was minted for hash H1. The body hashes to H2, or the caller recomputes `sig` with a different hash without the MAC key. The route returns `403` for a bad `sig`, and commit returns `eq/invalid_id` if a body was stored under a matching signature but the bytes were swapped before commit. The MAC key never appears in the tool result.

## Residual risks, accepted

- The model can read the URL out of the tool result and repeat it in chat. The mitigation is the checksum bind, the 900 second cap, PUT-only, and the instruction not to repeat it. Bearer bind is the setting that removes the bare capability.
- Commit reads up to 50 MiB into memory, as the temp-file path already does. A stream into `admit_bytes` is a later change and must keep one admission function.
- Chunked upload stays single-process until the wave that writes the same row. Until then a chunked begin still fails across replicas. The PDF path does not use that wave as a prerequisite.
