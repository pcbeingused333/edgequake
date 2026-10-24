# 06 — Graph, query, and the model

Parent: [README](README.md) · Prev: [05-policy-model](05-policy-model.md) ·
Next: [07-data-and-db-contract](07-data-and-db-contract.md)

[LAW-174-9](01-first-principles.md) and [LAW-174-16](01-first-principles.md)
are the laws for this chapter. The code they close is
[F-174-03](04-findings.md), [F-174-04](04-findings.md),
[F-174-05](04-findings.md), [F-174-14](04-findings.md).

## One list, five modes

`execute_query`, chat completion, and chat streaming already thread
`allowed_document_ids` into naive, local, global, hybrid, and mix
([02-surfaces](02-surfaces.md)). When enforcement is on, the handler sets
that list from `VisibilityScope` before `resolve_document_filter` runs.
The client filter is intersected afterwards. `None` is no longer all-pass
on an enforcing workspace.

Chunk ANN and chunk FTS already accept `document_id = ANY`. Entity and
relationship ANN already overlap `source_chunk_ids` with chunk keys of
those documents. W5 fills the list. It does not add a sixth retrieval mode.

Post-filter after a global top-k is not the control. Hidden neighbors
would occupy the k slots and the latency would depend on how many hidden
hits were discarded ([16-performance](16-performance.md)). The SQL
predicate runs inside the scan.

## Contributions

Today the merger writes one description. Under enforcement the ingest
worker writes a contribution and does not ask the LLM to summarize across
documents.

```text
  chunk of document D
           |
           v
  +--------------------------------------+
  | graph_contributions                  |
  | entity or edge key                   |
  | document_id, chunk_key, fragment     |
  | rank + compartments copied from D    |
  +------------------+-------------------+
                     |
                     |  subject dominance (same allows)
                     v
  +--------------------------------------+
  | served node                          |
  | name, type                           |
  | description = visible fragments      |
  | degree = visible edges               |
  +--------------------------------------+
```

Rules for the served node:

- The node is absent when every contribution is hidden.
- The name and entity type come from the node shell. They are visible
  when any contribution is visible. A name that itself is the secret
  ("Project Atlas codename") is still a name; stewards who need the name
  hidden must not extract that entity from a hidden document into a shared
  node. The mitigation in this pack is: contributions are the only text.
  The name is the entity key the pipeline already publishes inside the
  workspace. Document this residual in the threat model
  ([11-threat-model](11-threat-model.md)): a shared canonical name can
  confirm that an entity exists. Fragments and edges cannot confirm a
  hidden source. W6 does not invent a second private node id per label.
- Description shown to the user is the concatenation of visible fragments,
  in `(document_id, chunk_key)` order, truncated by the existing context
  budget. No cross-document summary while enforcement is on.
- An edge is served when at least one of its contributions is visible.
  Edge text follows the same fragment rule.
- `degree` in the API is the count of served edges, not `eq_graph_nodes.degree`.
- BFS (`pg_bfs_expand`) joins contributions. It does not load the workspace
  and drop rows in Rust.
- `eq_graph_search_nodes` gains the same join, or a sibling function that
  searches fragment text the subject may see. Searching the merged
  description column is forbidden while enforcement is on.
- When enforcement is off, the current merger and the current degree stay.
  Acc does not pay for the join.
- A node that has no contribution rows yet keeps the merged description
  only when every `source_document_id` is visible. Otherwise the
  description is omitted. Enable does not split that string
  ([RB-174-03](17-roadblocks.md), [EC-174-42](12-edge-cases.md)).
- Search while enforcement is on does not call `eq_graph_search_nodes`
  unchanged. That function is security definer ([RB-174-04](17-roadblocks.md)).

Ingest uses a system writer. It stores every fragment, including
`unverified`. It does not run `allows` as the uploading user, or a
restricted user would persist only their own view and destroy the other
fragment.

## Citations and prompts

`source_reference_builder` and the context bundle mapper copy chunk text
into the citation. Those chunks already passed `filter_context_by_document_ids`.
W5 makes that filter the visible set, so the citation cannot name a hidden
document.

The prompt built in `query_pipeline.rs` is the only text the answer cache
hashes ([F-174-08](04-findings.md)). Hidden fragments must be absent before
that hash. The system prompt must not add a line such as "some documents
were omitted". That line is an existence leak ([LAW-174-8](01-first-principles.md)).

SPEC-126 provider prompt cache: any prefix that includes retrieved text
includes the clearance fingerprint (a hash of rank, compartment ids, deny
bits, and `policy_epoch`). A prefix that is only the static instruction
may stay shared.

Glossary injections are retrieved text. They pass the same contribution
or document predicate ([EC-174-38](12-edge-cases.md)).

## Graph picture

`GET /api/v1/graph`, node search, popular nodes, label facets, and
`eq_graph_image` all take the served neighborhood. The PNG is drawn from
that neighborhood, not from a second unfiltered layout. A node the analyst
cannot see is not drawn, not greyed, and not counted in a "hidden" badge.

Label search returns entity labels and compartment names the subject holds.
It does not return the workspace vocabulary of compartments
([EC-174-39](12-edge-cases.md)). Stewards see the vocabulary in the admin
screen, which is a different route.

## Query contract under enforcement

| Input | Result |
|-------|--------|
| No `document_filter` | Visible set |
| `document_ids` subset of visible | That subset |
| `document_ids` naming a hidden id | Id dropped. If none remain, empty result, not 404 on the query itself |
| `document_ids` the client hopes will widen | Intersection only |
| Pattern or date | Resolved ids intersected with the visible set |
| Mode mix / hybrid / local / global / naive | Same id list |

A direct document route that names one hidden id is 404
([08-api-contract](08-api-contract.md)). A query that merely fails to
match stays 200 with an empty context. Those are different oracles: the
query never confirms that the id exists.

Next: [07-data-and-db-contract](07-data-and-db-contract.md).
