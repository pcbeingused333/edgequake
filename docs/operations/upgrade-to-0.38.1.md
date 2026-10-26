---
title: "Upgrade to EdgeQuake v0.38.1"
description: "Upgrade from v0.38.0 to v0.38.1. Schema moves from 219 to 224."
---

# Upgrade to EdgeQuake v0.38.1

> **From:** v0.38.0 (schema **219**) · **To:** v0.38.1 (schema **224**)

`edgequake migrate` applies migrations **220** through **224**. All five are expand-only. They do not rewrite documents, chunks, or the graph. Restart every API replica after migrate so they serve the new binary against schema 224.

| Migration | What it does |
|-----------|----------------|
| 220 | Document clearance tables. `enforcement` defaults to false. Existing documents stay readable until a workspace owner turns clearance on. |
| 221 | `model_catalog_policy`. An empty table means unconstrained. Existing workspace model assignments keep running. |
| 222 | Secret-manager tables. The database backend is the default. Infisical is optional and per tenant. Existing connection ciphertext stays readable. |
| 223 | Provider offers. `provider_offer_constrained` defaults to false, so each scope still offers every provider its parent offers. |
| 224 | Provider refresh cache, offer revision, and a unique fleet connection slug. |

Set `EDGEQUAKE_SECRETS_KEY` before anyone saves a new provider key. The server still boots when it is unset. Saving a key without it is refused. Generate a 32-byte key with `openssl rand -base64 32`. Changing the key later makes rows encrypted with the old key unreadable. They are treated as having no key.

Clearance does not hide documents on upgrade. `clearance_covers` is true while the workspace flag is off, including when the settings row is missing.

**SPEC-001 Acc:** not re-scored on this cut. An empty model-catalog policy still runs the previous assignment. The Acc harness still pins `EDGEQUAKE_LLM_MODEL=mistral-small-latest` and leaves the extract and keyword locks empty, so they follow that pin. Mix retrieval is unchanged. The latest valid pack remains [`medical-mid-20261024T031725Z`](../../specs/001-benchmark/e2e/artifacts/history/medical-mid-20261024T031725Z/) (`valid: true`). EdgeQuake **0.8084**, LightRAG **0.7681**, delta **+0.0403**, 95% CI **[-0.0241, +0.1036]** (n_paired 72). The interval includes 0, so this is a statistical tie. EdgeQuake does not claim a win.
