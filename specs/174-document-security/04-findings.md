# 04 — Findings (F-174)

Parent: [README](README.md) · Prev: [03-product-spec](03-product-spec.md) ·
Next: [05-policy-model](05-policy-model.md)

Each finding is a fact about the tree at migration head 219. The law that
closes it is in [01-first-principles](01-first-principles.md). None of these
are fixed by this pack; W1–W11 do that.

## F-174-01 — Document permission is workspace-wide

`require_request_permission`
(`edgequake/crates/edgequake-api/src/services/request_authorization.rs`)
allows a query POST for every role that may read, and rejects readonly on
other writes. It does not receive a document id. `edgequake-api` does not
call `RbacService::require_permission`. A readonly membership rewrites the
request role; it does not filter rows.

- **Law:** [LAW-174-1](01-first-principles.md)
- **EC:** [EC-174-31](12-edge-cases.md)

## F-174-02 — RLS stops at tenant and workspace

`tenant_access_guard` in `edgequake/migrations/167_tenant_access_rls.sql`
matches `current_tenant_id()` and `current_workspace_id()` (InitPlan form
after migration 208). The same shape covers `documents`, `chunks`,
`entities`, `relationships`, `chunk_embeddings`, `eq_graph_nodes`, and
`eq_graph_edges` (migration 170). No policy names a label, a user
attribute, or a document id. `set_tenant_context` in
`edgequake/crates/edgequake-storage/src/adapters/postgres/rls.rs` sets three
GUCs.

- **Law:** [LAW-174-4](01-first-principles.md) · [LAW-174-6](01-first-principles.md)
- **EC:** [EC-174-33](12-edge-cases.md) · [EC-174-34](12-edge-cases.md)

## F-174-03 — Empty document filter is all-pass

`resolve_document_filter`
(`handlers/query/document_filter_resolver.rs`) returns `Ok(None)` when
`filter.is_empty()`. The comment in that file calls `None` all-pass.
Explicit ids with no date or pattern are returned without a membership
intersection beyond the later workspace scan on other branches.
`execute_query` leaves `allowed_document_ids` unset when the body omits
the filter. `filter_context_by_document_ids` returns immediately on `None`.

- **Law:** [LAW-174-10](01-first-principles.md)
- **EC:** [EC-174-14](12-edge-cases.md) · [EC-174-15](12-edge-cases.md)

## F-174-04 — Merged description is one blob

`edgequake/crates/edgequake-pipeline/src/merger/entity.rs` keeps the longer
description or resolves both through the LLM summarizer. Names are unique
on `(tenant_id, workspace_id, name)` (`migrations/001_init_database.sql`).
`merger/lineage.rs` unions `source_document_ids`.
`filter_context_by_document_ids` keeps an entity when any lineage id
intersects the allowed set, and does not split `entity.description`.

- **Law:** [LAW-174-9](01-first-principles.md)
- **EC:** [EC-174-16](12-edge-cases.md)

## F-174-05 — Graph browse has no document predicate

`pg_bfs_expand` (`graph/query_ops/expand.rs`) and plain `search_nodes`
(`graph/plain/search.rs`, function `eq_graph_search_nodes` from migration
206) filter tenant and workspace. `get_graph` and node search handlers do
not pass `allowed_document_ids`. Global `degree` on `eq_graph_nodes` counts
edges from every source.

- **Law:** [LAW-174-8](01-first-principles.md) · [LAW-174-9](01-first-principles.md)
- **EC:** [EC-174-17](12-edge-cases.md) · [EC-174-18](12-edge-cases.md) · [EC-174-19](12-edge-cases.md)

## F-174-06 — Markdown download ignores the tenant context

`download_document_markdown`
(`handlers/documents/query/download.rs`) binds `_context: TenantContext`
and never reads it. `shell_value_by_key`
(`edgequake-storage/.../document_shell.rs`) loads `documents` by id. A
caller who can name the id receives the markdown whenever the connection
can see the row. Workspace RLS may still hide a cross-workspace id; it
does not hide a same-workspace id.

- **Law:** [LAW-174-4](01-first-principles.md)
- **EC:** [EC-174-13](12-edge-cases.md) · [EC-174-29](12-edge-cases.md)

## F-174-07 — Context cache and retrieval id are not clearance-bound

`cache_key` in `edgequake-query/src/cache/query_result_cache.rs` hashes
workspace, query, mode, optional allowed ids, and mix weights. It stores
the `QueryContext` (chunk text and entity descriptions).
`global_retrieval_cache` (`services/retrieval_id_cache.rs`) stores the
bundle under `ret_{uuid}` for about 15 minutes. `eq_fetch` and
`GET /api/v1/query/context/{retrieval_id}` return that bundle without
rebuilding the question.

- **Law:** [LAW-174-11](01-first-principles.md)
- **EC:** [EC-174-25](12-edge-cases.md) · [EC-174-27](12-edge-cases.md)

## F-174-08 — Answer cache is safe only after the prompt is filtered

`llm_cache_storage_key` (`cache/llm_response_cache.rs`) is called from
`query_pipeline.rs` after `postprocess_retrieved_context`. The key is the
mode plus a hash of the prompt. It has no user id. Two principals who
built the same prompt share the answer, which is correct. Two principals
who should have seen different chunks share an answer if the forbidden
text was already in the prompt. Keyword cache
(`hash_keyword_args`) stores keyword JSON, not document text.

- **Law:** [LAW-174-11](01-first-principles.md)
- **EC:** [EC-174-26](12-edge-cases.md)

## F-174-09 — Admin and master key see the workspace corpus

`bind_request` treats `Role::Admin` as a workspace owner after
`require_active_scope`, and skips membership for the master API key
(`user_id = "master-api-key"`). The membership-bypass audit in
`middleware.rs` records success. RLS then returns every document in the
bound workspace. There is no clearance GUC to fail.

- **Law:** [LAW-174-12](01-first-principles.md)
- **EC:** [EC-174-30](12-edge-cases.md)

## F-174-10 — Content-hash dedup is an existence oracle

`document_admission.rs` calls
`resolve_workspace_duplicate_for_reingestion` with the workspace content
hash (`document_reingest.rs`). A second upload of the same bytes can
attach to, or name, the first document. If that document is hidden, the
response tells the uploader it exists.

- **Law:** [LAW-174-8](01-first-principles.md)
- **EC:** [EC-174-28](12-edge-cases.md)

## F-174-11 — Side channels are workspace aggregates

These paths count or echo workspace rows with no document predicate:

- `get_workspace_stats` (`handlers/workspaces/stats.rs`), cache key is the workspace id
- `list_tasks` / task payloads (`handlers/tasks.rs`, migration 149)
- pipeline progress websocket
- `messages.content` and `messages.context` (`handlers/chat/history.rs`)
- lineage export (`handlers/lineage/export.rs`)
- workspace injections (glossary text)
- popular nodes and label search on `eq_graph_nodes`

- **Law:** [LAW-174-8](01-first-principles.md) · [LAW-174-11](01-first-principles.md)
- **EC:** [EC-174-20](12-edge-cases.md) through [EC-174-24](12-edge-cases.md) · [EC-174-38](12-edge-cases.md) · [EC-174-39](12-edge-cases.md)

## F-174-12 — MCP document ids are optional

`eq_search` uses `retrieve_context` and applies a document scope only when
the caller passes one (`mcp/project/search.rs`). `entity_has_docs` in
`mcp/project/graph.rs` post-filters after `search_nodes`. The description
it returns is still the merged node. `eq_fetch` does not re-check.
Download and asset tools use the REST loaders.

- **Law:** [LAW-174-10](01-first-principles.md) · [LAW-174-16](01-first-principles.md)
- **EC:** [EC-174-14](12-edge-cases.md) · [EC-174-18](12-edge-cases.md)

## F-174-13 — There is no label store

`documents` has no classification or compartment columns (migration 001
plus later stats and lifecycle columns). `metadata` is operational
(`current_stage` is copied to a generated column in migration 206 for
indexing, not for security). SPEC-158 links
`specs/146-rbac-attributes-based-securty/`. That directory is absent.
SPEC-169 did not add custom roles.

- **Law:** [LAW-174-2](01-first-principles.md) · [LAW-174-3](01-first-principles.md)
- **EC:** [EC-174-01](12-edge-cases.md) · [EC-174-35](12-edge-cases.md)

## F-174-14 — Vector rows do not carry a document label

`chunk_embeddings` (migration 108) is `(model_id, chunk_id, workspace_id,
embedding)`. Document scope is a join to `chunks.document_id`. Entity and
relationship embeddings join `entities` / `relationships` and overlap
chunk keys. A post-filter after ANN both shortens top-k and spends scan
work on hidden rows ([16-performance](16-performance.md)).

- **Law:** [LAW-174-6](01-first-principles.md)
- **EC:** [EC-174-12](12-edge-cases.md)

## Finding index

| ID | One line |
|----|----------|
| F-174-01 | Route RBAC, not document RBAC |
| F-174-02 | RLS is tenant and workspace |
| F-174-03 | Omitted filter is all-pass |
| F-174-04 | One description for many documents |
| F-174-05 | Graph browse is the whole workspace |
| F-174-06 | Markdown download ignores tenant context |
| F-174-07 | Bundle caches have no clearance |
| F-174-08 | Answer cache follows the prompt |
| F-174-09 | Admin and master key see every row |
| F-174-10 | Dedup names a hidden document |
| F-174-11 | Stats, tasks, chat, facets leak |
| F-174-12 | MCP scope is opt-in |
| F-174-13 | No tag tables |
| F-174-14 | Embeddings join for document scope |

Next: [05-policy-model](05-policy-model.md).
