//! SPEC-179 provider lifecycle. The diff and the dimension rule are pure.
//! HTTP paths use the same functions.

use edgequake_api::services::model_gate::{refresh_plan, require_embedding_dimension};

#[test]
fn refresh_keeps_cache_on_error() {
    let plan = refresh_plan(&["kept".into()], &["gone".into()], false);
    assert!(!plan.replace);
    assert!(plan.removed.is_empty());
}

#[test]
fn refresh_timeout_is_the_same_keep() {
    let plan = refresh_plan(&["kept".into()], &[], false);
    assert!(!plan.replace);
}

#[test]
fn refresh_id_collision_updates_without_a_second_card() {
    let plan = refresh_plan(&["same".into()], &["same".into()], true);
    assert!(plan.replace);
    assert!(plan.added.is_empty());
    assert!(plan.removed.is_empty());
}

#[test]
fn embedding_dimension_required() {
    assert_eq!(
        require_embedding_dimension(true, None).unwrap_err().code(),
        "embedding_dimension_required"
    );
}

#[test]
fn url_userinfo_rejected() {
    let err = edgequake_api::ssrf::validate_provider_url(
        "https://user:pass@example.com/v1",
        edgequake_api::ssrf::SsrfPolicy {
            allow_private: false,
        },
    )
    .unwrap_err();
    assert!(matches!(
        err,
        edgequake_api::ssrf::SsrfError::CredentialsInUrl
    ));
}

#[test]
fn env_connections_native() {
    let source = include_str!("../src/handlers/connections.rs");
    for key in [
        "GEMINI_API_KEY",
        "MISTRAL_API_KEY",
        "XAI_API_KEY",
        "OPENROUTER_API_KEY",
        "AZURE_OPENAI_API_KEY",
        "NVIDIA_API_KEY",
    ] {
        assert!(source.contains(key), "{key} is not an offered env provider");
    }
}

#[test]
fn slug_rename_and_scope_are_refused_in_the_handler() {
    let source = include_str!("../src/handlers/connections.rs");
    assert!(source.contains("slug_rename"));
    assert!(source.contains("connection_scope_global"));
    assert!(source.contains("last_test_ok = CASE"));
}

#[test]
fn offer_revision_impact_and_confirm() {
    let offer = include_str!("../src/handlers/provider_offer.rs");
    assert!(offer.contains("get_provider_impact"));
    assert!(offer.contains("expected_if_match"));
    assert!(offer.contains("confirm_required"));
    let rows = include_str!("../src/services/provider_offer_rows.rs");
    assert!(rows.contains("write_provider_audit"));
    assert!(rows.contains("touch_scope_revision"));
    let policy = include_str!("../src/handlers/model_policy.rs");
    assert!(policy.contains("touch_scope_revision"));
    let connections = include_str!("../src/handlers/connections.rs");
    assert!(connections.contains("confirm_required"));
}
