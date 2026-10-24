---
title: "Upgrade to EdgeQuake v0.38.0"
description: "Upgrade from v0.37.0 to v0.38.0. Schema moves from 218 to 219."
---

# Upgrade to EdgeQuake v0.38.0

> **From:** v0.37.0 (schema **218**) · **To:** v0.38.0 (schema **219**)

`edgequake migrate` applies migration **219** only. It creates `mcp_upload_sessions`. It does not move original bytes.

| Migration | What it does |
|-----------|----------------|
| 219 | MCP upload sessions for a checksum-bound agent PUT. Expand only. |

Restart every API replica after migrate so they serve the new binary against schema 219.

A presigned PUT is used when `EDGEQUAKE_ORIGINAL_STORE=object` and the store can sign the content type (and `x-amz-checksum-sha256` on S3). Otherwise the server mints a same-origin HMAC PUT. Set `EDGEQUAKE_MCP_UPLOAD_HMAC_KEY` in production. When that variable is unset, the JWT secret signs the URL. The default JWT secret cannot mint one. `EDGEQUAKE_MCP_UPLOAD_TTL_SECS` is capped at 900. `EDGEQUAKE_MCP_UPLOAD_BIND=bearer` also requires an `Authorization` header.

**SPEC-001 Acc:** not re-scored on this cut. SPEC-173 does not change mix retrieval. The latest valid pack remains [`medical-mid-20261024T031725Z`](../../specs/001-benchmark/e2e/artifacts/history/medical-mid-20261024T031725Z/) (`valid: true`). EdgeQuake **0.8084**, LightRAG **0.7681**, delta **+0.0403**, 95% CI **[-0.0241, +0.1036]** (n_paired 72). The interval includes 0, so this is a statistical tie. EdgeQuake does not claim a win.
