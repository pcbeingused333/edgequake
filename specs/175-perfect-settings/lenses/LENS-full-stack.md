# Lens — Full Stack

Parent: [README](../README.md) · Plan: [12-implementation-plan](../12-implementation-plan.md) ·
Surfaces: [02-surfaces](../02-surfaces.md)

## What this lens owns

[LAW-175-1](../01-first-principles.md) and [LAW-175-15](../01-first-principles.md).
One function (`resolve_usage` in `edgequake-core`). One HTTP mapping.
One test name per edge case.

## Call order

Load tenant, workspace, server defaults, locks, and policy. Call
`resolve_usage`. If `broken_reason` is set, return the status in
[08-api-contract](../08-api-contract.md). Only then build a provider
from the connection. Do not call `ProviderFactory::from_env` after a
failure.

## Insertion points (extend these, do not add a parallel ladder)

| Need | Existing symbol |
|------|-----------------|
| LLM pair today | `resolve_llm_choice` in `model_resolution.rs` |
| Extract pin | `resolve_extract_role_llm` becomes a caller |
| Query client | `resolve_llm_provider_for_workspace` |
| VLM | `resolve_workspace_vlm_config` |
| Effort | `resolve_role_reasoning_effort` |
| Effective JSON | `GET /config/effective` |
| Workspace write gate | `can_manage_workspaces` on a membership that `applies_to` that workspace |
| Tenant ceiling | `require_tenant_access(Manage)` inside the handler. `/api/v1/tenants` is `global_resource`, so `bind_request` will not do it ([EC-175-51](../11-edge-cases.md)) |

`resolve_llm_choice` can remain as the helper for layers 4–8 of the chat
family. It must not be called by a handler that skipped `resolve_usage`.

## Waves this lens sequences

W1 runtime, W2 HTTP, W3 move, W4 typed UI, W5 contextual, W6 demo.
W1 and W4 are different pull requests
([12-implementation-plan](../12-implementation-plan.md)).
