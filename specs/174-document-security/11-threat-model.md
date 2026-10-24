# 11 — Threat model

Parent: [README](README.md) · Prev: [10-frontend-architecture](10-frontend-architecture.md) ·
Next: [12-edge-cases](12-edge-cases.md)

Assets: document bytes, chunk text, embeddings (they reconstruct neighbors
of the text), entity fragments, answers, and the fact that a document or
entity exists. Trust boundary is the API process. The browser, the MCP
client, and the model are outside it.

OWASP Authorization Cheat Sheet applies: enforce on the server, deny by
default, do not trust client filters.
<https://cheatsheetseries.owasp.org/cheatsheets/Authorization_Cheat_Sheet.html>

## Actors

| Actor | Capability before this spec | After, enforcement on |
|-------|-----------------------------|------------------------|
| Workspace member | Every document | Visible set only |
| Readonly member | Every document, no writes | Visible set, no writes |
| Instance admin | Every document in a bound workspace | Clearance, unless break-glass |
| Master API key | Same, membership skipped | Same as admin |
| Uploader | The document, because the workspace can | Own unverified body; then clearance |
| Steward | (role does not exist) | All labels, deny rules still subtract |
| Ingest worker | Writes merged descriptions | Writes every fragment, system scope |

## Abuse cases

| Threat | Today | Control | EC |
|--------|-------|---------|----|
| Read a sibling department's PDF | RLS allows the SQL row; KV is read by id | `allows` before `load_document_body`; then RLS | EC-174-13, EC-174-40 |
| Read a merged sentence from before labels existed | One description blob | Serve it only if every source is visible | EC-174-42 |
| Omit `document_filter` | All-pass | Server fills the visible set | EC-174-15 |
| Send the hidden id in `document_ids` | Trusted | Intersection | EC-174-14 |
| Read merged entity text | Longer description or LLM summary | Fragments | EC-174-16 |
| Infer hidden links from degree | Global degree | Visible degree | EC-174-17 |
| Read the PNG | Unfiltered neighborhood | Same neighborhood as the API | EC-174-18 |
| Count hidden files via stats | Workspace counts | Visible counts | EC-174-20 |
| Watch the pipeline socket | Workspace events | Visible events, plus own uploads | EC-174-21 |
| Read a task error | Payload may quote text | Strip chunk text the subject cannot read | EC-174-22 |
| Replay chat after revoke | `messages.context` | Filter at read | EC-174-23 |
| Open a shared chat | `share_id` | Refuse new shares | EC-174-24 |
| Fetch another user's retrieval id | Cache returns the bundle | 404 unless fingerprint matches the caller | EC-174-25 |
| Poison the answer cache | Shared if prompts match | Hidden text never enters the prompt | EC-174-26 |
| Share the context cache | Key ignores clearance | Fingerprint + epoch | EC-174-27 |
| Upload identical bytes | Dedup returns the first id | Hidden duplicate is a new document, id not echoed | EC-174-28 |
| Download markdown by id | Tenant context ignored | Same 404 as get | EC-174-29 |
| Use admin or master key | Full corpus | No skip without the flag and an audit row | EC-174-30 |
| Use a write API key | Write scope is workspace-wide | Scope does not add compartments | EC-174-32 |
| Relabel restricted down to public | No labels | Steward only, audited | LAW-174-7 |
| Delete a tag to drop the predicate | No tags | `ON DELETE RESTRICT` | EC-174-11 |
| Rely on `tenant_id IS NULL` | Legacy policy branch | Clearance guard still applies | EC-174-33 |
| Reuse a pooled session GUC | Reset on release | `SET LOCAL` only | EC-174-34 |
| Ask the model "what else is in the workspace?" | It saw everything | Prompt has no hidden text and no omission sentence | LAW-174-16 |
| Read glossary from a hidden source | Injections are workspace text | Same predicate | EC-174-38 |
| List departments in a facet | Workspace vocabulary | Only held compartments | EC-174-39 |

## Residual risks (accepted in this pack)

1. **Canonical entity name.** If HR and R&D both extract `PERSON_A`, the
   R&D analyst learns the name exists. They do not learn the HR fragment
   or the HR document id. A deployment that cannot accept that must put
   the two corpora in two workspaces. Splitting nodes per label is out of
   scope ([06-graph-query-ai](06-graph-query-ai.md)).
2. **Embeddings of hidden chunks remain in the index.** The scan must not
   return them. A database superuser or `BYPASSRLS` role can. The app role
   `edgequake_tenant_access` is `NOBYPASSRLS` (migration 167). Operators
   do not run the API as a superuser.
3. **Timing.** A very selective clearance can make iterative HNSW scans
   longer. Do not return a header or a log line to the client that states
   how many rows were skipped. Server metrics may count skips without
   document ids.
4. **Break-glass flag.** A process with the flag on is a full reader.
   The audit row is the detective control, not a preventative one. Default
   off.
5. **Uploader of unverified content** can still exfiltrate their own upload.
   That is the point of the exception. The peer cannot.

## STRIDE (short)

| Category | Notes |
|----------|-------|
| Spoofing | Unchanged SPEC-154 / SPEC-158. Clearance is not taken from the JWT body |
| Tampering | Label columns change only in the repository transaction. Epoch bumps |
| Repudiation | Relabel, enable, simulate, and break-glass hit `audit_logs` |
| Information disclosure | The table above |
| Denial of service | Iterative scan cap stays pgvector's `hnsw.max_scan_tuples`. A hostile filter cannot request "all hidden ids" as a widen |
| Elevation | Deny subtracts after allow. Steward is a flag, not `users.role = admin` |

Next: [12-edge-cases](12-edge-cases.md).
