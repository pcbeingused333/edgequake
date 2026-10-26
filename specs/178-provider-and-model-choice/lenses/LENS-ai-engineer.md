# Lens — AI engineer

Parent: [README](../README.md) · Law: [05-choice-law](../05-choice-law.md)

## What this lens owns

The pair that runs is the pair the usage allows.
[LAW-178-5](../01-first-principles.md) · [LAW-178-6](../01-first-principles.md) ·
[LAW-178-9](../01-first-principles.md). Capability, lock, request, and
connection stay in that order. A miss does not call a different model.

## Decisions this lens will not reopen

- `ModelFacts::supports` stays the predicate. Extract, query, summary,
  keyword, and decision require chat. Vlm requires chat and vision.
  Embedding requires the embedding flag. Rerank matches nothing. This
  pack does not add a JSON-mode predicate for extract. That remains a
  SPEC-175 note. The family intersection still requires the pair to be
  eligible for extract under the predicate that exists today.
- Discovery-only ids stay unregistered. A live `/v1/models` id without
  a card is not a choice, on any scope. [EC-178-07](../11-edge-cases.md).
- Operator lock bypasses the offer and still passes credential and
  capability checks. A request pair in the same call is
  `422 usage_locked`, not a silent lock substitution.
  [EC-178-37](../11-edge-cases.md).
- For query and upload vlm, a complete request pair is step 2.
  `llm_roles.query.connection_id` is step 3 and runs when the request
  is empty. [EC-178-11](../11-edge-cases.md). Other usages have no
  request step. Their connection branch is unchanged.
- Image turns use `vlm`. The process vision client on
  `state.query.vision_llm_provider` is not a fallback.
  [EC-178-24](../11-edge-cases.md).
- Embedding dimension is the card's dimension. A chat model is not an
  embedding choice. An embedding-only model is not a query choice.
  [EC-178-36](../11-edge-cases.md) · [EC-178-14](../11-edge-cases.md).
- `reasoning_effort` is not a model. It rides on the existing resolver.
  This pack does not add it to the eligible DTO.
- Decision model is `usage=decision`. The tev1 list is not merged into
  the chat chip.
- Keyword pins and extract pins stay operator locks. Acc does not gain
  a workspace metadata dependency.

## What this lens will not let the others break

- Gap-filling a provider-only body with another layer's model and then
  calling that hybrid. [EC-178-12](../11-edge-cases.md).
- Auth fallback onto the process chat model. The trace would name a
  model the user did not choose and the ceiling may have removed.
  [EC-178-30](../11-edge-cases.md).
- Treating `probe = Some(false)` as eligible because the static card
  said `available: true` at startup. The live probe wins when it is a
  hard failure. [EC-178-19](../11-edge-cases.md).
- Running a stored assignment that has left the set, on the argument
  that the user did not touch the chip. [EC-178-23](../11-edge-cases.md).

## Out of this lens

The menu's source labels and the migration head. This lens owns the
pair. The database lens owns the fact that the pair is not a new row.
