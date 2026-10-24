---
title: "Upgrade to EdgeQuake v0.37.0"
description: "Upgrade from v0.36.0 to v0.37.0. Schema moves from 214 to 218."
---

# Upgrade to EdgeQuake v0.37.0

> **From:** v0.36.0 (schema **214**) · **To:** v0.37.0 (schema **218**)

`edgequake migrate` applies migrations **215** through **218**. Text, chunks, and the graph stay in Postgres.

| Migration | What it does |
|-----------|----------------|
| 215 | Ledger for original object refs. Expand only. |
| 216 | Maintenance role `edgequake_originals_maint` for `edgequake originals`. |
| 217 | Page images and extracted markdown may live in the same private bucket. |
| 218 | PDF window digest column. `NULL` means a partial range is unchecked. |

The code default remains `EDGEQUAKE_ORIGINAL_STORE=database`. Object writes happen only when that variable is `object`. `make dev` sets `object` and starts MinIO. `make dev-database` stays on Postgres. The process does not create a bucket. `EDGEQUAKE_ORIGINAL_STORE_CREATE_BUCKET=true` fails at boot.

`documents.content` and chunk text stay in Postgres. PDF bytes, uploaded originals, page images, and extracted markdown can move to the bucket. Operator steps: [originals](originals.md).

```bash
edgequake migrate
edgequake originals
edgequake originals copy
edgequake originals verify
edgequake originals clear --yes
```

`clear` runs only after verify is clean, and only in object mode. It needs `--yes`.

**SPEC-001 Acc:** scored on this cut, medical-mid n=200, archive [`medical-mid-20261024T031725Z`](../../specs/001-benchmark/e2e/artifacts/history/medical-mid-20261024T031725Z/) (`valid: true`). EdgeQuake **0.8084**, LightRAG **0.7681**, delta **+0.0403**, 95% CI **[-0.0241, +0.1036]** (n_paired 72). The interval includes 0, so this is a statistical tie. EdgeQuake does not claim a win.
