# 08 — E2E test matrix

Parent: [README](README.md) · Prev: [07 Implementation plan](07-implementation-plan.md) · Next: [09 Cross-ref](09-cross-ref.md)

Cases live in `edgequake/crates/edgequake-api/tests/e2e_spec173_mcp_upload.rs` unless a row says otherwise. MinIO rows skip unless `EDGEQUAKE_ORIGINAL_STORE_E2E=1`. Database rows always run.

Each T-173 id proves the EC-173 id of the same number. Edge-case text is in [06](06-edge-cases.md).

| Test | Proves | Mode | Assert |
|------|--------|------|--------|
| T-173-01 | EC-173-01 | MinIO presign, and a relay unit | PUT with the wrong body fails checksum; commit of a swapped object is `eq/invalid_id` |
| T-173-02 | EC-173-02 | MinIO and relay | Wrong `content-type` does not admit |
| T-173-03 | EC-173-03 | Either | Expired row: commit is `eq/not_found` and the relay is 404 |
| T-173-04 | EC-173-04 | Postgres | Other workspace: `eq/forbidden`; RLS hides the row |
| T-173-05 | EC-173-05 | Either | Second commit returns the same ids and the admit count stays 1 |
| T-173-06 | EC-173-06 | Either | Abort then commit is `eq/not_found` |
| T-173-07 | EC-173-07 | Either | Oversize begin has no `url` and is `eq/budget_exceeded` |
| T-173-08 | EC-173-08 | Either | Zero length is `eq/invalid_id` and has no `url` |
| T-173-09 | EC-173-09 | Either | `../café.pdf` stores a sanitized name and a uuid key |
| T-173-10 | EC-173-10 | Either | Same bytes twice: second status is `duplicate_processing` |
| T-173-11 | EC-173-11 | Either | Commit before PUT is `eq/not_found` |
| T-173-12 | EC-173-12 | Postgres | Two concurrent commits: one admit, identical ids |
| T-173-13 | EC-173-13 | Unit | Mint refuses `exp` beyond 900s; verify rejects `now - exp > 30s` |
| T-173-14 | EC-173-14 | Relay | Captured tracing fields omit `sig`, the presign query, and `Authorization` |
| T-173-15 | EC-173-15 | Gateway | Query profile omits the tools and rejects the call; `memory` lists them |
| T-173-16 | EC-173-16 | Gateway | Body of `MCP_MAX_BODY_BYTES + 1` is rejected; the PUT route accepts a larger declared body under the cap |
| T-173-17 | EC-173-17 | Memory store and Azure double | `NotSupported` yields `transfer: relay` and no bucket host in `url` |
| T-173-18 | EC-173-18 | Object mode with bearer bind | No bearer is 403; bearer plus HMAC is 204; `url` host is the EdgeQuake host |
| T-173-19 | EC-173-19 | Unit | `presign_put` is the only signer call and the method is PUT |
| T-173-20 | EC-173-20 | MinIO | After commit the staging key HEAD is not found; reaper clears a key left on a crashed commit |
| T-173-21 | EC-173-21 | Two app states, one database, MinIO | Begin on A, commit on B, document id returned |
| T-173-22 | EC-173-22 | Gateway | `data_base64` alone writes; schema JSON contains `chunk_base64` |
| T-173-23 | EC-173-23 | Gateway | Extra `url` or `file://` does not change the staging key; commit does not fetch it |
| T-173-24 | EC-173-24 | Gateway | Begin without `sha256` is `eq/invalid_id` and has no `url` |
| T-173-25 | EC-173-25 | Postgres | Different `user_id` is `eq/forbidden`; a null user does not forbid a tenant match |
| T-173-26 | EC-173-26 | Relay | Body longer than `byte_length` is 413 and commit does not admit |
| T-173-27 | EC-173-27 | Gateway | Token without `edgequake:write` is `insufficient_scope` |
| T-173-28 | EC-173-28 | Config unit | Without `ALLOW_HTTP`, the presign scheme is `https` |
| T-173-29 | EC-173-29 | Either | PDF begin admits through the PDF response shape; a `.txt` begin admits through the document shape |
| T-173-30 | EC-173-30 | Relay | Tampered `sig` is 403; a body that does not match the signed hash does not admit |

## Gates that already exist and must stay green

- [`spec161_mcp_control_e2e.rs`](../../edgequake/crates/edgequake-api/tests/spec161_mcp_control_e2e.rs) upload begin / write / commit / abort
- [`spec028_mcp_transport.rs`](../../edgequake/crates/edgequake-api/tests/spec028_mcp_transport.rs) body-limit assertion
- [`mcp/tests/server.test.ts`](../../mcp/tests/server.test.ts) tool name list, including the four `eq_upload_*` tools

W1 updates the spec161 write payload to `chunk_base64` and adds the alias call. It does not delete those tests.

## Out of the MinIO gate

T-173-04, T-173-07, T-173-08, T-173-15, T-173-16, T-173-22, T-173-23, T-173-24, T-173-27 run with the database store and the mock or local auth the other MCP e2e tests already use. They do not need a bucket.
