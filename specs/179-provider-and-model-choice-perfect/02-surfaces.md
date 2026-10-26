# 02 — Surfaces

Parent: [README](README.md) · Prev: [01-first-principles](01-first-principles.md) ·
Next: [03-product-spec](03-product-spec.md)

## WHY

A surface is any place a human picks a model or a machine builds a client.
SPEC-178 closed the human side for query and the workspace editor. The
machine side, and a few wizards, still read a different list. This page is
the inventory of those places so the gate has a complete caller list.

## Runtime callers

| Surface | File | Usage | Today | After |
|---------|------|-------|-------|-------|
| Query | `handlers/query/query_execute.rs`, `query_stream.rs` | query | judged | gate |
| Chat | `handlers/chat/completion.rs`, `streaming.rs` | query or vlm | judged | gate |
| Title | `handlers/chat/title_generator.rs` | query (same client) | inherits chat client | no second client |
| MCP | `mcp/gateway/dispatch.rs` via `query_context.rs` | query | offer only if pair omitted | gate |
| Keyword | `services/query_execution.rs` `resolve_keyword_llm` | keyword | env or role, errors swallowed | gate, fail closed |
| Query embed | `providers/resolver.rs` `resolve_embedding_provider` | embedding | process fallback | gate |
| Ingest LLM | `workspace_pipeline_factory.rs` | extract | ladder, then `LenientGlobal` | gate |
| Ingest embed | `workspace_pipeline_factory.rs` | embedding | same | gate |
| Summarizer | `processor/injection_processing.rs`, `text_insert/persist.rs` | summary | `self.llm_provider` fallback | gate |
| PDF vision | `handlers/pdf_upload/upload.rs`, `edgequake-pdf` `backend/vision.rs` | vlm | client pair only, else factory | gate |
| VLM chain | `services/vlm_provider_resolver.rs` | vlm | workspace, env, startup, text LLM | gate |
| Reprocess | `handlers/documents/recovery/reprocess.rs` | vlm | raw column, `"ollama"` default | gate |
| Rebuild | `bulk_ops/rebuild_embeddings.rs`, `rebuild_knowledge_graph.rs` | embedding, extract | unvalidated write | `ensure_workspace_choice` then gate |
| Parse | `handlers/parse/service.rs` | vlm or query | `ProviderFactory` | gate |
| Decision | `workspace_pipeline_decision.rs` | decision | host `EDGEQUAKE_DECISION_*` | gate for the embedder; decision backend stays the SPEC-160 host when `decision` usage is unset |

Boot (`state/provider_setup.rs`, `ProviderFactory::from_env`) stays. It builds
the process client the operator lock and health checks use. It is not a
per-request choice. The allow-list for the constructor guard is that module
plus `model_gate.rs`.

## Human surfaces

| Surface | Reads today | Must read |
|---------|-------------|-----------|
| Query chip | eligible `query` | same, plus capability fields |
| Workspace role editor | eligible per usage | same |
| Tenant ceiling | tenant catalog | parent offer, not raw `/models` |
| Workspace narrowing | eligible | same |
| Deployment fleet card | eligible `scope=deployment` | same |
| `ModelPickerPanel` | eligible **and** `/models/search` | eligible only, except the refresh review |
| Create-workspace wizard | tenant eligible + search | workspace-to-be: tenant eligible |
| First-run / create-tenant | tenant eligible with no tenant | `scope=deployment` |
| Reconfigure wizard | tenant eligible | workspace eligible |
| Server-defaults card | `GET /models` | `GET /config/effective` or deployment eligible |
| Upload dropzone | no vision picker | eligible `vlm` |
| Vision card and role row | two writers | role row is the writer; card links to it |

## Inventory, kept

`GET /models`, `/models/llm`, `/models/embedding`, `/models/search`,
`/models/health` stay for the Deployment refresh review and for diagnostics.
Choice components do not import them. [LAW-179-9](01-first-principles.md).
