# 14 — E2E test matrix

Parent: [README](README.md) · Prev: [13-implementation-plan](13-implementation-plan.md) ·
Next: [15-cross-ref](15-cross-ref.md)

One primary gate per [EC-174](12-edge-cases.md). Names are the test
functions to add. Unit tests live next to `edgequake-core/src/clearance/`.
API tests follow `e2e_spec158_*` style under `edgequake/crates/edgequake-api/tests/`.
Playwright lives under `edgequake_webui/e2e/`.

| EC | Gate | Kind | Wave |
|----|------|------|------|
| EC-174-01 | `e2e_spec174_unverified_upload` | e2e | W4 |
| EC-174-02 | `e2e_spec174_no_security_role` | e2e | W3 |
| EC-174-03 | `clearance_deny_wins` | unit | W1 |
| EC-174-04 | `clearance_equal_rank` | unit | W1 |
| EC-174-05 | `clearance_subject_superset` | unit | W1 |
| EC-174-06 | `clearance_both_compartments` | unit + `e2e_spec174_both_compartments` | W1 / W5 |
| EC-174-07 | `e2e_spec174_steward_and_simulate_header` | e2e | W3 |
| EC-174-08 | `e2e_spec174_enable_bootstrap` | e2e | W4 |
| EC-174-09 | `e2e_spec174_uploader_after_raise` | e2e | W4 |
| EC-174-10 | `e2e_spec174_epoch_cache` | e2e | W7 |
| EC-174-11 | `e2e_spec174_tag_delete_restrict` | e2e | W4 |
| EC-174-12 | `e2e_spec174_null_rank_hidden` | e2e | W4 |
| EC-174-13 | `e2e_spec174_hidden_id_404` | e2e | W4 |
| EC-174-14 | `e2e_spec174_filter_intersection` | e2e | W5 |
| EC-174-15 | `e2e_spec174_empty_filter_is_visible_set` | e2e | W5 |
| EC-174-16 | `e2e_spec174_contribution_fragments` | e2e | W6 |
| EC-174-17 | `e2e_spec174_visible_degree` | e2e | W6 |
| EC-174-18 | `e2e_spec174_graph_png` | e2e | W6 |
| EC-174-19 | `e2e_spec174_graph_search` | e2e | W6 |
| EC-174-20 | `e2e_spec174_stats_visible` | e2e | W7 |
| EC-174-21 | `e2e_spec174_pipeline_events` | e2e | W7 |
| EC-174-22 | `e2e_spec174_task_error_redaction` | e2e | W7 |
| EC-174-23 | `e2e_spec174_history_redaction` | e2e | W7 |
| EC-174-24 | `e2e_spec174_share_refused` | e2e | W7 |
| EC-174-25 | `e2e_spec174_retrieval_id` | e2e | W7 |
| EC-174-26 | `e2e_spec174_answer_cache_prompt` | e2e | W7 |
| EC-174-27 | `e2e_spec174_context_cache_isolation` | e2e | W7 |
| EC-174-28 | `e2e_spec174_dedup_no_oracle` | e2e | W7 |
| EC-174-29 | `e2e_spec174_markdown_download` | e2e | W4 |
| EC-174-30 | `e2e_spec174_admin_master_key` | e2e | W3 |
| EC-174-31 | `e2e_spec174_readonly_clearance` | e2e | W4 |
| EC-174-32 | `e2e_spec174_api_key_scope` | e2e | W9 |
| EC-174-33 | `e2e_spec174_null_tenant_still_guarded` | e2e | W10 |
| EC-174-34 | `e2e_spec174_pool_guc_reset` | e2e | W10 |
| EC-174-35 | `e2e_spec174_enforcement_off` | e2e | W2 |
| EC-174-36 | `e2e_spec174_cross_workspace` | e2e | W10 |
| EC-174-37 | `e2e_spec174_bulk_upload` | e2e | W4 |
| EC-174-38 | `e2e_spec174_glossary` | e2e | W7 |
| EC-174-39 | `e2e_spec174_facet_compartments` | e2e | W10 |
| EC-174-40 | `e2e_spec174_no_kv_on_deny` | e2e | W4 |
| EC-174-41 | `e2e_spec174_enable_flag_last` | e2e | W4 |
| EC-174-42 | `e2e_spec174_legacy_description` | e2e | W6 |

## Playwright (W8)

| Flow | Spec |
|------|------|
| HR user does not see the R&D title on list, detail URL, or graph | `edgequake_webui/e2e/document-clearance.spec.ts` |
| Enable dialog is one sentence and Turn on, with no checkbox | same file |
| Empty state for a member with no security role has no document count | same file |
| Upload select does not offer a rank above the actor | same file |

The detail URL test asserts the not-found copy and asserts the hidden
title string is absent.

## 404 parity

`e2e_spec174_hidden_id_404` records the JSON for a random uuid and for a
hidden but existing id, on each route in [EC-174-13](12-edge-cases.md),
and compares them. A field that appears on only one of the two is a failure.

## What is not a substitute

An EXPLAIN plan ([16-performance](16-performance.md)) does not replace a
leak test. A unit test of `allows` does not replace
`e2e_spec174_hidden_id_404`. Acc staying green does not prove enforcement
on; `e2e_spec174_enforcement_off` is the Acc-shaped gate, run without the
medical corpus.

Next: [15-cross-ref](15-cross-ref.md).
