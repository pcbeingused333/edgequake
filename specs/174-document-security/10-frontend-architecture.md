# 10 — Frontend architecture

Parent: [README](README.md) · Prev: [09-ux-ui-spec](09-ux-ui-spec.md) ·
Next: [11-threat-model](11-threat-model.md)

The WebUI does not decide visibility. It renders what the API returns and
sends labels the server will check again. A second filter in React would
drift from `allows` ([LAW-174-5](01-first-principles.md)).

## One client type

New module `edgequake_webui/src/lib/clearance/`:

| File | Responsibility |
|------|----------------|
| `types.ts` | `ClearanceStatus`, `Classification`, `Compartment`, `SecurityRole`, `ClearanceRule` |
| `api.ts` | The routes in [08-api-contract](08-api-contract.md). The only `fetch` calls for this feature |
| `use-clearance.ts` | React Query keys `['clearance', workspaceId]` and `['clearance-roles', workspaceId]` |

`ClearanceStatus` is `{ enforcement: boolean; policyEpoch: number; rank: number | null; compartmentIds: string[] }`.
Components receive it as a prop or from the hook. They do not re-read
localStorage for a role.

Document list types gain optional `classification` and `compartments`.
When `enforcement` is false the fields are absent and the chips do not mount.

## Routes that must stay consistent

Both trees call the same hooks. A label edited on one tree shows on the
other after the shared query invalidation:

- `edgequake_webui/src/app/(dashboard)/documents/page.tsx`
- `edgequake_webui/src/app/(dashboard)/documents/[id]/page.tsx`
- `edgequake_webui/src/app/(dashboard)/graph/page.tsx`
- `edgequake_webui/src/app/(dashboard)/query/page.tsx`
- `edgequake_webui/src/app/(dashboard)/page.tsx`
- `edgequake_webui/src/app/(dashboard)/pipeline/page.tsx`
- `edgequake_webui/src/app/(dashboard)/knowledge/page.tsx`
- `edgequake_webui/src/app/w/[slug]/documents/page.tsx`
- `edgequake_webui/src/app/w/[slug]/graph/page.tsx`
- `edgequake_webui/src/app/w/[slug]/query/page.tsx`
- `edgequake_webui/src/app/w/[slug]/page.tsx`

Invalidation: after enable, relabel, or role save, invalidate `clearance`,
`documents`, `graph`, and `workspace-stats` for that workspace id. Do not
keep a document in the list cache if a later GET returns 404; drop that id.

## Where the controls mount

| Control | Existing component |
|---------|-------------------|
| Switch, strict-start row, profile list, preview sheet | Settings Access section, `components/settings/settings-nav.tsx` id `access` |
| Access property and popover | `components/documents/document-preview-panel.tsx` Details grid |
| Muted classification token | `components/documents/document-table-row.tsx` |
| Profile checklist | `components/members/member-detail-sheet.tsx` |
| Operation matrix, unchanged | `components/members/roles-tab.tsx` |

Preview calls `POST .../clearance/simulate` and renders the returned
titles in a Sheet. It does not set a header, a query param, or a second
React Query key for documents. [EC-174-07](12-edge-cases.md) is a
non-steward receiving 403 from that route, and the documents list staying
on the session subject.

## What not to add

- No clearance math in `components/documents/`.
- No filtering of graph nodes in the canvas. If the API sent it, it is visible.
- No copy of `security_role_rules` into the bundle as an authorization check.
- No `X-Clearance-Simulate` header and no "viewing as" route
  ([RB-174-05](17-roadblocks.md)).
- No document rows inside the `GET /roles` matrix.

## Tests

Component tests for the Access switch, the preview sheet, the Details
popover, and the empty state. Playwright for list, detail, and graph on
one workspace with two users ([14-e2e-test-matrix](14-e2e-test-matrix.md),
W8 rows). A 404 page does not contain the hidden title. The enable
control is the AlertDialog, not a checkbox the owner must tick first.

Next: [11-threat-model](11-threat-model.md).
