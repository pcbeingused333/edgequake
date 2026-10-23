# Lens — Full stack

Parent: [README](../README.md) · Facts: [03](../03-code-facts.md) · Plan: [07](../07-implementation-plan.md)

## The path a task actually takes

Prepare and extract live in `text_insert`. The checkpoint is written. `text_insert_persist` drops the fairness permit and awaits `persist_with_providers_progress_and_embedder` with no deadline ([F-171-02](../03-code-facts.md), [F-171-03](../03-code-facts.md)). Errors from that call are stored and the function returns `Ok` ([F-171-04](../03-code-facts.md)). Finalize writes `failed` and returns `Ok` ([F-171-05](../03-code-facts.md)). The worker therefore takes the success arm.

The wall-clock arm is a different bug on top: even a real `Err` from a Rust timeout never reaches the retry decision ([F-171-01](../03-code-facts.md)).

## One error, one arm

W2 makes transient persist a `TaskError`. W3 points both the `Ok(Err)` arm and the `Err(_elapsed)` arm at one function. That function is the one that already computes `will_retry`. `mark_failed` runs once, because it increments `retry_count` ([F-171-17](../03-code-facts.md)).

Phase logs are a helper used at the five phase boundaries. Both `document_id` and `task_id` are fields, not only span context ([LAW-171-4](../01-first-principles.md), [F-171-14](../03-code-facts.md)).

## Shutdown is part of this path

`Server::run` closes pools, then `main` stops workers ([F-171-13](../03-code-facts.md)). The claim loop’s `break` leaves the `'worker` loop. W5 flips the order and breaks `'worker` on `closed pool`. [LAW-171-5](../01-first-principles.md).

## Files

| Wave | Files |
|------|--------|
| W1 | `text_insert/persist.rs`, `worker.rs` message |
| W2 | `persist.rs` return type, `ingestion_reliability.rs`, `connection.rs`, `statement_timeout.rs`, committer |
| W3 | `worker.rs` elapsed arm, finalize only on the permanent path |
| W5 | `server.rs`, `main.rs`, claim loop, fairness sweep |

The worker stays free of sqlx ([DIP](../01-first-principles.md)). Tests: [T-171-01](../08-e2e-test-matrix.md) through [T-171-14](../08-e2e-test-matrix.md), [T-171-24](../08-e2e-test-matrix.md), [T-171-25](../08-e2e-test-matrix.md), [T-171-38](../08-e2e-test-matrix.md).
