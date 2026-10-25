# Lens — UX / UI

Parent: [README](../README.md) · Wireframes: [09-ux-ui-spec](../09-ux-ui-spec.md)

## What this lens owns

[LAW-175-8](../01-first-principles.md) and [LAW-175-7](../01-first-principles.md).
Scope is the navigation. A control writes the field the pipeline reads,
or the control is not drawn.

## Patterns we copy

- Vercel and GitHub: the chrome names the owner (you, workspace,
  organization, deployment) and the role chip names the grant. Shells
  are [W-31](../09-ux-ui-spec.md#w-31-super-admin) through
  [W-34](../09-ux-ui-spec.md#w-34-platform-admin). The permission story
  is [15-roles](../15-roles.md).
  [Survey](https://github.com/onetimesecret/onetimesecret/issues/2307).
- Linear: search above the list, and a way back to the app.
  [The Settings Page Problem](https://brainy.ink/paper/the-settings-page-problem).
- Notion: query, graph, and upload keep their local sheets.
- K2View: the catalog is a show/hide list, then the picker is the
  remainder.
  [Providers and Models](https://support.k2view.com/Academy/articles/04_fabric_studio/studio_ai/06_ai_configuration_and_settings.html).
- BYOK: effort and sampling render only when the capability map says so.
  [Model controls](https://byok.pro/blog/design-model-reasoning-controls/).

## What the screens must show

The layer that won (`inherited`, `this workspace`, `locked by operator`,
`broken`). A disabled lock. A broken panel that does not offer a silent
substitute. Inherit as a real row in the query chip.

Every current settings surface is drawn or marked removed in the coverage
table of [09-ux-ui-spec](../09-ux-ui-spec.md). A new screen is not done
until that table has a wire id for it. SPEC-174 keeps the inner label
dialogs. This pack only places that card.

## What this lens will not add

A fifth scope. A page-level save for eight usages. A temperature control
on extract. A second "Provider Status" title.
