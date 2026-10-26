# Lens — UX and UI

Parent: [README](../README.md) · Wires: [06-screens](../06-screens.md) ·
Copy: [09-ux-ui-spec](../09-ux-ui-spec.md)

## What this lens owns

The menu is one pattern at three scopes. Inherit is first and names the
model that will run. A provider the parent removed has no row. A broken
assignment is visible to the admin and is not an extra disabled item
the member can send.

## Decisions this lens will not reopen

- Menu order: Inherit, then providers in offer order, models in catalog
  order. Search does not invent ids.
- Source labels are the table in [09-ux-ui-spec](../09-ux-ui-spec.md).
  `env_bootstrap` and `compiled_default` both read Deployment. The
  member does not see env-var names.
- Locked is a disabled trigger, not an open menu with one row.
- Empty eligible disables Send for a member and explains why. It does
  not offer the inventory.
- The toast on a cleared slot fires once. It does not fire on every
  refetch of an already empty slot.
- The embedding dimension is read-only text. There is no dimension
  input.
- Rebuild keeps its own confirmation. The save toast does not claim the
  library was re-embedded.
- The fleet card uses the same menu. The helper sentence is the only
  difference ([06-screens](../06-screens.md)).

## Broken and empty copy

The sentences in [09-ux-ui-spec](../09-ux-ui-spec.md) are the set. A
new `broken_reason` needs a sentence in that table in the same change.
The composer does not retry.

## What this lens will not let the others break

- Flashing `default_model` while the effective query loads.
  [EC-178-06](../11-edge-cases.md).
- A disabled checkbox for a withdrawn provider. Absence is the signal.
  [EC-178-20](../11-edge-cases.md).
- Two visual languages for the chip and the workspace row. Same menu.
- Hiding a broken stored id from the admin. They need to see what is
  stuck. [S5](../03-product-spec.md).

## Out of this lens

Which grant returns 403, and whether `supports_streaming` is true.
This lens places the bit on the switch. The AI lens defines the bit.
