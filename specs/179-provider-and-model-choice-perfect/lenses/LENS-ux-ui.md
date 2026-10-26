# Lens — UX and UI

Parent: [15-cross-ref](../15-cross-ref.md)

## WHY this lens

People forgive a short model list. They do not forgive a list that shows a
model and then rejects it, or an empty state that is actually an error.
The job of the UI is to make the parent's decision visible and the
consequence of a save visible before the save.

## Rules

- Parent first, then the toggle, then the impact, then save.
  [10-screens](../10-screens.md).
- Five states on every card. [09-ux-ui-spec](../09-ux-ui-spec.md). The empty
  sentence is only for an empty success.
- Inherit is a choice, named with the model it will run.
- Refresh is a row action with a time and a diff. A global "refresh
  everything" stays as a secondary control on Deployment, not the only one.
- Destructive buttons are last: Withdraw, Delete. Test and Rotate are not
  styled as destructive.
- 390px stacks. The matrix is not a horizontal scroller as the only layout.
- Dark and light use the same spacing. Screenshots of both are the review.

## Copy test

The confirm dialog says what will fail, and that nobody will pick a
replacement. It does not say "some features may not work".
