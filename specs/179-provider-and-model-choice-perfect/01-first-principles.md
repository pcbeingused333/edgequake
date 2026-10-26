# 01 — First principles

Parent: [README](README.md) · Prev: [00-why](00-why.md) · Next: [02-surfaces](02-surfaces.md)

```text
  fact  --->  one writer  --->  one judge  --->  every caller
  offer        offer PUT         ModelGate        chip, save, query,
  policy       policy PUT                         ingest, vision, MCP
  secret       connection PUT
  refresh      refresh POST
```

## LAW-179-1 — One judge

`ModelGate::resolve` is the only function that returns a provider client for
a usage. `eligible_models` stays pure and decides membership.
`resolve_usage` stays pure and decides which stored layer wins.
The gate loads facts, calls both, and builds the client.

A handler that calls `ProviderFactory` itself is a second judge.

## LAW-179-2 — Fail closed

| Situation | Result |
|-----------|--------|
| Pair not in the eligible set | `422 model_not_eligible` or task `Failed` with that code |
| Model lacks the usage capability | `422 capability_mismatch` |
| One of provider or model | `422 model_pair_incomplete` |
| Operator lock and a request pair | `422 usage_locked` |
| Offer rows cannot be loaded | `503 offer_unavailable` |
| No credential for the resolved pair | `422 credential_missing` |
| Operator lock set, request empty | the lock runs, offer skipped |

No branch substitutes `state.query.llm_provider`, `ProviderFactory::from_env`,
or the global pipeline because the preferred pair failed.

## LAW-179-3 — Same set everywhere

These usages go through the gate: `extract`, `query`, `summary`, `vlm`,
`keyword`, `embedding`, `decision`. `rerank` stays deployment-only and is
not a workspace choice (SPEC-175).

Callers include HTTP query and chat, streaming, MCP retrieve, title
generation, the ingest pipeline, the summarizer, PDF and manuscript vision,
reprocess, rebuild embeddings, and rebuild knowledge graph.

## LAW-179-4 — Slug resolves

The gate maps an offered slug to a fleet `provider_connections` row when the
slug is not a built-in provider. The connection id does not have to be
repeated on `llm_roles`. A slug with no passing probe is not offered
(SPEC-177) and the gate refuses it.

## LAW-179-5 — Secrets stay on Deployment

Create, rotate, test, and delete of a connection are Platform Admin routes.
Responses carry `key_configured` and a fingerprint, never the key. A URL
with userinfo or a query string is rejected at write time. A blank key on
update keeps the stored key. The UI says so.

## LAW-179-6 — Refresh is per provider and durable

`POST /deployment/providers/{slug}/refresh` uses that connection's base URL
and secret. The diff is stored. New ids stay discovery-only until
`PUT .../models` registers them. An embedding model requires
`embedding_dimension`.

## LAW-179-7 — A keyed native provider is offered

If `models.toml` has the provider and the credential env var is set,
`env_connections()` emits a row. The admin can withdraw it. The admin does
not need a fleet duplicate to see it.

## LAW-179-8 — Writes preview impact

Withdraw, offer replace, policy replace, and connection delete compute the
stored assignments they will break. The UI confirms. The write does not
choose a replacement model.

## LAW-179-9 — The server is the authority

The picker hiding a row is not the control. A raw PUT or a raw query with
the same pair gets `422`. Inventory routes stay for operators and are not
what choice controls call.

## LAW-179-10 — One hook, one menu

`useScopedEligibleModels(scope, usage)` and `ModelChoiceMenu` are the choice
UI. `useLlmModels` / `useEmbeddingModels` duplicates go away. Wizards with
no tenant id use `scope=deployment`. Reconfigure uses the workspace scope.

## LAW-179-11 — No new secret backend

SPEC-176 `database` and `infisical` remain. This pack does not add Vault
or AWS Secrets Manager.

## LAW-179-12 — Every EC has a test

[12-edge-cases](12-edge-cases.md) names a test in
[14-e2e-test-matrix](14-e2e-test-matrix.md). A case without a test is not done.

## DRY and SOLID

| Principle | Where it binds |
|-----------|----------------|
| SRP | The gate judges. Handlers translate HTTP. The pipeline does not know offer SQL. |
| OCP | A new usage is a `Usage` arm plus a caller. It does not grow a new eligibility function. |
| LSP | Every caller gets `Result<Resolved, GateError>`. `Ok` means the pair may run. |
| ISP | The picker asks for one usage. It does not download the inventory to filter locally. |
| DIP | Callers depend on `ModelGate`, not on `ProviderFactory`. |
| DRY | One offer load per resolve. One menu component. One eligible hook. |

## What an operator lock still may do

A pin (`EDGEQUAKE_EXTRACT_LLM_MODEL` and the siblings in
`operator_lock_set`) skips the offer for that usage when the request does
not name a pair. The pin's own credential must exist. A request pair on a
locked usage is `usage_locked`. This is [LAW-177-8](../177-provider-cascade/01-first-principles.md), unchanged.
