---
title: "Knowledge graph catalog"
description: "The serving graph is eq_graph_nodes and eq_graph_edges. This page keeps the pre-cutover Ref ID catalog."
---

# Knowledge graph

The serving graph is `public.eq_graph_nodes` and `public.eq_graph_edges` (migration `170_spec164_plain_graph.sql`). A ready row in `public.eq_graph_cutover` means reads and writes use those tables. Product images do not install a graph extension. Tables, indexes, and tenant isolation are in [postgres.md](./postgres.md) and [indexes.md](./indexes.md). The overview is in [README.md](./README.md).

Entity names are `node_id` values. The primary key is `(namespace, node_id)` on nodes and `(namespace, source_id, target_id, rel_type)` on edges. `tenant_id` and `workspace_id` are columns. `properties` holds `source_ids` and `source_chunk_ids`.

The sections below the serving-graph summary are the pre-cutover catalog. Ref IDs in that catalog do not change. When `eq_graph_cutover.status` is `ready`, the same entry points read and write the plain tables.

## Pre-cutover layout

Before migration 170 finished cutover, one label graph existed per namespace. The namespace `default` mapped to the name `eq_eq_default_graph` through `age_graph_name_for_namespace` in `edgequake/crates/edgequake-storage/src/namespace_tables.rs`. Migration 013 defined a helper and did not create that graph. The storage adapter created it at runtime from `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/helpers/graph_lifecycle.rs`. That path runs only while the ledger status is still `age`.

The first diagram is the graph itself: every `Node` can point to any other `Node` through an `EDGE`. The second diagram lists the properties on each label and how they mirror the relational read models.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart LR
    A["Node (entity)"] -->|"EDGE (relationship)"| B["Node (entity)"]
    B -->|"EDGE (relationship)"| C["Node (entity)"]
    A -->|"EDGE (relationship)"| C
```

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
erDiagram
    AGE_Node ||--o{ AGE_EDGE : "out"
    AGE_EDGE }o--|| AGE_Node : "in"
    AGE_Node ||--o| entities : "mirrored as"
    AGE_EDGE ||--o| relationships : "mirrored as"
    chunks ||--o{ chunk_entity_links : "mentions"
    entities ||--o{ chunk_entity_links : "found in"
    chunks ||--o{ chunk_relation_links : "evidence"
    relationships ||--o{ chunk_relation_links : "supported by"
    AGE_Node {
        text node_id PK
        text entity_type
        text description
        uuid tenant_id
        uuid workspace_id
        text_array source_chunk_ids
    }
    AGE_EDGE {
        text source_id FK
        text target_id FK
        text relation_type
        float weight
        uuid tenant_id
        uuid workspace_id
        text_array source_chunk_ids
    }
    entities {
        uuid id PK
        text name
        uuid workspace_id FK
    }
    relationships {
        uuid id PK
        uuid source_id
        uuid target_id
        uuid workspace_id FK
    }
    chunks {
        uuid id PK
        uuid document_id
    }
    chunk_entity_links {
        uuid chunk_id PK
        text entity_name PK
        uuid workspace_id PK
    }
    chunk_relation_links {
        uuid chunk_id PK
        text source_entity PK
        text target_entity PK
        uuid workspace_id PK
    }
```

Read the second diagram as the old label layout. `entities` and `relationships` are still the searchable copies. The link tables record which chunk produced which fact. The live graph tables are `eq_graph_nodes` and `eq_graph_edges`. Full relational diagrams are in [schema-er.md](./schema-er.md).

### Node properties (label `Node`)

| Property | Meaning |
|---|---|
| `node_id` | The entity name in upper case with underscores, for example `SARAH_CHEN`. Unique. |
| `entity_type` | Type such as `PERSON` or `ORGANIZATION`. |
| `description` | Merged description text. |
| `importance`, `sources`, `label`, `display_name` | Ranking and display data. |
| `tenant_id`, `workspace_id` | Who owns the node. |
| `source_ids`, `source_chunk_ids` | Which chunks and documents produced it. |
| `page_num`, `figure_index`, `asset_id`, `mm_subtype` | Set only for multimodal figure nodes. |

### Edge properties (label `EDGE`)

| Property | Meaning |
|---|---|
| `source_id`, `target_id` | `node_id` of each end. |
| `relation_type` | Kind of link, for example `WORKS_AT`. |
| `description`, `weight`, `keywords` | Content and strength. |
| `source_chunk_ids`, `source_chunk_id` | Evidence chunks. |
| `tenant_id`, `workspace_id` | Who owns the edge. |

On the label tables, each label lived in its own table inside a schema named after the graph (`Node` and `EDGE`). The extra columns `eq_node_id`, `eq_source_id`, `eq_target_id`, and `eq_rel_type` copied the properties above, maintained by triggers (`edgequake/migrations/support/092/apply.sql`). The function `eq_merge_graph_properties` (migration 090) still merges property maps on `eq_graph_nodes`.

The same entities and relationships also exist as plain rows in `entities` and `relationships`. See [postgres.md](./postgres.md). Traversal reads `eq_graph_nodes` and `eq_graph_edges`. The copy tables serve search and counts.

## Pre-cutover session setup

While the ledger status is `age`, a connection that touches the label graph runs `LOAD 'age'` and sets `search_path = ag_catalog, "$user", public` (`edgequake/crates/edgequake-storage/src/adapters/postgres/graph/helpers/session.rs`). It also sets a `statement_timeout`. DDL sessions set `statement_timeout = 0` and use a short `lock_timeout` instead. A ready ledger does not open that session.

There are two ways to write and read:

| Path | Used for | Notes |
|---|---|---|
| Native SQL | Batch node and edge upserts, deletes, BFS expansion, neighbor lists | Default. Plain `INSERT ... ON CONFLICT (eq_node_id)` with `unnest` batches. Fast and predictable. |
| Cypher | Fallback writes, a few scoped deletes (`DETACH DELETE`), ad-hoc queries | Sent as `SELECT ... FROM cypher('graph', $tag$ ... $tag$, $1) AS (...)`. |

`EDGEQUAKE_NATIVE_GRAPH_WRITES` set to `0`, `false`, `off`, or `no` forces the label-table `MERGE` path. Leave it unset. A ready ledger writes `eq_graph_nodes` and does not take that path.

The diagram shows the read path for a graph query.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart TD
    Q["Graph query"] --> P{"Native path?"}
    P -->|"yes"| N["SQL on the label tables"]
    P -->|"no"| C["Cypher via cypher()"]
    N --> I["Btree and GIN indexes"]
    C --> I
    I --> R["Rows with agtype values"]
    R --> T["Tenant and workspace filter"]
%% eq-classes
classDef eqStore fill:#D1FAE5,stroke:#10B981,color:#064E3B
class N eqStore
```

Practical limits that the code enforces:

- Neighbor expansion depth is clamped to the range 1 to 3, and a single call returns at most 500 neighbors.
- Full scans (`get_all_nodes`, `get_all_edges`) are forbidden on the request path. They exist for admin and tests.
- The label-table `graphid` type had no `=` operator, so that SQL cast with `graphid::text`.
- The COPY bulk loader ran when a batch had at least `EDGEQUAKE_BULK_COPY_MIN_ROWS` rows (default 1000) and the extension was 1.7.0 or later. A ready ledger inserts into `eq_graph_nodes`.

## Pre-cutover indexes

`ensure_indexes` in `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/helpers/graph_lifecycle.rs` created these indexes on the label tables. A single-flight lock (SPEC-069) lets only one build run at a time in a process. Serving-graph indexes are created by migration 170 and listed in [indexes.md](./indexes.md).

| Label | Indexes |
|---|---|
| `Node` | `idx_node_props_gin`, `idx_node_id`, `idx_node_tenant_id`, `idx_node_workspace_id`, `idx_node_source_id_expr`, `idx_node_source_ids_gin`, `idx_node_source_chunk_ids_gin`, and the unique `idx_node_prop_node_id_unique` |
| `EDGE` | `idx_edge_start_id`, `idx_edge_end_id`, `idx_edge_source_id`, `idx_edge_target_id`, `idx_edge_source_ids_gin`, `idx_edge_source_chunk_ids_gin`, `idx_edge_source_chunk_id`, `idx_edge_source_document_id`, `idx_edge_props_gin`, `idx_edge_tenant_id`, `idx_edge_workspace_id`, `idx_edge_start_id_text`, `idx_edge_end_id_text` |

Operation-to-index mapping is on [indexes.md](./indexes.md).

## Tenant isolation

`eq_graph_nodes` and `eq_graph_edges` are shared across workspaces. Isolation is the `tenant_id` and `workspace_id` columns, application filters, and row-level security installed by `eq_graph_install_plain_rls` in migration 170 when the role `edgequake_tenant_access` exists. See [postgres.md](./postgres.md#tenant-isolation).

## Pre-cutover version notes

Product images do not install a graph extension (from v0.35.5). See [version-matrix.md](./version-matrix.md).

| PostgreSQL | Extension build used before the cutover | Notes |
|---|---|---|
| 16 | 1.6.0 | Label-table RLS and the COPY loader were absent. |
| 17 | 1.7.0 | See [pg17-differential.md](./pg17-differential.md). |
| 18 | 1.8.0 | See [pg18-adoption.md](./pg18-adoption.md). |

## Pre-cutover operation catalog

Ref IDs do not change. A ready ledger serves these entry points from `eq_graph_nodes` and `eq_graph_edges`. Rows with a linked Ref ID have a benchmark template in [benchmarks/](./benchmarks/README.md). Cost and failure notes are in [complexity-matrix.md](./complexity-matrix.md). File paths are relative to `edgequake/crates/`. Line numbers were removed because they drift; search by entry point name. Some entry points are descriptive, not exact function names.

### Graph reads (34)

Lookups, batches, search, traversal, and counts. Entries marked FORBIDDEN in the notes must not run on the request path.

| Ref ID | Entry point | File | Type | Tx | Notes |
|---|---|---|---|---|---|
| `DATA-AGE-GRAPH-HAS-NODE-025` | `GraphStorage::has_node` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Read | Yes |  |
| `DATA-AGE-GRAPH-GET-NODE-026` | `GraphStorage::get_node` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Read | Yes |  |
| `DATA-AGE-GRAPH-NODE-DEGREE-027` | `GraphStorage::node_degree` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Read | Yes |  |
| `DATA-AGE-GRAPH-NODE-DEGREES-BATCH-028` | `GraphStorage::node_degrees_batch` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Read | Yes |  |
| `DATA-AGE-GRAPH-GET-ALL-NODES-029` | `GraphStorage::get_all_nodes` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Read | Yes | FORBIDDEN request path |
| `DATA-AGE-GRAPH-GET-NODES-BY-IDS-030` | `GraphStorage::get_nodes_by_ids` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Read | Yes |  |
| [`DATA-AGE-GRAPH-GET-NODES-BATCH-031`](./benchmarks/031.md) | `GraphStorage::get_nodes_batch` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Read | Yes | native SQL preferred |
| `DATA-AGE-GRAPH-GET-EDGES-FOR-NODES-BATCH-032` | `GraphStorage::get_edges_for_nodes_batch` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Read | Yes |  |
| `DATA-AGE-GRAPH-HAS-EDGE-033` | `GraphStorage::has_edge` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Read | Yes |  |
| `DATA-AGE-GRAPH-GET-EDGE-034` | `GraphStorage::get_edge` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Read | Yes |  |
| `DATA-AGE-GRAPH-GET-NODE-EDGES-035` | `GraphStorage::get_node_edges` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Read | Yes |  |
| `DATA-AGE-GRAPH-GET-INCIDENT-EDGES-BATCH-036` | `GraphStorage::get_incident_edges_batch` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Read | Yes |  |
| `DATA-AGE-GRAPH-GET-ALL-EDGES-037` | `GraphStorage::get_all_edges` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Read | Yes | FORBIDDEN request path |
| `DATA-AGE-GRAPH-GET-KNOWLEDGE-GRAPH-038` | `GraphStorage::get_knowledge_graph` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Read | Yes | bounded expand |
| `DATA-AGE-GRAPH-GET-POPULAR-LABELS-039` | `GraphStorage::get_popular_labels` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Read | Yes |  |
| `DATA-AGE-GRAPH-SEARCH-LABELS-040` | `GraphStorage::search_labels` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Read | Yes |  |
| `DATA-AGE-GRAPH-SEARCH-NODES-041` | `GraphStorage::search_nodes` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Read | Yes |  |
| `DATA-AGE-GRAPH-GET-NEIGHBORS-042` | `GraphStorage::get_neighbors` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Read | Yes |  |
| `DATA-AGE-GRAPH-GET-POPULAR-NODES-DEGREE-043` | `GraphStorage::get_popular_nodes_with_degree` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Read | Yes |  |
| `DATA-AGE-GRAPH-GET-EDGES-FOR-NODE-SET-044` | `GraphStorage::get_edges_for_node_set` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Read | Yes |  |
| `DATA-AGE-GRAPH-NODE-COUNT-057` | `GraphStorage::node_count` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Read | Yes | O(N) exact |
| `DATA-AGE-GRAPH-EDGE-COUNT-058` | `GraphStorage::edge_count` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Read | Yes |  |
| `DATA-AGE-GRAPH-NODE-COUNT-FAST-059` | `GraphStorage::node_count_fast` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Read | Yes | reltuples |
| `DATA-AGE-GRAPH-EDGE-COUNT-FAST-060` | `GraphStorage::edge_count_fast` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Read | Yes |  |
| `DATA-AGE-GRAPH-NODE-COUNT-BY-WORKSPACE-061` | `GraphStorage::node_count_by_workspace` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Read | Yes |  |
| `DATA-AGE-GRAPH-EDGE-COUNT-BY-WORKSPACE-062` | `GraphStorage::edge_count_by_workspace` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Read | Yes |  |
| `DATA-AGE-GRAPH-DISTINCT-NODE-TYPE-COUNT-063` | `GraphStorage::distinct_node_type_count_by_workspace` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Read | Yes |  |
| `DATA-AGE-GRAPH-NODE-COUNT-BY-SOURCE-PREFIX-064` | `GraphStorage::node_count_by_source_prefix` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Read | Yes |  |
| `DATA-AGE-GRAPH-NODE-COUNTS-BY-SOURCE-PREFIXES-065` | `GraphStorage::node_counts_by_source_prefixes` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Read | Yes | batched list reconcile |
| `DATA-AGE-GRAPH-LIST-NODES-FILTERED-066` | `GraphStorage::list_nodes_filtered` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Read | Yes |  |
| `DATA-AGE-GRAPH-LIST-EDGES-FILTERED-067` | `GraphStorage::list_edges_filtered` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Read | Yes |  |
| `DATA-AGE-GRAPH-FIND-NODES-BY-SOURCE-PREFIXES-068` | `GraphStorage::find_nodes_by_source_prefixes` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Read | Yes |  |
| `DATA-AGE-GRAPH-FIND-EDGES-BY-SOURCE-PREFIXES-069` | `GraphStorage::find_edges_by_source_prefixes` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Read | Yes |  |
| `DATA-AGE-GRAPH-FIND-EDGE-BY-RELATIONSHIP-ID-070` | `GraphStorage::find_edge_by_relationship_id` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Read | Yes |  |

### Graph writes (13)

Upserts and deletes. Batch forms use native SQL by default.

| Ref ID | Entry point | File | Type | Tx | Notes |
|---|---|---|---|---|---|
| `DATA-AGE-GRAPH-UPSERT-NODE-045` | `GraphStorage::upsert_node` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Write | Yes |  |
| [`DATA-AGE-GRAPH-UPSERT-NODES-BATCH-046`](./benchmarks/046.md) | `GraphStorage::upsert_nodes_batch` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Write | Yes | native ON CONFLICT |
| `DATA-AGE-GRAPH-DELETE-NODE-047` | `GraphStorage::delete_node` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Write | Yes |  |
| `DATA-AGE-GRAPH-DELETE-NODES-BATCH-048` | `GraphStorage::delete_nodes_batch` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Write | Yes |  |
| `DATA-AGE-GRAPH-DELETE-NODE-SCOPED-049` | `GraphStorage::delete_node_scoped` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Write | Yes |  |
| `DATA-AGE-GRAPH-UPSERT-EDGE-050` | `GraphStorage::upsert_edge` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Write | Yes |  |
| `DATA-AGE-GRAPH-UPSERT-EDGES-BATCH-051` | `GraphStorage::upsert_edges_batch` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Write | Yes |  |
| `DATA-AGE-GRAPH-DELETE-EDGE-052` | `GraphStorage::delete_edge` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Write | Yes |  |
| `DATA-AGE-GRAPH-DELETE-EDGES-BATCH-053` | `GraphStorage::delete_edges_batch` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Write | Yes |  |
| `DATA-AGE-GRAPH-DELETE-EDGE-SCOPED-054` | `GraphStorage::delete_edge_scoped` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Write | Yes |  |
| `DATA-AGE-GRAPH-CLEAR-055` | `GraphStorage::clear` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Write | Yes | ADMIN |
| `DATA-AGE-GRAPH-CLEAR-WORKSPACE-056` | `GraphStorage::clear_workspace` | `edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs` | Write | Yes | ADMIN |
| `DATA-AGE-GRAPH-COPY-LOAD-VERTICES-073` | `load_vertices_from_csv` | `edgequake-storage/src/adapters/postgres/age_csv_loader.rs` | Write | Yes | COPY bulk |

### Graph infrastructure (3)

Cypher execution, index setup, the COPY loader, and session setup.

| Ref ID | Entry point | File | Type | Tx | Notes |
|---|---|---|---|---|---|
| `DATA-AGE-GRAPH-CYPHER-EXEC-071` | `execute_cypher / cypher_query` | `edgequake-storage/src/adapters/postgres/graph/helpers/cypher_exec.rs` | Read/Write | Yes | AGE session wrapper |
| `DATA-AGE-GRAPH-LIFECYCLE-ENSURE-INDEXES-072` | `ensure_indexes` | `edgequake-storage/src/adapters/postgres/graph/helpers/graph_lifecycle.rs` | DDL | No | boot-time index reconcile |
| `DATA-AGE-GRAPH-SESSION-LOAD-AGE-074` | `set_age_session / search_path` | `edgequake-storage/src/adapters/postgres/graph/helpers/session.rs` | Session | Yes |  |

### Workspace statistics (1)

Counts a workspace's graph size. Its main data lives in PostgreSQL tables.

| Ref ID | Entry point | File | Type | Tx | Notes |
|---|---|---|---|---|---|
| `DATA-AGE-WORKSPACE-GET-STATS-157` | `pg_get_workspace_stats` | `edgequake-core/src/workspace_service_impl/workspace_ops.rs` | Read | Yes | secondary: PG |
