# Lens — AI engineer

Parent: [README](../README.md) · Protocol: [03](../03-protocol.md) · Laws: [LAW-173-1](../01-first-principles.md), [LAW-173-8](../01-first-principles.md)

## Context budget

The model context holds handles and evidence. It does not hold a PDF. Base64 of a 50 MiB file would dominate the window and a single flipped character would corrupt the object ([WHY-173-1](../00-why.md)).

The tool result the model should quote back is `upload_id`, `byte_length`, and `sha256`. The URL is for the host process. The instruction says not to repeat it ([LAW-173-3](../01-first-principles.md)). A leaked URL still only writes the signed hash ([LAW-173-4](../01-first-principles.md), [EC-173-01](../06-edge-cases.md)).

## Tool list stability

`tools/list` gains no new name. Descriptions change in W1 and W6. That is one cache break, then the list is stable again, which is what the 2026-07-28 tools page asks for when it requires a deterministic order ([03](../03-protocol.md)). Do not add `files/authorizeUpload` ([LAW-173-8](../01-first-principles.md)).

`EDGEQUAKE_MCP_PROFILE=query` keeps the write tools out of the prompt entirely ([EC-173-15](../06-edge-cases.md)).

## The loop the agent is allowed to run

```text
eq_upload_begin
    -> host PUT (shell or bridge, not a second model sampling of the bytes)
eq_upload_commit
eq_task_get until indexed | failed | cancelled
eq_search / eq_fetch
```

Begin without `sha256` is an error with no URL ([EC-173-24](../06-edge-cases.md)). Stuffing the file into `eq_upload_write` until the 1 MiB body fails is the wrong recovery ([EC-173-16](../06-edge-cases.md)). The recovery is the PUT.

The agent does not claim the document is searchable on `pending`. That rule is already in [`profile.rs`](../../../edgequake/crates/edgequake-api/src/mcp/project/profile.rs) and stays.

## Eval

A useful check is a scripted host, not a live model: begin, PUT a fixture PDF, commit, poll, search for a string that exists only in that PDF. T-173-29 covers the admit split. The search step can reuse the SPEC-161 control e2e once the task is indexed, or stop at `pending` when the suite has no LLM. Say which one the test did. Do not mark a `pending` document as retrieved.
