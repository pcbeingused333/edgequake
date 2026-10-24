# Lens — Database

Parent: [README](../README.md) · Laws: [LAW-172-1](../01-first-principles.md), [LAW-172-3](../01-first-principles.md), [LAW-172-4](../01-first-principles.md)

## What changes in Postgres

Migration 215, not 172. Nullable `pdf_document_blobs.pdf_data` and `document_originals.original_data`, plus `byte_location` on the same row. The check is `database` implies bytes present, `object` implies bytes null. It does not subquery `original_object_refs` ([constraints](https://www.postgresql.org/docs/current/ddl-constraints.html)).

`original_object_refs` has `pdf_id` and `document_id`, exactly one of them set, each a foreign key `ON DELETE CASCADE`. A `BEFORE DELETE` trigger copies `object_key` into `original_object_deletes`. That is how [`delete_relational_documents_for_workspace`](../../../edgequake/crates/edgequake-api/src/document_read_model.rs) picks up bucket keys without calling `delete_pdf`.

RLS follows migration 167: forced, restrictive, workspace id, tenant proven through `workspaces`. `pdf_document_blobs` can stay without its own policy because the blob is reached from `pdf_documents`. The ref table cannot, because it is the new place a guessed UUID would land.

## What does not change

`documents.content`, chunks, embeddings, graph tables, `markdown_content`, `document_mm_assets`. List queries keep avoiding detoast of `documents.content` ([F-172-10](../02-code-facts.md)).

`pdf_documents.sha256_checksum` stays the dedup key. The ref stores the same hex. Non-PDF originals get a checksum for the first time, on the ref, because [F-172-05](../02-code-facts.md) has none.

## Locks and the role

The migrator claim is `FOR UPDATE SKIP LOCKED` on a `BYPASSRLS` maintenance role, and the lock ends before PUT. The API role does not have `BYPASSRLS`. A lock held across the network call will show up as `active` sessions and will stall uploads.

`plan` prints `pg_total_relation_size`. `VACUUM FULL` rewrites the table and needs about that much free space ([VACUUM](https://www.postgresql.org/docs/current/sql-vacuum.html)). The CLI tells the operator and does not run it ([EC-172-17](../05-edge-cases.md), [EC-172-37](../05-edge-cases.md)).

## Cascade

`document_originals` and `pdf_document_blobs` disappear with the parent row. The ref disappears by its own foreign key. The trigger has already queued the object key. A trigger that tried to call the network is the wrong tool.
