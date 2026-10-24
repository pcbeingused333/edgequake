# 12 — Edge cases (EC-174)

Parent: [README](README.md) · Prev: [11-threat-model](11-threat-model.md) ·
Next: [13-implementation-plan](13-implementation-plan.md)

Each case has one primary gate in [14-e2e-test-matrix](14-e2e-test-matrix.md)
([LAW-174-15](01-first-principles.md)). "Unit" means the pure `allows` tests.
"e2e" means a Postgres-backed API test unless the row says Playwright.

## Register

### EC-174-01 — New untagged upload

**Given** enforcement on. **When** a member uploads with no labels.
**Then** the document is `unverified`. The uploader can GET the body. A
peer member receives 404.
**Mitigation:** rank 5 plus uploader exception. [LAW-174-3](01-first-principles.md) · [LAW-174-13](01-first-principles.md).

### EC-174-02 — Member with no security role

**Given** enforcement on and no `membership_security_roles` row.
**Then** list, query, and graph are empty. Stats are zero. The API does
not return the workspace document count.
**Mitigation:** compiled rank 0, default deny. [LAW-174-3](01-first-principles.md).

### EC-174-03 — Deny wins

**Given** role A allows `department:hr` and role B denies `project:atlas`.
**When** both are assigned. **Then** an HR document tagged Atlas is hidden,
and an HR document without Atlas is visible.
**Mitigation:** deny subtracts after union. [05-policy-model](05-policy-model.md).

### EC-174-04 — Equal to the ceiling

**Given** subject rank confidential (3). **When** the document rank is 3.
**Then** read is allowed if compartments match.
**Mitigation:** `<=`, not `<`.

### EC-174-05 — Subject holds a superset

**Given** subject compartments `{hr, legal}`. **When** the document has
`{hr}`. **Then** allow.
**Mitigation:** document set ⊆ subject set.

### EC-174-06 — Document holds both

**Given** subject `{hr}`. **When** the document has `{hr, legal}`.
**Then** deny.
**Mitigation:** subset fails. Unit and one e2e query.

### EC-174-07 — Steward sees unverified

**Given** a steward role with no deny. **Then** rank 5 documents are
visible. A non-steward who calls `POST .../clearance/simulate` receives
403. Their document list is unchanged.
**Mitigation:** steward flag. No simulate header
([RB-174-05](17-roadblocks.md)).

### EC-174-08 — Owner bootstrap

**Given** enable. **Then** the acting owner has the steward role in the
same transaction, existing documents are internal (or unverified if
strict-start), and the owner list is non-empty when the corpus was
non-empty. A failed grant rolls back enforcement.
**Mitigation:** [LAW-174-14](01-first-principles.md).

### EC-174-09 — Uploader after a raise

**Given** the uploader could read their unverified document. **When** a
steward sets confidential and the uploader's rank is internal.
**Then** GET, download, chunks, and contributions 404 for the uploader.
**Mitigation:** exception is rank 5 only. [LAW-174-13](01-first-principles.md).

### EC-174-10 — Relabel during a query

**Given** a query transaction started before a relabel commits.
**Then** that transaction keeps its snapshot. The next request uses the
new epoch. A context-cache entry from the old epoch is not reused.
**Mitigation:** `policy_epoch` in the cache key. PostgreSQL snapshot
isolation.

### EC-174-11 — Delete a tag that is assigned

**When** `DELETE` a compartment still on a document. **Then** 409 and the
label remains.
**Mitigation:** `ON DELETE RESTRICT`.

### EC-174-12 — Orphan chunk or NULL rank

**Given** enforcement on and a chunk whose document rank is NULL, or a
chunk whose document row is gone. **Then** the chunk is not returned by
search or by id.
**Mitigation:** `clearance_covers` is false for NULL. FK cascade is the
normal delete path. [F-174-14](04-findings.md).

### EC-174-13 — Hidden id on every direct route

**When** the caller names a hidden document id. **Then** 404, byte-identical
to an unknown id, on GET document, markdown, original, PDF, asset, page,
lineage, lineage export, and task detail.
**Mitigation:** [LAW-174-4](01-first-principles.md) · [F-174-06](04-findings.md).

### EC-174-14 — Client filter wider than clearance

**When** `document_ids` includes a hidden id and a visible id.
**Then** retrieval uses only the visible id. MCP `document_ids` does the same.
**Mitigation:** [LAW-174-10](01-first-principles.md).

### EC-174-15 — Empty filter

**When** the query body omits `document_filter`. **Then** the id list is
the visible set, not the workspace.
**Mitigation:** handler fills `allowed_document_ids` before the engine.
[F-174-03](04-findings.md).

### EC-174-16 — Merged entity and merged edge

**Given** one entity and one edge extracted from an HR document and an
R&D document. **Then** each analyst receives only their fragment, and the
edge disappears for the analyst who cannot see its only visible support
if the other side is hidden.
**Mitigation:** `graph_contributions`. [LAW-174-9](01-first-principles.md).

### EC-174-17 — Degree

**Given** the node has four edges and two are HR-only. **Then** the R&D
payload `degree` is 2. `eq_graph_nodes.degree` may stay 4 in the table.
**Mitigation:** served degree. [F-174-05](04-findings.md).

### EC-174-18 — Graph PNG

**When** `eq_graph_image` or the graph page renders the node.
**Then** the image bytes do not contain the hidden neighbor's label.
**Mitigation:** PNG input is the served neighborhood.

### EC-174-19 — Popular nodes and label search

**Then** ranking uses visible degree, and search hits come from visible
fragment text. A hidden fragment's unique word does not match.
**Mitigation:** sibling of `eq_graph_search_nodes` under enforcement.

### EC-174-20 — Workspace stats

**Then** document, chunk, entity, and relationship counts are visible
counts. The 60s stats cache key includes the clearance fingerprint and epoch.
**Mitigation:** [F-174-11](04-findings.md).

### EC-174-21 — Pipeline websocket

**Then** events for hidden documents are not sent. The uploader still
receives events for their own unverified upload.
**Mitigation:** filter in the publisher, not only in the client.

### EC-174-22 — Task error text

**Given** a task error that quotes a chunk. **When** the caller cannot
read that document. **Then** the task is 404 if the whole task is about
that document, or the error string is replaced with a generic failure
if the task is workspace-scoped.
**Mitigation:** do not store a second copy of chunk text in `tasks.result`
for display. [F-174-11](04-findings.md).

### EC-174-23 — Conversation replay

**Given** a stored answer whose context cites a document the user could
read, then the role is revoked. **When** history loads. **Then** that
message's context is omitted and the content is replaced with a short
redaction marker that does not quote the text. The next prompt does not
include it.
**Mitigation:** [LAW-174-11](01-first-principles.md).

### EC-174-24 — Share link

**When** enforcement is on and the caller creates a conversation share.
**Then** the API refuses. Existing `share_id` links resolve as not found
while enforcement is on.
**Mitigation:** refuse, rather than re-evaluate a share as the recipient
without a product for that. A later spec can add recipient clearance.

### EC-174-25 — Retrieval id

**When** user B calls `eq_fetch` or `GET /api/v1/query/context/{id}` with
user A's id. **Then** 404. **When** user A calls it and the epoch still
matches. **Then** the bundle.
**Mitigation:** bind id to user, fingerprint, and epoch. [F-174-07](04-findings.md).

### EC-174-26 — Answer cache

**Given** two users who must see different chunks for the same question.
**Then** the prompts differ and the answer-cache keys differ. **Given**
two users with the same visible set and the same question. **Then** they
may share the answer row.
**Mitigation:** hash the filtered prompt only. [F-174-08](04-findings.md).

### EC-174-27 — Context cache

**Then** the context-cache key includes fingerprint and epoch. User B
does not receive user A's `QueryContext`.
**Mitigation:** extend `cache_key` in `query_result_cache.rs`.

### EC-174-28 — Hidden duplicate bytes

**When** user B uploads bytes whose hash matches user A's hidden document.
**Then** the API creates a new document (or rejects with a generic
duplicate message) and does not return A's id, title, or label.
**Mitigation:** dedup lookup goes through `allows`. [F-174-10](04-findings.md).

### EC-174-29 — Markdown download

**When** the hidden id is passed to `download_document_markdown`.
**Then** 404, and the handler no longer ignores `TenantContext`.
**Mitigation:** [F-174-06](04-findings.md). Same expectation for MCP download.

### EC-174-30 — Admin and master key

**Given** the break-glass flag unset. **Then** instance admin and the
master key see only their security-role clearance (empty if they have
no role). **Given** the flag set. **Then** they see the corpus and an
audit row with result `Warning` is written before the body.
**Mitigation:** [LAW-174-12](01-first-principles.md) · [F-174-09](04-findings.md).

### EC-174-31 — Readonly and a high clearance

**Given** membership readonly and rank restricted. **Then** query and GET
succeed for a matching document, and upload returns the existing readonly
rejection.
**Mitigation:** operation axis first. [LAW-174-1](01-first-principles.md).

### EC-174-32 — API key scopes

**Given** a stored key with `edgequake:write` whose user has HR clearance.
**Then** the key can write HR documents and cannot read R&D documents.
A key scope string `document:read` remains dropped by the existing
normalizer; it does not become a clearance.
**Mitigation:** scope is the operation axis. [02-surfaces](02-surfaces.md).

### EC-174-33 — NULL tenant legacy branch

**Given** a row the old policy would show because `tenant_id` is NULL.
**When** enforcement is on. **Then** `clearance_guard` still hides it
from a non-covering subject.
**Mitigation:** restrictive policies AND. [F-174-02](04-findings.md).

### EC-174-34 — Pooled connections

**Given** request A set rank 4 and the connection returned to the pool.
**When** request B checks out that connection and does not set a rank.
**Then** B does not observe rank 4. GUCs were `SET LOCAL` and
`after_release` resets the session.
**Mitigation:** migration 096 lesson. [07-data-and-db-contract](07-data-and-db-contract.md).

### EC-174-35 — Acc and enforcement off

**Given** a workspace with no settings row, or `enforcement = false`.
**Then** query results match the pre-174 path. SPEC-001 is not required
to set the flag.
**Mitigation:** [LAW-174-2](01-first-principles.md).

### EC-174-36 — Cross-workspace name

**Given** the same entity name in workspace A and workspace B.
**Then** clearance in A does not reveal B. The unique key stays
`(tenant_id, workspace_id, name)`.
**Mitigation:** existing RLS tenant/workspace guard remains mandatory.

### EC-174-37 — Bulk upload

**Given** a batch with one file labeled HR and one labeled R&D, by a
user who holds only HR. **Then** the R&D file is rejected and the HR
file is stored. The rejection does not contain the other file's server id.
**Mitigation:** per-file `allows` on the resulting label.

### EC-174-38 — Glossary injections

**Given** an injection whose source document is hidden. **Then** it is
absent from query context and from the injections list for that user.
**Mitigation:** [LAW-174-16](01-first-principles.md).

### EC-174-39 — Department facet

**Given** the workspace has compartments HR and R&D. **When** the HR
analyst loads facets or tag search. **Then** the payload includes HR and
omits R&D.
**Mitigation:** [LAW-174-8](01-first-principles.md).

### EC-174-40 — Body load is after the decision

**When** a hidden id is requested. **Then** `load_document_body` is not
called, and the KV key `{id}-content` is not read. The list query does
not return that id's `content_summary`.
**Mitigation:** [RB-174-01](17-roadblocks.md) · [RB-174-02](17-roadblocks.md) ·
[LAW-174-6](01-first-principles.md).

### EC-174-41 — Enable flag is last

**Given** a workspace whose chunks are not yet labeled. **When** enable
is in progress. **Then** `enforcement` is still false, and those chunks
remain readable by a workspace member. **When** enable commits.
**Then** the flag is true and the ranks are filled.
**Mitigation:** [RB-174-06](17-roadblocks.md) · [LAW-174-14](01-first-principles.md).

### EC-174-42 — Legacy description needs every source

**Given** an entity whose description was merged from an HR document and
an R&D document, and no contribution rows exist. **When** the HR analyst
loads the node. **Then** the name may appear and the merged description
does not.
**Mitigation:** [RB-174-03](17-roadblocks.md) · [LAW-174-9](01-first-principles.md).

## Case index

| ID | Topic |
|----|-------|
| 01 | Unverified upload |
| 02 | No security role |
| 03 | Deny wins |
| 04 | Equal rank |
| 05 | Superset subject |
| 06 | Both compartments |
| 07 | Steward and simulate header |
| 08 | Enable bootstrap |
| 09 | Raise above uploader |
| 10 | Epoch and snapshot |
| 11 | Tag delete |
| 12 | NULL rank / orphan |
| 13 | 404 matrix |
| 14 | Widen attempt |
| 15 | Empty filter |
| 16 | Fragments |
| 17 | Degree |
| 18 | PNG |
| 19 | Search and popular |
| 20 | Stats |
| 21 | Websocket |
| 22 | Task text |
| 23 | History redaction |
| 24 | Share refused |
| 25 | Retrieval id |
| 26 | Answer cache |
| 27 | Context cache |
| 28 | Dedup oracle |
| 29 | Markdown download |
| 30 | Admin and master key |
| 31 | Readonly plus clearance |
| 32 | API key scope |
| 33 | NULL tenant |
| 34 | Pool GUC |
| 35 | Enforcement off / Acc |
| 36 | Cross-workspace |
| 37 | Bulk upload |
| 38 | Glossary |
| 39 | Facet leak |
| 40 | KV and summary before the decision |
| 41 | Enable flag last |
| 42 | Legacy merged description |

Next: [13-implementation-plan](13-implementation-plan.md).
