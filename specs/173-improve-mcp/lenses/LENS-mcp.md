# Lens — MCP

Parent: [README](../README.md) · Protocol: [03](../03-protocol.md) · Law: [LAW-173-8](../01-first-principles.md)

## What the revision gives us

MCP `2026-07-28` is stateless. The handle in the tool argument is the session. `upload_id` is that handle. The server does not add `Mcp-Session-Id`, and it does not require `initialize` beyond what the gateway already answers.

Tools stay model-controlled, with `readOnlyHint: false` on begin, write, and commit. Commit’s `idempotentHint` becomes `true` because a second call returns the same ids ([05](../05-contract-delta.md), [EC-173-05](../06-edge-cases.md)).

Resources stay the channel for small reads. This spec does not put the PDF into a `BlobResourceContents` block.

## What the revision does not give us

There is no published client-to-server file upload. SEP-2631 is closed. The experimental extension is a draft ([03](../03-protocol.md)). Implementing `files/authorizeUpload` would be a private dialect.

173 keeps the four tool names and adds an ordinary HTTPS PUT beside them. Hosts that can run `curl` can use it. Hosts that cannot, use the stdio helper in W6, which is still `tools/call` plus an HTTP PUT the bridge performs.

## Profile and scope

Query profile: tools omitted, direct call rejected with the existing control-profile error ([EC-173-15](../06-edge-cases.md), [F-173-10](../02-code-facts.md)).

Write scope: `edgequake:write` via `required_scope_for_tool`. Missing scope is `insufficient_scope` and `WWW-Authenticate` ([EC-173-27](../06-edge-cases.md)).

The relay is not a JSON-RPC method. It does not appear in `tools/list`. A client that only speaks MCP still needs a host that can PUT.

## Schema honesty

The advertised write field becomes `chunk_base64`, matching the handler, with `data_base64` accepted for one release ([EC-173-22](../06-edge-cases.md), [LAW-173-9](../01-first-principles.md)). A schema that names a field the server ignores is a broken tool, not a style nit ([F-173-2](../02-code-facts.md)).

## Presign is not an MCP feature

`Method::PUT` on `Signer` is an object-store call. The MCP result only carries the URL string. No GET is signed ([EC-173-19](../06-edge-cases.md)).
