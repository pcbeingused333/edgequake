# Lens — UX and UI

Parent: [README](../README.md) · Spec: [09-ux-ui-spec](../09-ux-ui-spec.md)

## What this lens owns

The presentation of [LAW-174-8](../01-first-principles.md) and
[LAW-174-14](../01-first-principles.md). Absence is the privacy signal.
The enable dialog is the consent signal.

## Rules for the screens

Follow [09-ux-ui-spec](../09-ux-ui-spec.md). The shape comes from Gr8r
Studio settings rows, property popovers, and the role pill on a person.
The components stay EdgeQuake's.

- One muted classification token on a table row. Not two chips on every card.
- Access is one property row on the document preview, opening a popover.
- Settings → Access is four rows. The enable confirm is an AlertDialog
  with one sentence. No checkbox.
- Preview is a sheet of titles. It does not restyle the rest of the app.
- The members capability matrix stays the operation axis.
- No lock, no grey graph node, no "3 hidden" badge, no "viewing as" banner.
- Empty library: the two lines in [03-product-spec](../03-product-spec.md).

## Two layouts

Check the document list and the enable table at a narrow width. The deny
column in the rule table scrolls; it is not dropped. Graph canvas gets no
second legend.

## What this lens does not add

A per-document share dialog. A free-text policy box. A clearance
calculator. Those would contradict the product non-goals.
