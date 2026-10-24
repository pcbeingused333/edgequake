# Lens — Graph Algorithms

Parent: [README](../README.md) · Projection: [06-graph-query-ai](../06-graph-query-ai.md)

## What this lens owns

[LAW-174-9](../01-first-principles.md). BFS, search, degree, and the PNG
run on the visible projection. They do not run on the full workspace and
delete vertices afterwards.

## Why the merged node is the wrong structure

`entities` is unique on `(tenant_id, workspace_id, name)`.
`merger/entity.rs` keeps one description. A traversal that starts at a
visible node and walks `eq_graph_edges` will enter a hidden neighbor
whenever the edge row exists. Degree stored on the node counts those
edges. Any-overlap lineage keeps the node and still serves the mixed
sentence ([F-174-04](../04-findings.md), [F-174-05](../04-findings.md)).

## Algorithm under enforcement

```text
  contributions covered by clearance_covers
           |
           v
  vertices = names with at least one covered fragment
  edges    = relations with at least one covered fragment
  degree   = count of those edges
  text     = covered fragments, ordered by (document_id, chunk_key)
           |
           v
  BFS and search and PNG use only that graph
```

BFS joins `graph_contributions` and stops at edges the predicate rejects.
It does not materialize the hidden neighbor and then drop it: the neighbor
id would still have been read, and a limit-k walk would spend steps on
hidden edges.

Search matches fragment text, not `properties.description`. The security
definer function used today (`eq_graph_search_nodes`) must not be called
unchanged while enforcement is on, because it bypasses table policies.
See the database lens.

Popular nodes sort by visible degree. A node that is popular only because
of hidden edges falls in rank for this subject ([EC-174-19](../12-edge-cases.md)).

## What stays global

The canonical name. If both corpora extracted `PERSON_A`, both analysts
who can see at least one fragment see the name. This pack does not build
a private node per label. Workspaces remain the way to split a name that
is itself secret ([11-threat-model](../11-threat-model.md)).

Enforcement off keeps the current merger, the current BFS, and the current
degree, so Acc does not pay for the join ([EC-174-35](../12-edge-cases.md)).

A description written before contribution rows exist cannot be split.
Serve it only when every source document is visible
([RB-174-03](../17-roadblocks.md), [EC-174-42](../12-edge-cases.md)).
`eq_graph_search_nodes` stays on the enforcement-off path because it is
security definer ([RB-174-04](../17-roadblocks.md)).

## Ingest

Write one contribution per source chunk that produced the entity or the
edge. Do not call the cross-document description summarizer while
enforcement is on. The ingest worker is a system writer and stores
fragments the uploader could not read, so the other department's text
is not discarded.
