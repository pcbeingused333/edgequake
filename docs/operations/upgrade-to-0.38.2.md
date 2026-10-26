---
title: "Upgrade to EdgeQuake v0.38.2"
description: "Upgrade from v0.38.1 to v0.38.2. Schema stays at 224."
---

# Upgrade to EdgeQuake v0.38.2

> **From:** v0.38.1 (schema **224**) · **To:** v0.38.2 (schema **224**)

`edgequake migrate` applies nothing new. Restart every API replica after the image bump so they serve the new binary.

A live model list that leaves out the embedding size keeps the static card, then the known-model table. `mistral-embed` stays 1024. A size the provider reports still wins.

Saving a provider offer reads `provider_offer_revision` as a 32-bit integer, so `If-Match` matches the value the server stored.

The workspace wizard and the model picker list only models the gate will run.

**SPEC-001 Acc:** not re-scored on this cut. Mix retrieval is unchanged. The latest valid pack remains [`medical-mid-20261024T031725Z`](../../specs/001-benchmark/e2e/artifacts/history/medical-mid-20261024T031725Z/) (`valid: true`). EdgeQuake **0.8084**, LightRAG **0.7681**, delta **+0.0403**, 95% CI **[-0.0241, +0.1036]** (n_paired 72). The interval includes 0, so this is a statistical tie. EdgeQuake does not claim a win.
