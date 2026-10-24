# 01 — First principles (LAW-174)

Parent: [README](README.md) · Prev: [00-why](00-why.md) · Next: [02-surfaces](02-surfaces.md)

These laws are non-negotiable for SPEC-174. A wave may defer a surface, and
must not ship a surface that violates a law. Enforcement off is the
pre-feature state, not a violation: the laws that mention "when enforcement
is on" are idle while the workspace flag is off.

## DRY / SOLID application

| Principle | Application in SPEC-174 |
|-----------|-------------------------|
| **DRY** | One pure `allows`. One `VisibilityScope` built in `bind_request`. Query, graph, MCP, list, download, tasks, stats, and websocket consume it. No second match in the WebUI. |
| **SRP** | Algebra in `edgequake-core/src/clearance/`. SQL and RLS in `edgequake-storage`. HTTP in `edgequake-api` services. React renders the scope it is given. |
| **OCP** | A new tag kind or match kind is a new enum arm and a storage row, not a fork of list/query/graph handlers. |
| **LSP** | REST and MCP both see a `VisibilityScope`. A handler cannot tell which credential produced it. |
| **ISP** | Callers that only need "is this document visible?" do not take the rule editor. Ingest takes a system writer, not an end-user scope. |
| **DIP** | Handlers depend on the scope trait. They do not read `security_role_rules` themselves. |

The unused `Permission` enum stays the operation catalog. Document visibility
is not a new permission string. See [LAW-174-1](#law-174-1--two-axes-compose).

---

## LAW-174-1 — Two axes compose

Instance role and membership role answer "may this principal run this
operation in this workspace?" Security roles answer "which documents may
this principal read or label?" Both must pass. A readonly member with an HR
clearance reads HR documents and cannot write them. A member with write
permission and no security role, once enforcement is on, writes nothing
they cannot read.

Do not encode document tags as new `Permission` variants. Do not replace
`memberships.role`.

- **Closes:** WHY A · [F-174-01](04-findings.md) · [F-174-13](04-findings.md)
- **Tests:** [EC-174-31](12-edge-cases.md) readonly plus high clearance

## LAW-174-2 — Enforcement off matches today

`workspace_clearance_settings.enforcement` defaults to false. While false,
list, query, graph, MCP, and Acc behave as they do on v0.38.0. SPEC-001
does not enable the flag. Turning it on is an owner action, not a migration
side effect.

- **Closes:** [EC-174-35](12-edge-cases.md)
- **Tests:** Acc doctor unchanged; query with the flag off returns the full workspace

## LAW-174-3 — Default deny when enforcement is on

A principal with no security role has an empty visible set. An untagged new
upload is `unverified` and is visible to stewards only. A document whose
rank or compartments are NULL fails closed for non-stewards. There is no
implicit public.

- **Closes:** WHY A · [EC-174-01](12-edge-cases.md) · [EC-174-02](12-edge-cases.md)
- **Tests:** member with no role; untagged upload hidden from that member

## LAW-174-4 — Fail closed

A hidden document id is 404 on get, download, asset, PDF, lineage, and task
detail. An orphan chunk (no parent document, or NULL rank while enforcement
is on) is not returned. A legacy `tenant_id IS NULL` RLS branch must not
widen clearance. An unset clearance GUC, while enforcement is on, denies
row-owned reads for the tenant role.

- **Closes:** [F-174-02](04-findings.md) · [EC-174-12](12-edge-cases.md) · [EC-174-13](12-edge-cases.md) · [EC-174-33](12-edge-cases.md)
- **Tests:** 404 matrix; NULL tenant does not reveal a labeled row

## LAW-174-5 — One PDP and one PEP

`allows(subject, object, action) -> Allow | Deny` is a pure function in
`edgequake-core`. It does not touch SQL or Axum. `bind_request` loads rules
once, builds `VisibilityScope`, and attaches it to the request.
`with_rls_transaction` sets the clearance GUCs in the same transaction as
`set_tenant_context`. Handlers, the query engine, and MCP do not reimplement
the match.

- **Closes:** WHY C · [13-implementation-plan](13-implementation-plan.md)
- **Tests:** algebra unit tests; one scope drives list and query in the same request

## LAW-174-6 — RLS is the backstop for row-owned data

Documents, chunks, originals, PDF bytes, page renders, assets, and tasks
carry stored `classification_rank` and `compartment_ids`. Policies call
leakproof functions as `(SELECT current_clearance_rank())` so the planner
InitPlan-caches them (the migration 208 lesson). Predicates do not use
`jsonb ->>` (the migration 206 lesson). Graph nodes are not given a single
mask; they are projected from contributions ([LAW-174-9](#law-174-9--contributions-stay-per-source)).

RLS does not see the key-value store. `load_document_body` reads
`{doc_id}-content` from KV before SQL ([RB-174-01](17-roadblocks.md)).
The PEP calls `allows` and only then opens KV, PDF bytes, or chunk text.
A list that loads `content_summary` and filters afterwards is a leak
([RB-174-02](17-roadblocks.md)).

- **Closes:** [F-174-02](04-findings.md) · [F-174-14](04-findings.md) · [EC-174-40](12-edge-cases.md)
- **Tests:** tenant role without the GUC sees no labeled chunk; hidden id never reaches `load_document_body`; [16-performance](16-performance.md) EXPLAIN

## LAW-174-7 — Labels are a security boundary

Creating a tag, attaching a tag, or removing a tag requires `tags.manage`
(workspace owner or admin) and a clearance that covers the label. A
principal cannot write a document they would not be allowed to read
afterwards. Downgrading rank, or removing a compartment, is steward-only
and writes `audit_logs`. Deleting a tag that is still assigned is rejected.

- **Closes:** [EC-174-09](12-edge-cases.md) · [EC-174-11](12-edge-cases.md) · [11-threat-model](11-threat-model.md)
- **Tests:** member cannot relabel restricted to public; tag delete returns 409

## LAW-174-8 — No existence leak

Lists omit hidden rows. Counts, popular nodes, label search, graph degree,
pipeline events, task errors, glossary listings, and dedup responses are
computed on the visible set. The product does not say how many documents
were hidden. Facets do not name a compartment the caller does not hold.

- **Closes:** WHY E · [F-174-10](04-findings.md) · [F-174-11](04-findings.md)
- **Tests:** [EC-174-17](12-edge-cases.md) through [EC-174-22](12-edge-cases.md), [EC-174-28](12-edge-cases.md), [EC-174-39](12-edge-cases.md)

## LAW-174-9 — Contributions stay per source

While enforcement is on, ingestion writes one contribution row per entity
or edge fragment (document id, chunk key, text, copied rank and
compartments). It does not replace that fragment with a cross-document LLM
summary. A node name and type are visible when at least one contribution
is visible. The served description, edges, and degree come from visible
contributions only. `eq_graph_nodes.properties.description` is not the
read authority. The ingest worker uses a system writer, not the end-user
scope.

Rows written before contributions exist have one description string
(`merger/entity.rs`). That string is served only when every
`source_document_id` is visible. Otherwise the name and type may show and
the description is omitted. Enable does not try to split the string
([RB-174-03](17-roadblocks.md), [EC-174-42](12-edge-cases.md)).

- **Closes:** WHY B · [F-174-04](04-findings.md) · [F-174-05](04-findings.md)
- **Tests:** [EC-174-16](12-edge-cases.md) · [EC-174-17](12-edge-cases.md) · [EC-174-18](12-edge-cases.md) · [EC-174-42](12-edge-cases.md)

## LAW-174-10 — Client scope only narrows

When enforcement is on, an empty `document_filter` becomes the server
visible set. Explicit ids, title patterns, and date bounds are intersected
with that set. MCP `document_ids` obey the same rule. A pattern that
matches nothing returns an empty result, not the workspace.

- **Closes:** WHY C · [F-174-03](04-findings.md) · [F-174-12](04-findings.md)
- **Tests:** [EC-174-14](12-edge-cases.md) · [EC-174-15](12-edge-cases.md)

## LAW-174-11 — Stored context carries the clearance

The answer cache may stay a hash of the full prompt, because a legal prompt
contains only visible text. The context cache and `retrieval_id` store the
bundle: their keys include the clearance fingerprint and the workspace
policy epoch, and `eq_fetch` / `GET /api/v1/query/context/{id}` re-check
the caller. Keyword cache stores no document text and stays shared.
Conversation history is filtered at read. New conversation shares are
refused while enforcement is on. SPEC-126 prompt-cache prefixes that
include document text vary by the same fingerprint.

- **Closes:** WHY D · [F-174-07](04-findings.md) · [F-174-08](04-findings.md)
- **Tests:** [EC-174-23](12-edge-cases.md) · [EC-174-24](12-edge-cases.md) · [EC-174-25](12-edge-cases.md) · [EC-174-26](12-edge-cases.md) · [EC-174-27](12-edge-cases.md)

## LAW-174-12 — Platform roles do not skip clearance

`Role::Admin` still bypasses membership for tenant selection. It does not
bypass document clearance. The master API key does not either. The only
skip is `EDGEQUAKE_CLEARANCE_BREAK_GLASS=1`, which writes an
`audit_logs` row with `event_type = Authorization` and result `Warning`
before the read. The flag defaults off whenever any workspace on the
process has enforcement on. Operators who need a full read turn the flag
on deliberately.

- **Closes:** WHY E · [F-174-09](04-findings.md)
- **Tests:** [EC-174-30](12-edge-cases.md)

## LAW-174-13 — Uploader exception is bounded

The uploader may read a document they created, including while it is
`unverified`, until a steward sets a label above that uploader's
clearance. Other principals do not see `unverified` unless they are
stewards. After the raise, get, download, chunks, and graph contributions
from that document are hidden from the uploader as well (404 on direct
id). The exception is `uploaded_by = current user` and is not a grant to
read other unverified documents.

- **Closes:** [EC-174-09](12-edge-cases.md)
- **Tests:** uploader reads their unverified body; a peer member does not; body 404 after a raise above the uploader

## LAW-174-14 — Enable cannot lock the owner out

Enabling enforcement backfills document and child `classification_rank`
while the flag is still off, grants the acting owner the steward role,
then sets `enforcement = true` and bumps `policy_epoch` in the last
statement. A failed grant rolls the flag back. Flipping the flag first
would hide every NULL-rank chunk mid-backfill
([RB-174-06](17-roadblocks.md), [EC-174-41](12-edge-cases.md)). The UI
states, in one sentence, that members without a security role will see
an empty workspace.

- **Closes:** [EC-174-08](12-edge-cases.md) · [EC-174-41](12-edge-cases.md) · [03-product-spec](03-product-spec.md)
- **Tests:** owner still lists documents after enable; member without a role lists none; a chunk read during backfill is not denied by a NULL rank

## LAW-174-15 — Proof is a named test

Every [EC-174](12-edge-cases.md) maps to one gate in
[14-e2e-test-matrix](14-e2e-test-matrix.md). Algebra cases are unit tests.
HTTP and MCP cases are e2e. The EXPLAIN check in
[16-performance](16-performance.md) is the performance gate, not a
substitute for the leak tests.

- **Closes:** all EC
- **Tests:** the matrix itself

## LAW-174-16 — The model sees only the visible projection

Naive, local, global, hybrid, and mix all receive the same
`allowed_document_ids` from the scope. Citations are visible chunks.
Graph PNG uses the visible neighborhood. The prompt does not contain
hidden chunk text, hidden descriptions, or a statement that hidden
documents exist. Glossary injections are filtered the same way.

- **Closes:** WHY B · WHY C · [06-graph-query-ai](06-graph-query-ai.md)
- **Tests:** [EC-174-16](12-edge-cases.md) · [EC-174-38](12-edge-cases.md)

---

## Inheritance

| Prior law | SPEC-174 relationship |
|-----------|------------------------|
| SPEC-154 session, audience, cookies | Unchanged. Clearance is not a JWT claim of document ids. |
| SPEC-158 tenant from identity | Unchanged. Clearance is loaded after the tenant bind. |
| SPEC-169 two role axes | Unchanged. Security roles are a third assignment on the membership. |
| SPEC-164 plain graph | Contributions sit beside `eq_graph_nodes`, they do not revive AGE. |
| Migration 096 `SET LOCAL` | Clearance GUCs are transaction-local. [EC-174-34](12-edge-cases.md). |
