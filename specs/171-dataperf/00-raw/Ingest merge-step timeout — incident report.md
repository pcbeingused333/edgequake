# Ingest merge-step timeout — incident report

Oct 21, 2026 · @steven

## Summary

Document `01a15d3e-edbe-730c-b1da-0481207f4915` (`Cv_FR_2_.pdf`, task `insert-7fb6804f-7e7c-47b1-8dee-363d0859e67b`) failed on 2026-10-21 because the ingest worker's 7200 s wall-clock expired while the task was inside the graph persist/merge step. Extraction, embedding and the resume checkpoint all completed in 26 s. The task then logged nothing for two hours. Re-uploading the same file alone succeeded, so the document is not at fault: the stall was a transient condition inside the persist step, and nothing bounds that step short of the whole-task timeout.

Three gaps turned a transient stall into a two-hour permanent failure:

- The persist/merge step (`persist_with_providers_progress_and_embedder`) has no deadline of its own.
- A task-level timeout goes straight to permanent failure and is never retried, even though a resume checkpoint had just been written.
- Nothing serializes graph merges per workspace, so documents that share entities (CVs share `FRANCE`, `PYTHON`, `MACHINE_LEARNING`) contend on the same rows.

Fix direction: give the persist step its own deadline (about 15 min, configurable), make that deadline retryable with resume from the checkpoint, and set `lock_timeout` on ingest sessions. Details and code locations are in Recommendations.

## Incident timeline

All useful work finished in 26 s; the failure is the 7194 s of silence that followed. Source: production logs from `quantalogic-ppd-edgequake`, worker 1, tenant `be1dd9bb-…`, workspace `6bd5cbe4-…`.

| Time (UTC, 2026-10-21) | Event | Log target |
| --- | --- | --- |
| 08:56:07.715 | `Permanent task failure — updating document status to 'failed'`, error `Task processing timed out after 7200 seconds`, `retry_count: 1` | `edgequake_api::processor::task_impl` |
| 08:56:07.715 | `Task timed out — marking as permanently failed`, `timeout_secs: 7200` | `edgequake_tasks::worker` |
| 06:56:33 → 08:56:07 | No log line from this task for 1 h 59 min 34 s | — |
| 06:56:33.609 | `Saved pipeline checkpoint`: 2 chunks, 126 entities, 55 relationships, 225 781 bytes, `typed_write: true` | `processor::pipeline_checkpoint` |
| 06:56:32.274 | Relationship embed, unique-before-embed: 54 unique of 55 mentions | `pipeline::helpers::embeddings` |
| 06:56:32.273 | Entity embed, unique-before-embed: 112 unique of 126 mentions | `pipeline::helpers::embeddings` |
| 06:56:32.271 | `Resilient extraction completed`: 2/2 chunks, 100 % success | `pipeline::processing` |
| 06:56:07.746 | `[PIPELINE] Processing document with workspace-specific pipeline`: `Cv_FR_2_.pdf`, extraction `mistral` / `mistral-small-latest`, embedding `mistral` | `processor::text_insert::prepare` |

The timeout fires exactly 7200.0 s after the first line, which identifies it as the worker's `processing_timeout_secs` (`edgequake-tasks/src/worker.rs:280`), not an LLM or database error. A later upload of the same PDF, alone, completed normally.

## Where the task stalled

The last line the task wrote comes from step 3 below; the first line it never wrote belongs to step 4. The stall is inside `text_insert_persist`, between the checkpoint and `P-G2 persist completed`. Paths are relative to `edgequake/crates/`.

1. **Prepare** — `edgequake-api/src/processor/text_insert/prepare.rs`. Logs `[PIPELINE] Processing document with workspace-specific pipeline` (06:56:07).
2. **Extract + embed** — `text_insert/extraction.rs`. Mistral entity extraction over 2 chunks, then entity and relationship embeddings (06:56:32).
3. **Checkpoint** — `text_insert/extraction.rs:448` calls `pipeline_checkpoint::save_pipeline_checkpoint`. Logs `Saved pipeline checkpoint` (06:56:33). This is the last line from the task.
4. **Persist** — `text_insert/persist.rs`. Releases the per-tenant fairness permit (`persist.rs:25`), sets status `indexing` (`:125`), resolves the Summary-role LLM, then calls `persist_with_providers_progress_and_embedder` (`:345`, defined in `edgequake-api/src/services/ingestion_persist.rs:280`). Inside, in order:
   1. `allocate_ingest_generation` — reads the authority document revision.
   2. Chunk rows + chunk vectors — KV and vector storage.
   3. **`KnowledgeGraphMerger`** — upserts 112 entities and 54 relationships into `eq_graph_nodes` / `eq_graph_edges`, `merge_max_async` wide; calls the Summary LLM only when an entity has 8 or more description fragments or exceeds 1200 tokens (`edgequake-pipeline/src/merger/description_merge.rs:30`).
   4. **`IngestionCommitter`** — one transaction that takes `SELECT … FOR UPDATE` on the document row and its bindings (`edgequake-storage/src/adapters/postgres/ingestion_committer.rs:267`, `:363`, `:590`), writes the relational spine and the chunk embeddings.
   5. Relational entity spine + fleet vectors (`edgequake-pipeline/src/merger/durable_fleet.rs`).

   On success logs `P-G2 persist completed` (`persist.rs:388`). Never reached.
5. **Finalize** — `text_insert/finalize.rs`. Lineage, stats, snapshot, `Document processed successfully`. Never reached.

Two lines in step 4 would have been written before the stall but carry no `document_id` field: `Storing entities with tenant_id=…` (`persist.rs:118`) and the status change to `indexing`. A search filtered on `document_id` or on a top-level `task_id` does not return them, which is why the gap looks total (see Observability gaps).

## Root cause analysis

The root cause is structural: the persist step can block indefinitely, and the only guard around it is the whole-task 7200 s timeout, which is terminal. The specific trigger was a transient contention event that a solo re-upload did not reproduce.

**Why a stall lasts two hours.** `persist_with_providers_progress_and_embedder` is awaited with no `tokio::time::timeout` and no Postgres `lock_timeout` on the session (`persist.rs:345`). The existing guards do not cover this case: `statement_timeout` is applied only on interactive read paths through `LocalTimeoutTx` (`edgequake-storage/src/adapters/postgres/statement_timeout.rs`), and the connect-time `idle_in_transaction_session_timeout` of 60 s (`connection.rs:68`) only fires on a session that is idle inside a transaction, not on one waiting for a lock or for a pool slot. The LLM request timeout (`EDGEQUAKE_LLM_TIMEOUT_SECS`, default 600 s) bounds each Summary call but a two-chunk CV rarely crosses the 8-fragment gate, so the LLM is an unlikely source here.

**Why the timeout is terminal.** `worker.rs:1054-1069` handles `Err(_elapsed)` by calling `mark_failed` and `on_permanent_failure` directly. It never consults `max_retries` or `is_permanent_ingestion_failure`, so a timeout cannot be requeued even though `save_pipeline_checkpoint` had just stored everything needed to resume without re-paying extraction. The log's `retry_count: 1` reflects the attempt counter, not a retry.

**Why contention is plausible.** The worker pool is `num_cpus × 4` tasks with a per-tenant cap of three quarters of that (`worker.rs:263-275`), and the fairness permit is dropped before persist (`persist.rs:25`), so several documents in one workspace can run their merges concurrently. The committer takes row locks with `FOR UPDATE`; the merger upserts shared entity rows. CVs share many entities (`FRANCE`, `NICE`, `PYTHON`, `MACHINE_LEARNING`), so parallel CV ingests contend on the same `eq_graph_nodes` rows.

| Hypothesis | For | Against | How to confirm next time |
| --- | --- | --- | --- |
| Postgres row-lock wait (merger upsert or committer `FOR UPDATE`) behind a long transaction | Shared entities across CVs; no `lock_timeout` on ingest sessions; solo run succeeded | A true deadlock would have been detected and raised by Postgres within `deadlock_timeout` | `pg_stat_activity` where `wait_event_type = 'Lock'`, joined to `pg_locks`; `application_name` is pinned to `edgequake:*` |
| Pool starvation: tasks hold one connection while waiting for a second | Several nested acquires in the persist path (merger, committer, typed-embedding pool); silent, Postgres cannot detect it | Pool budget is enforced at boot (`pool_budget.rs`) | sqlx pool metrics; `pg_stat_activity` count by `application_name` vs. pool size |
| Summary-LLM hang during merge | Mistral provider; no log between calls | Each call capped at 600 s; a 2-chunk CV rarely hits the 8-fragment gate | Langfuse trace for the `ingest.document` span; provider latency metrics |
| Apache AGE dual-write path still active (cutover ledger not `ready`) | Cypher writes on label tables are slower and lock-heavier than plain tables | v0.36.0 product path is plain tables; only an upgraded database can still be on AGE | `SELECT status, age_absent FROM public.eq_graph_cutover WHERE id = 1` |

The first two hypotheses fit all the evidence; the last two are worth one query each to rule out.

## Observability gaps

The logs could place the stall only to within a two-hour window because the persist step emits nothing while it works, and what it does emit is not searchable by the ids an operator has.

- **No heartbeat inside persist.** Between `Saved pipeline checkpoint` and `P-G2 persist completed` the only lines are `Storing entities with tenant_id=…` and the `indexing` status write. Merge progress goes to the WebSocket bridge (`merge_progress_cb`, `persist.rs:216`) and to a KV patch, not to the log. A stall in chunk vectors, merger, committer or fleet vectors all look identical.
- **`task_id` lives in the span, not in the fields.** Pipeline lines carry `task_id` only inside `spans[].task_process`. A log search on the top-level `task_id` returns the two worker lines and nothing else; a search on `document_id` misses lines that log it as `?document_id` (`Some("…")`) or not at all.
- **The failure message hides the phase.** `Task processing timed out after 7200 seconds` says nothing about where the task was. The task struct already tracks a progress stage (`bump_task_progress(task, "extraction", 4, 60)` at `persist.rs:107`); the timeout handler does not include it.
- **`retry_count: 1` reads as a retry.** The permanent-failure line prints the attempt counter; combined with `Processing failed permanently after 1 attempts`, operators read it as "retried once", which did not happen.
- **No database-side snapshot at timeout.** When the deadline fires, nothing queries `pg_stat_activity` for the sessions tagged `edgequake:*`, so the one moment the lock graph was known is lost.

## Recommendations for the maintainer

R1 to R3 together turn this failure mode from "2 h, terminal, silent" into "about 15 min, retried from the checkpoint, with the blocking session named in the log". R4 removes the most likely trigger. R5 to R7 are small and independent.

| # | Change | Where | Effort | Effect |
| --- | --- | --- | --- | --- |
| R1 | Persist-step deadline, paired with DB-side cancellation | `processor/text_insert/persist.rs:345`; ingest session baseline | S | Stall fails in minutes, not hours |
| R2 | Treat a timeout as retryable while retries remain | `edgequake-tasks/src/worker.rs:1054` | S | Resume from checkpoint instead of permanent failure |
| R3 | `lock_timeout` on ingest sessions and inside the committer transaction | `connection.rs:apply_session_baseline`; `ingestion_committer.rs` | S | Lock waits surface as a named Postgres error |
| R4 | Serialize the graph merge per workspace with a transaction advisory lock | merger entry in `ingestion_persist.rs` or committer | M | Removes cross-document row contention |
| R5 | Phase heartbeat logs with top-level `document_id` and `task_id` | `persist.rs`, `ingestion_persist.rs` | S | Stall is placed to one sub-step |
| R6 | On timeout, log `pg_stat_activity` for `edgequake:ingest` sessions | worker timeout branch, `task_impl.rs:on_permanent_failure` | S | Lock graph captured at the one moment it is known |
| R7 | Include the last progress stage in the timeout message; print `attempt N of M` | `worker.rs:1055`, `task_impl.rs` | XS | Operators stop reading `retry_count: 1` as a retry |

**R1 — Persist-step deadline.** Wrap the call at `persist.rs:345` in `tokio::time::timeout`, with the budget from a new `EDGEQUAKE_PERSIST_TIMEOUT_SECS` (default 900, clamped below `processing_timeout_secs`). The repository already records why a Rust-side timeout alone is not enough: `statement_timeout.rs` notes that `tokio::time::timeout` abandons the future while Postgres keeps executing (GH-336). So R1 must ship with R3, so that the server cancels the same work the client stopped waiting for. Map the elapsed error to a `TaskError::Process` whose message starts with a stable prefix (`persist deadline exceeded`) that the retry classifier treats as transient.

```rust
let budget = persist_timeout_from_env();
let persisted = tokio::time::timeout(budget, persist_with_providers_progress_and_embedder(/* … */))
    .await
    .map_err(|_| TaskError::Process(format!("persist deadline exceeded after {}s (stage={})", budget.as_secs(), task.progress_stage())))?;
```

**R2 — Retryable timeout.** In `worker.rs:1054` the `Err(_elapsed)` arm bypasses the retry decision taken in the `Err(e)` arm above it. Route it through the same logic: if `task.retry_count < task.max_retries` and the error is not classified permanent, requeue with backoff; otherwise permanent. On the retry, `extraction.rs:81` already loads the checkpoint, so the second attempt skips the Mistral extraction and goes straight to persist. Guard against a hot loop by making the retry wait at least the backoff and by counting the timeout as an attempt.

**R3 — Lock timeouts.** Add `lock_timeout` to `apply_session_baseline` for the `edgequake:ingest` label (new `EDGEQUAKE_DB_INGEST_LOCK_TIMEOUT_SECS`, default 30, same clamp style as the idle-in-transaction setting at `connection.rs:88`). Inside the committer transaction, add `SET LOCAL lock_timeout` before the `FOR UPDATE` selects at `ingestion_committer.rs:267`, `:363`, `:590`. A `lock_not_available` (SQLSTATE 55P03) error is transient: classify it as retryable so R2 picks it up. The DDL path already does this with `EDGEQUAKE_GRAPH_DDL_LOCK_TIMEOUT` (`graph/helpers/session.rs:141`); this extends the same pattern to DML.

**R4 — Per-workspace merge serialization.** Take `pg_advisory_xact_lock(hashtext('merge:' || workspace_id))` at the start of the merge transaction, under the R3 `lock_timeout`. An advisory lock works across API instances (`pool_instance_count_from_env` implies more than one), costs nothing when uncontended, and makes two CVs in the same workspace merge one after the other instead of interleaving upserts on the same `eq_graph_nodes` rows. If the merger does not run in one transaction today, an in-process `tokio::sync::Mutex` keyed by workspace in `DocumentTaskProcessor` is a weaker single-instance fallback. Either way keep extraction concurrent; only the merge serializes.

**R5 — Heartbeat logs.** One `info!` at the start of each sub-step of `persist_with_providers_progress_and_embedder` (generation, chunk vectors, merger, committer, fleet vectors) and one at merger phase transitions, each with `document_id` and `task_id` as top-level fields. Keep the per-entity progress on the WebSocket path; the log needs phases, not counts.

**R6 — Timeout diagnostics.** In the timeout branch, before `on_permanent_failure`, run a best-effort query and log the rows:

```sql
SELECT pid, application_name, state, wait_event_type, wait_event,
       now() - xact_start AS xact_age, left(query, 120) AS query
FROM pg_stat_activity
WHERE application_name LIKE 'edgequake:%'
  AND (wait_event_type = 'Lock' OR now() - xact_start > interval '5 minutes');
```

Bound it with its own 5 s `statement_timeout` and never let it fail the handler.

**R7 — Message hygiene.** Append the last `bump_task_progress` stage to the timeout message (`… timed out after 7200 seconds in stage indexing`) and change `Processing failed permanently after {n} attempts` to `attempt {n} of {max}`.

**Not recommended:** raising `processing_timeout_secs`. The floor is already 7200 s and scales with document size (`large_document_profile.rs:31`); a larger value would only lengthen the silence.

## Verification plan

Reproduce the contention first, then prove each change against it. The reproduction needs no Mistral quota: the mock provider and a held row lock are enough.

1. **Reproduce.** On a dev stack (`make dev-bg`), upload 8 to 12 CVs into one workspace within a few seconds. Watch `pg_stat_activity` for `edgequake:ingest` sessions with `wait_event_type = 'Lock'`. If nothing waits, force it: open a `psql` transaction, `SELECT * FROM public.documents WHERE id = '<doc>' FOR UPDATE`, leave it open, and upload that document again. Today the task sits in `indexing` until 7200 s.
2. **Prove R1 + R3.** With the held lock, the task must fail within `EDGEQUAKE_PERSIST_TIMEOUT_SECS` (or the `lock_timeout`, whichever is first), the error must name the stage, and `pg_stat_activity` must show no leftover `edgequake:ingest` session in `active` state after the failure (the GH-336 zombie check).
3. **Prove R2.** Release the lock before the retry fires. The second attempt must log a checkpoint resume (`resumed = true` in `extraction.rs:483`), make no extraction LLM call, and end in `Document processed successfully`.
4. **Prove R4.** Re-run step 1 with the advisory lock in place: merges for the same workspace must not overlap in `pg_stat_activity`, and total wall-clock for the batch must stay within the sum of the solo runs plus a small margin.
5. **Prove R6 + R7.** Trigger the timeout once more with the held lock and confirm the log carries the blocking pid, the waiting pid, and the stage.

Tests to add, each small enough for `cargo test -p <crate> --lib`:

- [ ] `edgequake-tasks`: a timeout on a task with `retry_count < max_retries` is requeued, not sent to `on_permanent_failure`; a timeout at `max_retries` still is.
- [ ] `edgequake-api` (postgres feature): persist against a row held `FOR UPDATE` returns `persist deadline exceeded` within the configured budget and releases its pool connection.
- [ ] `edgequake-storage`: the committer transaction applies `SET LOCAL lock_timeout` and surfaces SQLSTATE 55P03 as a transient error.
- [ ] `edgequake-storage`: two merges for the same workspace serialize on the advisory lock; merges for different workspaces do not.
- [ ] Retry classifier: `persist deadline exceeded` and `lock_not_available` are transient; `CRITICAL: Cannot obtain workspace vector storage` stays permanent.
- [ ] Log contract: every `info!` in the persist path carries `document_id` and `task_id` as top-level fields (extend the existing log-shape tests if present).

Release gates stay as they are: `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --lib`, and a `make bench` smoke on the medical-mid pack to confirm the advisory lock adds no measurable ingest latency.

## Appendix

**Environment.** EdgeQuake v0.36.0 (schema 214), deployment `quantalogic-ppd-edgequake`, worker 1. Tenant `be1dd9bb-c64b-454f-a98c-ab1539bed6cc`, workspace `6bd5cbe4-7fa9-44a0-92ec-7d9f0238e8f4`. Extraction and embedding provider `mistral`, model `mistral-small-latest`. Graph backend expected to be the plain `eq_graph_*` tables; confirm with the ledger query below.

**Raw log lines (fields only).**

```
2026-10-21T06:56:07.746Z INFO  edgequake_api::processor::text_insert::prepare
  [PIPELINE] Processing document with workspace-specific pipeline
  document_id=01a15d3e-edbe-730c-b1da-0481207f4915 file_source=Cv_FR_2_.pdf
  extraction_provider=mistral extraction_model=mistral-small-latest embedding_provider=mistral

2026-10-21T06:56:32.271Z INFO  edgequake_pipeline::pipeline::processing
  Resilient extraction completed total_chunks=2 successful=2 failed=0 success_rate=100.0%

2026-10-21T06:56:33.609Z INFO  edgequake_api::processor::pipeline_checkpoint
  Saved pipeline checkpoint (extraction result persisted for resume; embeddings omitted)
  chunks=2 entities=126 relationships=55 embeddings_stripped=168 checkpoint_bytes=225781
  kv_write=false typed_write=true

2026-10-21T08:56:07.715Z ERROR edgequake_tasks::worker
  Task timed out — marking as permanently failed
  worker_id=1 task_id=insert-7fb6804f-7e7c-47b1-8dee-363d0859e67b timeout_secs=7200

2026-10-21T08:56:07.715Z ERROR edgequake_api::processor::task_impl
  Permanent task failure — updating document status to 'failed'
  retry_count=1 circuit_breaker_tripped=false
  error.message="Task processing timed out after 7200 seconds"
```

**Queries for the next occurrence.**

```sql
-- Graph backend in use (expect: ready, true on a v0.36.0 install)
SELECT status, age_absent FROM public.eq_graph_cutover WHERE id = 1;

-- While a task sits in 'indexing': who waits on whom
SELECT a.pid, a.application_name, a.wait_event_type, a.wait_event,
       now() - a.xact_start AS xact_age, pg_blocking_pids(a.pid) AS blocked_by,
       left(a.query, 120) AS query
FROM pg_stat_activity a
WHERE a.application_name LIKE 'edgequake:%'
  AND (a.wait_event_type = 'Lock' OR now() - a.xact_start > interval '2 minutes');
```

**Log search that covers the gap.** Filter on `worker_id = 1` and the window `06:56:33` to `08:56:07` with no `document_id` or `task_id` filter; pipeline lines carry `task_id` only inside `spans[].task_process`.

**Code anchors (edgequake/crates/).** `edgequake-tasks/src/worker.rs:280` default timeout, `:908` timeout wrapper, `:1054` timeout branch · `edgequake-api/src/processor/text_insert/extraction.rs:81` checkpoint load, `:448` checkpoint save · `text_insert/persist.rs:25` fairness release, `:345` persist call, `:388` success log · `edgequake-api/src/services/ingestion_persist.rs:280` persist entry · `edgequake-storage/src/adapters/postgres/statement_timeout.rs` GH-336 note · `connection.rs:68` idle-in-transaction default · `ingestion_committer.rs:267,363,590` row locks · `edgequake-pipeline/src/merger/description_merge.rs:30` LLM summary gate.
