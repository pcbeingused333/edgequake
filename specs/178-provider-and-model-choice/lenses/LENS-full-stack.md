# Lens — Full stack

Parent: [README](../README.md) · Plan: [12-implementation-plan](../12-implementation-plan.md) ·
Contract: [08-api-contract](../08-api-contract.md)

## What this lens owns

The same `PairVerdict` on the list, the save, and the run.
[LAW-178-2](../01-first-principles.md) · [LAW-178-5](../01-first-principles.md).
Streaming chat, blocking chat, `/query`, and `/query/stream` are one
policy. A scope is a loader of offer, ceiling, and narrowing. It is not
a second function and not a second route.

## Decisions this lens will not reopen

- `eligible_models` keeps its current signature. The API assembles the
  slices. Core does not learn HTTP or SQL.
- `apply_llm_config_update` keeps writing strings. The handler judges
  first and writes nothing on failure.
- `GET /models/llm` stays. Choice handlers do not start reading it to
  build the eligible payload. The eligible loader reads the catalog
  service the policy path already uses.
- No `usage=family` on the wire. Four usages, intersected in the handler
  for the save and in the hook for the menu.
- Auth fallback is not a choice-handler dependency. If a non-choice
  caller still needs the function, it stays in `query_execution.rs` and
  the grep gate names the files that must not import it.
- OpenAPI refresh is W4. W1 may ship the fields before codegen. W2
  writes the TypeScript types by hand and W4 reconciles them.

## Wave order

Loader and DTO, then verdict helpers with unit tests, then the four
run doors and the three saves, then `spec178_model_choice.rs`. The
client wave starts when `supports_streaming` is on the workspace
eligible payload. [12-implementation-plan](../12-implementation-plan.md).

## What this lens will not let the others break

- A client-only filter that "matches" the ceiling while the stream
  still accepts the inventory id. [EC-178-02](../11-edge-cases.md) lands
  in W1, before the chip moves in W2, so the door is shut even if a
  stale client remains.
- `Ok(None)` meaning the process provider. That branch is the bug
  [F-178-10](../04-findings.md).
- Returning 200 from a save that persisted a pair the following GET
  eligible does not list.
- A 403 on a wider scope that still includes the model names in the
  body. The body is empty. [EC-178-32](../11-edge-cases.md).

## Out of this lens

Menu copy, dimension semantics, and whether a card is registered.
Those are UX and the AI lens. This lens only carries their codes.
