# 13 — E2E test matrix

Parent: [README](README.md) · Prev: [12-implementation-plan](12-implementation-plan.md) ·
Next: [14-cross-ref](14-cross-ref.md)

One primary gate per [EC-175](11-edge-cases.md). The wave is when the
test must exist, from [12-implementation-plan](12-implementation-plan.md).

| EC | Gate | Kind | Wave | Name |
|----|------|------|------|------|
| 01 | Lock beats workspace extract role | unit | W1 | `usage_lock_beats_workspace_role` |
| 02 | Acc pin, empty metadata | unit | W1 | `usage_extract_pin_without_metadata` |
| 03 | Eligible request beats query connection | API | W1 | `e2e_spec175_request_beats_connection` |
| 04 | Revoked key is 503, process key unused | API | W1 | `e2e_spec175_fail_closed_credential` |
| 05 | Ceiling removal pauses that usage | API | W2 | `e2e_spec175_ceiling_breaks_assignment` |
| 06 | Embedding id on extract is 422 | API | W2 | `e2e_spec175_capability_mismatch` |
| 07 | Custom id hidden until probe ok | API | W4 | `e2e_spec175_custom_model_probe` |
| 08 | Decision model absent from query picker | Playwright | W5 | `spec175-decision-not-in-query-chip` |
| 09 | `rerank_model` 422 and no personal switch | API + Playwright | W2, W4 | `e2e_spec175_rerank_model_rejected` |
| 10 | Discovery off uses static cards | unit | W2 | `eligible_static_when_discovery_off` |
| 11 | Unknown OpenAI id hidden; no gpt-4o-mini snippet | API + Playwright | W4 | `spec175-no-stale-model-snippet` |
| 12 | Hybrid embed is env bootstrap | unit | W1 | `usage_embedding_env_is_bootstrap` |
| 13 | Priority mode does not beat tenant | unit | W1 | `usage_priority_swaps_leaf_only` |
| 14 | No omit-temperature row | Playwright | W5 | `spec175-no-transport-flag-row` |
| 15 | Cross-tenant connection 404 | API | W2 | `e2e_spec175_connection_other_tenant` |
| 16 | Member sees narrowed set, 403 on write | Playwright | W4 | `spec175-member-picker-ceiling` |
| 17 | Header and You language agree | Playwright | W5 | `spec175-language-one-writer` |
| 18 | Model change does not reprocess | API | W5 | `e2e_spec175_no_auto_reprocess` |
| 19 | Zero policy rows unconstrained | API | W2 | `e2e_spec175_empty_policy` |
| 20 | Keyword pin locks the role | unit | W1 | `usage_keyword_pin_is_lock` |
| 21 | Manuscript env locks manuscript pages only | unit | W1 | `usage_manuscript_lock_scoped` |
| 22 | Upload vision beats vlm role | unit | W1 | `usage_upload_vision_is_request` |
| 23 | No gleaning control; old persist ignored | Playwright | W4 | `spec175-gleaning-removed` |
| 24 | `/w/{slug}/settings` renders the shell | Playwright | W3 | `spec175-deeplink-settings` |
| 25 | Fleet connection loads; key stays hidden | API | W2 | `e2e_spec175_fleet_connection` |
| 26 | Dimension change sets `rebuild_required` | API | W5 | `e2e_spec175_embedding_rebuild_flag` |
| 27 | Answer cache misses across models | unit | W1 | `answer_cache_key_includes_model` |
| 28 | Fresh client omits provider; switch clears sticker | Playwright | W5 | `spec175-query-inherit` |
| 29 | One provider health region | Playwright | W3 | `spec175-one-health-view` |
| 30 | Workspace PUT cannot widen | API | W2 | `e2e_spec175_no_widen` |
| 31 | Readonly cannot assign | API | W2 | `e2e_spec175_readonly_forbidden` |
| 32 | Probe failure does not run compiled default | API | W1 | `e2e_spec175_no_compiled_fallback` |
| 33 | Search "embedding" opens the row | Playwright | W4 | `spec175-settings-search` |
| 34 | Tenant admin does not see Deployment | Playwright | W3 | `spec175-deployment-hidden` |
| 35 | Graph popover title and fields | Playwright | W5 | `spec175-graph-window` |
| 36 | Workspace Admin cannot write the ceiling | API | W2 | `e2e_spec175_workspace_admin_cannot_write_ceiling` |
| 37 | Super Admin assigns any workspace | API | W2 | `e2e_spec175_super_admin_any_workspace` |
| 38 | Last tenant Owner cannot be removed | API | W2 | `e2e_spec175_last_tenant_owner` |
| 39 | Other workspace shows the member models page | Playwright | W3 | `spec175-workspace-admin-other-workspace-readonly` |
| 40 | Platform admin chip is not Super admin | Playwright | W3 | `spec175-platform-badge-not-super` |
| 41 | SSO max_role cannot mint a tenant Owner | API | W2 | `e2e_spec175_sso_max_role_no_owner` |
| 42 | Workspace Admin cannot write another workspace | API | W2 | `e2e_spec175_other_workspace_forbidden` |
| 43 | Organization URL and search stay hidden | Playwright | W3 | `spec175-organization-url-hidden` |
| 44 | Workspace Admin cannot invite or create a workspace | API | W2 | `e2e_spec175_workspace_admin_no_invite` |
| 45 | Other tenant's policy id is 404 | API | W2 | `e2e_spec175_cross_tenant_policy_404` |
| 46 | Tenant-wide member is not Super Admin | Playwright | W3 | `spec175-member-grant-not-super-admin` |
| 47 | Inactive membership grants nothing | API | W2 | `e2e_spec175_inactive_membership` |
| 48 | Ceiling PUT replaces and does not merge | API | W2 | `e2e_spec175_policy_replace_no_merge` |
| 49 | GET ceiling uses the manage gate | API | W2 | `e2e_spec175_get_ceiling_forbidden` |
| 50 | Last Workspace Admin may be removed | API | W2 | `e2e_spec175_last_workspace_admin_removable` |
| 51 | Binding off still blocks the ceiling | API | W2 | `e2e_spec175_binding_off_ceiling` |
| 52 | Master API key is not a Super Admin | API | W2 | `e2e_spec175_master_key_not_super_admin` |
| 53 | Allow-list and stale probe beat a ceiling row | API | W2 | `e2e_spec175_catalog_filter_before_policy` |
| 54 | Extract ignores echoed request model fields | unit | W1 | `usage_extract_ignores_request_pair` |

W6 runs 04, 05, 16, and 01 together as the demo script
`spec175-owner-ceiling-member-lock`. It does not replace the rows above.

Existing specs to retarget, not delete:

- `edgequake_webui/e2e/spec100-settings-cls.spec.ts` — shell layout
- `edgequake_webui/e2e/spec124-langfuse-settings.spec.ts` — Deployment observability
- `edgequake_webui/e2e/costs-and-settings.spec.ts` — section move
- `edgequake_webui/e2e/spec037-query-settings-scroll.spec.ts` — chip still on the query page

## Status (2026-10-25)

Named tests that exist in the tree. "Partial" means the UI or unit half exists and the API or
Playwright half named in the row does not.

| EC | Status | Where |
|----|--------|-------|
| 01 | Done (unit) | `usage_resolution::tests::usage_lock_beats_workspace_role` |
| 02 | Done (unit) | `usage_extract_pin_without_metadata` |
| 03 | Not started | needs W2 eligible set |
| 04 | Not started | needs W1 call-site switch |
| 05, 06, 07, 08 | Not started | W2 / W4 / W5 |
| 09 | Partial | API: `services::query_execution::tests::reject_rerank_model_is_422_with_stable_code`; personal rerank removed from the UI |
| 10, 11 | Not started | W2 / W4 |
| 12 | Done (unit) | `usage_embedding_env_is_bootstrap` |
| 13 | Done (unit) | `usage_priority_swaps_leaf_only` |
| 14, 15 | Not started | W5 / W2 |
| 16 | Partial | Playwright: `a member reads the models matrix and cannot save it`; narrowed picker not built |
| 17, 18, 19 | Not started | W5 / W2 |
| 20 | Done (unit) | `usage_keyword_pin_is_lock` |
| 21 | Done (unit) | `usage_manuscript_lock_scoped` |
| 22 | Done (unit) | `usage_upload_vision_is_request` |
| 23 | Partial | controls removed; no Playwright gate yet; dead locale keys removed |
| 24 | Done | Playwright: `the workspace deeplink shows Models, not /workspace` |
| 25, 26 | Not started | W2 / W5 |
| 27 | Done | `usage_resolution::tests::answer_cache_key_includes_model`; `llm_response_cache::tests::answer_cache_key_changes_with_model` (hash now includes the answering model) |
| 28 | Not started | W5 |
| 29 | Done | Playwright: `one provider health region on the fleet page` |
| 30, 31 | Partial | UI read-only done; API 403 on write not yet verified |
| 32 | Not started | the probe-failure 503 and the removal of `ProviderFactory::from_env` fallback are W1 call-site work |
| 33 | Not started | Playwright search for `embedding` |
| 34 | Done | Playwright: `a workspace admin cannot reach Deployment by URL or search` |
| 35 | Not started | W5 |
| 36, 37, 38 | Not started | W2 |
| 39 | Done | Playwright: `a workspace admin of another workspace sees this one read-only` |
| 40 | Done | Playwright: `platform admin badge says Platform admin, never Super admin` |
| 41, 42, 44, 45 | Not started | W2 |
| 43 | Done | Playwright: `a member cannot reach Organization by URL or search` |
| 46 | Partial | badge shows Member; no dedicated gate |
| 47–53 | Not started | W2 |
| 54 | Done (unit) | `usage_extract_ignores_request_pair` |

The `usage_resolution` module is the pure resolver of LAW-175-1. Its call sites are not switched
yet: extract, query, summary, vlm, keyword, embedding, and decision still run their own ladders
(`resolve_extract_role_llm`, `resolve_llm_provider_for_workspace`, `resolve_workspace_vlm_config`).
