# Lens — Front

Parent: [README](../README.md) · Law: [LAW-172-5](../01-first-principles.md)

## What the viewer does

Nothing new. PDF.js already requests `GET /api/v1/documents/pdf/{pdf_id}/download` with `Range`. The handler already turns that into `get_pdf_bytes_range` ([F-172-12](../02-code-facts.md)). Object mode fills those ranges from the bucket. A front-end change that points the viewer at a presigned URL is out of spec.

Non-PDF download keeps `Content-Disposition` so the saved name stays the original filename, including non-ASCII names ([EC-172-18](../05-edge-cases.md)).

## What may show

If a settings or status screen already renders `storage_mode` from `/health`, add a second read-only line:

```text
Originals    Database | Object store
```

Source fields: `original_store`, `components.original_store_ready`. Do not rename `storage_mode`. Do not add an edit control. Mode is an environment variable, not a per-workspace toggle. A control that wrote it would desync replicas ([LAW-172-7](../01-first-principles.md)).

If no screen renders `storage_mode` today, ship no UI in this spec. The CLI is enough.

## Empty, error, loading

- Object missing after a bad clear: the API returns the same 404 the missing-BYTEA path returns today. The viewer's existing error state is the UI.
- 416 is already the range error. No new toast.
- `/ready` 503 while `/health` is 200 is an operator signal, not a page in the document viewer.

## Check

T-172-06 and T-172-07 are the front contract: 206 and 416 on the current route, no new path in the web app.
