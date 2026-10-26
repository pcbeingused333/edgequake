# 00 — Why

Parent: [README](README.md) · Next: [01-first-principles](01-first-principles.md)

SPEC-177 decided which providers a scope may see. SPEC-178 decided that the
list, the save, and the query use one set. Both are in the tree. A document
can still be extracted by a model the workspace admin turned off, because the
pipeline never asks the set. This pack exists to close that hole and to make
adding a provider obvious.

## WHY A — A filter that a worker can skip is not a filter

The chip reads `GET /models/eligible`. The ingest worker builds a client from
`llm_roles` and, when that fails, from the process environment. The two
answers diverge the moment an admin withdraws a provider. Members see one
model. The corpus is built with another.

**First principle:** the constraint is a property of the model use, not of the
HTTP handler that happened to grow the check first.

**Closes with:** [LAW-179-1](01-first-principles.md) · [LAW-179-3](01-first-principles.md)

## WHY B — Silent fallback hides the admin's decision

`Ok(None)` from the resolver means "use `state.query.llm_provider`".
`LenientGlobal` means "use the process pipeline". A missing key, a withdrawn
slug, and a database error take the same path as "the user did not choose".
The admin cannot tell a run ignored them.

**First principle:** absence of a legal model is an error. The process default
is a model, so it needs the same judgment. An operator lock is the only case
where the process wins on purpose.

**Closes with:** [LAW-179-2](01-first-principles.md)

## WHY C — An offered provider that cannot run is a lie

A fleet connection is a slug plus a secret plus a base URL. The offer lists
the slug. Runtime resolution looks up a client by the provider name in
`models.toml` or by `llm_roles.*.connection_id`. A custom slug has neither,
unless a role happens to store the connection id. The admin enabled something
the product cannot call.

**First principle:** the name on the offer is the name the gate resolves.
Resolution does not require a second identifier the UI forgot to save.

**Closes with:** [LAW-179-4](01-first-principles.md)

## WHY D — The catalog is frozen to whoever edited `models.toml`

Gemini, Mistral, xAI, OpenRouter, Azure, Bedrock, and Vertex have cards.
`env_connections()` does not list them, so a deployment that holds
`GEMINI_API_KEY` still cannot offer Gemini. Discovery for a fleet connection
uses the process environment, not the connection's own key and base URL.
Refreshing models asks the wrong credential.

**First principle:** a provider the binary knows, and for which a secret
exists, is a provider the admin can offer. Refresh uses that secret.

**Closes with:** [LAW-179-6](01-first-principles.md) · [LAW-179-7](01-first-principles.md)

## WHY E — The screen teaches the wrong model

The Deployment connections card cannot delete or rotate. Refresh is a global
button with no time and no diff. The model picker searches the raw catalog,
so a ceiling the admin just saved is not what the wizard shows. Two hooks
fetch the same inventory on two clocks. The person configuring the product
cannot see cause and effect.

**First principle:** every choice control renders the eligible set for its
scope. Every mutation shows what it will break before it breaks it.

**Closes with:** [LAW-179-8](01-first-principles.md) · [LAW-179-9](01-first-principles.md) · [LAW-179-10](01-first-principles.md)

## What this pack refuses to do

- Add Vault or AWS Secrets Manager. [LAW-179-11](01-first-principles.md).
- Promote a discovered id to a registered card without an admin tick.
- Retarget a broken assignment to "the next model". The admin chooses.
- Invent a fifth scope. Deployment, Organization, Workspace, You stay.
