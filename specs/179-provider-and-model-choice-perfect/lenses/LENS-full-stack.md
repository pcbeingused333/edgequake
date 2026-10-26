# Lens — Full stack

Parent: [15-cross-ref](../15-cross-ref.md)

## WHY this lens

The bug is a missing function call, repeated. Adding a check inside each
handler copies the bug. One `ModelGate::resolve` returns the client or a
typed error. HTTP maps the error. The worker maps it to a failed task.
Those are the only two adapters.

## Shape

```text
  ModelGate::resolve(...) -> Result<ResolvedClient, GateError>
  ResolvedClient { provider, model, usage, client, embedding_dimension }
  GateError::NotEligible | Capability | Incomplete | Locked
           | OfferUnavailable | CredentialMissing
```

`pair_verdict` and `eligible_models` stay pure in `edgequake-core`. The gate
is the only new type in the API crate. It does not grow a scope enum that
reimplements the offer loader; it calls `load_workspace_offer`,
`load_tenant_offer`, and `global_offer`.

## Call-site rule

A file that needs a model takes `ResolvedClient`. It does not take
`Arc<dyn LlmProvider>` from `AppState` and decide later. Boot still builds
`state.query` for health and for the operator lock. Request paths do not
read it as a fallback.

## Tests

Behavior tests hit the handler or the gate with a workspace whose offer
excludes the pair. Source-grep tests are limited to `constructor_guard`.
The SPEC-178 grep tests stay; they are not the proof for this pack.
