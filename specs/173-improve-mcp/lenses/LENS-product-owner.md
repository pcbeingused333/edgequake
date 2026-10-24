# Lens — Product owner

Parent: [README](../README.md) · Why: [00](../00-why.md) · Laws: [LAW-173-1](../01-first-principles.md), [LAW-173-2](../01-first-principles.md), [LAW-173-10](../01-first-principles.md)

## The job

A coding agent can put a PDF into the workspace and later search it. The agent does not become a file server, and the chat does not become the file.

Success looks like this for the operator:

- The agent calls begin, the host PUTs, the agent commits, the agent polls.
- The document shows up in the same list the Web UI uses, with the same pending then indexed states.
- A second copy of the same bytes is `duplicate_processing`, not a second document ([EC-173-10](../06-edge-cases.md)).
- A file over the existing 50 MiB cap is refused before any URL is issued ([EC-173-07](../06-edge-cases.md)).

## What the customer can rely on

| Promise | Bound |
|---------|--------|
| Claude, Cursor, Codex, or ChatGPT with an HTTP client can upload | [LAW-173-1](../01-first-principles.md) |
| The PDF is searchable only after `eq_task_get` says `indexed` | [LAW-173-2](../01-first-principles.md), same rule as SPEC-162 |
| A public demo can hide upload | `EDGEQUAKE_MCP_PROFILE=query` ([EC-173-15](../06-edge-cases.md)) |
| The Web UI upload the operator already has does not move | [LAW-173-10](../01-first-principles.md) |

## What we will not sell

- Upload of files larger than the cap the product already documents
- A link the end user can paste into a browser to download the original ([LAW-173-3](../01-first-principles.md))
- A new MCP method that only some hosts will speak in 2026 ([03](../03-protocol.md))

## Acceptance the owner can watch

T-173-07, T-173-10, T-173-15, and T-173-29 in [08](../08-e2e-test-matrix.md). T-173-29 is the PDF versus text split the product already has.
