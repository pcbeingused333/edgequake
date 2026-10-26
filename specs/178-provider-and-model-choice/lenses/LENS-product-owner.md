# Lens — Product Owner

Parent: [README](../README.md) · Stories: [03-product-spec](../03-product-spec.md)

## What this lens owns

[LAW-178-1](../01-first-principles.md) and [LAW-178-3](../01-first-principles.md).
The person who configures a workspace and the person who asks a question
see one set of models. A model that leaves the set stops being a choice
and stops being an answer. The product does not quietly answer on a
different model.

## Decisions this lens will not reopen

- No per-user allowlist. The member picks inside the workspace set.
- No new role. Platform Admin, tenant owner, and workspace admin stay
  the SPEC-175 names.
- Inherit is a real choice. The product shows which model Inherit will
  run. It does not label Inherit with the process model.
- A leftover assignment stays visible and broken. The product does not
  rewrite it to a substitute.
- Extract, summary, keyword, embedding, and decision are not rows on
  the query chip.
- Changing the embedding model does not re-embed the library.
- Rerank stays off the picker.
- Keys stay on Deployment. This pack does not put a key field on a
  choice menu.

## Acceptance the owner can demo

The ten stories and the six-step demo in
[03-product-spec](../03-product-spec.md). In particular
[EC-178-01](../11-edge-cases.md), [EC-178-02](../11-edge-cases.md),
and [EC-178-23](../11-edge-cases.md): the chip, the stream, and the
inherited run agree.

## Copy

Sentences live in [09-ux-ui-spec](../09-ux-ui-spec.md). UX may edit
words. It may not say the server model was used, and it may not offer
a disabled row for a provider the parent removed.

## What this lens will not let the others break

- A second catalog "just for the chip," even as a transition flag.
- A fallback answer when the chosen provider returns 401.
- A family default that is legal for answers and illegal for extract.
  The owner was told one default covers both.

## Out of this lens

SQL, query keys, and the capability bits on a card. Those are the
database, front, and AI lenses.
