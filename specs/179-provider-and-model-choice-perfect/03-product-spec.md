# 03 — Product spec

Parent: [README](README.md) · Prev: [02-surfaces](02-surfaces.md) · Next: [04-findings](04-findings.md)

## WHY

Four people configure models. They do not share a screen, and they must not
share a list that is wider than the list above them. The stories below are
the acceptance tests in prose. Each one maps to an EC and a test.

## Personas

| Persona | May |
|---------|-----|
| Platform Admin | Create connections, store secrets, refresh, register models, withdraw, set fleet defaults |
| Org Admin | Offer a subset of the deployment, set a model ceiling |
| Workspace Admin | Offer a subset of the org, narrow models, assign `llm_roles` |
| Member | Pick a query model inside the workspace set, for this turn |

## Stories

### S1 — Add a provider

The Platform Admin opens Deployment → Connections, picks a shape, sets a
slug, base URL, and key, and tests. A passing test makes the slug appear
under Providers. Failure stays on the card with the latency and the reason.
The key field shows `••••` plus the last four of the fingerprint after save.
Rotate replaces the key. A blank field keeps it. Delete asks who is using
the slug and then removes it.

### S2 — Refresh models

On the provider row, Refresh calls that provider with its own key. The row
shows the time, `+N new`, `-N gone`, and an error if the upstream failed.
New ids are unchecked. Register writes `connection_models` including
`embedding_dimension` when the model is an embedder. Until register, the id
is not in any choice list.

### S3 — Org filters

The Org Admin sees only deployment slugs. They uncheck two providers and
save. A workspace under that org loses those providers. Stored assignments
that used them are listed before save and marked broken after confirm. The
next ingest of a document that needed one fails with `model_not_eligible`
and a link to Workspace → Models.

### S4 — Workspace assigns

The Workspace Admin picks extract, query, embedding, and vision from the
narrowed set. The embedding row shows the dimension and warns that a
dimension change needs a rebuild. Save sends one `llm_roles` patch. A pair
outside the set is `422` and nothing is written.

### S5 — Member asks

The query chip lists eligible query models and an Inherit row named with the
effective model. A model removed by the ceiling disappears within one
invalidation. Send with a stale id returns `422`, not a different model.

### S6 — Upload a scanned PDF

The dropzone vision picker lists eligible `vlm` models. Inherit uses the
workspace vision role. There is no second vision editor writing a legacy
column.

## Acceptance

- Withdrawing the process-default provider stops ingest that resolved to it.
- A fleet slug with a passing probe and a registered model runs from
  `{provider: slug, model}` with no `connection_id` in the body.
- `GEMINI_API_KEY` set makes `gemini` appear in `env_connections` output.
- Wizards before a tenant exists list deployment-eligible models.
- No choice component issues `GET /models/search` or `GET /models/llm`.
