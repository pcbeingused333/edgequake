# 00 — WHY

Parent: [README](README.md) · Next: [01 First principles](01-first-principles.md)

## The problem in one sentence

A coding agent has a PDF on disk and a 1 MiB JSON-RPC pipe. The pipe cannot carry the file, and the model must not be the process that types the bytes.

## Five WHYs

**WHY-173-1 — Why can the agent not upload the PDF it is holding?**

`POST /mcp` rejects a body over 1 MiB ([F-173-1](02-code-facts.md)). `eq_upload_write` expects the next chunk as base64 inside that body ([F-173-2](02-code-facts.md)). A 50 MiB PDF is about 67 MiB of base64, split across dozens of tool calls. The model would have to emit those tokens. Hosts such as Claude, ChatGPT, Cursor, and Codex do not hand a user file to the model as a tool argument. They can run an HTTP PUT.

```text
  PDF on the agent disk
           |
           X  tools/call body  > 1 MiB rejected
           |
           X  model types base64  (tokens, and a lie if one character flips)
           |
           v
  host HTTP PUT of the raw bytes
```

ROOT → [LAW-173-1](01-first-principles.md), [LAW-173-9](01-first-principles.md)

**WHY-173-2 — Why is a signed URL dangerous if it is only "a link"?**

Whoever holds the URL can perform the signed method until it expires. If the signature does not bind the checksum and the content type, a leaked URL can store a different file under the victim's tenant. The October 2026 file-uploads discussion treats a pre-authorized URL as a capability, and it asks for bearer relay when the token must not leave the host ([03](03-protocol.md)).

ROOT → [LAW-173-3](01-first-principles.md), [LAW-173-4](01-first-principles.md), [LAW-173-5](01-first-principles.md)

**WHY-173-3 — Why declare the file before any byte moves?**

A server that accepts bytes before it knows the tenant, the size, and the hash spends bandwidth on an upload it will reject, and it has to pick a region before it knows the object. The 2 Oct 2026 working group locked the order: metadata, then the upload target, then the bytes, then finalize ([03](03-protocol.md)). EdgeQuake already has begin / write / commit. The write step is the part that must leave JSON-RPC.

ROOT → [LAW-173-2](01-first-principles.md)

**WHY-173-4 — Why must commit call the path that already admits a PDF?**

`commit_upload_id` already reads the temp file and calls `admit_bytes`, which branches to `admit_pdf_bytes` or `admit_document_for_processing` ([F-173-5](02-code-facts.md)). A second admission path would drift from the Web UI on hash, dedup, and the pending task. The signed PUT changes where the bytes sit before that call. It does not change the call.

ROOT → [LAW-173-6](01-first-principles.md), [LAW-173-10](01-first-principles.md)

**WHY-173-5 — Why does the in-memory session fail a second replica?**

`McpUploadStore` is a `HashMap` behind a `Mutex`. The bytes are a file under the process temp dir ([F-173-3](02-code-facts.md)). Begin on replica A and commit on replica B is a miss. `user_id` is stored and never compared ([F-173-4](02-code-facts.md)). The bucket from SPEC-172 is already shared. The session row has to be shared too, and the object the client writes has to be a staging key the commit can see from any replica.

ROOT → [LAW-173-6](01-first-principles.md), [LAW-173-7](01-first-principles.md)

**WHY-173-6 — Why not wait for the MCP file RPC?**

The published revision is `2026-07-28`. It has tools and resources. It does not have `files/authorizeUpload`. SEP-2631 was closed on 2026-10-14. The replacement is an experimental extension, still a draft ([03](03-protocol.md)). Agents that need memory now can use the tools EdgeQuake already advertises, with a PUT beside the tool call. Field names can rhyme with the draft so a later adapter stays small. The RPC itself stays unimplemented.

ROOT → [LAW-173-8](01-first-principles.md)

## Product statement

EdgeQuake is the memory the agent can search after the task says `indexed`. The upload is a short-lived, checksum-bound PUT that ends in the admission path that already exists. The model holds `upload_id`. The host holds the URL.
