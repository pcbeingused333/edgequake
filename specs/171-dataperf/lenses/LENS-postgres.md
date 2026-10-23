# Lens — Postgres

Parent: [README](../README.md) · Theory: [04](../04-postgres-theory.md) · Official: [Client Connection Defaults](https://www.postgresql.org/docs/current/runtime-config-client.html)

## The timer that was set is the wrong timer

Ingest sessions set `idle_in_transaction_session_timeout` to 60 s ([F-171-07](../03-code-facts.md)). That GUC aborts a session that is idle inside a transaction. Incident A’s session, if it was waiting on a lock, was `active` with `wait_event_type = Lock`. The timer does not apply. [LAW-171-1](../01-first-principles.md).

## The set this spec adds

| GUC | Value | Why this value |
|-----|-------|----------------|
| `lock_timeout` | 30 s | Shorter than any statement budget, so it can win. Manual: equal or larger is pointless. [EC-171-02](../06-edge-cases.md). |
| `statement_timeout` | persist budget minus 1 s | One statement cannot outlive the budget. |
| `transaction_timeout` | persist budget, PostgreSQL 17+ only | Bounds the sum of short statements. Must be strictly greater than `statement_timeout` and greater than the 60 s idle timer, or PostgreSQL ignores the longer one. [EC-171-01](../06-edge-cases.md). |

`SET LOCAL` for the write transaction, and the session pin for autocommit upserts. Both go through one literal helper ([LAW-171-6](../01-first-principles.md)). `RESET ALL` on release means the pin has to be in the baseline or the second checkout is unprotected ([EC-171-03](../06-edge-cases.md), [EC-171-27](../06-edge-cases.md)).

`0` disables these timers in the server. Ingest rejects `0` ([EC-171-39](../06-edge-cases.md)). Do not put any of them in `postgresql.conf`.

## Advisory lock

```sql
SELECT pg_advisory_xact_lock(hashtext('edgequake.merge'), hashtext($1));
```

Transaction scoped, so it cannot leak past commit or rollback ([advisory locks](https://www.postgresql.org/docs/current/functions-admin.html#FUNCTIONS-ADVISORY-LOCKS)). Two `hashtext` arguments, matching the community lock’s collision budget, without that path’s held connection ([F-171-12](../03-code-facts.md)).

## How to see the next one

`application_name` is already `edgequake:ingest`. The snapshot in [02](../02-incident-catalogue.md) joins `pg_stat_activity` to `pg_blocking_pids`. Codes `55P03`, `40P01`, and `57014` are in [Appendix A](https://www.postgresql.org/docs/current/errcodes-appendix.html). W6 runs that snapshot with a 5 s `statement_timeout` and ignores its own failure ([EC-171-19](../06-edge-cases.md), [EC-171-23](../06-edge-cases.md)).

PostgreSQL 16 is still a product image ([version matrix](../../../docs/data-layer/version-matrix.md)). `transaction_timeout` is not assumed there. It arrived in [PostgreSQL 17](https://www.postgresql.org/docs/17/release-17.html).
