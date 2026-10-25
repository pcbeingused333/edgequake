# Lens — Product Owner

Parent: [README](../README.md) · Stories: [03-product-spec](../03-product-spec.md)

## What this lens owns

[LAW-177-4](../01-first-principles.md) and [LAW-177-8](../01-first-principles.md).
An existing tenant keeps every provider the deployment can actually call.
The first time an admin saves the provider list, that organization stops
inheriting providers added later. An operator pin still runs when the
server key exists. Hiding the provider from tenants does not move the pin.

## Decisions this lens will not reopen

- No fifth scope named Global. The nav says Deployment.
- No new role. Super Admin, Workspace Admin, and Platform Admin stay
  [SPEC-175](../../175-perfect-settings/15-roles.md).
- "Offer nothing" is a saved choice. It is not the same as "use every
  provider the parent offers."
- A new provider does not reprocess the library.
  [EC-177-42](../11-edge-cases.md).
- Legacy organization keys are not deleted and not copied to other tenants.
- Chat, vision, and embeddings share one provider switch. The owner does
  not get three grants for one key.

## Acceptance the owner can demo

The six steps in [03-product-spec](../03-product-spec.md). The sentence
on a refused job says the provider is no longer offered. It does not say
the server key was used.

## Copy

Sentences live in [03-product-spec](../03-product-spec.md). UX may edit
words. It may not say a child can turn on a provider the parent does not
offer.

## Out of this lens

SQL, the partial unique indexes, and the capability predicates. Those
are the database and AI lenses.
