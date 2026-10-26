# 09 — UX and UI spec

Parent: [README](README.md) · Prev: [08-api-contract](08-api-contract.md) ·
Next: [10-screens](10-screens.md)

## WHY

The admin is deciding what every later screen may offer. The screen has to
show the parent, the current choice, the consequence, and the save, in that
order. A control that lists a model the parent removed teaches the admin
that the filter is optional.

## States, on every card

| State | What the person sees |
|-------|----------------------|
| loading | the card title and a single status line, not a fake empty offer |
| empty | "No provider is offered yet" plus the place to add one |
| error | the error text and Retry. Never the empty copy |
| read-only | the same data, no checkboxes, a one-line reason |
| locked | the usage name and "An operator lock chooses this model" |
| dirty | a sticky bar: "Unsaved changes", Save, Discard |
| saving | Save disabled, bar remains |
| saved | bar leaves, `aria-live` says "Saved" |

`GrantEditor` uses this table. Today a failed fetch renders the empty copy.
That is [F-179-12](04-findings.md).

## Copy

| Moment | Sentence |
|--------|----------|
| Blank key on edit | "Leave the key blank to keep the current key." |
| After save | "Key ending {fingerprint}." |
| Refresh running | "Asking {provider} for its models…" |
| Refresh ok | "Refreshed {relative time}. {n} new, {m} gone." |
| Refresh error | "Could not refresh. Last success {relative time}." |
| Withdraw confirm | "{n} workspaces use this provider. They will fail until an admin picks another model." |
| Inherit row | "Inherit · {effective provider} / {effective model}" |
| Broken banner | "This model is no longer allowed. Pick another to keep ingesting." |
| Dimension change | "Embedding dimension changed. Rebuild embeddings after you save." |

Strings live in the settings i18n namespace. Hard-coded English in the
provider and model cards is a bug.

## Choice menu

One menu. Groups by provider. Each row shows the model id, capability chips
(chat, vision, tools, embed), context length when known, and dimension for
embedders. A search field filters the open menu. Inherit is the first row
when the usage allows inherit. The menu does not call `/models/search`.

## Deployment provider row

Health from the last test: "Test passed · {ms} ms · {relative time}" or
"Last test failed · {reason}". Refresh sits on the row, not in a global
toolbar. Diff chips sit under the title until the admin registers or
dismisses them. Embedding registration asks for the dimension inline.

## Org and workspace matrix

The model column sticks on the left. Usage columns scroll. A filter chips
the list to chat, vision, or embedding. Select-all applies to the visible
rows. Save runs impact first when the change removes a pair that is in use,
and the confirm dialog lists those pairs.

## Narrow view

At 390px the matrix becomes one model per block: name, then a wrap of usage
toggles. No seven-column horizontal scroll as the only layout.

## Focus and keyboard

The menu is a listbox. Arrow keys move. Enter selects. Escape closes.
Refresh and Save results announce in an `aria-live="polite"` region that is
not only a toast.

## Dirty navigation

A dirty card sets `beforeunload` and blocks the settings router until
Discard or Save. The guard lives in one hook, `useDirtyGuard`, used by the
connection form, the offer editor, the ceiling, and the role editor.
