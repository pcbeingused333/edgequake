# 03 — Product spec

Parent: [README](README.md) · Prev: [02-surfaces](02-surfaces.md) ·
Next: [04-findings](04-findings.md)

## Personas

| Persona | Operation axis | Object axis | Job |
|---------|----------------|-------------|-----|
| Owner | `memberships.role = owner` | Steward, granted at enable | Turns enforcement on, defines tags, assigns security roles |
| Steward | Owner or admin | `security_roles.steward = true` | Reviews `unverified`, may downgrade, sees every label |
| HR analyst | `member` or `readonly` | Rank ≤ confidential, compartment `department:hr` | Reads and queries HR documents |
| R&D analyst | `member` | Rank ≤ confidential, compartment `department:rnd` | Must not learn that HR documents exist |
| Member, no security role | `member` | Empty, once enforcement is on | Sees an empty workspace and a sentence that explains why |
| Platform admin | `users.role = admin` | No skip | Can open the workspace. Document rows still follow clearance |
| Uploader | Whoever uploaded | [LAW-174-13](01-first-principles.md) | Sees their own `unverified` body until a raise exceeds their clearance |

## Stories

### US-174-1 — Enable without locking the owner out

The owner opens workspace settings and turns document security on. The
dialog says that members without a security role will see no documents.
On confirm, existing documents become `internal`, the owner is steward,
and the owner still sees the corpus. Strict-start is a second control
that labels the existing corpus `unverified` instead.

Accept: [EC-174-08](12-edge-cases.md) · [LAW-174-14](01-first-principles.md).

### US-174-2 — Tags are a small vocabulary

The steward creates classification values (the four ranks plus
`unverified` are seeded) and compartments such as `department:hr` and
`department:rnd`. Free-typed tags on a document are not a policy. A tag
must exist in the workspace vocabulary before it can be attached.

Accept: [LAW-174-7](01-first-principles.md) · [EC-174-11](12-edge-cases.md).

### US-174-3 — A role carries rules

The steward creates security role "HR reader" with allow
`classification_at_most(confidential)` and allow `any_of(department:hr)`,
and assigns it to the HR analyst's membership. The analyst lists and
queries HR documents up to confidential. An R&D-only document is absent.

Accept: [EC-174-02](12-edge-cases.md) · [EC-174-04](12-edge-cases.md) ·
[EC-174-06](12-edge-cases.md).

### US-174-4 — Upload stays inside the clearance

The upload dialog offers only labels the actor may apply. Omitting labels
stores `unverified`. The uploader can open that document. A peer member
cannot. A steward can raise it. If the new label is above the uploader,
the uploader receives 404 on the body afterwards.

Accept: [EC-174-01](12-edge-cases.md) · [EC-174-09](12-edge-cases.md) ·
[EC-174-37](12-edge-cases.md).

### US-174-5 — Query and graph agree with the list

The same analyst, with no `document_filter`, gets citations, a graph, and
a PNG that contain only visible sources. A node that also appears in a
hidden document shows the visible fragment and the visible degree. The
answer does not say that other documents exist.

Accept: [LAW-174-9](01-first-principles.md) · [LAW-174-16](01-first-principles.md) ·
[EC-174-15](12-edge-cases.md) · [EC-174-16](12-edge-cases.md).

### US-174-6 — Preview a profile before saving

On Settings → Access, the steward opens "See what this profile can open"
and reads up to 20 titles. The documents page, the query page, and the
graph do not change. The preview does not use the steward bit.

Accept: [08-api-contract](08-api-contract.md) simulate route ·
[09-ux-ui-spec](09-ux-ui-spec.md) · [RB-174-05](17-roadblocks.md).

## Acceptance (workspace with enforcement on)

1. HR analyst list count equals the HR-visible set, not the table count.
2. R&D analyst `GET` of an HR id returns 404, the same body as a missing id.
3. Query citations' document ids are a subset of the visible set.
4. Graph degree for a shared entity differs per analyst when hidden edges exist.
5. `eq_fetch` of the other analyst's retrieval id returns 404.
6. Enabling enforcement does not change a workspace that stays off, including Acc.
7. Readonly plus a high clearance can query and cannot upload.
8. Master key and instance admin, with the break-glass flag unset, do not
   read a document outside their security roles.

## Non-goals (product)

Repeated from the [README](README.md) so this chapter stands alone:

- Sharing one document with one person (ReBAC / Zanzibar).
- An embedded policy language product (Cedar, OPA, XACML).
- Watermarks, encryption, or rights management on the file bytes.
- Changing SPEC-001 scores. Acc runs with enforcement off.
- Hiding a document from its uploader while it is still `unverified`.

## Copy

Enable dialog, one sentence (wording may be tightened in
[09-ux-ui-spec](09-ux-ui-spec.md), the meaning may not):

"Members without an access profile will see an empty library. You will
still see every document."

Empty state, member with no profile:

- Title: "Nothing to show here."
- Line: "An owner can give you an access profile."

The empty state does not say how many documents exist. There is no
checkbox and no "viewing as" banner.

Next: [04-findings](04-findings.md).
