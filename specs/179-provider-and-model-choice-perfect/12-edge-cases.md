# 12 — Edge cases

Parent: [README](README.md) · Prev: [11-frontend-architecture](11-frontend-architecture.md) ·
Next: [13-implementation-plan](13-implementation-plan.md)

## WHY

The gate is wrong if any one of these still runs a model. Each case names
the test that locks it. The matrix is [14-e2e-test-matrix](14-e2e-test-matrix.md).

| ID | Case | Mitigation | Test |
|----|------|------------|------|
| EC-179-01 | Ingest after the extract model was withdrawn | gate, task `Failed`, no process pipeline | `spec179_model_gate` `ingest_withdrawn_fails` |
| EC-179-02 | Rebuild body names a model outside the ceiling | `ensure_workspace_choice` before write, 422, no metadata change | `rebuild_pair_rejected` |
| EC-179-03 | Reprocess with empty vision fields | gate on the stored `vlm` role, never the string `ollama` | `reprocess_uses_vlm_role` |
| EC-179-04 | Keyword role errors | gate returns the error, query does not silently swap models | `keyword_does_not_swap` |
| EC-179-05 | Fleet slug pair, no `connection_id` | gate loads the connection by slug | `fleet_slug_resolves` |
| EC-179-06 | `GEMINI_API_KEY` set, no fleet row | `env_connections` includes `gemini` | `env_connections_native` |
| EC-179-07 | Refresh upstream 401 | state `error`, previous cache kept, UI shows last success | `refresh_keeps_cache_on_error` |
| EC-179-08 | Embedding model registered without dimension | 422 `embedding_dimension_required` | `embedding_dimension_required` |
| EC-179-09 | Create with a duplicate slug | no orphan secret; row count unchanged | `duplicate_slug_no_orphan_secret` |
| EC-179-10 | Edit base URL | `last_test_ok` null, slug not offered until retest | `edit_clears_probe` |
| EC-179-11 | URL `https://user:pass@host` | 422 `url_rejected`, nothing stored | `url_userinfo_rejected` |
| EC-179-12 | Deployment defaults saved while the global offer is empty | `ensure_deployment_choice` refuses | `empty_offer_refuses_defaults` |
| EC-179-13 | Two admins PUT policy | second `If-Match` gets 409 `stale_revision`, first write stands | `stale_revision` |
| EC-179-14 | Withdraw with assignments, no confirm | 409 `confirm_required` and the list | `withdraw_requires_confirm` |
| EC-179-15 | Picker search returns an inventory-only id | the menu does not render it; a direct PUT is 422 | Playwright `choice-does-not-search-inventory` |
| EC-179-16 | First-run wizard, no tenant | deployment eligible, not an error state | Playwright `wizard-deployment-scope` |
| EC-179-17 | Offer fetch 500 | error copy and Retry, not "no providers yet" | Playwright `offer-error-state` |
| EC-179-18 | Delete connection from a tenant-scoped client | 422 `connection_scope_global` | `tenant_cannot_delete_fleet` |
| EC-179-19 | New file calls `ProviderFactory::from_env` | constructor guard test fails | `constructor_guard` |
| EC-179-20 | Ceiling shrinks while a document is queued | the worker re-judges at start and fails the task | `ingest_rejudge` |
| EC-179-21 | Embedding swap changes dimension | save allowed, UI warns, rebuild is a separate action, mixed dimensions do not silently query | `dimension_warned` |
| EC-179-22 | Discovered id equals a registered id | refresh updates `last_seen_at`, does not duplicate the card | `refresh_id_collision` |
| EC-179-23 | Slug rename while offers exist | 422 `slug_rename` | `slug_rename_refused` |
| EC-179-24 | Deleted connection, role still names it | next run `credential_missing` or `model_not_eligible` | `deleted_connection_fails_closed` |
| EC-179-25 | Member sends `X-Tenant-ID` of another tenant | existing tenant guard; gate loads the caller's workspace only | `cross_tenant_offer` |
| EC-179-26 | Secret backend unavailable | 503 on create and on resolve, no fallback client | `secret_down_fails_closed` |
| EC-179-27 | `EDGEQUAKE_SECRETS_KEY` does not decrypt | connection shows keyless, gate returns `credential_missing` | `bad_master_key` |
| EC-179-28 | Browser keeps a pair after workspace switch | per-workspace slot; the chip reloads eligible | Playwright `chip-slot-per-workspace` |
| EC-179-29 | Viewer opens Models | read-only matrix, no Save | Playwright `viewer-readonly` |
| EC-179-30 | Operator lock on extract, request empty | lock runs even if the offer withdrew that provider | `operator_lock_bypasses_offer` |
| EC-179-31 | Operator lock on extract, request pair set | 422 `usage_locked` | `operator_lock_refuses_request` |
| EC-179-32 | Refresh timeout | status `error`, no partial wipe | `refresh_timeout` |
| EC-179-33 | Concurrent offer PUT | loser gets 409, no merged row set | `stale_revision` |
| EC-179-34 | MCP retrieve with no pair | gate judges the stored query role | `mcp_judges_stored` |
| EC-179-35 | Parse endpoint names a provider | gate, not `ProviderFactory` | `parse_uses_gate` |

EC-179-13 and EC-179-33 share `stale_revision` because they are the same race
on two resources. The test hits both offer and policy.
