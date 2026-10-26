# Lens — AI engineer

Parent: [15-cross-ref](../15-cross-ref.md)

## WHY this lens

Model choice is a capability problem pretending to be a dropdown. An
embedder cannot extract. A chat model cannot embed. A vision page cannot
go to a model with `supports_vision = false`. The gate already has
`capability_mismatch`. The holes are the callers that never reach it, and
the refresh path that never records the dimension.

## Capability rules

| Usage | Card must say |
|-------|----------------|
| extract, query, summary, keyword | `supports_chat` |
| vlm | `supports_vision` |
| embedding | `supports_embedding` and a dimension |
| decision | chat, and the SPEC-160 host when the usage is unset |

`pair_verdict` already returns `CapabilityMismatch` when the card exists and
`facts.supports(usage)` is false. Refresh must fill those flags from the
provider when it can, and from the admin's register form when it cannot.
Guessing `supports_chat = true` for every discovered id is how an embedder
becomes a chat model. The register form starts from the provider's hint and
the admin can correct it. The server stores what was submitted. It does not
invent a flag the admin did not send.

## Discovery versus registered

A cache row is not a catalog card. `eligible_models` keeps
`facts.registered`, which is true for `models.toml` and for
`connection_models`. Cache-only ids stay off the choice list. This preserves
the SPEC-178 rule that a discovery-only id is not eligible.

## Operator lock

Extraction quality pins (`EDGEQUAKE_EXTRACT_LLM_*`) still win, because a
benchmark run must not depend on a tenant ceiling. They win only when the
request did not try to override them. A pin without a credential fails
closed. It does not fall through to another provider.

## What this lens will not add

A router that picks the cheapest eligible model. A retry on a second
provider after a 401. Both hide the admin's choice, which is the failure
this pack removes.
