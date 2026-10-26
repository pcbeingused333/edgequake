# 04 — Findings

Parent: [README](README.md) · Prev: [03-product-spec](03-product-spec.md) ·
Next: [05-gate-law](05-gate-law.md)

Each finding is a behavior read in the tree at SPEC-179 W0. SPEC-178
findings that are already fixed are not repeated. Line numbers drift; the
symbol is the stable cite.

## F-179-01 — Ingest does not ask the gate

`workspace_pipeline_factory` resolves extract and embedding through the
SPEC-123 ladder and `create_safe_*`. `LenientGlobal` then substitutes the
global pipeline (`processor/workspace_resolver.rs`). A withdrawn provider
still extracts.

- **Closes with:** [LAW-179-3](01-first-principles.md) · [LAW-179-2](01-first-principles.md)
- **EC:** [EC-179-01](12-edge-cases.md)

## F-179-02 — Rebuild writes an unjudged pair

`handlers/workspaces/bulk_ops/rebuild_embeddings.rs` and
`rebuild_knowledge_graph.rs` call `update_workspace` with the body's pair.
They do not call `ensure_workspace_choice`.

- **Closes with:** [LAW-179-3](01-first-principles.md)
- **EC:** [EC-179-02](12-edge-cases.md)

## F-179-03 — Vision has a private fallback chain

`services/vlm_provider_resolver.rs` walks workspace, env, the startup vision
provider, then the server text LLM. `reprocess.rs` reads `vision_llm_provider`
and defaults the provider string to `ollama`. PDF upload judges only a
client-sent pair (`pdf_upload/upload.rs`).

- **Closes with:** [LAW-179-2](01-first-principles.md)
- **EC:** [EC-179-03](12-edge-cases.md)

## F-179-04 — Keyword, summary, and query embedding fail open

`resolve_keyword_llm` swallows errors and uses the query client.
`injection_processing.rs` falls back to `self.llm_provider`.
`resolve_embedding_provider` warns and returns the process embedder.

- **Closes with:** [LAW-179-2](01-first-principles.md)
- **EC:** [EC-179-04](12-edge-cases.md)

## F-179-05 — A fleet slug does not resolve from the pair

`try_llm_from_workspace_connection` runs only when `llm_roles` stores
`connection_id`. A choice of `{provider: <fleet slug>, model}` has no path
to that row. `refuse_unoffered` checks the slug is offered, then the factory
looks up a built-in name and misses.

- **Closes with:** [LAW-179-4](01-first-principles.md)
- **EC:** [EC-179-05](12-edge-cases.md)

## F-179-06 — Native providers are invisible to the offer

`env_connections` in `handlers/connections.rs` emits ollama, openai,
openai-compatible, omlx, anthropic, lmstudio, llamacpp, mlx-lm, vllm-mlx,
and mtplx. `models.toml` also has mistral, azure, xai, openrouter, gemini,
and others. Those stay un-offered even when their API key env is set.

- **Closes with:** [LAW-179-7](01-first-principles.md)
- **EC:** [EC-179-06](12-edge-cases.md)

## F-179-07 — Refresh ignores the connection

`POST /models/discover/refresh` clears a process-wide cache. Discovery reads
process env, not `provider_connections`. There is no per-slug route, no
stored diff, and no `last_refreshed_at`. The UI then reads
`GET /models/llm?provider=`, which is chat inventory. Embedding models cannot
be registered because the client body forces `supports_embedding: false`.

- **Closes with:** [LAW-179-6](01-first-principles.md)
- **EC:** [EC-179-07](12-edge-cases.md) · [EC-179-08](12-edge-cases.md)

## F-179-08 — Connection writes are unsafe

`handlers/connections.rs` `upsert` stores the secret before the duplicate
check. `parse_slug` is not applied on create. `last_test_ok` survives an
edit of `base_url`, key, or shape. `delete_connection` matches
`tenant_id IS NULL OR tenant_id = $2`, so a tenant-scoped request can delete
a fleet row. `ssrf.rs` does not reject userinfo. The fingerprint uses
`DefaultHasher`.

- **Closes with:** [LAW-179-5](01-first-principles.md)
- **EC:** [EC-179-09](12-edge-cases.md) · [EC-179-10](12-edge-cases.md) · [EC-179-11](12-edge-cases.md)

## F-179-09 — Deployment choice fails open when the offer is empty

`ensure_deployment_choice` returns `Ok` when `global_offer` is empty.
Workspace and tenant checks do not. `refuse_unoffered` returns 400 and does
not judge the model against the ceiling.

- **Closes with:** [LAW-179-2](01-first-principles.md)
- **EC:** [EC-179-12](12-edge-cases.md)

## F-179-10 — No impact, no revision, no audit

Offer and policy PUTs replace rows. Nothing lists the assignments that
become illegal. Nothing sends `ETag`. `services/audit.rs` exists and these
handlers do not call it. Two admins saving the ceiling can merge or 409
without a retry hint.

- **Closes with:** [LAW-179-8](01-first-principles.md)
- **EC:** [EC-179-13](12-edge-cases.md) · [EC-179-14](12-edge-cases.md)

## F-179-11 — Choice UI still leaks inventory

`components/models/model-picker-panel.tsx` calls `GET /models/search` and
merges hits into the eligible options. `use-server-model-defaults.ts` reads
`GET /models`. `model-defaults-step.tsx` uses `choiceScope="tenant"` with no
tenant. `reconfigure-workspace-wizard.tsx` passes the tenant id.
`use-providers.ts` and `use-models.ts` both export `useLlmModels`.

- **Closes with:** [LAW-179-9](01-first-principles.md) · [LAW-179-10](01-first-principles.md)
- **EC:** [EC-179-15](12-edge-cases.md) · [EC-179-16](12-edge-cases.md)

## F-179-12 — Provider screens omit the lifecycle

`connections-card.tsx` has no delete, no rotate, and a toast-only test.
`provider-offer-editor.tsx` `GrantEditor` renders an empty offer on error.
`FleetModels` is a chat-id checklist. Refresh has no progress. The vision
card and the `vlm` role row both write. The dropzone has no vision picker.
Settings strings are hard-coded. The matrix does not stick the model column
at 390px.

- **Closes with:** [LAW-179-10](01-first-principles.md)
- **EC:** [EC-179-17](12-edge-cases.md) · [EC-179-18](12-edge-cases.md)

## F-179-13 — Tests grep source instead of behavior

`edgequake-api/tests/spec178_model_choice.rs` locks strings in source files.
No test runs ingest with a withdrawn provider and expects `model_not_eligible`.

- **Closes with:** [LAW-179-12](01-first-principles.md)
- **EC:** [EC-179-19](12-edge-cases.md)
