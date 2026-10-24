# Lens — Full stack

Parent: [README](../README.md) · Shape: [04](../04-architecture.md) · Plan: [07](../07-implementation-plan.md)

## One path through the stack

```text
tools/call begin
    -> upload_session.rs maps args
    -> storage inserts mcp_upload_sessions
    -> presign.rs or the relay URL
host PUT
    -> bucket staging key, or PUT /api/v1/mcp/uploads/{id}
tools/call commit
    -> read staging
    -> admit_bytes (pdf_upload or document admission)
    -> task row the existing eq_task_get already reads
```

The handler does not grow a second PDF parser. [LAW-173-6](../01-first-principles.md) and [LAW-173-10](../01-first-principles.md) are the rule. [F-173-5](../02-code-facts.md) is the function that already exists.

## Boundaries

- API crate calls storage. Storage does not return MCP envelopes.
- `select_transfer` is the only branch between presign and relay. Commit reads the stored `transfer` and does not guess.
- W1 lands the `chunk_base64` / `data_base64` alias before any URL work, so current chunk clients keep working ([EC-173-22](../06-edge-cases.md), [LAW-173-9](../01-first-principles.md)).

## Races the stack has to get right

- Two commits: one `UPDATE … WHERE status = 'open'` ([EC-173-12](../06-edge-cases.md)).
- Begin on one process, commit on another: the row and the staging key are the shared state, not the `HashMap` ([EC-173-21](../06-edge-cases.md), [F-173-3](../02-code-facts.md)).
- A replay returns the stored ids ([EC-173-05](../06-edge-cases.md)).

## Tests the implementer runs first

W1: `cargo test -p edgequake-api --lib` on the upload module, plus `spec161_mcp_control_e2e`.

W5 and W8: `e2e_spec173_mcp_upload` from [08](../08-e2e-test-matrix.md).
