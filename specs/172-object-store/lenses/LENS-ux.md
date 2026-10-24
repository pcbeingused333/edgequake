# Lens — UX

Parent: [README](../README.md) · Law: [LAW-172-4](../01-first-principles.md) · Script: [08](../08-migration-ux.md)

## The person

An operator who has run `edgequake migrate` and expects the same kind of message: what happened, what is safe, what to type next. They are not looking at a dashboard for this move. The tool is the interface.

## Rules for every screen

- The default command changes nothing. `edgequake originals` is `plan`.
- Destructive steps are separate and need `--yes`.
- A failure names a class and the next command. It does not print a stack trace as the only guidance.
- Numbers are counts and bytes, plus a plain elapsed time. No progress bar library is required.
- Secrets and URL userinfo never appear.

## The sentence that must not be omitted

After clear, the tool says disk is unchanged until `VACUUM FULL` or `pg_repack`, and it names `pdf_document_blobs` and `document_originals`. Hiding that line would make a correct migration look like a failed one when `du` does not move. Test T-172-17 locks the words.

## States

The four boxes in [08](../08-migration-ux.md) (BYTEA only, both, verified, ref only) are the only states `status` is allowed to print. A fifth label means the implementation invented a state the operator cannot undo.

## Tone

Short lines. `next` is a copy-paste command. `kept` tells them the database copy is still there when a checksum fails, so they do not restore from backup out of fear.
