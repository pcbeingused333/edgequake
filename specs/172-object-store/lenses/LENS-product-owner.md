# Lens — Product owner

Parent: [README](../README.md) · Laws: [LAW-172-1](../01-first-principles.md), [LAW-172-7](../01-first-principles.md)

## Job to be done

An operator with a Postgres full of uploaded files wants those files in S3, GCS, Azure Blob, or a local MinIO, without a new viewer and without a flag day that refuses to boot.

## What ships

- A config switch. Unset means today's behavior. Existing installs do nothing.
- The same download URLs. Support does not rewrite bookmarks.
- A command that previews the move, copies, checks hashes, and only then offers to drop the database copy.
- A health field so a dashboard can show `database` or `object` next to the database status that already exists.

## What does not ship

Search, chat, and the markdown pane do not change store. Page images stay where they are. There is no browser upload straight to the bucket in this spec. Those are different products.

## Acceptance the owner can demo

1. Start with database mode, upload a PDF, download it.
2. Restart with object mode and MinIO. Upload another PDF. The first PDF still downloads.
3. Run `edgequake originals plan` and read the size.
4. Run `migrate`, then download both PDFs.
5. Run `clear --yes` and read the sentence about disk space. Then download again.

If step 5 surprises the operator, the copy failed the spec. The sentence is specified in [08](../08-migration-ux.md).

## Risks to name in the release note

Disk does not shrink on clear. A mixed fleet (some replicas still in database mode) will not boot once refs exist. That is intentional ([EC-172-20](../05-edge-cases.md)).
