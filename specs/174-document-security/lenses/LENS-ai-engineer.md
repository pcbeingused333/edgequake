# Lens — AI Engineer

Parent: [README](../README.md) · Query and prompts: [06-graph-query-ai](../06-graph-query-ai.md)

## What this lens owns

[LAW-174-11](../01-first-principles.md) and [LAW-174-16](../01-first-principles.md).
The model, the answer cache, and the retrieval bundle see only the visible
projection.

## Prompt

`query_pipeline.rs` hashes the prompt after context filtering
([F-174-08](../04-findings.md)). W5 must make that context the visible set
before the hash. Do not append "N documents were omitted." That sentence
is an existence leak.

Citations are chunk texts that survived `filter_context_by_document_ids`.
Entity descriptions in the prompt are visible fragments, not
`entities.description`, while enforcement is on.

Keyword extraction does not see document text in its cache
(`hash_keyword_args`). Leave that cache shared.

## Five modes, one list

Naive, local, global, hybrid, and mix already take `allowed_document_ids`.
Fill it from `VisibilityScope`. Do not add a mode. Do not post-filter a
global top-k and call it authorization: hidden hits would consume the
budget and change latency ([16-performance](../16-performance.md)).

## Caches

| Store | Change |
|-------|--------|
| Answer | Keep prompt hash. Test that two clearances produce two prompts ([EC-174-26](../12-edge-cases.md)) |
| Context | Add fingerprint and `policy_epoch` to `cache_key` ([EC-174-27](../12-edge-cases.md)) |
| Retrieval id | Bind to user, fingerprint, epoch. Mismatch is 404 ([EC-174-25](../12-edge-cases.md)) |
| SPEC-126 prefix | Vary the prefix when it contains retrieved text. Static instructions may stay shared |

Conversation history is filtered at read ([EC-174-23](../12-edge-cases.md)).
A revoked clearance must not re-enter the next prompt through `messages.context`.

## Ingest

The extraction LLM may read the document it was given. It must not be
asked to summarize two documents into one description while enforcement
is on. The system writer stores both fragments. It does not run as the
uploader's clearance, or the other fragment would never be stored.

Glossary injections are context. They use the same predicate
([EC-174-38](../12-edge-cases.md)).
