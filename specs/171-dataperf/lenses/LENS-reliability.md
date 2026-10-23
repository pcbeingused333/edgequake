# Lens — Reliability

Parent: [README](../README.md) · Laws: [01](../01-first-principles.md) · Cases: [06](../06-edge-cases.md)

## Bounded silence

A fault that cannot be named cannot be retried. Incident A was 7194 s with no log line ([WHY-171-1](../00-why.md)). [LAW-171-1](../01-first-principles.md) bounds the wait in the server. [LAW-171-4](../01-first-principles.md) names the phase. The outer 7200 s clock remains, and it is no longer the first clock.

## Transient versus permanent

| Signal | Class | Retry |
|--------|-------|-------|
| `persist deadline exceeded` | `PersistContention` | while budget remains |
| `55P03` `lock_not_available` | `PersistContention` | while budget remains |
| `40P01` `deadlock_detected` | `PersistContention` | while budget remains |
| `57014` `query_canceled` | `PersistContention` | while budget remains |
| `knowledge-graph merge error(s) during persist` | `GraphMerge` | no |
| Tombstone / document deleted | `DocumentDeleted` | no |
| Circuit breaker open | existing | no |
| Operator or shutdown cancel | `Cancelled` | no |

[LAW-171-2](../01-first-principles.md). The order of the matcher is the whole reliability property. If `GraphMerge` is tested first, the new class never wins ([EC-171-08](../06-edge-cases.md), [F-171-06](../03-code-facts.md)).

`mark_failed` on the wall-clock path today increments `retry_count` and clears the typed-timeout counter, then declares the task permanent ([F-171-17](../03-code-facts.md)). That is a false attempt. W3 uses one transition and the existing backoff, so a hot loop has to wait `calculate_backoff_delay`.

## Zombies and shutdown

Dropping a future is not a cancel (GH-336, [EC-171-07](../06-edge-cases.md)). The server timer or an explicit connection drop is the cancel. Returning a live backend to the pool lets the next checkout inherit a session that still holds locks.

Closing the pool before workers stop ([WHY-171-5](../00-why.md)) fails `claim_next` and can fail the write of the task row. The document stays `processing`. [LAW-171-5](../01-first-principles.md) drains first. A forced abort may leave `idle in transaction` until the 60 s idle timer or the TCP close. That case is [EC-171-14](../06-edge-cases.md), and the test does not accept a live lease afterwards.

## Checkpoint is the cheap retry

The expensive work (extraction) is already durable when persist stalls. Retrying extraction is the wrong layer. [LAW-171-2](../01-first-principles.md) resumes. Missing checkpoint falls through to extraction ([EC-171-22](../06-edge-cases.md)). A partial upsert is safe because of `ON CONFLICT` ([EC-171-31](../06-edge-cases.md)).

Proof: [T-171-01](../08-e2e-test-matrix.md), [T-171-02](../08-e2e-test-matrix.md), [T-171-28](../08-e2e-test-matrix.md), [T-171-29](../08-e2e-test-matrix.md), [T-171-34](../08-e2e-test-matrix.md).
