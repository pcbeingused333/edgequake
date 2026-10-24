# 09 — Cross-ref

Parent: [README](README.md) · Prev: [08 E2E test matrix](08-e2e-test-matrix.md)

Run `python3 scripts/validate-cross-ref.py` from this directory. It checks in-pack links and that every id below appears where it must.

## Laws

| Id | Statement | Why | Proved by |
|----|-----------|-----|-----------|
| LAW-173-1 | Bytes stay out of JSON-RPC. No caller-supplied URL. | [WHY-173-1](00-why.md) | [EC-173-16](06-edge-cases.md), [EC-173-23](06-edge-cases.md) |
| LAW-173-2 | Metadata, target, bytes, finalize, poll. | [WHY-173-3](00-why.md) | [EC-173-07](06-edge-cases.md), [EC-173-08](06-edge-cases.md), [EC-173-24](06-edge-cases.md) |
| LAW-173-3 | One result shape. PUT only. No presigned GET. | [WHY-173-2](00-why.md) | [EC-173-14](06-edge-cases.md), [EC-173-19](06-edge-cases.md) |
| LAW-173-4 | S3 and GCS bind content-type and checksum. | [WHY-173-2](00-why.md) | [EC-173-01](06-edge-cases.md), [EC-173-02](06-edge-cases.md) |
| LAW-173-5 | Database, Azure, and bearer bind use the HMAC relay. | [WHY-173-2](00-why.md) | [EC-173-17](06-edge-cases.md), [EC-173-18](06-edge-cases.md), [EC-173-30](06-edge-cases.md) |
| LAW-173-6 | Staging key, then `admit_bytes`. | [WHY-173-4](00-why.md) | [EC-173-06](06-edge-cases.md), [EC-173-10](06-edge-cases.md), [EC-173-20](06-edge-cases.md), [EC-173-29](06-edge-cases.md) |
| LAW-173-7 | Postgres row, RLS, 900 second cap, user check. | [WHY-173-5](00-why.md) | [EC-173-03](06-edge-cases.md), [EC-173-04](06-edge-cases.md), [EC-173-13](06-edge-cases.md), [EC-173-21](06-edge-cases.md), [EC-173-25](06-edge-cases.md) |
| LAW-173-8 | No new MCP method. Scope and query profile stay. | [WHY-173-6](00-why.md) | [EC-173-15](06-edge-cases.md), [EC-173-27](06-edge-cases.md) |
| LAW-173-9 | Schema advertises `chunk_base64`. Alias for one release. | [WHY-173-1](00-why.md) | [EC-173-22](06-edge-cases.md) |
| LAW-173-10 | One admission function. Web UI and the 50 MiB cap stay. | [WHY-173-4](00-why.md) | [EC-173-09](06-edge-cases.md), [EC-173-29](06-edge-cases.md) |

Full text: [01](01-first-principles.md).

## Findings

| Id | Fact | Law that answers it |
|----|------|---------------------|
| F-173-1 | MCP body is 1 MiB | LAW-173-1 |
| F-173-2 | Schema says `data_base64`, handler reads `chunk_base64` | LAW-173-9 |
| F-173-3 | Session is an in-memory map and a temp file | LAW-173-7 |
| F-173-4 | `user_id` is stored and never checked | LAW-173-7 |
| F-173-5 | Commit already calls `admit_bytes` | LAW-173-6, LAW-173-10 |
| F-173-6 | `sha256` is optional on begin | LAW-173-2 |
| F-173-7 | TTL defaults to 3600 and uses `Instant` | LAW-173-7 |
| F-173-8 | The tree never calls `Signer` | LAW-173-4 |
| F-173-9 | No `upload` object kind | LAW-173-6 |
| F-173-10 | Write scope and the query profile already gate the tools | LAW-173-8 |
| F-173-11 | Instructions still say chunked upload | LAW-173-3 |
| F-173-12 | The stdio bridge does not PUT a local file | LAW-173-10 |

Full text: [02](02-code-facts.md).

## Edge cases

EC-173-01 checksum mismatch. EC-173-02 wrong content-type. EC-173-03 expired URL. EC-173-04 cross-workspace. EC-173-05 replay commit. EC-173-06 abort then commit. EC-173-07 oversize. EC-173-08 empty. EC-173-09 path in filename. EC-173-10 duplicate hash. EC-173-11 missing bytes. EC-173-12 concurrent commit. EC-173-13 clock skew. EC-173-14 URL absent from logs. EC-173-15 query profile. EC-173-16 1 MiB body. EC-173-17 signer fallback. EC-173-18 bearer bind. EC-173-19 no presigned GET. EC-173-20 staging deleted. EC-173-21 other replica. EC-173-22 schema alias. EC-173-23 caller-supplied URL. EC-173-24 missing sha256. EC-173-25 user mismatch. EC-173-26 length mismatch. EC-173-27 missing write scope. EC-173-28 https presign. EC-173-29 PDF versus text. EC-173-30 HMAC versus body.

Narrative: [06](06-edge-cases.md). Tests T-173-01 through T-173-30: [08](08-e2e-test-matrix.md).

## Lenses

| Lens | Owns |
|------|------|
| [Product owner](lenses/LENS-product-owner.md) | LAW-173-1, LAW-173-2, LAW-173-10, EC-173-07, EC-173-10, EC-173-15 |
| [Full stack](lenses/LENS-full-stack.md) | LAW-173-6, LAW-173-9, LAW-173-10, EC-173-05, EC-173-12, EC-173-21, EC-173-22 |
| [Database](lenses/LENS-database.md) | LAW-173-7, EC-173-04, EC-173-12, EC-173-20, EC-173-21, EC-173-25 |
| [UX and UI](lenses/LENS-ux-ui.md) | LAW-173-3, EC-173-03, EC-173-07, EC-173-11, EC-173-14 |
| [Front](lenses/LENS-front.md) | LAW-173-1, LAW-173-3, LAW-173-10, EC-173-19, EC-173-22, EC-173-23 |
| [AI engineer](lenses/LENS-ai-engineer.md) | LAW-173-1, LAW-173-3, LAW-173-4, LAW-173-8, EC-173-01, EC-173-15, EC-173-16, EC-173-24 |
| [MCP](lenses/LENS-mcp.md) | LAW-173-8, LAW-173-9, EC-173-05, EC-173-15, EC-173-19, EC-173-22, EC-173-27 |
| [Security](lenses/LENS-security.md) | LAW-173-1, LAW-173-3, LAW-173-4, LAW-173-5, LAW-173-7, EC-173-01, EC-173-04, EC-173-14, EC-173-18, EC-173-19, EC-173-23, EC-173-25, EC-173-27, EC-173-30 |
| [Object store](lenses/LENS-object-store.md) | LAW-173-4, LAW-173-6, LAW-173-10, EC-173-17, EC-173-19, EC-173-20, EC-173-28 |

## Waves

W0 this pack. W1 schema alias. W2 migration 219. W3 presign. W4 relay. W5 commit and reaper. W6 instructions and the bridge. W7 env and OpenAPI. W8 e2e. Detail: [07](07-implementation-plan.md).

## Inherited specs

- [SPEC-161](../161-improve-mcp/README.md) — control tools and the 1 MiB body
- [SPEC-162](../162-improve-mcp/README.md) — handles and poll until indexed
- [SPEC-172](../172-object-store/README.md) — private bucket; [LAW-172-5](../172-object-store/01-first-principles.md) still forbids a presigned GET
- [SPEC-152](../152-new-mcp-contract/README.md) — envelope and schema home
- [SPEC-154](../154-sec-hardening/README.md) — scopes
