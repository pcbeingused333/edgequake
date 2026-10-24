# 05 — Policy model

Parent: [README](README.md) · Prev: [04-findings](04-findings.md) ·
Next: [06-graph-query-ai](06-graph-query-ai.md)

## Names

NIST SP 800-162 defines ABAC as a decision from attributes of the subject,
the object, the operation, and, when needed, the environment. The policy
decision point (PDP) evaluates. The policy enforcement point (PEP) refuses
the request. This pack uses those two roles and no others.

INCITS 359-2012 (R2022) is the RBAC reference: users, roles, permissions,
operations, objects. EdgeQuake already has the operation half
([F-174-01](04-findings.md)). SPEC-174 adds object attributes and keeps
the operation half where it is.

| Word in this pack | NIST / standard word | Stored as |
|-------------------|----------------------|-----------|
| Membership role | RBAC role for operations | `memberships.role` (unchanged) |
| Security role | RBAC role whose permissions are tag rules | `security_roles` |
| Classification | Object attribute, totally ordered | `classification_rank` smallint |
| Compartment | Object attribute, unordered set | `compartment_ids uuid[]` |
| Rule | Policy rule | `security_role_rules` |
| `allows` | PDP | pure function in `edgequake-core` |
| `VisibilityScope` | PEP input | request extension + SQL GUCs |

Citations:

- NIST SP 800-162 (updated 2 August 2019): <https://csrc.nist.gov/pubs/sp/800/162/upd2/final> and <https://nvlpubs.nist.gov/nistpubs/specialpublications/NIST.SP.800-162.pdf>
- NIST RBAC project and INCITS 359-2012 (R2022): <https://csrc.nist.gov/projects/role-based-access-control> and <https://webstore.ansi.org/standards/incits/incits3592012r2022>
- AWS IAM ABAC (tags on principal and resource, fewer policies as the corpus grows): <https://docs.aws.amazon.com/IAM/latest/UserGuide/introduction_attribute-based-access-control.html>
- Databricks Unity Catalog ABAC tag practices (controlled vocabulary, tag edits are privileged, default restrictive for unclassified, few policies): <https://docs.databricks.com/aws/en/data-governance/unity-catalog/abac/best-practices>
- OWASP Authorization Cheat Sheet (deny by default, server-side enforcement, no client-supplied authorization): <https://cheatsheetseries.owasp.org/cheatsheets/Authorization_Cheat_Sheet.html>
- Microsoft Purview sensitivity labels (one label on the item; a container label does not replace item labels): <https://learn.microsoft.com/en-us/purview/sensitivity-labels>
- Zanzibar (relationship tuples, per-object shares). Deferred: <https://www.usenix.org/system/files/atc19-pang.pdf>

Zanzibar answers "who is related to this object?" This pack answers "which
tags does this object carry, and which tags does this role hold?" A later
spec can add share tuples. This one must not grow a tuple store in disguise.

## Ranks

| Value | Rank | Who can read it |
|-------|------|-----------------|
| `public` | 1 | Subject rank ≥ 1, compartments satisfied |
| `internal` | 2 | Subject rank ≥ 2, compartments satisfied |
| `confidential` | 3 | Subject rank ≥ 3, compartments satisfied |
| `restricted` | 4 | Subject rank ≥ 4, compartments satisfied |
| `unverified` | 5 | Steward, or the uploader until a raise ([LAW-174-13](01-first-principles.md)) |

Rank 5 is above every grant of `classification_at_most`. A normal allow
never covers `unverified`. Seed these five values when enforcement is
enabled. Operators may not reorder them and may not delete them.

Compartment tags are `key:value` strings in the vocabulary (`department`,
`hr`). The document stores tag ids. Display names are not compared.

## Compiled subject

Assigned security roles compile once per request:

1. `rank` = max `classification_at_most` among **allow** rules. No such rule → rank 0.
2. `compartments` = union of tags named by allow `any_of` and allow `all_of`.
3. Each **deny** rule then subtracts, and subtraction wins over every allow:
   - deny `any_of` or `all_of` removes those tag ids from `compartments`
   - deny `classification_at_most(R)` sets `rank = min(rank, R - 1)`
   - deny `untagged` sets `allow_empty_compartments = false` (default is true)
4. `steward` is true if any assigned role has `steward`. Deny still applies
   to a steward. The bootstrap steward role has no deny rules.
5. `break_glass` is true only when the server flag is on. It is not a role.

`all_of` and `any_of` both add tags to the subject. They are not two
different read algebras. The read algebra is always subset, below. The
two match kinds exist so a rule can name "these tags" without a second
grammar. A role that must hold HR and Legal uses one `all_of` listing
both. A role that holds either uses `any_of`. After deny subtraction the
subject simply holds a set.

## Decision

`allows(subject, object, Read)`:

```text
  enforcement off -----------------> Allow
  break_glass ---------------------> Allow (PEP writes the audit row)
  deny subtracted into subject
           |
           v
  +--------------------------------------+
  | steward and object not denied        |
  | by a subtracted compartment or rank  |
  +------------------+-------------------+
                     |
                     v
  uploader and object.rank = unverified -> Allow
  uploader and dominance holds ---------> Allow
  dominance holds -----------------------> Allow
  else ----------------------------------> Deny
```

Dominance holds when all of these are true:

- `object.rank != unverified`
- `object.rank <= subject.rank`
- if `object.compartments` is empty: `subject.allow_empty_compartments`
- else: `object.compartments ⊆ subject.compartments`

Equal rank is allowed ([EC-174-04](12-edge-cases.md)). A document tagged
HR and Legal is visible only to a subject who holds both
([EC-174-06](12-edge-cases.md)). A subject who holds HR and Legal may read
a document tagged only HR ([EC-174-05](12-edge-cases.md)).

Write and relabel:

- Write is allowed when Read would be allowed on the **resulting** label.
  A member cannot upload into a compartment they do not hold, and cannot
  create a rank above their rank. Creating `unverified` is allowed for the
  uploader (the exception), not for a peer.
- Relabel requires `tags.manage` (owner or admin) and Read on both the old
  and the new label. If the new rank is lower, or the compartment set
  shrinks, the actor must be steward. The PEP writes an audit row.
- `tags.manage` without clearance does not reveal the document body.

Operation axis is applied before `allows`. A readonly membership with a
high rank still cannot upload ([LAW-174-1](01-first-principles.md)).

## Worked workspace

Roles:

| Role | Allow | Deny |
|------|-------|------|
| Steward (owner) | rank 4, no compartments required, `steward` | none |
| HR reader | `classification_at_most(confidential)`, `any_of(department:hr)` | none |
| R&D reader | `classification_at_most(confidential)`, `any_of(department:rnd)` | none |
| HR without Atlas | same as HR reader | `any_of(project:atlas)` |

Documents:

| Document | Rank | Compartments | HR reader | R&D reader | HR without Atlas |
|----------|------|--------------|-----------|------------|------------------|
| Handbook | internal | (empty) | read | read | read |
| Payroll | confidential | hr | read | 404 | read |
| Chip spec | confidential | rnd | 404 | read | 404 |
| Payroll + legal | confidential | hr, legal | 404 | 404 | 404 |
| Atlas payroll | confidential | hr, atlas | read | 404 | 404 |
| New upload | unverified | (empty) | 404 (unless uploader) | 404 | 404 |

## What we refuse to generalize

- No user-supplied policy expression language. Match kinds are the four
  names above.
- No per-document ACL row. A rule that names a document id is out of scope.
- No environment attributes in W1–W11 (time of day, IP, device). The NIST
  definition allows them. This pack does not need them for the job.
- No copying of tags onto the JWT. The token stays a session. Clearance is
  loaded from membership assignments so a rule edit applies on the next
  request without waiting for refresh ([LAW-174-5](01-first-principles.md)).

## Module

`edgequake-core/src/clearance/` exports `SubjectClearance`, `ObjectLabel`,
`ClearanceAction`, and `allows`. Unit tests in that module cover the table
above, rank 0, deny-after-allow, steward plus deny, and the uploader
exception. No test in that module opens a database.

Next: [06-graph-query-ai](06-graph-query-ai.md).
