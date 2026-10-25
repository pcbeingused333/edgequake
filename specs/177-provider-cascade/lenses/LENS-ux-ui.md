# Lens — UX and UI

Parent: [README](../README.md) · Screens: [06-screens](../06-screens.md) ·
Behavior: [09-ux-ui-spec](../09-ux-ui-spec.md)

## What this lens owns

[LAW-177-2](../01-first-principles.md) and [LAW-177-7](../01-first-principles.md).
The admin sees providers first and models second. The nav keeps You,
Workspace, Organization, and Deployment. A provider the parent did not
offer is not a disabled checkbox.

## The pages

| Page | Parent the lead names | Primary action |
|------|----------------------|----------------|
| Deployment → Connections | the deployment itself | Test, then Save |
| Deployment → Providers | organizations | Withdraw, or save models |
| Organization → Providers | this deployment | Save the subset |
| Workspace → Models | the organization | Save the subset, then assign jobs |

People returns to members. The model grid does not stay there as a
second copy. [EC-177-29](../11-edge-cases.md).

## States that must stay distinct

Offered, Not offered (probe), Withdrawn, Legacy, Locked, Broken.
Color is not the only difference. Each state has the sentence in
[03-product-spec](../03-product-spec.md). [EC-177-46](../11-edge-cases.md)
keeps a withdrawn provider on the deployment list so the admin can
restore it. It is absent everywhere below.

The first constrained save and the return to unconstrained are two
dialogs. One button must not do both. [EC-177-16](../11-edge-cases.md).

## Layout

Desktop is one column with a horizontal model table, matching the
ceiling card. At 390 the provider rows stack and the table still scrolls.
The key field exists only on Connections, full width, and clears after
save.

Search is the existing settings search. A workspace admin who types
"provider" lands on Workspace → Models. [EC-177-30](../11-edge-cases.md).

## Out of this lens

The HTTP codes, the boolean column, and whether an embedding model can
be selected for extract.
