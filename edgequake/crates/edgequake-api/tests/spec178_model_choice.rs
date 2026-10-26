//! SPEC-178 choice coherence. Pure gates that do not need a live server.
//! HTTP doors share `ensure_resolved_pair`; the unit tests for that verdict
//! live in `model_policy_enforcement`.

use std::path::PathBuf;

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(crate_root().join(rel)).unwrap_or_default()
}

#[test]
fn spec178_adds_no_migration() {
    let migrations = crate_root().join("../../migrations");
    let hit = std::fs::read_dir(&migrations)
        .into_iter()
        .flatten()
        .flatten()
        .any(|entry| entry.file_name().to_string_lossy().contains("spec178"));
    assert!(!hit, "SPEC-178 must not add a migration");
}

#[test]
fn choice_handlers_do_not_retry_on_the_process_provider() {
    for rel in [
        "src/handlers/chat/completion.rs",
        "src/handlers/chat/streaming.rs",
        "src/handlers/query/query_execute.rs",
        "src/handlers/query/query_stream.rs",
    ] {
        let src = read(rel);
        assert!(
            !src.contains("with_auth_fallback"),
            "{rel} still falls through to the process provider"
        );
        assert!(
            src.contains("ensure_resolved_pair"),
            "{rel} must judge the pair that will run"
        );
    }
    for rel in [
        "src/services/query_context.rs",
        "src/services/query_generation.rs",
    ] {
        let src = read(rel);
        assert!(
            !src.contains("with_auth_fallback"),
            "{rel} still falls through to the process provider"
        );
        assert!(src.contains("execute_sota_query("), "{rel}");
    }
}

#[test]
fn stream_and_query_reject_the_same_pair() {
    let stream = read("src/handlers/chat/streaming.rs");
    let query = read("src/handlers/query/query_execute.rs");
    assert!(stream.contains("ensure_resolved_pair"));
    assert!(query.contains("ensure_resolved_pair"));
    assert!(stream.contains("Usage::Vlm") || stream.contains("edgequake_core::Usage::Vlm"));
}

#[test]
fn legacy_tenant_connection_absent_from_eligible() {
    let sql = read("src/services/model_policy_catalog.rs");
    assert!(
        sql.contains("WHERE c.tenant_id IS NULL"),
        "a tenant-scoped connection must stay out of the choice catalog"
    );
}

#[test]
fn offer_load_error_is_503() {
    let error = read("src/error.rs");
    assert!(error.contains("offer_unavailable"));
    assert!(error.contains("SERVICE_UNAVAILABLE"));
    assert!(read("src/providers/resolver.rs").contains("OfferUnavailable"));
}

#[test]
fn credential_miss_does_not_call_process_provider() {
    for rel in [
        "src/handlers/chat/completion.rs",
        "src/handlers/chat/streaming.rs",
        "src/handlers/query/query_execute.rs",
        "src/handlers/query/query_stream.rs",
    ] {
        let src = read(rel);
        assert!(
            src.contains("credential"),
            "{rel} must refuse a missing credential"
        );
        assert!(!src.contains("with_auth_fallback"), "{rel}");
    }
}

#[test]
fn member_scopes_are_gated() {
    let src = read("src/handlers/model_policy.rs");
    assert!(src.contains("TenantAccess::Manage"));
    assert!(src.contains("deployment_admin_required"));
    assert!(src.contains("Role::Admin"));
}

#[test]
fn embedding_save_does_not_enqueue_rebuild() {
    let src = read("src/handlers/workspaces/workspace_crud.rs");
    assert!(src.contains("ensure_workspace_choice"));
    assert!(!src.contains("rebuild"));
}

#[test]
fn setup_create_judges_the_tenant_pair() {
    let src = read("src/handlers/setup.rs");
    assert!(src.contains("ensure_tenant_choice"));
}
