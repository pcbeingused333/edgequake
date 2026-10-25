# 06 — Catalog, capabilities, probes

Parent: [README](README.md) · Prev: [05-resolution-law](05-resolution-law.md) ·
Next: [07-data-and-db-contract](07-data-and-db-contract.md)

[LAW-175-3](01-first-principles.md) and [LAW-175-6](01-first-principles.md).
The catalog answers "what exists and what it can do." Policy answers
"what this tenant or workspace turned on." The resolver answers "what
runs."

## Where models come from today

| Source | Path | Used for |
|--------|------|----------|
| Static cards | `edgequake/models.toml`, else `EDGEQUAKE_MODELS_CONFIG`, else `~/.edgequake/models.toml` | capabilities, compiled defaults |
| Provider list | enabled providers minus `mock`, optional `EDGEQUAKE_ALLOWED_PROVIDERS` | `GET /models` |
| Live discovery | `ModelCatalog::fetch_discovered` | Ollama tags, OpenAI ids, others |
| Discovery off | `EDGEQUAKE_DYNAMIC_MODELS=0` | static cards only |
| Connections | `provider_connections` | URL and key, not the model list |
| Decision | `GET /decision/models` | `tev1` only |

OpenAI `GET /v1/models` returns ids. Capabilities stay on the static card.
Ollama `/api/tags` can overwrite vision, thinking, and embedding flags.
An id with no card and no Ollama capability array is hidden, unless the
provider allows a custom id and a probe with that id succeeded.
[EC-175-07](11-edge-cases.md) · [EC-175-10](11-edge-cases.md) ·
[EC-175-11](11-edge-cases.md).

## Capability predicates

| Usage | Eligible when |
|-------|----------------|
| extract | `ModelType` is llm or multimodal, `supports_json_mode`, provider `features.chat` |
| query | provider `features.chat`, model is not an embedding card |
| summary | same as query |
| keyword | same as query |
| vlm | `supports_vision` or Ollama capability `vision` |
| embedding | provider `features.embedding` and `embedding_dimension` is known |
| decision | id is in the decision catalog for the configured backend |
| rerank | not listed in the chat or embed pickers |

Reasoning effort UI: show the select only when
`reasoning_capabilities::capabilities` returns a table, or when live
Ollama tags include `thinking`. Otherwise the control is absent, not
disabled with a fake value. Omit-temperature stays a fleet env flag
([SPEC-131](../131-env-config-temp-openai-support/)).

`EDGEQUAKE_ALLOWED_PROVIDERS` remains a deployment filter under the
catalog. It is not the tenant ceiling. A provider absent from that list
never appears, even if a tenant row names it.
[EC-175-53](11-edge-cases.md). A probe older than 24 hours is the same
kind of filter unless env health still reports the provider up.

## Policy

```text
  healthy models for the usage
           |
           v
  +---------------------------+
  | tenant ceiling            |
  | zero rows = pass all      |
  +-------------+-------------+
                |
                v
  +---------------------------+
  | workspace narrowing       |
  | zero rows = pass ceiling  |
  | rows may only subtract    |
  +-------------+-------------+
                |
                v
  eligible_models(usage, workspace)
```

A ceiling row is `workspace_id` null. A narrowing row sets
`workspace_id`. `enabled = false` on a ceiling row removes the pair for
every workspace. `enabled = false` on a workspace row removes it there
only. A workspace cannot insert an `enabled = true` row for a pair the
ceiling has disabled or omitted once the ceiling is non-empty.
[EC-175-30](11-edge-cases.md).

The first ceiling write is a mode change. [LAW-175-9](01-first-principles.md).
The API returns `policy_mode: unconstrained | constrained` so the UI can
show the confirm sentence in [03-product-spec](03-product-spec.md).

## Probes

A provider is healthy when at least one of these is true:

- a `provider_connections` row for this tenant, or a fleet row, has
  `last_test_ok = true` and `last_test_at` within 24 hours
- the process env for that provider is set and `GET /models/health`
  reports it up

Failed probe: the model stays in the admin catalog with status "probe
failed" and is absent from member pickers. Assignments that point at it
become `broken_reason = probe`. [EC-175-04](11-edge-cases.md).

Custom model ids (Ollama, OpenAI-compatible proxies, LM Studio):

- the provider card says `allows_custom_model`
- the admin types an id
- `POST /providers/test` with that id must succeed
- the id is then a normal policy row, still tagged with usages whose
  predicates the admin confirms (the probe response includes the
  capability bits the provider returned; missing bits stay false)

## What the member sees

`eligible_models(query, workspace)` after ceiling, narrowing, capability,
and probe. Inherit is a row, not a blank. The effective model for inherit
is `resolve_usage` with an empty request. The chip does not offer extract
models that failed the JSON predicate, embedding models, or decision
models.

## Deployment filters that stay env

These do not become tenant rows in this pack:

- `EDGEQUAKE_DYNAMIC_MODELS`
- `EDGEQUAKE_ALLOWED_PROVIDERS`
- `EDGEQUAKE_DECISION_BACKEND` and the decision URL and key
- `EDGEQUAKE_RERANKER*`
- `EDGEQUAKE_LLM_API_FORMAT`, omit-temperature, omit-reasoning
- manuscript DPI and verify flags (SPEC-134)

The Deployment scope may explain them. It does not pretend to edit them
until a later spec stores them. Langfuse stays read-only
([SPEC-124](../124-langfuse-support/)).
