# Lens — Full stack

Parent: [README](../README.md) · Laws: [LAW-172-6](../01-first-principles.md)

## Where the change is allowed

Handlers in [`content.rs`](../../../edgequake/crates/edgequake-api/src/handlers/pdf_upload/content.rs) and [`download.rs`](../../../edgequake/crates/edgequake-api/src/handlers/documents/query/download.rs) stay on the traits. The byte source changes inside `pdf_storage_impl.rs` and `original_storage_impl.rs`.

New module: `edgequake-storage/src/originals/`. The CLI is `edgequake/src/originals_cmd.rs` and a verb in `main.rs`. If a function hashes bytes in the CLI and again in the adapter, the wave is not done.

## Call path

```text
HTTP  -> trait method -> adapter
                      -> originals::io   (object mode)
                      -> sqlx BYTEA      (database mode)
CLI   -> originals::io
      -> sqlx claim
```

The adapter owns the transaction. `originals::io` owns the HTTP call to the bucket. Mixing those (a PUT inside `tx.commit`'s closure, or SQL inside `io.rs`) breaks [LAW-172-3](../01-first-principles.md) and [LAW-172-6](../01-first-principles.md).

`get_pdf` fills `pdf_data` from the object when `byte_location` is `object`. `pdf_processing` and `download_document_original` both read that field ([F-172-17](../02-code-facts.md), [F-172-18](../02-code-facts.md)). Leaving `fetch_pdf_row` to decode a null `BYTEA` fails the converter before pdfium starts.

## Signatures that stay

- `get_pdf_bytes_range(&pdf_id, start, end) -> Option<Vec<u8>>`
- `get_original(&workspace_id, &document_id) -> Option<DocumentOriginal>`
- `create_pdf` still returns `Uuid` and still maps `23505` to conflict

`DocumentOriginal.original_data` stays a `Vec<u8>` for this spec because the cap is 50 MiB ([F-172-08](../02-code-facts.md)). Streaming the non-PDF response is a follow-on, not a reason to fork the handler.

## Tests the stack owes

T-172-14 and T-172-29 prove the CLI and the server share the claim rule. T-172-25 proves the range helper matches the slice the handler already streams.
