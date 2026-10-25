# 09 — UX and UI spec

Parent: [README](README.md) · Prev: [08-api-contract](08-api-contract.md) ·
Next: [10-frontend-architecture](10-frontend-architecture.md)

Layouts are [06-screens](06-screens.md). Sentences the owner locked are
[03-product-spec](03-product-spec.md). This file is how those sentences
behave. Words may change. The fact in each sentence may not.

## Hierarchy the eye can see

The page lead names the parent in one line.

| Page | Lead when unconstrained | Lead when constrained |
|------|-------------------------|------------------------|
| Deployment → Providers | "These are the providers organizations can turn on." | same, this page has no unconstrained mode |
| Organization → Providers | "People in this organization can use every provider this deployment offers." | "Only the providers enabled here appear in workspaces." |
| Workspace → Models | "This workspace can use every provider the organization offers." | "Only the providers enabled here appear in this workspace." |

A count sits on the lead: "2 of 3 providers." Unconstrained shows
"3 of 3" and does not look like a saved subset.

## Provider before models

The provider control is a checkbox with the display name and the model
count. The model table is indented under the provider and is visible
when the provider row is expanded. The first provider is expanded. The
rest are collapsed.

Turning a provider off collapses its models and disables their
checkboxes. The save payload omits those model rows. Turning it back on
restores the last model ticks for that provider from the draft, not from
"all checked."

Usages a model cannot fill render an em dash, as the ceiling card does
today. They are not a disabled checkbox.

## Confirm

The first transition from unconstrained to constrained uses the existing
settings confirm pattern (a dialog, not `window.confirm`). The body is
the first-save sentence. Cancel keeps the draft. Confirm sends the PUT.

"Use every provider the parent offers" is a text button, not the primary
Save. It has its own dialog: "Workspaces will be able to use every
provider this deployment offers, including ones added later." That is
the unconstrained write.

Save stays disabled when the draft matches the server. The bar uses the
same `ModelSaveBar` the ceiling card uses, so the dirty rules stay one
implementation ([10-frontend-architecture](10-frontend-architecture.md)).

## Empty, failed, withdrawn

| State | What the admin sees | What a member sees |
|-------|---------------------|--------------------|
| No global providers | Empty sentence and a link to Connections, if the viewer is Platform Admin. Otherwise the sentence alone. | Chip has Inherit only, and Inherit shows broken if a job needs a model. |
| Probe failed | Row on Connections and on Deployment → Providers. Badge "Check failed." Models disabled. | Nothing. |
| Withdrawn | Row remains on Deployment. Badge "Hidden from organizations." | Nothing. |
| Tenant offers none | Organization page explains that workspaces have no providers. | Chip has no named models. |
| Stale slug on save | Inline error under the list, the parent sentence. The checkbox is not added. | Unchanged until a valid save. |
| Broken assignment | The job row uses the broken sentence. The control to clear the assignment stays. | The job refuses. The chip does not list that model. |
| Operator lock | The job row shows the lock sentence. Provider checkboxes for other jobs still work. | The menu does not open. |
| Legacy key | A quiet block under the provider list. No toggle. | Not in the chip. |

Badges use the same treatment as `HEALTH_BADGE` on the ceiling card:
a soft tint, not a full-width alarm.

## Keys

The key field is `type="password"`, autocomplete `new-password`, and is
not copied into the query string. After save it is cleared. The list
never receives the key from the server. Fingerprints are truncated.

Test runs before Save is enabled for a new cloud provider. A failed test
leaves the form dirty and shows the probe `message` under the button.
Save on update may rotate the key. Rotation runs Test with the new key
before the PUT, same rule.

Organization and Workspace do not render an input that could hold a key.
The legacy block does not have a reveal control.

## Responsive

At 1440 the provider list and the model table sit in one column, table
scrolls horizontally, same as the ceiling card's `overflow-x-auto`.

At 390 the provider rows stack. The model table keeps horizontal scroll.
The shape select and the key field are full width. Primary actions stay
on one row: Test, then Save.

Focus order is the nav, the lead, the provider checkboxes, the expanded
model table, then Save. The confirm dialog traps focus.

## Accessibility

Provider checkboxes have a name that includes the display name. Model
checkboxes have a name that includes the model id and the usage. The
probe badge is text, not color alone. Withdrawn and Not offered are
text badges.

`aria-busy` on the list while the offer query loads. Errors from a 422
are in an `role="alert"` under the list.

## What this page does not do

- It does not show `models.toml` ids that failed the offer.
- It does not animate the cascade.
- It does not put Organization and Deployment in the nav for a workspace admin, including inside search results.
