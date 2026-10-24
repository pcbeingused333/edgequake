# 13 — Implementation plan

Parent: [README](README.md) · Prev: [12-edge-cases](12-edge-cases.md) ·
Next: [14-e2e-test-matrix](14-e2e-test-matrix.md)

W0 is this pack. W1–W11 are later changes. A wave does not start until the
previous wave's tests are green. No wave ships a surface that breaks a
[LAW-174](01-first-principles.md).

## Modules (DRY / SOLID)

| Piece | Crate | Depends on | Must not |
|-------|-------|------------|----------|
| `allows`, `SubjectClearance`, `ObjectLabel` | `edgequake-core/src/clearance/` | nothing I/O | SQL, Axum, React |
| Tag, rule, contribution repository | `edgequake-storage` | core types | HTTP status codes |
| `VisibilityScope` built in `bind_request` | `edgequake-api` services | core `allows` + storage | a second match in a handler |
| Query id list | existing `allowed_document_ids` | the scope | a new query mode |
| Ingest writer | `edgequake-pipeline` merger | contribution repository | end-user clearance as the write scope |
| WebUI | `edgequake_webui/src/lib/clearance/` | HTTP | local allow/deny math |

`RbacService::Permission` is not extended. Operation checks stay in
`require_request_permission`. Object checks stay in `allows`.

Handlers that today take only `TenantContext` gain the scope by reading
the request extension. They do not call `security_role_rules` queries
of their own. MCP dispatch uses the same extension
([LAW-174-5](01-first-principles.md)).

## Waves

### W1 — Pure policy

Unit tests for [EC-174-03](12-edge-cases.md) through
[EC-174-06](12-edge-cases.md), rank 0, unverified versus uploader, steward
plus deny. No migration.

**DoD:** `cargo test -p edgequake-core clearance` green. No API behavior change.

### W2 — Migration 220, enforcement off

Tables, columns, leakproof functions, `clearance_guard` whose first clause
is enforcement off. No backfill of ranks. Acc path untouched
([EC-174-35](12-edge-cases.md)).

**DoD:** migration applies on a head-219 database. A workspace with no
settings row still returns the full corpus. `cargo test` RLS smoke:
enforcement off ignores NULL rank.

### W3 — PDP and GUCs

`bind_request` loads assignments, compiles the subject, stores
`VisibilityScope`. `set_tenant_context` writes the clearance GUCs with
`set_config(..., true)`. Break-glass flag read once at process start.
Admin and master key go through the same compiler
([EC-174-30](12-edge-cases.md), [EC-174-34](12-edge-cases.md)).

**DoD:** a member with no role, enforcement on (set by a test fixture, not
by the enable API yet), sees no document rows under the tenant role.

### W4 — Row-owned reads and writes

List, get, download, original, PDF, assets, pages, lineage, tasks.
Upload and label patch. Tag delete 409. Bulk upload per file.
Markdown handler uses the tenant context ([EC-174-29](12-edge-cases.md)).
Enable transaction ([EC-174-08](12-edge-cases.md)).

**DoD:** [EC-174-01](12-edge-cases.md), [EC-174-09](12-edge-cases.md),
[EC-174-11](12-edge-cases.md), [EC-174-13](12-edge-cases.md),
[EC-174-37](12-edge-cases.md), [EC-174-40](12-edge-cases.md),
[EC-174-41](12-edge-cases.md). The list key query applies
`clearance_covers` before metadata blobs are loaded
([RB-174-02](17-roadblocks.md)).

### W5 — Query intersection

`execute_query`, chat completion, chat streaming set
`allowed_document_ids` from the scope, then intersect the client filter.
All five modes. Empty filter is the visible set
([EC-174-14](12-edge-cases.md), [EC-174-15](12-edge-cases.md)).

**DoD:** a query citation document id is always in the visible set. Prompt
fixture contains no hidden chunk.

### W6 — Graph projection

Contribution rows on ingest when enforcement is on. BFS, search, popular,
degree, and PNG read fragments. Enforcement off keeps the current merger
([EC-174-16](12-edge-cases.md) through [EC-174-19](12-edge-cases.md)).

**DoD:** HR and R&D fixtures share an entity name; each user sees one
fragment. A pre-existing merged description is withheld unless both
sources are visible ([EC-174-42](12-edge-cases.md)). Search does not use
the security-definer function while enforcement is on
([RB-174-04](17-roadblocks.md)).

### W7 — Stores of text

Context cache key, retrieval id binding, answer-cache test that prompts
differ, conversation redaction, share refusal, stats cache key, websocket
filter, task error strip, dedup, glossary
([EC-174-10](12-edge-cases.md), [EC-174-20](12-edge-cases.md) through
[EC-174-28](12-edge-cases.md), [EC-174-38](12-edge-cases.md)).

**DoD:** user B cannot `eq_fetch` user A's id. Dedup response has no hidden id.

### W8 — WebUI

Chips, upload bounds, enable dialog, members column, empty states,
simulate banner. Both route trees in
[10-frontend-architecture](10-frontend-architecture.md). Playwright on
list, detail, graph, query.

**DoD:** hidden title string is absent from the other user's DOM.

### W9 — MCP

Same scope on catalog, search, fetch, graph, image, download, asset, ingest.
[EC-174-32](12-edge-cases.md) API key.

**DoD:** MCP and REST 404 the same hidden id.

### W10 — Matrix and EXPLAIN

Remaining e2e: [EC-174-33](12-edge-cases.md), [EC-174-36](12-edge-cases.md),
[EC-174-39](12-edge-cases.md), pool test [EC-174-34](12-edge-cases.md).
One EXPLAIN measurement from [16-performance](16-performance.md).

**DoD:** every row in the matrix is green or explicitly mapped to a unit
test that is green.

### W11 — Contract surface

OpenAPI regen, Rust SDK, `.env.example` entry for
`EDGEQUAKE_CLEARANCE_BREAK_GLASS` (default unset). Operator note in
`docs/security/best-practices.md`: enforcement off by default, break-glass
audited, Acc leaves the flag off. Point the SPEC-158 peer line at this
pack in a docs-only edit if that README is touched for another reason.
Do not create `specs/146-*`.

**DoD:** `cargo test -p edgequake-api --test spec027_api_contract` and
`cargo fmt --check` on the touched crates.

## Order of checks inside one request

```text
  1. Authentication          (SPEC-154 / SPEC-158, unchanged)
  2. Operation RBAC          require_request_permission
  3. Compile subject         allows() inputs
  4. SET LOCAL tenant + clearance GUCs
  5. Handler uses VisibilityScope
  6. SQL restrictive policies AND
```

Step 2 before step 5 is why readonly-plus-clearance cannot upload
([EC-174-31](12-edge-cases.md)).

## Out of wave scope

Per-document shares, Cedar/OPA, environment attributes, splitting entity
nodes per label, re-enabling conversation share, changing Acc scores.
See [README](README.md) non-goals.

Next: [14-e2e-test-matrix](14-e2e-test-matrix.md).
