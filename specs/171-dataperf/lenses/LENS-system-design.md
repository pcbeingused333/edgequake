# Lens — System design

Parent: [README](../README.md) · WHY: [00](../00-why.md) · Plan: [07](../07-implementation-plan.md)

## Single writer, concurrent readers of other work

Extraction is independent per chunk and stays parallel. The shared mutable state is the workspace graph. [LAW-171-3](../01-first-principles.md) puts one writer on that state and leaves everyone else alone. An advisory transaction lock is the multi-replica form of that rule. A process-local mutex is not, because `pool_instance_count` is already a real deployment ([EC-171-13](../06-edge-cases.md)).

Fairness stays dropped so other tenants are not stuck behind one workspace’s lock ([EC-171-29](../06-edge-cases.md), [F-171-03](../03-code-facts.md)). The lock is per workspace. It is not a global ingest lock.

## Backpressure is a timeout, not a bigger queue

A waiter with no `lock_timeout` consumes a worker and a pool slot until the outer clock. That is unbounded queueing inside the database. `lock_timeout` of 30 s turns the wait into a failure the retry budget can absorb. The persist budget of 15 minutes bounds work that is running, not waiting. The 7200 s clock stays the last resort for extraction and conversion ([F-171-18](../03-code-facts.md)).

## Shutdown is a state machine

The safe order is stop claiming, cancel in flight, wait the drain, close pools, drop the runtime. The current order closes pools while claimers are alive ([WHY-171-5](../00-why.md), [LAW-171-5](../01-first-principles.md)). `Pool::close` then waits on any checked-out connection, so a stuck persist also sticks the shutdown, and Kubernetes kills the pod with the task still `processing`.

W5 makes `PoolClosed` a terminal signal for the worker loop, not a poll error. One warning, then stop ([EC-171-21](../06-edge-cases.md), [EC-171-32](../06-edge-cases.md)).

## What this lens will not trade

Do not serialize extraction to make persist look simpler. Do not hold the merge lock across the LLM to avoid the optimistic re-read. Both of those move the bottleneck onto the expensive, independent work.
