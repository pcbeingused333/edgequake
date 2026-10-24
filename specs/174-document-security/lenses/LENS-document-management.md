# Lens — Document Management

Parent: [README](../README.md) · Product: [03-product-spec](../03-product-spec.md) ·
API: [08-api-contract](../08-api-contract.md)

## What this lens owns

[LAW-174-7](../01-first-principles.md) and [LAW-174-13](../01-first-principles.md).
Labels are how a corpus stays governable. Tagging is a privileged
operation. The uploader exception is narrow.

## Vocabulary

Seed five classification values at enable. Do not let operators delete or
reorder them. Compartments are a workspace vocabulary (`department:hr`,
`project:atlas`), created before they can be attached. A free string on
the upload form is not a tag.

This matches the practice of keeping a small taxonomy and treating tag
changes as security events (Databricks ABAC practices, cited in
[05-policy-model](../05-policy-model.md)). Purview's point that a container
label does not replace an item label is why the workspace flag is only
the on/off switch: each document still has its own rank and compartments.
<https://learn.microsoft.com/en-us/purview/sensitivity-labels>

## Lifecycle

| Moment | Label |
|--------|-------|
| Enforcement off | No label required. Behavior matches today |
| Enable, strict start off | Existing documents become Internal |
| Enable, strict start on | Existing documents become Unverified |
| New upload, labels omitted | Unverified. Uploader can read. Peers cannot |
| Steward raises within the uploader's clearance | Uploader still reads |
| Steward raises above the uploader | Uploader receives 404 on the body |
| Downgrade | Steward only, audited |
| Tag still assigned | Delete returns 409 |

Bulk upload checks each file. A file outside the actor's clearance is
rejected without revealing another document's id
([EC-174-37](../12-edge-cases.md)).

## Copies of the bytes

Originals, PDF blobs, page renders, and assets inherit the document
columns in the same transaction as the relabel. A markdown download is
not a side door ([EC-174-29](../12-edge-cases.md)). Content-hash dedup
must not point at a document the uploader cannot see
([EC-174-28](../12-edge-cases.md)).

## What this lens refuses

Automatic relabel by the extraction model. The model does not choose
confidentiality. A steward or an uploader within their clearance does.
