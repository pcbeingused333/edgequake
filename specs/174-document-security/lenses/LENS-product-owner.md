# Lens — Product Owner

Parent: [README](../README.md) · Stories: [03-product-spec](../03-product-spec.md)

## What this lens owns

[LAW-174-2](../01-first-principles.md) and [LAW-174-14](../01-first-principles.md).
Enforcement is off until an owner turns it on. Acc scores do not move.
Enable cannot lock the owner out. The confirm dialog is one sentence,
not a checklist ([09-ux-ui-spec](../09-ux-ui-spec.md)).

## Decisions this lens will not reopen

- No per-document share. The job is tags and roles, not "send this file
  to Dana."
- No policy language the steward has to learn. Four match kinds.
- Classification names stay Public, Internal, Confidential, Restricted,
  plus Unverified for "not reviewed." A workspace may add compartments.
  It may not rename the five ranks in W1–W11.
- Hidden documents are absent, not locked. A lock is a hint.

## Acceptance the owner can demo

1. Turn the feature on. The owner still sees the library. A new member
   sees the empty sentence and no count.
2. Assign "HR reader." That person sees payroll and not the chip spec.
3. Ask the same question from both accounts. Answers and citations do not
   overlap on the hidden document.
4. Turn the feature off in a second workspace. That workspace matches
   today's product ([EC-174-35](../12-edge-cases.md)).

## Copy

The enable sentence and the empty sentence live in
[03-product-spec](../03-product-spec.md). UX may edit words. It may not
add a number of hidden documents.

## Out of this lens

Schema, SQL, and prompt hashes. Those are the database and AI lenses.
This lens only checks that a demo of US-174-1 through US-174-6 is possible
without a second product.
