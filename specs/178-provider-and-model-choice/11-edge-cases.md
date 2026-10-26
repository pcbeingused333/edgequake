# 11 — Edge cases

Parent: [README](README.md) · Prev: [10-frontend-architecture](10-frontend-architecture.md) ·
Next: [12-implementation-plan](12-implementation-plan.md)

Each case names the mitigation and the gate. The gate's file is in
[13-e2e-test-matrix](13-e2e-test-matrix.md). A case with no mitigation
is not done.

## The list and the chip

### EC-178-01 — Chip lists a model the ceiling or the narrowing removed

Today the chip reads `/models/llm` ([F-178-01](04-findings.md)).
Mitigation: the chip reads `eligible(workspace, query)`. The id is
absent. [LAW-178-2](01-first-principles.md).

### EC-178-07 — Discovery-only id

`discovery_source = dynamic_api` is on the inventory and unregistered
([F-178-17](04-findings.md)). Mitigation: choice lists keep
`registered`. The id stays off the chip, the role row, and the fleet
card. The inventory may keep it. [LAW-178-8](01-first-principles.md).

### EC-178-08 — Static card with no credential

`available: Some(true)` on every static card ([F-178-03](04-findings.md)).
Mitigation: a provider absent from the scope's offer is absent from the
choice list. The inventory may still show the card. Choice does not.
[LAW-178-8](01-first-principles.md).

### EC-178-09 — Unconstrained is not the inventory

Zero ceiling rows means every registered, capable, offered model, not
every `models.toml` row ([LAW-177-4](../177-provider-cascade/01-first-principles.md)).
Mitigation: the chip length equals that eligible set, including when
`policy_mode` is `unconstrained`. [LAW-178-2](01-first-principles.md).

### EC-178-19 — Failed probe

`probe = Some(false)` fails `eligible_models`. Mitigation: the member
menu omits the model. The admin catalog endpoint still returns it with
`health = probe_failed`. A stored assignment shows Broken.
[09-ux-ui-spec](09-ux-ui-spec.md).

### EC-178-20 — Withdrawn or ungranted provider

Mitigation: no group in the menu, not a disabled row.
[LAW-177-1](../177-provider-cascade/01-first-principles.md) ·
[LAW-178-1](01-first-principles.md).

### EC-178-21 — Legacy tenant connection

A `provider_connections` row with `tenant_id` set stays out of the
global offer. Mitigation: it is not in tenant-scope or workspace-scope
eligible. Existing assignments that name it resolve as broken for a new
save; a save cannot select it. [LAW-178-1](01-first-principles.md).

### EC-178-22 — `mock`

Mitigation: the server omits `mock` from eligible payloads. The menu
also drops `mock` if a fixture injects it. [LAW-178-8](01-first-principles.md).

### EC-178-36 — Embedding-only model on a chat menu

Mitigation: `supports(query)` requires chat. The query chip, the family
control, and a query save omit an embedding-only id. The embedding row
lists it. A query body that names it is `422 capability_mismatch`.
[LAW-178-3](01-first-principles.md) · [LAW-178-6](01-first-principles.md).

## The save

### EC-178-03 — Free-string assignment

PUT workspace, PUT tenant, and setup accept any pair
([F-178-04](04-findings.md), [F-178-05](04-findings.md)).
Mitigation: `422 model_not_eligible` and no write.
[LAW-178-3](01-first-principles.md).

### EC-178-12 — One of provider or model

Chat fills the missing half ([F-178-08](04-findings.md)).
Mitigation: `422 model_pair_incomplete` on chat, query, and the saves.
No gap-fill. [LAW-178-6](01-first-principles.md).

### EC-178-14 — Embedding dimension guessed

Unknown models become 1536. Mitigation: the save copies the card
dimension. A disagreeing body is `422 embedding_dimension_mismatch`.
The 1536 guess is not on this path. Legacy rows stay until edited.
[LAW-178-6](01-first-principles.md).

### EC-178-29 — Family key versus one usage

A model allowed for query and removed for extract must not become
`llm_model`. Mitigation: the family control and the family save use the
intersection. The extract role may still select a model the family
control hides, if that model is eligible for extract.
[05-choice-law](05-choice-law.md).

### EC-178-34 — Clear is inherit

`""`, `"none"`, and null clear the override today.
Mitigation: those values skip `PairVerdict` and persist the clear.
They are not `422`. [LAW-178-3](01-first-principles.md).

### EC-178-35 — Narrowing cannot widen

Unchanged SPEC-175 rule. Mitigation: this pack does not add a choice
write that inserts an enabled narrowing row for a pair the ceiling
omitted. Assignment of such a pair is `422 model_not_eligible`.

### EC-178-38 — Save does not rebuild

Mitigation: workspace embedding save does not enqueue rebuild. The
rebuild route is the only enqueuer. [LAW-175-14](../175-perfect-settings/01-first-principles.md).

### EC-178-41 — No new relation

Mitigation: the migration directory gains no `224_spec178_*.sql`.
Schema tests that list relations see the same set as head 223.
[LAW-178-4](01-first-principles.md).

## The run

### EC-178-02 — Stream and `/query` disagree

Streaming skips `ensure_pair_eligible` ([F-178-07](04-findings.md)).
Mitigation: both doors return `422 model_not_eligible` for the same
pair. [LAW-178-5](01-first-principles.md).

### EC-178-11 — Connection discards the chip

`connection_id` returns before the request ([F-178-11](04-findings.md)).
Mitigation: a request pair that is eligible runs. The connection runs
only when the request is empty. The effective row's `source` matches
the winner. [LAW-178-9](01-first-principles.md).

### EC-178-13 — Credential miss uses the process provider

`Ok(None)` falls through ([F-178-10](04-findings.md)).
Mitigation: the handler returns the credential or eligibility error and
does not call `state.query.llm_provider`. [LAW-178-5](01-first-principles.md).

### EC-178-23 — Omitted pair, stored illegal assignment

`ensure_pair_eligible` returns Ok when the body is empty
([F-178-06](04-findings.md)). Mitigation: the resolved pair is judged.
Illegal stored assignment refuses. The row is not rewritten.
[LAW-178-3](01-first-principles.md).

### EC-178-24 — Images swap the process vision model

([F-178-16](04-findings.md)). Mitigation: image turns resolve `vlm`.
No substitution of `state.query.vision_llm_provider`. A chat-only
request pair is `422 capability_mismatch`. [LAW-178-6](01-first-principles.md).

### EC-178-25 — Offer load fails open

([F-178-12](04-findings.md)). Mitigation: `503 offer_unavailable` on
the choice paths. No provider call. [LAW-178-5](01-first-principles.md).

### EC-178-30 — Auth failure retries the process provider

([F-178-09](04-findings.md)). Mitigation: chat and query handlers do
not call the auth-fallback functions. The auth error is the response.
[LAW-178-5](01-first-principles.md).

### EC-178-31 — Raw client

Mitigation: a curl or MCP body with a pair outside the set receives the
same 422 as the WebUI. No picker is required for the refusal.
[LAW-178-7](01-first-principles.md).

### EC-178-37 — Pair sent while locked

Mitigation: `422 usage_locked`. The lock model is not silently run in
place of the named pair. An empty body still runs the lock.
[LAW-178-5](01-first-principles.md).

## Scope and who may look

### EC-178-15 — No workspace id yet

Create and onboarding read the inventory ([F-178-02](04-findings.md)).
Mitigation: `tenant_id` scope. The ids match the new workspace's
eligible set before any narrowing. [LAW-178-2](01-first-principles.md).

### EC-178-16 — Fleet card shrinks to one workspace

Mitigation: deployment scope passes an empty ceiling and an empty
narrowing. A workspace narrowing row does not remove a fleet option.
[LAW-178-2](01-first-principles.md).

### EC-178-32 — Member calls a wider scope

Mitigation: `tenant_id` or `scope=deployment` without the grant is
`403` and an empty body. [08-api-contract](08-api-contract.md).

### EC-178-39 — Workspace id in another tenant

Mitigation: the existing workspace not-found response. The body is not
the other tenant's models.

## Browser

### EC-178-04 — Persisted pair survives a ceiling save

The sanitizer compares against the inventory ([F-178-13](04-findings.md)).
Mitigation: compare against the eligible payload. Loaded-and-absent
clears the slot. Not-loaded does not. [LAW-178-7](01-first-principles.md).

### EC-178-05 — Switch keeps the previous pair

One global slot ([F-178-19](04-findings.md)).
Mitigation: the slot is keyed by workspace id. [LAW-178-7](01-first-principles.md).

### EC-178-06 — Inherit shows the process model

([F-178-14](04-findings.md)). Mitigation: the label is the effective
query row. The trigger does not read `default_model`.
[LAW-178-3](01-first-principles.md).

### EC-178-10 — Operator lock

Mitigation: the trigger is disabled, the label says Locked, the pair
shown is the lock pair. [09-ux-ui-spec](09-ux-ui-spec.md).

### EC-178-17 — Clocks

Two inventory hooks and a 15-second eligible query ([F-178-15](04-findings.md)).
Mitigation: choice uses the eligible key only. Offer, policy, and
withdraw mutations invalidate that prefix. [LAW-178-7](01-first-principles.md).

### EC-178-18 — Empty eligible set

Mitigation: Inherit only. Member Send is disabled. The query refuses
rather than picking an inventory model. Copy in
[09-ux-ui-spec](09-ux-ui-spec.md). [LAW-178-3](01-first-principles.md).

### EC-178-27 — Two workspaces, one browser

Mitigation: `queryModelsByWorkspace[id]`. Rehydrate copies a legacy
global pair onto the current workspace once. [10-frontend-architecture](10-frontend-architecture.md).

### EC-178-28 — Composer open while the ceiling changes

Mitigation: invalidation refetches eligible. The toast fires when the
open slot disappears. An in-flight stream that already left the browser
still receives `422` from the server if the pair is now illegal.
[EC-178-02](#ec-178-02--stream-and-query-disagree).

### EC-178-33 — Stream bit from a second endpoint

Mitigation: `supports_streaming` is on `EligibleModelDto`. The chip
does not call `/models/llm` to draw the switch. [LAW-178-8](01-first-principles.md).

### EC-178-40 — Choice files import the inventory hook

Mitigation: a source test fails if the Choice paths in
[02-surfaces](02-surfaces.md) import `useLlmModels` or
`useEmbeddingModels`. [LAW-178-8](01-first-principles.md).

## Inherited, must not regress

### EC-178-26 — Policy save that empties a usage

`usages_left_empty` still refuses that policy write. Choice work does
not catch the error and write the rows anyway.

```text
  EC-178-01  the menu
  EC-178-02  the stream
  EC-178-03  the save
  EC-178-23  the inherited run
       |
       v
  one PairVerdict
```
