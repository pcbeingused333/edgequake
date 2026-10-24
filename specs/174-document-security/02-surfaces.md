# 02 — Surfaces (code map)

Parent: [README](README.md) · Prev: [01-first-principles](01-first-principles.md) ·
Next: [03-product-spec](03-product-spec.md)

This is the map the later chapters cite. It records what the code does on
v0.38.0 (migration head 219). Findings that name a gap are in
[04-findings](04-findings.md).

## Operation axis (kept)

| Symbol | Path | What it decides |
|--------|------|-----------------|
| `Role` | `edgequake/crates/edgequake-auth/src/types.rs` | Instance `admin` / `user` / `readonly` |
| `Permission`, `RbacService` | `edgequake/crates/edgequake-auth/src/rbac.rs` | Fixed permission sets. Not called by the API request path |
| `MembershipRole` | `edgequake/crates/edgequake-core/src/types/multitenancy/membership.rs` | `owner` / `admin` / `member` / `readonly` |
| `require_request_permission` | `edgequake/crates/edgequake-api/src/services/request_authorization.rs` | Admin routes, write vs read, query POSTs allowed for readonly |
| `bind_request` | same file | Header tenant/workspace, membership, admin skip, master-key skip |
| `build_role_catalog` | `edgequake/crates/edgequake-api/src/services/role_catalog.rs` | Seven membership capabilities. No document attributes |
| `access_token_claims_scoped` | `edgequake/crates/edgequake-api/src/services/identity_storage.rs` | JWT carries role, tenant, workspace. Not a document set |

SPEC-169 aliases (`viewer` → readonly, `member` → user) stay. Security roles
in this pack are a new assignment, not a fourth value in those enums.
[LAW-174-1](01-first-principles.md).

## Row security (extended, not replaced)

| Symbol | Path | What it decides |
|--------|------|-----------------|
| `set_tenant_context` | `edgequake/crates/edgequake-storage/src/adapters/postgres/rls.rs` | `SET LOCAL` of `app.current_tenant_id`, `app.current_workspace_id`, `app.current_user_id` |
| `with_rls_transaction` | same file | `SET LOCAL ROLE edgequake_tenant_access`, then the GUCs |
| `tenant_access_guard` | `edgequake/migrations/167_tenant_access_rls.sql` | Restrictive policy. Tenant + workspace, or workspace plus a `workspaces` existence check |
| InitPlan rewrite | `edgequake/migrations/208_rls_initplan_predicates.sql` | `(SELECT current_tenant_id())` |
| Leakproof copies | `edgequake/migrations/206_rls_leakproof_predicates.sql` | Stored columns instead of `jsonb ->>` |
| Plain graph RLS | `edgequake/migrations/170_spec164_plain_graph.sql` | `eq_graph_nodes`, `eq_graph_edges` by tenant and workspace text |

Clearance GUCs are added beside the existing three, in the same transaction
([07-data-and-db-contract](07-data-and-db-contract.md), [LAW-174-6](01-first-principles.md)).

## Documents and bytes

| Symbol | Path | Selection today |
|--------|------|-----------------|
| `relational_workspace_doc_ids` | `edgequake/crates/edgequake-api/src/services/workspace_document_index.rs` | `documents.workspace_id` union `metadata->>'workspace_id'` |
| `list_documents_inner` | `edgequake/crates/edgequake-api/src/handlers/documents/query/list.rs` | That set, then in-memory status and title |
| `download_document_markdown` | `edgequake/crates/edgequake-api/src/handlers/documents/query/download.rs` | `_context: TenantContext` is unused |
| `shell_value_by_key` | `edgequake/crates/edgequake-storage/src/adapters/postgres/document_shell.rs` | `WHERE id = $1` on `documents` |
| `load_document_body` | `edgequake/crates/edgequake-api/src/services/document_body_loader.rs` | KV content, else chunks, else PDF markdown |
| `resolve_workspace_duplicate_for_reingestion` | `edgequake/crates/edgequake-api/src/services/document_reingest.rs` | Called from `document_admission.rs` with the workspace content hash |

Related tables, all keyed by `document_id` and usually `workspace_id`:
`chunks`, `document_originals`, `pdf_documents`, `document_pages`,
`document_mm_assets`, `original_object_refs`, `page_layout_regions`.

`documents.metadata` holds title, file name, pdf id, progress, and
extraction fields. It is not a label store. [F-174-13](04-findings.md).

## Query

| Symbol | Path | Selection today |
|--------|------|-----------------|
| `execute_query` | `edgequake/crates/edgequake-api/src/handlers/query/query_execute.rs` | Sets `allowed_document_ids` only when the body has `document_filter` |
| `resolve_document_filter` | `edgequake/crates/edgequake-api/src/handlers/query/document_filter_resolver.rs` | Empty filter → `None` (all-pass). Explicit ids returned as sent |
| `make_scope_metadata_filter` | `edgequake/crates/edgequake-query/src/engine_impl/modes/mod.rs` | Copies `allowed_document_ids` into `MetadataFilter` |
| `query_naive_with_vector_storage` | `edgequake/crates/edgequake-query/src/engine_impl/modes/naive.rs` | Chunk ANN + BM25 |
| `query_local_with_vector_storage` | `.../modes/local.rs` | Entity ANN, then chunks |
| `query_global_with_vector_storage` | `.../modes/global.rs` | Relationship ANN, then chunks |
| `query_hybrid_with_vector_storage` | `.../modes/hybrid.rs` | Local + global |
| `query_mix_with_vector_storage` | `.../modes/mix.rs` | Local + global + naive |
| `filter_context_by_document_ids` | `edgequake/crates/edgequake-query/src/context_filter.rs` | `None` returns immediately. Entities kept on any lineage overlap |
| `PgChunkEmbeddingIndex::search` | `edgequake/crates/edgequake-storage` chunk embedding index | `workspace_id` and optional `document_id = ANY` |
| Chat completion / streaming | `edgequake/crates/edgequake-api/src/handlers/chat/` | Same resolver (SPEC-005 comment in those files) |

The engine already accepts a document id list. The gap is who fills it.
[LAW-174-10](01-first-principles.md).

## Graph and merge

| Symbol | Path | Selection today |
|--------|------|-----------------|
| Description merge | `edgequake/crates/edgequake-pipeline/src/merger/entity.rs` | Longer text, or LLM summary of both |
| Lineage properties | `edgequake/crates/edgequake-pipeline/src/merger/lineage.rs` | `source_chunk_ids`, `source_document_ids` union |
| `pg_bfs_expand` | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/query_ops/expand.rs` | Tenant and workspace |
| `search_nodes` | `.../graph/plain/search.rs` | `eq_graph_search_nodes` (owner function, migration 206) |
| `get_graph` | `edgequake/crates/edgequake-api/src/handlers/graph/graph_query/traversal.rs` | BFS on that storage |
| `entities` unique | `edgequake/migrations/001_init_database.sql` | `(tenant_id, workspace_id, name)` |

`chunk_embeddings` has no `document_id`. The chunk search joins `chunks`.
Entity search overlaps `source_chunk_ids` with chunk keys of the allowed
documents when a filter is present (`document_lineage_sql`). [F-174-14](04-findings.md).

## Caches

| Store | Path | Key today |
|-------|------|-----------|
| Answer | `llm_cache_storage_key` in `edgequake/crates/edgequake-query/src/cache/llm_response_cache.rs`, called from `query_pipeline.rs` | Mode + hash of the prompt after context filtering |
| Keywords | `hash_keyword_args` in `edgequake/crates/edgequake-query/src/keywords/llm_extractor.rs` | Query, mode, model, language. No document text |
| Context | `cache_key` in `edgequake/crates/edgequake-query/src/cache/query_result_cache.rs` | Workspace, query, mode, `allowed_document_ids`, mix weights |
| Retrieval id | `global_retrieval_cache` in `edgequake/crates/edgequake-api/src/services/retrieval_id_cache.rs` | `ret_{uuid}`. Process-global. Fetch does not rebuild |

`public.llm_cache` primary key is `(cache_key, namespace)` with namespace
`"default"`. [LAW-174-11](01-first-principles.md).

## MCP

Dispatch: `edgequake/crates/edgequake-api/src/mcp/gateway/dispatch.rs`.

| Tool | Path | Engine |
|------|------|--------|
| `eq_search`, `eq_retrieve` | `mcp/project/search.rs` | `retrieve_context` (`services/query_context.rs`) |
| `eq_fetch` | `mcp/project/fetch.rs` | `global_retrieval_cache`, no re-query |
| `eq_entity_search`, `eq_neighborhood` | `mcp/project/graph.rs` | `search_nodes` / BFS. `entity_has_docs` is a post-filter when ids are sent |
| `eq_document_download` | `mcp/project/download.rs` | Same loaders as REST |
| `eq_graph_image` | `mcp/project/graph_image.rs` | Same neighborhood, then PNG |
| `eq_document_list`, `eq_document_get` | `mcp/project/catalog.rs` | Workspace list |
| `eq_asset_get` | `mcp/project/assets.rs` | `document_mm_assets` |

Scopes stay `edgequake:read` / `query` / `write` (`mcp/oauth/scopes.rs`).
They do not name a document. [F-174-12](04-findings.md).

## Other reads

| Surface | Path | Scope today |
|---------|------|-------------|
| Workspace stats | `handlers/workspaces/stats.rs` | `workspace_id`, 60s cache |
| Lineage and export | `handlers/lineage/queries.rs`, `handlers/lineage/export.rs` | Tenant/workspace on the document, then its chunks |
| Tasks | `handlers/tasks.rs` | Tenant and workspace. `tasks.document_id` (migration 149) |
| Conversations | `handlers/chat/history.rs` | `conversations.user_id` and workspace. `messages.context` stores the bundle |
| Injections | workspace injections route | Glossary text merged into the same graph |
| Pipeline websocket | `GET /ws/pipeline/progress` | Workspace document status |

## WebUI routes

Dashboard and slug routes both call the same APIs:

- `edgequake_webui/src/app/(dashboard)/documents/page.tsx` and `documents/[id]/page.tsx`
- `edgequake_webui/src/app/(dashboard)/graph/page.tsx`
- `edgequake_webui/src/app/(dashboard)/query/page.tsx`
- `edgequake_webui/src/app/(dashboard)/page.tsx` (stats)
- `edgequake_webui/src/app/(dashboard)/pipeline/page.tsx`
- `edgequake_webui/src/app/(dashboard)/knowledge/page.tsx`
- `edgequake_webui/src/app/w/[slug]/documents/page.tsx`
- `edgequake_webui/src/app/w/[slug]/graph/page.tsx`
- `edgequake_webui/src/app/w/[slug]/query/page.tsx`
- `edgequake_webui/src/app/w/[slug]/page.tsx`

Components live under `edgequake_webui/src/components/documents/`.
[10-frontend-architecture](10-frontend-architecture.md).

## Where the PEP attaches

```text
  bind_request
       |
       +-- membership role          (unchanged operation axis)
       |
       +-- VisibilityScope          (new, LAW-174-5)
              |
              +-- list / get / download / assets / pdf
              +-- execute_query and chat  -> allowed_document_ids
              +-- graph BFS / search / PNG
              +-- MCP dispatch
              +-- stats, tasks, lineage, websocket
              |
              +-- with_rls_transaction sets clearance GUCs
```

Next: [03-product-spec](03-product-spec.md).
