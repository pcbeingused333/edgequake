# Lens — Security

Parent: [README](../README.md) · Threats: [11-threat-model](../11-threat-model.md) ·
Algebra: [05-policy-model](../05-policy-model.md)

## What this lens owns

[LAW-174-3](../01-first-principles.md), [LAW-174-4](../01-first-principles.md),
[LAW-174-7](../01-first-principles.md), [LAW-174-8](../01-first-principles.md),
and [LAW-174-12](../01-first-principles.md).

Default deny when enforcement is on. Fail closed on NULL rank, orphan
chunks, and unset GUCs. Hidden ids are 404 with the same body as a
missing id. Tag edits are a boundary. Admin and the master key do not
skip clearance.

## Standards this design is checked against

| Control | Source |
|---------|--------|
| Subject, object, operation attributes; PDP and PEP | NIST SP 800-162, <https://csrc.nist.gov/pubs/sp/800/162/upd2/final> |
| Roles grant permissions; this pack does not replace that axis | INCITS 359-2012 (R2022), <https://csrc.nist.gov/projects/role-based-access-control> |
| Tags on principals and resources, fewer policies as the set grows | AWS IAM ABAC, <https://docs.aws.amazon.com/IAM/latest/UserGuide/introduction_attribute-based-access-control.html> |
| Controlled vocabulary, privileged tag edits, unclassified is restrictive, avoid policy sprawl | Databricks ABAC practices, <https://docs.databricks.com/aws/en/data-governance/unity-catalog/abac/best-practices> |
| Server-side checks, deny by default, no client-supplied authz | OWASP Authorization Cheat Sheet, <https://cheatsheetseries.owasp.org/cheatsheets/Authorization_Cheat_Sheet.html> |
| One label on the item | Microsoft Purview, <https://learn.microsoft.com/en-us/purview/sensitivity-labels> |
| Per-object relationship shares | Zanzibar, deferred, <https://www.usenix.org/system/files/atc19-pang.pdf> |

## Review checks

- Deny subtracts after allow, including when the subject is also steward
  ([EC-174-03](../12-edge-cases.md)).
- Downgrade writes `audit_logs` and requires steward ([LAW-174-7](../01-first-principles.md)).
- Break-glass writes `Warning` before the body and defaults off
  ([EC-174-30](../12-edge-cases.md)).
- Dedup, stats, degree, websocket, task errors, and facets do not name
  a hidden object ([LAW-174-8](../01-first-principles.md)).
- `document_filter` and MCP `document_ids` cannot widen
  ([EC-174-14](../12-edge-cases.md)).
- Simulate does not keep the steward bit ([EC-174-07](../12-edge-cases.md)).

## Accepted residual

A shared entity name can confirm the name exists. Fragments and document
ids must not. Two workspaces are the isolation tool when the name itself
is the secret ([11-threat-model](../11-threat-model.md)).

## Block the wave if

`e2e_spec174_hidden_id_404` shows any JSON difference, or
`e2e_spec174_admin_master_key` reads a document with the flag unset.
