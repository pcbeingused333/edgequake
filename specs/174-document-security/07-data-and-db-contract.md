# 07 — Data and database contract

Parent: [README](README.md) · Prev: [06-graph-query-ai](06-graph-query-ai.md) ·
Next: [08-api-contract](08-api-contract.md)

Migration number is **220** (`220_spec174_document_clearance.sql`). Head
today is 219. The migration is additive. It does not enable enforcement.
Backfill of ranks runs in the enable transaction, not in 220, so applying
the migration does not relabel a corpus ([LAW-174-2](01-first-principles.md)).

PostgreSQL row security: <https://www.postgresql.org/docs/current/ddl-rowsecurity.html>.
Functions used in policies are `LEAKPROOF` and policies call them as
`(SELECT current_clearance_rank())`, matching migrations 206 and 208.

## Tables

```text
  workspace_clearance_settings
           |
           v
  +------------------------------------------+
  | workspace_id          PK                 |
  | enforcement           boolean default off|
  | policy_epoch          bigint default 0   |
  +------------------------------------------+
           |
           +-----------------------------+
           |                             |
           v                             v
  clearance_tags                  security_roles
  key, value, rank                name, steward
           |                             |
           v                             v
  document_labels                 security_role_rules
  document_id + tag_id            effect, match_kind
                                         |
                                         v
                               membership_security_roles
                               membership_id + role_id

  graph_contributions (one row per source fragment)
  +--------------------------------------------------+
  | workspace_id, tenant_id                          |
  | kind: entity or edge                             |
  | document_id, chunk_key, fragment                 |
  | classification_rank, compartment_ids             |
  +--------------------------------------------------+
```

`clearance_tags` is unique on `(workspace_id, kind, key, value)`.
Classification rows use `key = 'classification'` and `rank` in 1..5.
Compartment rows use `rank` NULL.

`document_labels` is the assignment. The document also stores a copy used
by RLS: `documents.classification_rank`, `documents.compartment_ids`,
`documents.uploaded_by`. The copy is the authority for the predicate.
`document_labels` is the authority for the editor. A trigger, or one
repository function, updates both in the same transaction. Handlers do
not update the columns by hand ([LAW-174-5](01-first-principles.md)).

The same two columns are copied onto `chunks`, `document_originals`,
`pdf_documents`, `document_mm_assets`, `document_pages`, and `tasks`
(where `tasks.document_id` is not null). Copy happens when the document
label changes, in one transaction, and bumps `policy_epoch`.

`graph_contributions.fragment` is the read authority for description text
while enforcement is on. `eq_graph_nodes.properties.description` stays for
the enforcement-off path and for Acc.

## GUCs

Extended inside `set_tenant_context`, still `set_config(..., true)` so the
value dies at commit (migration 096, [EC-174-34](12-edge-cases.md)):

| GUC | Meaning |
|-----|---------|
| `app.clearance_enforced` | `on` or `off` for this workspace |
| `app.clearance_rank` | compiled rank, empty if none |
| `app.clearance_compartments` | uuid array literal |
| `app.clearance_allow_empty` | `on` or `off` |
| `app.clearance_steward` | `on` or `off`. Audit and UI only. Rank and compartments already include the steward grant after deny subtraction |
| `app.clearance_break_glass` | `on` only when the process flag is on |
| `app.current_user_id` | already set; uploader exception uses it |

`current_clearance_rank()` and `clearance_covers(rank smallint, compartments uuid[])`
are `STABLE LEAKPROOF`. `clearance_covers` returns true when enforcement
is off. When enforcement is on it returns false if the rank GUC is unset,
except break-glass and except the uploader clause for rank 5. It does not
treat steward as a bypass: a steward who lost a compartment to a deny rule
fails subset on that compartment, because the compiler already subtracted
it before setting the GUC ([05-policy-model](05-policy-model.md)).
NULL `classification_rank` on a row does not match (fail closed,
[EC-174-12](12-edge-cases.md)).

## Policy shape

Existing `tenant_access_guard` stays. A second restrictive policy
`clearance_guard` is AND-ed by PostgreSQL with the tenant guard:

```text
  tenant_access_guard
  AND (
        app.clearance_enforced is off
        OR clearance_covers(classification_rank, compartment_ids)
        OR (
             uploaded_by = current_user_id()
             AND classification_rank = 5
           )
      )
```

The uploader exception in SQL covers rank 5 only. Dominance for the
uploader's other documents is ordinary `clearance_covers`. After a raise
above the uploader, rank is no longer 5 and `clearance_covers` fails, so
the exception ends ([LAW-174-13](01-first-principles.md)).

`clearance_covers` implements subset with `compartments <@
current_clearance_compartments()` and the empty-set bit. It does not read
`jsonb`.

Graph tables do not get this policy. A node has many labels. Filtering
the node with one mask would hide a public fragment because a restricted
fragment shares the row ([F-174-04](04-findings.md)). Reads go through
`graph_contributions`, which does get `clearance_guard`.

## Indexes

- `documents (workspace_id, classification_rank)`
- same on `chunks`
- GIN on `compartment_ids` for `<@`
- `graph_contributions (workspace_id, entity_name)` and
  `(workspace_id, document_id)`
- do not build one partial HNSW index per user

Vector search:

- Small visible set (threshold in [16-performance](16-performance.md)):
  keep `document_id = ANY($ids)`.
- Large visible set: the chunk query joins `chunks` and applies
  `clearance_covers` on the stored columns inside the scan.
- For that statement only, `SET LOCAL hnsw.iterative_scan = relaxed_order`.
  Do not change the server default. pgvector filtering and iterative
  scans: <https://github.com/pgvector/pgvector#filtering>.

Relabel updates the copied columns. It does not re-embed.

## Epoch and pools

`policy_epoch` increments on enable, tag edit, rule edit, assignment edit,
and document relabel. Cache keys include it ([LAW-174-11](01-first-principles.md)).
A statement sees one snapshot. A relabel committed after the statement
started is invisible to that statement and visible to the next, under the
new epoch.

`after_release` already runs `RESET ALL` and re-pins the session baseline
(SPEC-171). Clearance GUCs must not be set outside the transaction. A
pooled connection must not keep the previous user's rank
([EC-174-34](12-edge-cases.md)).

## Enable transaction

The flag stays off until labels are copied ([RB-174-06](17-roadblocks.md)):

1. Insert settings with `enforcement = false` if the row is missing. Seed
   the five classification tags.
2. Backfill documents with no classification to rank 2 (`internal`), or
   rank 5 when strict-start. Copy those columns onto child rows. Batches
   by `document_id` are allowed. Each batch commits while the flag is
   still false.
3. Insert the steward role if missing and assign it to the acting owner's
   membership. If that grant cannot be written, stop. The flag stays off.
4. Set `enforcement = true` and bump `policy_epoch` in the last statement.
   If this statement fails, the previous batches remain and enforcement
   stays off so the job can resume.

220 itself only creates tables, columns, functions, and policies. The
policy's first clause is "enforcement off → cover", so existing rows with
NULL rank stay readable until step 4. After step 4, NULL rank fails closed.

## Delete

`DELETE` of a `clearance_tags` row that `document_labels` still references
is rejected (`ON DELETE RESTRICT`, [EC-174-11](12-edge-cases.md)).
Deleting a document cascades contributions and labels with the document,
which existing `ON DELETE CASCADE` from `documents` already implies for
chunks. Contributions use `ON DELETE CASCADE` from `documents` as well.

Next: [08-api-contract](08-api-contract.md).
