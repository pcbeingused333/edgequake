# Lens — AI Engineer

Parent: [README](../README.md) · Law: [05-resolution-law](../05-resolution-law.md)

## What this lens owns

[LAW-175-2](../01-first-principles.md), [LAW-175-3](../01-first-principles.md),
and [LAW-175-10](../01-first-principles.md).

Usage is a type. The model that runs is the model `resolve_usage`
returns. Provenance is a field, not a log line someone might forget.

## Usage map

| Usage | Must be able to | Must not be |
|-------|-----------------|-------------|
| extract | JSON mode | an embedding card |
| query, summary, keyword | chat | a vision-only id unless it also chats |
| vlm | vision | a text embed |
| embedding | known dimension | a chat id |
| decision | `tev1` catalog | a `llm_roles` chat row |
| rerank | the process reranker | a workspace assignment in W1–W6 |

Reasoning effort stays SPEC-109. Pass the tenant into
`resolve_role_reasoning_effort`. The query path passes `None` today.
Clamp stays in `edgequake-llm`. Omit flags stay env
([EC-175-14](../11-edge-cases.md)).

## Locks

`EDGEQUAKE_EXTRACT_LLM_*` and `EDGEQUAKE_KEYWORD_LLM_*` are
`operator_lock`. `EDGEQUAKE_LLM_PROVIDER` is `env_bootstrap`.
Manuscript env locks manuscript pages only ([EC-175-21](../11-edge-cases.md)).
The compiled model fill inside `env_role_llm` (the `"gpt-5.4-nano"`
branch) moves to the catalog default for that provider so two tables
cannot disagree ([F-175-17](../04-findings.md)).

## Caches and corpus

Answer cache hash includes the resolved model
([EC-175-27](../11-edge-cases.md)). A new extract model does not
reprocess ([EC-175-18](../11-edge-cases.md)). Lineage columns stay the
record of what already ran.

## Fail closed

A probe failure does not run the compiled default and does not run the
process key ([EC-175-32](../11-edge-cases.md)). A wrong model in the
graph is worse than a 503 the owner can see.
