# 01 — First principles (LAW-178)

Parent: [README](README.md) · Prev: [00-why](00-why.md) · Next: [02-surfaces](02-surfaces.md)

These laws are non-negotiable for SPEC-178. A wave may defer a screen.
A wave must not ship a choice control that violates a law. SPEC-175 and
SPEC-177 laws stay in force. This pack amends none of them. Where the
code still diverges from those laws on a choice path, the wave closes
the divergence. It does not invent a second ladder.

## DRY / SOLID application

| Principle | Application in SPEC-178 |
|-----------|-------------------------|
| **DRY** | One filter, `eligible_models`, already in `edgequake-core`. Three scopes differ by the offer and the rows the API passes in. One client hook, `useEligibleModels(scope, usage)`. One picker. One verdict type, `PairVerdict`, shared by the list, the save, and the run. The client does not recompute the ladder. |
| **SRP** | Core decides membership of a pair. The API loads offer, ceiling, and narrowing, and maps 422 and 503. React renders the payload and sends a pair or an empty inherit. `workspace_model_update` keeps writing strings. The handler refuses before it calls that writer. |
| **OCP** | A new usage is a SPEC-175 enum arm plus a capability predicate. A new scope is a new loader of the same three arguments (offer, ceiling, narrowing). It is not a new eligibility function and not a new table. |
| **LSP** | REST, streaming chat, `/query`, MCP, and the worker see the same set for the same scope and usage. A caller cannot tell a model was removed by the offer, the ceiling, or the narrowing except by `broken_reason` and `policy_mode`. |
| **ISP** | The query chip asks for `usage=query` at workspace scope. It does not receive keys, withdrawn slugs, the deployment inventory, or models eligible only for embedding. The deployment defaults card asks for deployment scope. It does not receive one workspace's narrowing. |
| **DIP** | Handlers depend on `eligible_models` and `resolve_usage`. Pages depend on `GET /models/eligible` and `GET /config/effective`. They do not import `models.toml` and they do not filter `GET /models/llm` in the browser. |

Detail of the algebra: [05-choice-law](05-choice-law.md).

---

## LAW-178-1 — Choice sits inside the universe

Settings define the universe. This pack defines the selection inside it.

```text
  Deployment offer          SPEC-177
  Tenant offer + ceiling    SPEC-177, then SPEC-175
  Workspace offer + narrowing + assignment
  then a person picks a pair, or leaves it empty
```

SPEC-178 does not add a provider, does not store a secret, and does not
change `resolve_usage` order ([05-resolution-law](../175-perfect-settings/05-resolution-law.md),
[05-cascade-law](../177-provider-cascade/05-cascade-law.md)). A model
absent from the parent universe is absent from the choice payload. It is
not a disabled row the child can turn on ([LAW-177-1](../177-provider-cascade/01-first-principles.md)).

- **Closes:** [WHY A](00-why.md)
- **Tests:** [EC-178-20](11-edge-cases.md) · [EC-178-21](11-edge-cases.md)

## LAW-178-2 — One function, three scopes

`eligible_models` in
[`eligibility.rs`](../../edgequake/crates/edgequake-core/src/model_policy/eligibility.rs)
stays the only membership test. The scope selects the arguments:

| Scope | Offer | Ceiling | Narrowing |
|-------|-------|---------|-----------|
| `deployment` | global offer | none | none |
| `tenant` | tenant offer | tenant rows for the usage | none |
| `workspace` | workspace offer | tenant rows for the usage | workspace rows for the usage |

Zero policy rows for a usage still means unconstrained for that usage
([LAW-175-9](../175-perfect-settings/01-first-principles.md),
[LAW-177-4](../177-provider-cascade/01-first-principles.md)). The offer
still applies. "Unconstrained" is every registered, capable, probed-ok
model of the offered providers. It is not every `models.toml` card.

`GET /models/llm` is not a fourth scope. [LAW-178-8](#law-178-8--inventory-is-not-a-choice-list).

Who may call which scope is in [08-api-contract](08-api-contract.md).
A workspace member calls workspace scope. Tenant scope is for a person
who may manage the tenant or create a workspace there. Deployment scope
is Platform Admin. Calling a wider scope to see models the narrower
scope hid is `403`.

- **Closes:** [WHY A](00-why.md) · [F-178-01](04-findings.md) · [F-178-02](04-findings.md)
- **Tests:** [EC-178-09](11-edge-cases.md) · [EC-178-15](11-edge-cases.md) · [EC-178-16](11-edge-cases.md) · [EC-178-32](11-edge-cases.md) · [EC-178-39](11-edge-cases.md)

## LAW-178-3 — The list, the save, and the run are the same set

For a scope and a usage, one set answers three questions:

1. What may the picker show?
2. What may a save persist, when the body names a pair?
3. What may a run call, when the winning pair is that usage?

A non-empty pair outside the set is `422 model_not_eligible`, or
`422 capability_mismatch` when the card exists and the capability does
not fit ([`pair_verdict`](../../edgequake/crates/edgequake-api/src/services/model_policy_enforcement.rs)).
The same codes on the save and on the run.

Empty means inherit. Inherit persists no pair. The screen shows the
`query` row of `GET /config/effective` (or the matching usage), including
`source` and `broken_reason`. It does not show `default_model` from
`GET /models/llm`.

A family key (`llm_provider` + `llm_model`) serves extract, query,
summary, and keyword. The pair must sit in the intersection of those four
eligible sets. A role key (`llm_roles.{usage}`) is judged for that usage
only. Vision is judged for `vlm`. Embedding is judged for `embedding`.
Decision is judged for `decision`.

Clearing a key (`""`, `"none"`, or JSON null, the clears already honored
by `apply_llm_config_update`) is inherit. It is not a pair, and it is not
a 422.

A stored pair that later leaves the set stays on disk. The effective
endpoint reports `eligible: false` and `broken_reason`. The run refuses.
The editor shows the broken row. The product does not rewrite the row to
a different model. [LAW-175-5](../175-perfect-settings/01-first-principles.md).

- **Closes:** [WHY B](00-why.md) · [WHY D](00-why.md) · [F-178-04](04-findings.md) · [F-178-05](04-findings.md) · [F-178-06](04-findings.md) · [F-178-14](04-findings.md)
- **Tests:** [EC-178-03](11-edge-cases.md) · [EC-178-06](11-edge-cases.md) · [EC-178-18](11-edge-cases.md) · [EC-178-23](11-edge-cases.md) · [EC-178-29](11-edge-cases.md) · [EC-178-34](11-edge-cases.md) · [EC-178-36](11-edge-cases.md)

## LAW-178-4 — No new table

Choice is not a stored fact. The writers stay the ones SPEC-177 named:

| Fact | Writer | This pack |
|------|--------|-----------|
| Secret | connection | untouched |
| Withdrawal | `deployment_provider_overrides` | untouched |
| Provider visibility | `provider_offers` | read |
| Model ticks | `model_catalog_policy` | read |
| Assignment | `llm_roles`, family metadata, tenant `default_*`, `server_config.llm_defaults` | write only after `PairVerdict::Eligible` |
| The run | `resolve_usage` | read |

Migration head stays
[223](../../edgequake/migrations/223_spec177_provider_offers.sql).
No eligible-set cache table. The set is computed from rows the request
already loads. [07-data-and-db-contract](07-data-and-db-contract.md).

- **Closes:** [F-178-04](04-findings.md)
- **Tests:** [EC-178-41](11-edge-cases.md)

## LAW-178-5 — Fail closed on the choice paths

These paths refuse instead of calling the process provider:

- streaming chat
- non-streaming chat
- `POST /query` and `POST /query/stream`
- a save of a workspace, tenant, or deployment pair

The refusal conditions:

- the winning pair is outside the eligible set, and the usage is not operator-locked
- the provider is outside the workspace offer
- the credential or the probe fails
- loading the offer returns an error

`execute_sota_query_with_auth_fallback` and
`execute_sota_query_stream_with_auth_fallback` are not success paths for
these handlers. An auth failure on the chosen provider is that failure,
returned to the caller.

`Ok(None)` from the resolver is not a license to use
`state.query.llm_provider`. The handler maps it to `broken_reason`.

An operator lock still runs when the process credential exists, and the
offer does not apply ([LAW-177-8](../177-provider-cascade/01-first-principles.md)).
A request pair sent while the usage is locked is `422 usage_locked`.
The lock is not silently substituted for the pair the client named.

Offer-load failure on these paths is `503 offer_unavailable`. It is not
the current fail-open branch in
[`refuse_unoffered`](../../edgequake/crates/edgequake-api/src/providers/resolver.rs).

- **Closes:** [WHY C](00-why.md) · [F-178-07](04-findings.md) · [F-178-09](04-findings.md) · [F-178-10](04-findings.md) · [F-178-12](04-findings.md)
- **Tests:** [EC-178-02](11-edge-cases.md) · [EC-178-13](11-edge-cases.md) · [EC-178-25](11-edge-cases.md) · [EC-178-30](11-edge-cases.md) · [EC-178-37](11-edge-cases.md)

## LAW-178-6 — Query override is query-only

The composer chip lists `eligible(workspace, query)`. Extract, summary,
keyword, embedding, and decision are assigned on the workspace Models
screen. They are not chip rows.

Upload vision lists `eligible(workspace, vlm)`. A message that carries
images resolves `vlm`, not the process vision client on
`state.query.vision_llm_provider`. A chip selection that cannot see
images is `422 capability_mismatch` when the message has images. The
handler does not swap in a different model.

Embedding dimension is the dimension on the eligible card. A save that
names an embedding model stores that dimension. A body dimension that
disagrees with the card is `422 embedding_dimension_mismatch`. An unknown
model is `422 model_not_eligible`. The save does not guess 1536, and it
does not start a rebuild. Rebuild stays the explicit action
([LAW-175-14](../175-perfect-settings/01-first-principles.md)).

`rerank` stays env-only. A request `rerank_model` stays
`422 rerank_model_rejected`.

A choice body sends both `provider` and `model`, or neither. One without
the other is `422 model_pair_incomplete`. The server does not fill the
missing half from a lower layer and then run that hybrid.

- **Closes:** [WHY C](00-why.md) · [F-178-08](04-findings.md) · [F-178-16](04-findings.md)
- **Tests:** [EC-178-12](11-edge-cases.md) · [EC-178-14](11-edge-cases.md) · [EC-178-24](11-edge-cases.md) · [EC-178-36](11-edge-cases.md) · [EC-178-38](11-edge-cases.md)

## LAW-178-7 — The server is the authority

The picker, the sanitizer, and the browser store are conveniences.
MCP, the SDK, and a hand-written client receive the same 422 as the
WebUI.

The persisted query pair is stored per workspace id. On read, the client
drops a pair that is absent from `eligible(workspace, query)` for the
workspace on screen. The comparison set is that payload, not
`GET /models/llm` and not provider health alone.

A ceiling or offer save invalidates the eligible query keys the chip
uses. The chip and the editor share those keys.

- **Closes:** [WHY E](00-why.md) · [F-178-13](04-findings.md) · [F-178-15](04-findings.md)
- **Tests:** [EC-178-04](11-edge-cases.md) · [EC-178-05](11-edge-cases.md) · [EC-178-17](11-edge-cases.md) · [EC-178-27](11-edge-cases.md) · [EC-178-28](11-edge-cases.md) · [EC-178-31](11-edge-cases.md)

## LAW-178-8 — Inventory is not a choice list

`GET /models`, `GET /models/llm`, and `GET /models/embedding` stay.
They answer "what cards does this process know." They stay available to
operators and to existing catalog clients.

No choice control calls them. Choice controls are the surfaces in
[02-surfaces](02-surfaces.md) marked Choice. After W2, those files do
not import `useLlmModels` or `useEmbeddingModels`.

Two hooks named `useLlmModels` exist today, with two query keys and two
stale times ([F-178-15](04-findings.md)). W2 leaves one inventory hook,
on the inventory key, and moves every choice caller off it. The hook is
not deleted. Deleting the endpoint would break catalog clients. Hiding
the endpoint behind the eligible filter would mix the two jobs.

Static cards marked `available: true` without a live credential check
([`model_catalog.rs`](../../edgequake/crates/edgequake-api/src/model_catalog.rs))
may remain on the inventory. They enter a choice list only through the
offer, the capability, and the probe rules already in `eligible_models`.

Discovery-only ids (`discovery_source = dynamic_api`) stay on the
inventory and stay unregistered for eligibility
([`model_policy_catalog.rs`](../../edgequake/crates/edgequake-api/src/services/model_policy_catalog.rs)).
This pack does not make them eligible.

- **Closes:** [WHY A](00-why.md) · [F-178-03](04-findings.md) · [F-178-17](04-findings.md)
- **Tests:** [EC-178-07](11-edge-cases.md) · [EC-178-08](11-edge-cases.md) · [EC-178-22](11-edge-cases.md) · [EC-178-33](11-edge-cases.md) · [EC-178-40](11-edge-cases.md)

## LAW-178-9 — A request pair is kept

For `query` and for upload `vlm`, resolution order stays SPEC-175:

```text
  operator_lock
  request pair          both fields set
  workspace role        including connection_id
  workspace default
  tenant default
  server_config
  env_bootstrap
  compiled_default
```

A non-empty request pair is judged against the eligible set. Eligible:
that pair runs. Ineligible: `422`, and the role's `connection_id` is not
a fallback. Empty request: the role runs, including a `connection_id`
on `llm_roles.query`.

Today [`resolve_llm_provider_for_workspace`](../../edgequake/crates/edgequake-api/src/providers/resolver.rs)
returns the connection client before it reads the request. W1 reverses
that branch for these two usages so the code matches the order above.
Other usages have no request layer. Their connection branch is unchanged.

The effective row for `query` shows `source=request` when the request
pair won, and `source=workspace_role` when the connection won because
the request was empty. The chip's inherit line uses that row.

- **Closes:** [WHY D](00-why.md) · [F-178-11](04-findings.md)
- **Tests:** [EC-178-11](11-edge-cases.md)

## LAW-178-10 — Every EC-178 has a named test

[11-edge-cases](11-edge-cases.md) is the register.
[13-e2e-test-matrix](13-e2e-test-matrix.md) names the gate.
A wave that closes an EC lands the test in the same wave. A wave that
does not touch an EC leaves its test for the wave the matrix names.

The workspace Models editor already has
[`e2e/spec175/models-editor.spec.ts`](../../edgequake_webui/e2e/spec175/models-editor.spec.ts).
This pack cites it. It does not copy it.

- **Closes:** every WHY
- **Tests:** the matrix
