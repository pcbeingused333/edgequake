---
title: "Upgrade to EdgeQuake v0.38.3"
description: "Upgrade from v0.38.2 to v0.38.3. Schema stays at 224."
---

# Upgrade to EdgeQuake v0.38.3

> **From:** v0.38.2 (schema **224**) · **To:** v0.38.3 (schema **224**)

`edgequake migrate` applies nothing new. Restart every API replica after the image bump so they serve the new binary.

Creating a workspace with organization defaults may store the decision-engine model `tev1:0.8b`. That name is a decision-host tag, not a chat-catalog pair. The create succeeds when the name is valid.

**SPEC-001 Acc:** not re-scored on this cut. Mix retrieval is unchanged. The latest valid pack remains [`medical-mid-20261024T031725Z`](../../specs/001-benchmark/e2e/artifacts/history/medical-mid-20261024T031725Z/) (`valid: true`). EdgeQuake **0.8084**, LightRAG **0.7681**, delta **+0.0403**, 95% CI **[-0.0241, +0.1036]** (n_paired 72). The interval includes 0, so this is a statistical tie. EdgeQuake does not claim a win.
