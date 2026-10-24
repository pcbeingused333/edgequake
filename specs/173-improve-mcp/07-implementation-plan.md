# 07 — Implementation plan

Parent: [README](README.md) · Prev: [06 Edge cases](06-edge-cases.md) · Next: [08 E2E test matrix](08-e2e-test-matrix.md)

This pack is wave W0. Waves W1–W8 are the code. Each wave is shippable on its own and leaves the server booting. Do not start W3 before W2: a presigned URL without a row cannot be committed on another replica.

## DRY and SOLID

| Principle | How the waves honor it |
|-----------|------------------------|
| One reason to change | Presign changes only when SigV4 changes. The relay changes only when the HMAC format changes. Admission changes only inside `admit_bytes`, which 173 does not rewrite. |
| Open for a new backend | `select_transfer` treats `NotSupported` as relay. A new `Signer` impl needs no MCP edit. |
| Liskov | Commit always returns the envelope `admit_bytes` already returns, or an existing `eq/*` error. |
| Small interfaces | Handlers take a staging key and a byte reader. They do not take `AmazonS3`. |
| One function for the real work | `admit_bytes` admits. `object_key` formats. `signed_url_opts` signs. The reaper deletes keys the row names. |

Do not copy the SPEC-172 reaper. Call a new query that returns staging keys. Do not copy `put_bytes`. Call it.

## W0 — This pack

Done when `python3 specs/173-improve-mcp/scripts/validate-cross-ref.py` exits 0.

No runtime change.

## W1 — Schema and the alias

Files:

- [`tools.rs`](../../edgequake/crates/edgequake-api/src/mcp/gateway/tools.rs) — `chunk_base64` in the write schema; begin description names the PUT; commit `idempotentHint: true`
- [`upload_session.rs`](../../edgequake/crates/edgequake-api/src/mcp/project/upload_session.rs) — read `chunk_base64`, else `data_base64`
- [`mcp/tests/server.test.ts`](../../mcp/tests/server.test.ts) — the advertised field

Proves [EC-173-22](06-edge-cases.md) and the description half of [F-173-2](02-code-facts.md).

Exit: `cargo test -p edgequake-api --lib upload` and the existing [`spec161_mcp_control_e2e.rs`](../../edgequake/crates/edgequake-api/tests/spec161_mcp_control_e2e.rs) upload cases, updated to send `chunk_base64` and one case that sends only `data_base64`.

## W2 — Session row

New migration `edgequake/migrations/219_spec173_mcp_upload_sessions.sql`.

Columns: `upload_id` (primary key), `tenant_id`, `workspace_id`, `user_id` nullable, `filename`, `media_type`, `byte_length`, `sha256`, `staging_key` nullable, `transfer` (`presign`, `relay`, `chunk`), `bind` (`capability`, `bearer`), `status` (`open`, `committed`, `aborted`, `expired`), `document_id`, `task_id`, `commit_status`, `expires_at`, `created_at`.

RLS: enable and force, predicate matching migration 167’s documents predicate (`tenant_id` and optional `workspace_id`). Grant to `edgequake_tenant_access` the same way migration 215 grants the originals tables. Index `(status, expires_at)`.

Repository functions in `edgequake-storage`, not in the handler: insert, fetch-for-update, mark committed, mark aborted, list reclaimable staging keys.

`eq_upload_begin` inserts the row. The in-memory map is still filled for `transfer = chunk` until W5 removes it. Presign and relay do not use the map.

Proves the row half of [EC-173-21](06-edge-cases.md) with two connections in one test, not two OS processes yet.

Exit: migration test plus a round-trip insert under a tenant GUC that cannot see another tenant’s row ([EC-173-04](06-edge-cases.md) at SQL level).

## W3 — Presign

New `edgequake-storage/src/originals/presign.rs`.

- Add `OwnerKind::Upload` in [`key.rs`](../../edgequake/crates/edgequake-storage/src/originals/key.rs).
- Function `presign_put(store, key, expires, content_type, sha256_hex) -> Url` calls `signed_url_opts` with `Method::PUT` only.
- Signed headers: `content-type`, and `x-amz-checksum-sha256` set to the standard base64 of the 32 raw digest bytes.
- On `NotSupported`, return a typed error the API maps to relay. Do not catch it and call `signed_url` without headers.

`select_transfer` chooses presign only when the mode is `object`, bind is `capability`, and presign returns a URL.

Begin writes the staging key on the row before it returns the URL.

Unit test with the memory store expects `NotSupported` and therefore relay. Unit test with a mocked signer trait (a one-method test double in the storage crate, not a live AWS account) expects PUT and both headers.

Proves [EC-173-17](06-edge-cases.md), [EC-173-19](06-edge-cases.md), [EC-173-28](06-edge-cases.md).

## W4 — Relay

New route `PUT /api/v1/mcp/uploads/{upload_id}` in a new handler module. Body limit for that route is the row’s `byte_length`, not `MCP_MAX_BODY_BYTES`.

HMAC-SHA256, hex, over the concatenation of `upload_id`, `exp`, `sha256`, `content-type`, and `byte_length` separated by newlines. Key: `EDGEQUAKE_MCP_UPLOAD_HMAC_KEY`, else the JWT secret. Refuse the default insecure secret the startup check already knows.

Database mode writes the temp file with `tokio::fs` the way `eq_upload_write` does, then commit reads it. Object mode calls `put_bytes` to the staging key.

Bearer bind checks the same authenticator `/mcp` uses, then the HMAC. Both must succeed.

Tracing fields: `upload_id`, status, byte length. Not the query, not the signature, not the authorization header.

Proves [EC-173-14](06-edge-cases.md), [EC-173-18](06-edge-cases.md), [EC-173-26](06-edge-cases.md), [EC-173-30](06-edge-cases.md).

## W5 — Commit, abort, reaper

`commit_upload_id` loads the row with `SELECT … FOR UPDATE`.

- Terminal `committed`: return the stored envelope.
- `aborted` or past `expires_at`: `eq/not_found`.
- Owner miss: `eq/forbidden` ([EC-173-04](06-edge-cases.md), [EC-173-25](06-edge-cases.md)).
- Read staging, check length and hash, call `admit_bytes`, store ids, delete staging, set `committed`.

The `UPDATE … WHERE status = 'open'` makes the concurrent commit return the winner’s ids ([EC-173-12](06-edge-cases.md)).

Abort sets `aborted` and deletes staging ([EC-173-06](06-edge-cases.md)).

Reaper: a task beside the SPEC-172 reaper, separate function, selects reclaimable keys, deletes, nulls `staging_key`. No `ListObjects` ([EC-173-20](06-edge-cases.md)).

Remove the `HashMap` once chunked begin also inserts the row and chunked write appends to the temp file named by the row. Chunked commit calls the same function.

Proves [EC-173-01](06-edge-cases.md) at the commit hash, [EC-173-05](06-edge-cases.md), [EC-173-10](06-edge-cases.md), [EC-173-11](06-edge-cases.md), [EC-173-29](06-edge-cases.md).

## W6 — Instructions and the bridge

- [`profile.rs`](../../edgequake/crates/edgequake-api/src/mcp/project/profile.rs) instruction from [05](05-contract-delta.md)
- [`docs/integrations/mcp.md`](../../docs/integrations/mcp.md) sequence
- `mcp/src` helper `uploadLocalFile(path)` that calls begin, PUTs, and commits, and refuses a path that escapes the configured root. The remote server still has no `file://` argument ([EC-173-23](06-edge-cases.md)).

The helper is optional for hosts that already have a shell. It is the path for a desktop host that will not PUT on its own ([F-173-12](02-code-facts.md)).

## W7 — Docs pin and OpenAPI

If the relay route is in the public REST surface, regenerate the OpenAPI artifact the repo already generates for API routes (`make codegen-openapi-refresh` when that target covers it). The MCP tools are not OpenAPI. Do not invent a second spec file for them.

`.env.example` documents:

- `EDGEQUAKE_MCP_UPLOAD_TTL_SECS` (default 900, max 900)
- `EDGEQUAKE_MCP_UPLOAD_BIND` (`capability` default, or `bearer`)
- `EDGEQUAKE_MCP_UPLOAD_HMAC_KEY` (optional; otherwise the JWT secret)

## W8 — E2E

New `edgequake/crates/edgequake-api/tests/e2e_spec173_mcp_upload.rs`.

MinIO cases run only when `EDGEQUAKE_ORIGINAL_STORE_E2E=1`, the same opt-in as [`e2e_spec172_originals.rs`](../../edgequake/crates/edgequake-api/tests/e2e_spec172_originals.rs). They use `cgr.dev/chainguard/minio:latest`.

Database-mode cases run without MinIO: relay HMAC, oversize, profile, scope, schema alias, cross-workspace.

The matrix in [08](08-e2e-test-matrix.md) is the list. A case that needs a second process uses two app states over one database, which is enough to prove the row is the shared lock ([EC-173-21](06-edge-cases.md)).

Exit command:

```bash
cargo test -p edgequake-api --features postgres --test e2e_spec173_mcp_upload
EDGEQUAKE_ORIGINAL_STORE_E2E=1 cargo test -p edgequake-api --features postgres --test e2e_spec173_mcp_upload
python3 specs/173-improve-mcp/scripts/validate-cross-ref.py
```

## Order

```text
W0 spec
  |
  v
W1 schema alias          (no new failure mode for current chunk clients)
  |
  v
W2 session row
  |
  +--> W3 presign ----+
  |                    |
  +--> W4 relay -------+--> W5 commit / abort / reaper
                              |
                              v
                         W6 instructions + bridge
                              |
                              v
                         W7 env + OpenAPI if the route is public
                              |
                              v
                         W8 e2e green, including MinIO when the env is set
```

W3 and W4 can proceed in parallel after W2. W5 needs both.
