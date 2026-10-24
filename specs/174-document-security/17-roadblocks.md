# 17 — Roadblocks and mitigations

Parent: [README](README.md) · Laws: [01-first-principles](01-first-principles.md) ·
UX: [09-ux-ui-spec](09-ux-ui-spec.md)

Assessment after the first draft. Each block is a fact in the tree, not a
preference. The mitigation is the spec change. No product code in this pass.

## RB-174-01 — Document text is loaded from KV, which has no RLS

`load_document_body` in
`edgequake/crates/edgequake-api/src/services/document_body_loader.rs`
reads `{doc_id}-content` from KV, then PDF markdown, then chunk keys.
`download_document_markdown` calls it after `load_document_metadata`, and
the `TenantContext` parameter is unused ([F-174-06](04-findings.md)).
PostgreSQL policies never see that read.

**Mitigation.** `VisibilityScope` runs before any of those loads. A deny
returns 404 and does not call `load_document_body`. RLS remains the
backstop for SQL, not the only control. [LAW-174-6](01-first-principles.md).
Test: [EC-174-40](12-edge-cases.md).

```text
  named document id
           |
           v
  +----------------------------------+
  | allows(subject, label, Read)     |
  +----------------+-----------------+
                   |
          Deny     |     Allow
           |       |       |
           v       |       v
         404       |  KV, then PDF, then chunks
                   |
                   +-- SQL RLS still applies
```

## RB-174-02 — The list loads summaries, then could filter too late

`load_scoped_document_metadata_entries_limited` in
`edgequake/crates/edgequake-api/src/services/document_metadata_scan.rs`
selects metadata keys, then `list_documents_inner` copies
`content_summary` onto the response. A filter after that parse has already
held hidden summaries in the process. The same scan feeds
`handlers/documents/query/track_status.rs` and document search.

**Mitigation.** The key query joins `documents` and applies
`clearance_covers` before `get_by_ids`. Hidden ids are not fetched.
Pipeline progress uses that same key query ([EC-174-21](12-edge-cases.md)).
Do not filter the JSON in the handler or in React.

## RB-174-03 — A merged description cannot be split on enable

`merger/entity.rs` keeps the longer description or one LLM summary.
`source_document_ids` is a union. Enable cannot reconstruct which sentence
came from which document.

**Mitigation.** Until a `graph_contributions` row exists, the description
is served only when every source document is visible. A subject who sees
one of two sources gets the name and type, not the text. New ingest while
enforcement is on writes fragments and does not call the cross-document
summarizer. A steward may start a rebuild later. Enable does not rebuild.
[LAW-174-9](01-first-principles.md). Test: [EC-174-42](12-edge-cases.md).

## RB-174-04 — Graph search runs as the table owner

`eq_graph_search_nodes` (migration 206) is security definer and applies
the tenant itself. A new `clearance_guard` on the table does not bind
inside that function. Calling it unchanged while enforcement is on skips
the label.

**Mitigation.** Enforcement-on search is a sibling function that applies
`clearance_covers` to contribution rows before matching text. The current
function stays for enforcement off, so Acc does not change.
[16-performance](16-performance.md).

## RB-174-05 — A simulate header is a second session

The first UX draft put `X-Clearance-Simulate` on later list, query, and
graph calls, plus a banner that restyles the app. That header is easy to
forward, and it splits the React Query cache from the real subject.

**Mitigation.** `POST .../clearance/simulate` returns a count and up to
20 titles. The settings Access section shows them in a sheet and closes.
List, query, and graph keep using the session subject. No header.
[09-ux-ui-spec](09-ux-ui-spec.md), [08-api-contract](08-api-contract.md).

## RB-174-06 — Flipping enforcement before the backfill hides the corpus

`clearance_covers` fails closed on NULL rank. If the flag turns on before
chunks are copied, every chunk read 404s until the copy commits. A large
workspace makes that window visible.

**Mitigation.** Fill `documents` and child rank columns while
`enforcement` is still false. Grant the steward role. Set the flag and
bump `policy_epoch` in the last statement of the same transaction.
[LAW-174-14](01-first-principles.md). Test: [EC-174-41](12-edge-cases.md).

Child copies that do not fit one transaction are batched by `document_id`
while the flag is off. The flag still flips once, after the last batch
commits. A crash between batches leaves enforcement off and the job
resumable. It does not leave enforcement on with NULL ranks.

## RB-174-07 — Clearance GUCs must have one writer

`set_tenant_context` in
`edgequake/crates/edgequake-storage/src/adapters/postgres/rls.rs` is the
only place that sets `app.current_tenant_id`. Callers that open SQL
without it already bypass tenant RLS. A second helper for clearance would
drift.

**Mitigation.** Extend `set_tenant_context` with the clearance `set_config`
calls (`is_local = true`). W3 greps every SQL entry and rejects a path
that reads `documents` or `chunks` on a connection where this function
was not called. Pool release already `RESET ALL` (SPEC-171,
[EC-174-34](12-edge-cases.md)).

## RB-174-08 — In-flight ingest can still merge after enable

A task that started before the flag flipped can commit a summary after
the flag is on.

**Mitigation.** The commit path reads `enforcement` inside the write
transaction. When it is on, the writer inserts contributions and does not
replace an existing description with a cross-document summary. Tasks
already committed stay on the legacy rule in RB-174-03.

## What is not a roadblock

The operation-role matrix in `roles-tab.tsx` (`GET /roles`) is the
membership axis. Document labels do not become rows in that matrix.
Adding them would mix two decisions on one grid and would not match
`allows`. Assignment stays on the member sheet. Profiles stay in Settings
→ Access, which already exists as `settings-nav.tsx` section `access`.

Next: [09-ux-ui-spec](09-ux-ui-spec.md).
