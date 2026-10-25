# Lens — Product Owner

Parent: [README](../README.md) · Stories: [03-product-spec](../03-product-spec.md)

## What this lens owns

[LAW-175-7](../01-first-principles.md) and [LAW-175-9](../01-first-principles.md).
A control that nothing reads leaves the product. An empty catalog policy
matches today's "every healthy model" until an admin opts in. Acc scores
do not move when the extract pin is set.

## Decisions this lens will not reopen

- No per-user allowlist. The member picks inside the workspace set.
- No gleaning slider in this pack. The pipeline stage stays.
- No rerank model picker. Rerank stays a deployment fact.
- Changing extract does not reprocess the library. Rebuild stays a button
  with the sentence in [03-product-spec](../03-product-spec.md).
- Four scopes. Search does not replace the scope label.
- Super Admin is not Workspace Admin and is not Platform Admin. The
  stories in [03-product-spec](../03-product-spec.md) and the matrix in
  [15-roles](../15-roles.md) are the product names. No new role enum.

## Acceptance the owner can demo

The demo steps in [03-product-spec](../03-product-spec.md). In particular:
a removed model pauses that job and does not answer on the server key
([EC-175-04](../11-edge-cases.md), [EC-175-05](../11-edge-cases.md)).

## Copy

Sentences live in [03-product-spec](../03-product-spec.md). UX may edit
words. It may not say the server key was used.

## Out of this lens

SQL, cache hashes, and provider wire flags. Those are the database and
AI lenses.
