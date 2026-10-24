# SPEC-174 — Document security (RBAC x tag ABAC)

> **Status:** Specified (W0). No product code in this pack.
> **Product pin:** EdgeQuake v0.38.0 (migration head 219).
> **Scope:** Document-level visibility inside a workspace. Tags carry a
> classification and compartments such as department. Security roles carry
> allow and deny rules over those tags. Query, graph, caches, MCP, and
> downloads enforce the same decision.
> **Inherits:** [SPEC-158](../158-entreprise-grade-authentication/) ·
> [SPEC-154](../154-sec-hardening/) ·
> [SPEC-169](../169-perfect-user-management/) ·
> [SPEC-164](../164-pure-postgres-study/) ·
> [SPEC-103](../103-llm-cache/) ·
> [SPEC-126](../126-provider-kv-cache/) ·
> [SPEC-152 §08](../152-new-mcp-contract/08-security-observability.md) ·
> [SPEC-005](../005-filter.md) /
> [SPEC-031](../031-filter-document/000-overview.md) document filter ·
> [SPEC-001](../001-benchmark/) Acc stays on the enforcement-off default.
> **Peers:** The SPEC-158 README links
> `specs/146-rbac-attributes-based-securty/`. That directory was never
> written. This pack is that work. Do not create a 146 tree.

## Start here

1. [00-why.md](00-why.md) — Five WHYs + causal ASCII
2. [01-first-principles.md](01-first-principles.md) — LAW-174-1…16 + DRY/SOLID
3. [02-surfaces.md](02-surfaces.md) — Code map of auth, RLS, query, graph, MCP
4. [03-product-spec.md](03-product-spec.md) — Personas, stories, acceptance
5. [04-findings.md](04-findings.md) — F-174-* with file and symbol
6. [05-policy-model.md](05-policy-model.md) — Algebra, examples, citations
7. [06-graph-query-ai.md](06-graph-query-ai.md) — Query, graph vision, prompts
8. [07-data-and-db-contract.md](07-data-and-db-contract.md) — Migration 220
9. [08-api-contract.md](08-api-contract.md) — Routes, 404 rule
10. [09-ux-ui-spec.md](09-ux-ui-spec.md) — Access as a property, not a second app
11. [10-frontend-architecture.md](10-frontend-architecture.md) — One client type
12. [11-threat-model.md](11-threat-model.md) — Relabel, dedup, cache, degree
13. [12-edge-cases.md](12-edge-cases.md) — EC-174 register
14. [13-implementation-plan.md](13-implementation-plan.md) — Waves W0–W11
15. [14-e2e-test-matrix.md](14-e2e-test-matrix.md) — One gate per EC
16. [15-cross-ref.md](15-cross-ref.md) — Law, finding, EC, wave, test, lens
17. [16-performance.md](16-performance.md) — Indexes, iterative scan, EXPLAIN
18. [17-roadblocks.md](17-roadblocks.md) — KV, legacy graph text, enable order, UI
19. Lenses → [`lenses/`](lenses/)
    - [Product Owner](lenses/LENS-product-owner.md)
    - [Full Stack](lenses/LENS-full-stack.md)
    - [Database](lenses/LENS-database.md)
    - [UX / UI](lenses/LENS-ux-ui.md)
    - [Front](lenses/LENS-front.md)
    - [AI Engineer](lenses/LENS-ai-engineer.md)
    - [Security](lenses/LENS-security.md)
    - [Document Management](lenses/LENS-document-management.md)
    - [Graph Algorithms](lenses/LENS-graph-algo.md)

## Locked decisions (Wave 0)

1. **Two axes, composed.** Instance and membership roles stay the operation
   axis (read, write, query, manage). A new workspace security role is the
   object axis. `Permission` in `edgequake-auth` is not extended into a
   document policy.
2. **Enforcement off by default.** Existing corpora and SPEC-001 Acc are
   unchanged until a workspace owner turns enforcement on.
3. **Enable backfill.** Existing documents become `internal`. The acting
   owner becomes steward. New untagged uploads become `unverified`
   (steward-only). Strict-start labels the existing corpus `unverified`.
   The dialog states that a member with no security role will see nothing.
4. **Classification is one ordered tag.** `public < internal < confidential
   < restricted`, plus `unverified`. Rank is an integer column.
5. **Compartments are a set.** Read requires `subject_rank >= document_rank`
   and document compartments ⊆ subject compartments. An empty compartment
   set is visible to anyone who meets the rank.
6. **Several security roles.** Effective subject = max rank, union of
   compartments, then any explicit deny wins. Default deny while
   enforcement is on.
7. **Tag writes are a boundary.** `tags.manage` is owner and admin. A
   principal cannot apply, see, or remove a label their clearance does not
   cover. Downgrade is steward-only and audited.
8. **Admin and master key do not skip clearance.** Tenant bypass stays.
   `EDGEQUAKE_CLEARANCE_BREAK_GLASS=1` is the only skip, and it writes
   `audit_logs` (`Authorization`). Default off when enforcement is on.
9. **Hidden id is 404.** Lists, counts, degrees, facets, and events omit
   hidden objects. No "N hidden" hint.
10. **Uploader exception ends on raise.** The uploader can read their
    document until a steward sets a label above the uploader's clearance.
11. **Client `document_filter` only narrows.** An omitted filter is the
    server visible set, not the workspace.
12. **Contributions stay per source.** While enforcement is on, merged
    `description` and global degree are not the read authority.
13. **One PDP, one PEP.** `allows(subject, object, action)` is pure.
    `VisibilityScope` is built in `bind_request` and consumed everywhere.
14. **RLS is the backstop** for row-owned tables, using stored columns and
    leakproof InitPlan functions.
15. **Every EC-174 has a named test** (LAW-174-15).

## Job in one screen

```text
  Member / API key / MCP
            |
            v
  +-----------------------------+
  | bind_request                |
  | membership = operation      |
  +-----------------------------+
                |
                v
  +-----------------------------+
  | ClearanceDecision (PDP)     |
  | max rank, union, deny       |
  +-----------------------------+
                |
                v
  +--------------------+  +--------------------+
  | RLS + GUCs         |  | VisibilityScope    |
  | documents, chunks  |  | query, graph, MCP  |
  +--------------------+  +--------------------+
```

Decision flow (the same PEP; client ids never widen):

```mermaid
flowchart TD
  req[Request] --> bind[bind_request membership]
  bind --> pdp[ClearanceDecision pure PDP]
  pdp --> guc[SET LOCAL clearance GUCs]
  guc --> rls[RLS on row-owned tables]
  pdp --> scope[VisibilityScope]
  scope --> query[Query allowed ids intersect]
  scope --> graph[Graph facet projection]
  clientFilter[Client document_filter] --> narrow[Intersect only]
  narrow --> query
```

## Non-goals

- Per-document user or group shares (Zanzibar / ReBAC). Cited in
  [05-policy-model](05-policy-model.md) as the deferred model.
- Cedar, OPA, or XACML as the policy engine.
- Replacing `users.role` or `memberships.role`.
- A second `Permission` catalog for documents.
- Turning enforcement on for SPEC-001 Acc.
- Encrypting file bytes (labels here are authorization, not DRM).
- Cross-workspace clearance. The workspace key remains the isolation boundary
  inherited from SPEC-164 and migration 167.

## Success narrative

When W1–W11 land, an owner can turn enforcement on, define HR and R&D
compartments, and attach rules to security roles. An HR analyst then lists,
downloads, queries, and sees a graph built only from HR documents. An R&D
analyst does not learn that the HR document exists. A node extracted from
both corpora shows each analyst only the fragment from documents they may
read. A retrieval id minted for one analyst is useless to the other.
SPEC-001 scores do not move, because enforcement stays off there.
