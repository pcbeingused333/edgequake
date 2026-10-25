# Lens — AI engineer

Parent: [README](../README.md) · Cascade: [05-cascade-law](../05-cascade-law.md)

## What this lens owns

[LAW-177-2](../01-first-principles.md), [LAW-177-5](../01-first-principles.md),
and [LAW-177-8](../01-first-principles.md). The offer decides whether a
provider may be chosen. The capability predicate decides whether a model
may fill a usage. `resolve_usage` decides which pair runs. Those three
steps stay in that order.

## Usages

[LAW-175-3](../../175-perfect-settings/01-first-principles.md) is unchanged.

| Usage | Still required | Offer adds |
|-------|----------------|------------|
| extract | chat and JSON mode | provider in the workspace offer |
| query, summary, keyword | chat | same |
| vlm | vision | same |
| embedding | embedding and a known dimension | same |
| decision | `tev1` catalog, env backend | not a grantable provider in this pack |
| rerank | process flag | not a grantable provider in this pack |

Enabling a provider does not mark an embedding model eligible for extract.
[EC-177-17](../11-edge-cases.md). Withdrawing the provider drops chat,
vision, and embedding together. [EC-177-28](../11-edge-cases.md).

## The ladder

Steps stay [LAW-175-2](../../175-perfect-settings/01-first-principles.md).

- Step 1, operator lock: the offer does not apply. Missing credential is
  still a credential failure, not a swap to another provider.
  [EC-177-10](../11-edge-cases.md), [EC-177-27](../11-edge-cases.md),
  [EC-177-39](../11-edge-cases.md).
- Steps 2–8: a pair whose provider is outside the workspace offer is
  `not_eligible`. The resolver does not continue to the next layer and
  does not call `ProviderFactory::from_env`. [EC-177-06](../11-edge-cases.md).

A custom connection's models are `connection_models` keyed by that
slug. Shape `openai_chat` does not attach the `openai` cards.
[EC-177-44](../11-edge-cases.md). The probe that confirms a custom id
stays `POST /providers/test`. This pack does not add a second probe.

## Caches

The resolved model id stays inside the cache key the way SPEC-175 requires.
Changing the offer does not delete cache rows. A later call misses when
the model id differs. [EC-177-43](../11-edge-cases.md).

Manuscript vision locks stay `EDGEQUAKE_VISION_*_MANUSCRIPT` and follow
LAW-177-8 only on manuscript-class pages, as SPEC-175 already scopes that
lock.

## Out of this lens

The settings nav, the dialog copy, and the partial unique indexes.
