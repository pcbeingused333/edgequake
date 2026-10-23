# Lens — Database

Parent: [README](../README.md) · Theory: [04](../04-postgres-theory.md) · Architecture: [05](../05-architecture.md)

## The write is the critical section

Entity rows are `INSERT … ON CONFLICT DO UPDATE`. That locks the existing tuple. Documents in one workspace share those tuples. The committer also locks the document, its revision, and its bindings with `FOR UPDATE` ([F-171-10](../03-code-facts.md), [F-171-11](../03-code-facts.md)).

There is no transaction around the whole merge, so there is no place a lock can live for the whole merge today. W4 adds one write transaction for the fast path and a short read transaction plus a short write transaction for the slow path. The summary LLM is not inside either transaction ([LAW-171-3](../01-first-principles.md), [EC-171-05](../06-edge-cases.md)).

## Idempotent retry

[LAW-171-2](../01-first-principles.md) retries persist. That is safe only because upserts conflict on the entity key and the committer replaces the spine. A retry must not insert a second copy ([EC-171-10](../06-edge-cases.md), [EC-171-31](../06-edge-cases.md)). A deleted document still loses to `DocumentDeleted` ([EC-171-24](../06-edge-cases.md)).

## Do not copy the community lock

`pg_try_advisory_lock` plus a stored connection pins a pool slot ([F-171-12](../03-code-facts.md)). Merge uses `pg_advisory_xact_lock(hashtext('edgequake.merge'), hashtext(workspace_id))`, which ends at commit. Two keys, same style as community, not a new style ([LAW-171-6](../01-first-principles.md), [EC-171-04](../06-edge-cases.md)).

## Classifier

`PersistContention` is matched before `GraphMerge`. The permanent class is unchanged ([EC-171-08](../06-edge-cases.md)). SQLSTATEs `55P03`, `40P01`, and `57014` map to that transient class in the storage error string, so the worker never imports a database driver.

Tests: [T-171-07](../08-e2e-test-matrix.md), [T-171-19](../08-e2e-test-matrix.md), [T-171-26](../08-e2e-test-matrix.md) through [T-171-33](../08-e2e-test-matrix.md).
