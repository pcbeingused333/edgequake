# 03 — Product spec

Parent: [README](README.md) · Prev: [02-surfaces](02-surfaces.md) ·
Next: [04-findings](04-findings.md)

## Personas

| Persona | Choice they make | Scope |
|---------|------------------|-------|
| Member | The model for this question, or Inherit | workspace, `query` |
| Workspace admin | The model for each job, and the providers this workspace keeps | workspace |
| Tenant admin | The organization's default models, inside the tenant offer and ceiling | tenant |
| Platform admin | The fleet default, inside the global offer | deployment |

A member does not see models the workspace admin turned off. A workspace
admin does not assign a model the tenant ceiling removed. A tenant admin
does not default the organization to a provider the deployment withdrew.
A platform admin's fleet default is not filtered by one workspace.

Names of the admin roles stay [SPEC-175 roles](../175-perfect-settings/15-roles.md).
This pack adds no role.

## Stories

### S1 — The chip matches the workspace

A member opens the composer. The chip lists Inherit, then the models
`eligible(workspace, query)` returns, grouped by provider. A model the
tenant ceiling removed is absent. A provider the workspace offer omitted
is absent. The member sends a question. The answer is produced by the
pair on the chip, or by the effective query row when the chip says
Inherit.

Acceptance: the ids in the chip equal the ids of
`GET /models/eligible?workspace_id&usage=query`.
[EC-178-01](11-edge-cases.md) · [EC-178-09](11-edge-cases.md).

### S2 — Inherit names the model that will run

The chip is on Inherit. The label is the provider and model from
`GET /config/effective` for `query`, plus a short source
(`This workspace`, `Organization`, `Deployment`, `Locked`). It is not
the process model from `GET /models/llm`.

Acceptance: with a workspace query role set to model A and a process
model B, Inherit reads A. [EC-178-06](11-edge-cases.md).

### S3 — A removed model refuses

The member has model A selected. An admin removes A from the ceiling.
The chip drops A. If the request is already in flight with A, the
server returns `422 model_not_eligible` and does not answer on another
model. The composer shows the sentence in [09-ux-ui-spec](09-ux-ui-spec.md).

Acceptance: streaming and `POST /query` return the same code.
[EC-178-02](11-edge-cases.md) · [EC-178-28](11-edge-cases.md).

### S4 — Workspace admin assigns from the same set

On Workspace → Models, each usage row lists `eligible(workspace, usage)`.
The chat family control lists the intersection. Saving a pair outside
the list is impossible from the control. A hand-written PUT of an
outside pair is `422 model_not_eligible` and the stored assignment is
unchanged.

Acceptance: [EC-178-03](11-edge-cases.md) · [EC-178-29](11-edge-cases.md).

### S5 — A leftover assignment stays visible and does not run

Yesterday's assignment names a model today's ceiling removed. The row
shows the model, the word Broken, and `broken_reason`. The picker does
not offer it. A query that inherits this usage refuses. The product
does not substitute the process model and does not rewrite the row.

Acceptance: [EC-178-23](11-edge-cases.md).

### S6 — Create and onboarding use the tenant set

The create-workspace form and the onboarding defaults step have no
workspace id. They list `eligible(tenant, …)`. They do not list the
deployment inventory. A provider the tenant offer omitted is absent.
After create, the workspace is unconstrained against that tenant offer,
so the same ids are what the new workspace's Models page shows until
someone narrows it.

Acceptance: [EC-178-15](11-edge-cases.md).

### S7 — Fleet defaults stay fleet-wide

The deployment card lists `eligible(deployment, …)`. It includes a model
every tenant can still see and excludes a model no offered provider
serves. It does not shrink because one workspace narrowed its own list.

Acceptance: [EC-178-16](11-edge-cases.md).

### S8 — Switching workspace switches the chip

The member selects model A in workspace 1 and model B in workspace 2.
Returning to workspace 1 shows A, if A is still eligible there. If it
is not, the chip returns to Inherit and the stored pair for that
workspace is cleared.

Acceptance: [EC-178-05](11-edge-cases.md) · [EC-178-27](11-edge-cases.md).

### S9 — Embedding save carries the dimension and does not rebuild

The admin picks an embedding model. The saved dimension is the card's
dimension. The library is not re-embedded. The existing rebuild control
remains the way to re-embed.

Acceptance: [EC-178-14](11-edge-cases.md) · [EC-178-38](11-edge-cases.md).

### S10 — Images use the vision assignment

A message with an image runs the vlm resolution. It does not silently
switch to the process vision model. A selected chat model that cannot
see images yields `422 capability_mismatch` and the sentence for that
code.

Acceptance: [EC-178-24](11-edge-cases.md).

## Out of these stories

- Editing API keys. Deployment → Connections, SPEC-176 and SPEC-177.
- Turning a provider on. The offer editors, SPEC-177.
- Ticking a model in the ceiling or the narrowing. Those editors stay.
  This pack only forces choice lists to obey the ticks.
- Per-user model allowlists. The member picks inside the workspace set.
- A rerank picker.
- Reprocessing the library because extract changed.

## Demo script

1. Deployment offers OpenAI. Tenant ceiling leaves two chat models.
   Workspace narrows to one.
2. Open the composer. The chip shows Inherit and that one model.
3. Open Workspace → Models. The query row shows the same model.
4. Send a question on that model. The trace names it.
5. Remove it from the tenant ceiling. The chip drops it. A scripted
   POST with the old pair returns `422 model_not_eligible` on the
   stream and on `/query`.
6. Inherit now shows the next resolved row, or Broken if nothing legal
   remains. The broken query does not answer.

The sentences the demo must not hear: "answered with the server model,"
"fell back to the default provider."
