# 09 — UX and UI

Parent: [README](README.md) · Prev: [08-api-contract](08-api-contract.md) ·
Next: [10-frontend-architecture](10-frontend-architecture.md)

Document access should feel like a property on a document and a line on a
person, not like a second product. The interaction model is taken from
Gr8r Studio (`/Users/raphaelmansuy/Github/03-working/gr8r-studio`): a
settings row with the control on the right, a property line that opens a
small menu, and a permissions grid that stays a grid. The widgets are the
ones EdgeQuake already ships (Card, Switch, AlertDialog, Popover, Sheet,
the members sheet). Do not port Gr8r's CSS.

Gr8r references, translated:

| Gr8r | EdgeQuake |
|------|-----------|
| `srow` in `src/pages/settings.js`: title, one line, control at the right | A row inside the existing Settings → Access card (`settings-nav.tsx` id `access`) |
| `pillbtn` + label popover in `src/overlays/drawer.js` and `popovers.js` | One outline button in the document Details grid (`document-preview-panel.tsx`) opening a Popover |
| Role pill on a member row (`settings.js` members section) | Access profiles on `member-detail-sheet.tsx`, using the same Popover |
| `permsTable` in `src/pages/members.js` | Leave `roles-tab.tsx` as the operation matrix. Do not add labels to it |
| `empty(icon, title, line)` | One title and one sentence. No count of hidden documents |

Hidden documents are absent. No lock icon, no grey row, no banner that
says the app is "viewing as" someone else ([RB-174-05](17-roadblocks.md)).

## Settings → Access

One card, four rows. Each row is a label, a single supporting line, and
one control. Nothing else on the row.

1. **Document labels.** Supporting line: "People only see documents that
   match the access you give them." Control: Switch. Off is today's
   library. Turning it on opens the existing `AlertDialog`: one sentence
   that members without an access profile see an empty library, and that
   you will still see everything. Buttons: Cancel, Turn on. No checkbox.
   Strict start is a second row, shown only while the switch is off:
   "Mark the current library as not reviewed." Default off, so existing
   documents become Internal.
2. **Access profiles.** A short list (HR reader, R&D reader). Opening one
   shows two fields: ceiling (a select of Public through Restricted) and
   departments (the label popover). An optional "Except" line uses the
   same popover for a deny. Four match kinds stay in the API. The screen
   does not show a rule table, a position column, or a policy expression.
3. **Preview.** A text button "See what this profile can open." It calls
   simulate and shows up to 20 titles in a sheet, plus the count of that
   list if it is under 20. It does not navigate, and it does not change
   the documents page. Close returns to the same card.
4. **Vocabulary.** "Departments and projects" as the same popover used on
   documents, with an add row at the bottom. Classification names are
   fixed and are not edited here.

Owners and admins see the card. Other members do not see the switch or
the profile editor. They see nothing about labels until a document they
can open shows its property.

## Documents

The table row (`document-table-row.tsx`) gains one muted token when
enforcement is on: the classification word, then at most two compartment
names, then "+n" if there are more. The word is the accessible name.
Color may tint the token. Color is not the only signal.

The preview panel Details block (`document-preview-panel.tsx`) gains one
property row, "Access", aligned with the existing ID row. The value is a
button. People who may relabel get a popover: one select for the ceiling,
checkboxes for departments they hold. People who may not relabel see the
words, not a control. Unverified reads "Only you, until reviewed" for the
uploader, and the steward sees "Not reviewed."

The documents toolbar gains one select, the same size as the current
status filter: All, Public, Internal, Confidential, Restricted. It
filters the list the server already returned. It is not a second
authorization check.

A hidden id uses the current not-found view. The title string of the
hidden document is absent. The copy does not say "access" or "denied."

Empty library, member with no profile: title "Nothing to show here",
line "An owner can give you an access profile." No number.

## Upload

The existing upload dialog gains one row under the file list, only when
enforcement is on. Label: "Access". Default button text: "Only you, until
reviewed." The popover is the same component as the preview panel. Bulk
upload uses that one choice for the batch. A file the actor cannot label
fails on the server; the row shows the file name and "Could not add",
without another document's title ([EC-174-37](12-edge-cases.md)).

## People

`roles-tab.tsx` stays the Owner / Admin / Member / Viewer matrix. A
single line under the table: "Who can open which documents is set on
each person and under Settings → Access."

`member-detail-sheet.tsx` gains an "Access" row. The button lists profile
names or "None". The popover is a checklist of profiles. Saving it calls
the membership assignment route. It does not add a wrapping column to
`members-table.tsx`.

## Query and graph

No new chrome on the query page. Citations may show the same muted
classification token as the table. The graph draws the served
neighborhood. It does not add a legend for hidden nodes. Degree in the
side panel is the number the API returned.

Pipeline and tasks keep their current layout. A task the caller cannot
see is absent. An error the caller cannot read becomes "Processing
failed." without a quote from the document.

## What we removed from the first draft

- A checkbox the owner must tick before enable.
- Two chips on every card.
- A rule spreadsheet with match-kind and position columns.
- A "Viewing as" mode and `X-Clearance-Simulate` on later requests.
- A new settings information architecture. Access is the section that is
  already in `SETTINGS_SECTIONS`.

Next: [10-frontend-architecture](10-frontend-architecture.md).
