# 16 — Performance

Parent: [README](README.md) · Prev: [15-cross-ref](15-cross-ref.md) ·
Lenses: [database](lenses/LENS-database.md) · [graph](lenses/LENS-graph-algo.md)

Correctness tests are in [14-e2e-test-matrix](14-e2e-test-matrix.md). This
chapter is the cost model and the one plan check.

## What must stay cheap

- Compile the subject once per request. Rule rows for a workspace are a
  small set (Databricks: few policies). Cache the compiled subject on the
  request, not in a process-global map keyed only by user id (a role edit
  would be stale). `policy_epoch` can key a short process cache; W3 may
  skip that cache until a measurement says compile shows up.
- Do not expand "almost everyone" into a 100k-id array. Threshold: if the
  visible document count is ≤ 500, pass `document_id = ANY($1)`. Above
  that, filter with `clearance_covers` on the stored rank and compartments
  inside the chunk scan. 500 is the starting constant in the query
  service, not a GUCs knob, until a measurement moves it.
- Relabel copies columns. It does not recompute embeddings.
- Graph BFS joins `graph_contributions` for the workspace and the covering
  predicate. It does not `SELECT` every node into Rust and drop.
- Enforcement off does not join contributions and does not set iterative
  scan. Acc cost stays the current cost ([EC-174-35](12-edge-cases.md)).

## Indexes

From [07-data-and-db-contract](07-data-and-db-contract.md):

- B-tree `(workspace_id, classification_rank)` on `documents` and `chunks`.
- GIN on `compartment_ids` so `<@` is an indexable containment, not a
  sequential filter of the workspace.
- Contributions: `(workspace_id, entity_name)` and `(workspace_id, document_id)`.

RLS functions are `LEAKPROOF` and invoked as `(SELECT current_clearance_rank())`
so the planner treats them as an InitPlan (migration 208). A bare volatile
call would be re-evaluated per row and would also block index use, which
is the failure migration 206 fixed for `jsonb ->>`.

## Vector scans

pgvector applies a `WHERE` after an approximate index scan unless
iterative scan is on. With the default `hnsw.ef_search` of 40, a predicate
that matches 10% of rows returns about 4 rows. That both hurts recall and
invites a post-filter that still touched hidden vectors.

For the large-visible-set statement only:

```text
  SET LOCAL hnsw.iterative_scan = relaxed_order
```

Do not set it in `postgresql.conf` and do not set it for enforcement-off
queries. Reference: <https://github.com/pgvector/pgvector#filtering> and
the 0.8 iterative scan notes in that document.

`hnsw.max_scan_tuples` stays at the server default (20000) unless W10's
EXPLAIN shows the scan stopping early on a fixture where the visible set
is large and selective. Raising it is a measured change, not part of W5.

Partial HNSW indexes per classification value are optional later, and only
if the four ranks are stable. Do not build one partial index per user or
per compartment combination.

## EXPLAIN gate (W10)

Fixture: one workspace, 2_000 chunk rows, half rank internal with no
compartments, half confidential with `department:hr`. Subject rank is
internal with no HR compartment. Statement is the chunk ANN query used by
naive mode.

Pass when the plan for the tenant role shows the clearance predicate as
an index condition or as a filter on the chunk index scan, and does not
show a sequential scan of `chunk_embeddings` alone. Record the plan text
under `specs/174-document-security/measurements/` when W10 runs. This pack
does not add that directory yet.

Fail when the plan filters only after a limit that ignores rank (the
post-filter bug in [F-174-14](04-findings.md)).

## Graph cost

Visible degree is `COUNT(*)` of served edges, not a stored global counter
that every reader shares. A hot node with many hidden edges must not force
a full edge load: the count query applies `clearance_covers` on
contribution rows and uses the `(workspace_id, entity_name)` index.

Search uses fragment text the predicate allows. The owner function
`eq_graph_search_nodes` (migration 206) stays for enforcement off, because
it is `SECURITY DEFINER` and applies tenant itself. The enforcement-on
sibling must apply clearance inside the function too, or the owner bypass
would skip `clearance_guard`. That is a W6 requirement, not a follow-up.

## Caches

Epoch on the key means a relabel invalidates context and retrieval
entries for that workspace. That is a correctness cost. Answer cache stays
prompt-hashed, so two HR analysts still share an answer when the prompt
matches ([EC-174-26](12-edge-cases.md)). Keyword cache stays shared
([F-174-08](04-findings.md)).

Do not add the raw document id list to the answer-cache key. The prompt
hash already changes when the text changes, and a long id list would only
split entries that are already safe.

Numbered chapters end here. Lenses are linked from the [README](README.md).
Back to [15-cross-ref](15-cross-ref.md).
