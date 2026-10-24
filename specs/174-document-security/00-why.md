# 00 — WHY (5-WHY)

Parent: [README](README.md) · Next: [01-first-principles](01-first-principles.md)

## The job to be done

A workspace is a shared knowledge base. People in that workspace do not all
have the same need to know. A document can be confidential, or belong to HR,
or to R&D, or to both. A role should carry a rule over those tags, and every
read of the knowledge base should obey the rule: list, download, query,
citation, graph picture, MCP, and cache.

Today a member who may query a workspace sees every document in it. Tags and
document rules do not exist. That is why this pack exists.

The operation axis is already real: instance roles `admin` / `user` /
`readonly`, membership roles `owner` / `admin` / `member` / `readonly`, and
row security on tenant and workspace. This pack does not replace that axis.
It adds the object axis those checks never had.

---

## 5-WHY chain A — A workspace is the whole corpus

| # | Question | Answer |
|---|----------|--------|
| 1 | Why does an HR analyst see an R&D document? | List, get, and query select by `workspace_id` (and tenant). There is no document predicate. |
| 2 | Why is `document:read` not enough? | `RbacService::role_permissions` grants it for the role. The API never calls `require_permission`. The live check is write-vs-read on the route. |
| 3 | Why does Postgres not stop it? | RLS policies compare `current_tenant_id()` and `current_workspace_id()` only. |
| 4 | Why was that the product boundary? | SPEC-158 and SPEC-169 isolate tenants and memberships. They stop at the workspace. |
| 5 | **Root cause** | **Authorization names operations on a workspace, not attributes of a document.** |

```text
  HR analyst (member, query allowed)
           |
           v
  +----------------------------------+
  | GET /documents  POST /query      |
  +----------------+-----------------+
                   |
                   v
  +----------------------------------+
  | RLS tenant_id AND workspace_id   |
  +----------------+-----------------+
                   |
                   v
  +----------------------------------+
  | HR memo + R&D memo + untagged    |
  +----------------------------------+
```

Cross-ref: [LAW-174-1](01-first-principles.md) · [LAW-174-3](01-first-principles.md) ·
[F-174-01](04-findings.md) · [F-174-02](04-findings.md) ·
[EC-174-01](12-edge-cases.md) · [EC-174-02](12-edge-cases.md).

---

## 5-WHY chain B — The graph mixes sources into one sentence

| # | Question | Answer |
|---|----------|--------|
| 1 | Why can a graph node leak a document the user cannot open? | Entity names are unique per tenant and workspace. The second extraction merges into the same node. |
| 2 | Why does the description contain both documents? | `merger/entity.rs` keeps the longer description or asks the LLM to summarize both. |
| 3 | Why does "any overlapping source" still leak? | Query lineage keeps the node if one source is allowed, and still serves the merged text. |
| 4 | Why does the graph page skip even that test? | Browse, search, BFS, and the PNG path filter tenant and workspace only. |
| 5 | **Root cause** | **Derived text is stored as one blob. Visibility of a source is not visibility of the blob.** |

```text
  Doc HR (salary)          Doc R&D (chip)
       \                        /
        \                      /
         v                    v
  +------------------------------------+
  | entity PERSON_A                    |
  | description = longer or summary    |
  | source_document_ids = {HR, R&D}    |
  +------------------------------------+
                   |
                   v
  R&D analyst reads PERSON_A
  and the HR sentence
```

Cross-ref: [LAW-174-9](01-first-principles.md) · [F-174-04](04-findings.md) ·
[F-174-05](04-findings.md) · [EC-174-16](12-edge-cases.md) ·
[EC-174-17](12-edge-cases.md).

---

## 5-WHY chain C — The document filter is a client hint

| # | Question | Answer |
|---|----------|--------|
| 1 | Why does omitting `document_filter` search every document? | `resolve_document_filter` returns `None` when the filter is empty, and the engine treats `None` as all-pass. |
| 2 | Why can a client name another document's id? | The explicit-id fast path returns those ids with no server intersection. |
| 3 | Why do graph and download ignore the filter? | `allowed_document_ids` is a query-engine argument. Graph browse and markdown download never receive it. |
| 4 | Why is MCP the same shape? | `eq_search` and `eq_neighborhood` apply `document_ids` only when the caller sends them. |
| 5 | **Root cause** | **The only document predicate in the product is optional and caller-supplied.** |

```text
  Client body                         Server
  -----------                         ------
  document_filter omitted    -->      None --> whole workspace
  document_ids: [hidden]     -->      trusted --> hidden chunks
  eq_neighborhood (no ids)   -->      BFS of the workspace
```

Cross-ref: [LAW-174-10](01-first-principles.md) · [F-174-03](04-findings.md) ·
[F-174-12](04-findings.md) · [EC-174-14](12-edge-cases.md) ·
[EC-174-15](12-edge-cases.md).

---

## 5-WHY chain D — A cache hit is another user's context

| # | Question | Answer |
|---|----------|--------|
| 1 | Why would user B receive user A's chunks? | `retrieval_id` and the context cache store the bundle. Fetch does not rebuild the question. |
| 2 | Why is the key shared? | Context cache key has workspace, query, mode, and `allowed_document_ids`. No user, no policy epoch. An omitted filter is one shared entry. |
| 3 | Why is the answer cache different? | Its key is a hash of the prompt, built after context filtering. It is safe only when forbidden text never enters the prompt. |
| 4 | Why do old chats keep the leak after a revoke? | `messages.context` stores the bundle that was used. Replay loads it back into the next prompt. |
| 5 | **Root cause** | **Stores of retrieved text are keyed by the question, not by the clearance that produced the text.** |

```text
  User A query (sees HR)
           |
           v
  +----------------------------------+
  | retrieval_id -> full bundle      |
  | context cache (workspace, query) |
  +----------------+-----------------+
                   |
                   v
  User B eq_fetch(retrieval_id)
  same bundle, no re-check
```

Cross-ref: [LAW-174-11](01-first-principles.md) · [F-174-07](04-findings.md) ·
[F-174-08](04-findings.md) · [EC-174-25](12-edge-cases.md) ·
[EC-174-26](12-edge-cases.md) · [EC-174-27](12-edge-cases.md).

---

## 5-WHY chain E — Platform power is document power

| # | Question | Answer |
|---|----------|--------|
| 1 | Why does an instance admin see HR text in a workspace they do not belong to? | `Role::Admin` skips the membership row and is treated as owner for the workspace. |
| 2 | Why does the master API key do the same? | `bind_request` skips membership for `master-api-key`. The key is `Role::Admin`. |
| 3 | Why is the audit row not a control? | The bypass writes a success audit for membership, then RLS returns every row in the workspace. |
| 4 | Why is dedup another oracle? | Upload matches `content_hash` inside the workspace and can point at a document the uploader must not know. |
| 5 | **Root cause** | **Every bypass of membership is also a bypass of a document boundary that was never defined.** |

```text
  Instance admin / master key
           |
           v
  +----------------------------------+
  | membership check skipped         |
  +----------------+-----------------+
                   |
                   v
  +----------------------------------+
  | set_tenant_context(tenant, ws)   |
  | no clearance GUC                 |
  +----------------+-----------------+
                   |
                   v
  entire workspace corpus
```

Cross-ref: [LAW-174-12](01-first-principles.md) · [F-174-09](04-findings.md) ·
[F-174-10](04-findings.md) · [EC-174-28](12-edge-cases.md) ·
[EC-174-30](12-edge-cases.md).

---

## What the five causes require

| Cause | Required property |
|-------|-------------------|
| A workspace = corpus | Object rules over tags, default deny when on |
| B mixed description | Per-source contributions, visible degree |
| C client filter | Server visible set, client ids only narrow |
| D shared bundle | Fingerprint + epoch on stored context, re-check on read |
| E platform bypass | Clearance applies to admin and master key |

Next: [01-first-principles](01-first-principles.md).
