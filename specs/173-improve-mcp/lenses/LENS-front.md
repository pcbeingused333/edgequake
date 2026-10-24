# Lens — Front

Parent: [README](../README.md) · Law: [LAW-173-10](../01-first-principles.md) · Bridge: [F-173-12](../02-code-facts.md)

## Web UI

No route, component, or upload button changes. PDF.js keeps using `GET /api/v1/documents/pdf/{pdf_id}/download` with `Range`. That handler is the SPEC-172 path. It does not learn a presigned GET ([LAW-173-3](../01-first-principles.md), [EC-173-19](../06-edge-cases.md)).

A document admitted by the agent appears in the existing document list because `admit_bytes` writes the same rows the Web UI reads.

## The client that does change

The front for an agent is [`mcp/src/gateway-bridge.ts`](../../../mcp/src/gateway-bridge.ts). Today `callRemoteTool` forwards arguments. It does not open a disk path ([F-173-12](../02-code-facts.md)).

Wave W6 adds `uploadLocalFile` in that package:

1. Call `eq_upload_begin` with the filename, length, and sha256.
2. `PUT` the bytes to `url` with `headers`.
3. Call `eq_upload_commit`.
4. Leave polling to the host, which already has `eq_task_get`.

The helper refuses a path that escapes the root the operator configured. The remote server never receives that path ([EC-173-23](../06-edge-cases.md), [LAW-173-1](../01-first-principles.md)).

Hosts that already have a shell can `curl -T` the URL themselves. The helper is for a desktop host that will not do that.

## Contract the front keeps stable

Tool names stay `eq_upload_begin`, `eq_upload_write`, `eq_upload_commit`, `eq_upload_abort`. [`mcp/tests/server.test.ts`](../../../mcp/tests/server.test.ts) already asserts those names. W1 updates the write-field assertion to `chunk_base64` ([EC-173-22](../06-edge-cases.md)).
