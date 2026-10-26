# 09 — UX and UI spec

Parent: [README](README.md) · Prev: [08-api-contract](08-api-contract.md) ·
Next: [10-frontend-architecture](10-frontend-architecture.md)

Wires are [06-screens](06-screens.md). This file is the behavior of
those wires: order, copy, empty, locked, broken, and the moment a
policy change arrives.

## One menu

Every choice menu has the same order.

1. Inherit, when the field may be empty. Deployment fleet defaults may
   omit Inherit on the chat row if the product requires a fleet leaf.
   The compiled default still exists under the ladder, so Inherit on
   the fleet card means "compiled default" and the effective line says
   so. Workspace and tenant rows always offer Inherit.
2. Providers in offer order, display name, models in catalog order.
3. Nothing else. A withdrawn provider has no group. A failed probe has
   no row. `mock` has no row.

Search filters the open list. Search does not add ids from the
inventory. An empty search result reads "No model matches." It does not
read "No models configured" unless the eligible set itself is empty.

## Inherit copy

The trigger shows the effective model name, not the word Inherit alone,
except while the effective query is loading. Then the trigger reads
"Inherit" and does not flash the process model.

Source labels, from `ResolvedUsage.source`:

| `source` | Label |
|----------|-------|
| `operator_lock` | Locked |
| `workspace_role` | This workspace |
| `workspace_default` | This workspace |
| `tenant_default` | Organization |
| `server_config` | Deployment |
| `env_bootstrap` | Deployment |
| `compiled_default` | Deployment |
| `request` | not used on the effective GET |

`locked: true` replaces the source label with Locked and disables the
menu. [EC-178-10](11-edge-cases.md).

`eligible: false` replaces the model line with Broken and the
`broken_reason` sentence below. The menu still opens so the admin can
pick a legal model or Inherit. A member chip whose inherit target is
broken disables Send. [EC-178-18](11-edge-cases.md).

## Broken sentences

| `broken_reason` or code | Sentence |
|-------------------------|----------|
| `not_eligible` | This model is no longer allowed. Pick another, or use Inherit. |
| `capability` | This model cannot do that job. |
| `credential` | This provider has no key the workspace can use. |
| `probe` | This provider failed its last check. |
| `cross_tenant` | This provider belongs to another organization. |
| `usage_locked` | An operator lock chooses this model. |
| `model_pair_incomplete` | Choose a provider and a model together. |
| `embedding_dimension_mismatch` | The dimension does not match this embedding model. |
| `offer_unavailable` | Model settings could not be loaded. Try again. |

The composer shows the sentence in place of an answer. It does not
retry. It does not say the server model was used.

## Member and admin

| State | Member chip | Workspace Models row |
|-------|-------------|----------------------|
| Probe failed | Hidden | Hidden in the menu. A stored assignment of that model shows Broken. |
| Ceiling removed the model | Hidden | Hidden in the menu. Stored id shows Broken. |
| Offer removed the provider | No group | No group |
| Empty eligible set | Inherit only, Send disabled, empty sentence | Inherit only, plus the admin sentence |
| Operator lock | Disabled trigger, Locked | Disabled trigger, Locked |

Admin sentence when the set is empty: "Allow a model for this job in
the organization catalog, or enable a provider that has one."

Member sentence when the set is empty: "An admin needs to allow a model
for answers in this workspace."

## Workspace switch and policy change

The chip reads the slot for the workspace id on screen.
[EC-178-27](11-edge-cases.md).

When `eligible(workspace, query)` refreshes and the slot's pair is
absent, the client clears that slot and selects Inherit. A toast, one
line: "That model is no longer available in this workspace." The toast
fires once per clearance, not on every refetch that finds the slot
already empty. [EC-178-04](11-edge-cases.md) · [EC-178-28](11-edge-cases.md).

The menu does not keep the stale id as a disabled extra row. Absence is
the signal.

## Streaming toggle

The stream switch follows `supports_streaming` on the selected eligible
model. Inherit follows `supports_streaming` on the effective model's
eligible card, when that card is in the list. A broken inherit leaves
the switch disabled. The chip does not consult `/models/llm` for this
bit. [EC-178-33](11-edge-cases.md).

## Embedding row

The dimension is read-only text next to the model name, from
`embedding_dimension` on the card. The form does not offer a dimension
field the admin can type. Changing the model updates the text. The
rebuild control keeps its current confirmation. The save toast does not
say the library was re-embedded. [EC-178-38](11-edge-cases.md).

## Focus and keyboard

The menu is the existing command palette: type to search, Enter selects,
Escape closes. Inherit is the first item so Enter on an untouched search
selects Inherit only when Inherit is highlighted. Opening the menu
highlights the current selection, not Inherit, when a pair is selected.

Disabled Send on a broken inherit is a button with `aria-disabled` and
the empty-set sentence in `aria-describedby`. The locked trigger exposes
the lock sentence the same way.

## What the UI refuses to invent

- A client-side filter of `/models/llm` that "approximately" matches
  the ceiling. The payload is the list.
- A fallback label taken from `default_model` while the effective query
  loads. The trigger waits.
- A second visual language for the fleet card. Same menu, deployment
  scope, different helper sentence ([06-screens](06-screens.md)).
