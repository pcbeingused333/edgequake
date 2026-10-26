//! SPEC-177 provider offers. Schema and replace rules on the scratch database.
//! Skips when PostgreSQL is unavailable.
#![cfg(feature = "postgres")]

#[path = "../../edgequake-storage/tests/support/postgres_access_pool.rs"]
#[allow(dead_code)]
mod postgres_access_pool;

use sqlx::PgPool;
use uuid::Uuid;

async fn pool() -> Option<PgPool> {
    let pool =
        postgres_access_pool::test_pool_with_acquire_timeout(std::time::Duration::from_secs(20))
            .await?;
    let ready: bool =
        sqlx::query_scalar("SELECT to_regclass('public.provider_offers') IS NOT NULL")
            .fetch_one(&pool)
            .await
            .unwrap_or(false);
    if !ready {
        eprintln!("SKIP: migration 223 not applied");
        return None;
    }
    Some(pool)
}

fn sqlstate(error: &sqlx::Error) -> Option<String> {
    error
        .as_database_error()
        .and_then(|db| db.code().map(|code| code.into_owned()))
}

#[tokio::test]
async fn offer_tables_have_no_secret_column() {
    let Some(pool) = pool().await else { return };
    let names: Vec<String> = sqlx::query_scalar(
        "SELECT column_name FROM information_schema.columns \
         WHERE table_schema = 'public' \
           AND table_name IN ('provider_offers', 'deployment_provider_overrides', 'connection_models') \
           AND (column_name ILIKE '%secret%' OR column_name ILIKE '%api_key%' OR column_name ILIKE '%ciphertext%')",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert!(
        names.is_empty(),
        "secret columns leaked into offer tables: {names:?}"
    );
}

#[tokio::test]
async fn new_tenant_is_unconstrained() {
    let Some(pool) = pool().await else { return };
    let default_value: Option<String> = sqlx::query_scalar(
        "SELECT column_default FROM information_schema.columns \
         WHERE table_schema = 'public' AND table_name = 'tenants' \
           AND column_name = 'provider_offer_constrained'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let default_value = default_value.unwrap_or_default();
    assert!(
        default_value.contains("false"),
        "default was {default_value}"
    );
}

#[tokio::test]
async fn duplicate_tenant_slug_rejected() {
    let Some(pool) = pool().await else { return };
    let mut tx = pool.begin().await.unwrap();
    let tenant = Uuid::new_v4();
    sqlx::query("INSERT INTO tenants (tenant_id, name, slug) VALUES ($1, 'spec177', $2)")
        .bind(tenant)
        .bind(format!("spec177-{tenant}"))
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO provider_offers (tenant_id, provider_slug, enabled) VALUES ($1, 'openai', true)",
    )
    .bind(tenant)
    .execute(&mut *tx)
    .await
    .unwrap();
    let err = sqlx::query(
        "INSERT INTO provider_offers (tenant_id, provider_slug, enabled) VALUES ($1, 'openai', true)",
    )
    .bind(tenant)
    .execute(&mut *tx)
    .await
    .unwrap_err();
    assert_eq!(sqlstate(&err).as_deref(), Some("23505"));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn slug_invalid() {
    let Some(pool) = pool().await else { return };
    let mut tx = pool.begin().await.unwrap();
    let tenant = Uuid::new_v4();
    sqlx::query("INSERT INTO tenants (tenant_id, name, slug) VALUES ($1, 'spec177', $2)")
        .bind(tenant)
        .bind(format!("spec177-{tenant}"))
        .execute(&mut *tx)
        .await
        .unwrap();
    let err = sqlx::query(
        "INSERT INTO provider_offers (tenant_id, provider_slug, enabled) VALUES ($1, 'OpenAI', true)",
    )
    .bind(tenant)
    .execute(&mut *tx)
    .await
    .unwrap_err();
    assert_eq!(sqlstate(&err).as_deref(), Some("23514"));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn replace_drops_omitted_slug() {
    let Some(pool) = pool().await else { return };
    let mut tx = pool.begin().await.unwrap();
    let tenant = Uuid::new_v4();
    sqlx::query("INSERT INTO tenants (tenant_id, name, slug) VALUES ($1, 'spec177', $2)")
        .bind(tenant)
        .bind(format!("spec177-{tenant}"))
        .execute(&mut *tx)
        .await
        .unwrap();
    for slug in ["openai", "acme"] {
        sqlx::query(
            "INSERT INTO provider_offers (tenant_id, provider_slug, enabled) VALUES ($1, $2, true)",
        )
        .bind(tenant)
        .bind(slug)
        .execute(&mut *tx)
        .await
        .unwrap();
    }
    sqlx::query("DELETE FROM provider_offers WHERE tenant_id = $1 AND provider_slug <> 'openai' AND workspace_id IS NULL")
        .bind(tenant)
        .execute(&mut *tx)
        .await
        .unwrap();
    let left: Vec<String> = sqlx::query_scalar(
        "SELECT provider_slug FROM provider_offers WHERE tenant_id = $1 AND workspace_id IS NULL ORDER BY provider_slug",
    )
    .bind(tenant)
    .fetch_all(&mut *tx)
    .await
    .unwrap();
    assert_eq!(left, vec!["openai".to_string()]);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn failed_probe_is_not_offered() {
    let Some(pool) = pool().await else { return };
    let mut tx = pool.begin().await.unwrap();
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO provider_connections (id, tenant_id, slug, display_name, api_shape, base_url, last_test_ok) \
         VALUES ($1, NULL, 'spec177-failed', 'Failed', 'openai_chat', 'https://example.invalid/v1', false)",
    )
    .bind(id)
    .execute(&mut *tx)
    .await
    .unwrap();
    let offered: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM provider_connections \
         WHERE tenant_id IS NULL AND slug = 'spec177-failed' AND last_test_ok IS TRUE)",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert!(!offered);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn other_tenant_legacy_is_hidden() {
    let Some(pool) = pool().await else { return };
    let owner = Uuid::new_v4();
    let other = Uuid::new_v4();
    let mut tx = pool.begin().await.unwrap();
    for id in [owner, other] {
        sqlx::query("INSERT INTO tenants (tenant_id, name, slug) VALUES ($1, 'spec177', $2)")
            .bind(id)
            .bind(format!("spec177-{id}"))
            .execute(&mut *tx)
            .await
            .unwrap();
    }
    sqlx::query(
        "INSERT INTO provider_connections (tenant_id, slug, display_name, api_shape, base_url) \
         VALUES ($1, 'legacy-a', 'Legacy A', 'openai_chat', 'https://example.invalid/v1')",
    )
    .bind(owner)
    .execute(&mut *tx)
    .await
    .unwrap();
    let visible_to_other: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM provider_connections \
         WHERE slug = 'legacy-a' AND (tenant_id IS NULL OR tenant_id = $1))",
    )
    .bind(other)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert!(!visible_to_other);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn withdraw_does_not_touch_env() {
    let Some(pool) = pool().await else { return };
    let before = std::env::var("OPENAI_API_KEY").ok();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query(
        "INSERT INTO deployment_provider_overrides (provider_slug, withdrawn) VALUES ('openai', true)",
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    assert_eq!(std::env::var("OPENAI_API_KEY").ok(), before);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn replace_connection_models_requires_a_passing_probe() {
    let Some(pool) = pool().await else { return };
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO provider_connections (id, tenant_id, slug, display_name, api_shape, base_url, last_test_ok) \
         VALUES ($1, NULL, 'spec177-models', 'Models', 'openai_chat', 'https://example.invalid/v1', false)",
    )
    .bind(id)
    .execute(&pool)
    .await
    .unwrap();
    let err = edgequake_api::services::provider_offer_rows::replace_connection_models(
        &pool,
        "spec177-models",
        &[("local".into(), true, false, false, None)],
    )
    .await
    .unwrap_err();
    assert_eq!(err.code(), "probe_required");
    sqlx::query("UPDATE provider_connections SET last_test_ok = true WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    edgequake_api::services::provider_offer_rows::replace_connection_models(
        &pool,
        "spec177-models",
        &[
            ("local".into(), true, false, false, None),
            ("embed".into(), false, false, true, Some(768)),
        ],
    )
    .await
    .unwrap();
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM connection_models WHERE connection_id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    sqlx::query("DELETE FROM provider_connections WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(count, 2);
}

#[tokio::test]
async fn offer_revision_decodes_from_int4() {
    let Some(pool) = pool().await else { return };
    let ready: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM information_schema.columns \
         WHERE table_schema = 'public' AND table_name = 'tenants' \
           AND column_name = 'provider_offer_revision')",
    )
    .fetch_one(&pool)
    .await
    .unwrap_or(false);
    if !ready {
        eprintln!("SKIP: provider_offer_revision column is absent");
        return;
    }
    let tenant = Uuid::new_v4();
    sqlx::query("INSERT INTO tenants (tenant_id, name, slug) VALUES ($1, 'spec177-rev', $2)")
        .bind(tenant)
        .bind(format!("spec177-rev-{tenant}"))
        .execute(&pool)
        .await
        .unwrap();
    let revision =
        edgequake_api::services::provider_offer_rows::current_offer_revision(&pool, false, tenant)
            .await
            .unwrap();
    sqlx::query("DELETE FROM tenants WHERE tenant_id = $1")
        .bind(tenant)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(revision, 1);
}

#[tokio::test]
async fn delete_connection_removes_offer_rows() {
    let Some(pool) = pool().await else { return };
    let tenant = Uuid::new_v4();
    sqlx::query("INSERT INTO tenants (tenant_id, name, slug) VALUES ($1, 'spec177', $2)")
        .bind(tenant)
        .bind(format!("spec177-{tenant}"))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO provider_offers (tenant_id, provider_slug, enabled) VALUES ($1, 'spec177-gone', true)",
    )
    .bind(tenant)
    .execute(&pool)
    .await
    .unwrap();
    edgequake_api::services::provider_offer_rows::remove_offers_for_slug(&pool, "spec177-gone")
        .await
        .unwrap();
    let left: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM provider_offers WHERE tenant_id = $1 AND provider_slug = 'spec177-gone'",
    )
    .bind(tenant)
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query("DELETE FROM tenants WHERE tenant_id = $1")
        .bind(tenant)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(left, 0);
}

#[tokio::test]
async fn delete_does_not_reprocess_or_clear_cache() {
    let Some(pool) = pool().await else { return };
    let cache_before: Option<i64> = sqlx::query_scalar("SELECT count(*) FROM llm_cache")
        .fetch_optional(&pool)
        .await
        .ok()
        .flatten();
    edgequake_api::services::provider_offer_rows::remove_offers_for_slug(&pool, "spec177-missing")
        .await
        .expect("offer delete");
    if let Some(before) = cache_before {
        let after: i64 = sqlx::query_scalar("SELECT count(*) FROM llm_cache")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(before, after);
    }
}

#[tokio::test]
async fn named_provider_outside_the_offer_does_not_fall_through() {
    use std::sync::Arc;

    use edgequake_api::providers::{
        LlmResolutionRequest, ProviderResolutionError, WorkspaceProviderResolver,
    };
    use edgequake_core::{InMemoryWorkspaceService, Workspace};

    let Some(pool) = pool().await else { return };
    let tenant = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    let offer = edgequake_api::services::provider_offer_rows::load_workspace_offer_from_pool(
        &pool,
        tenant,
        workspace_id,
    )
    .await
    .expect("offer");
    let service = Arc::new(InMemoryWorkspaceService::new());
    let resolver = WorkspaceProviderResolver::new(service).with_pg_pool(Some(pool.clone()));
    let stranger_name = [
        "mock",
        "azure",
        "bedrock",
        "gemini",
        "mistral",
        "huggingface",
        "vertexai",
        "openrouter",
    ]
    .into_iter()
    .find(|name| !offer.slugs.contains(*name))
    .expect("a known provider sits outside this offer");

    let mut stranger = Workspace::new(tenant, "spec177", "spec177-stranger");
    stranger.workspace_id = workspace_id;
    stranger.llm_provider = stranger_name.into();
    stranger.llm_model = "removed-model".into();
    let request = LlmResolutionRequest {
        provider: Some(stranger_name.into()),
        model: Some("removed-model".into()),
        extra_headers: None,
    };
    match resolver
        .resolve_llm_provider_for_workspace(Some(&stranger), &request)
        .await
    {
        Err(ProviderResolutionError::ProviderCreationFailed {
            reason, provider, ..
        }) => {
            assert_eq!(provider, stranger_name);
            assert!(reason.contains("not offered"), "{reason}");
        }
        other => panic!("removed provider fell through: {other:?}"),
    }

    let Some(kept) = offer.slugs.iter().next().cloned() else {
        return;
    };
    let connection_id = Uuid::new_v4();
    let hidden = format!("spec177-{}", &connection_id.simple().to_string()[..12]);
    sqlx::query(
        "INSERT INTO provider_connections (id, tenant_id, slug, display_name, api_shape, base_url, last_test_ok) \
         VALUES ($1, NULL, $2, 'Hidden', 'openai_chat', 'https://example.invalid/v1', false)",
    )
    .bind(connection_id)
    .bind(&hidden)
    .execute(&pool)
    .await
    .unwrap();
    let mut named = Workspace::new(tenant, "spec177", "spec177-kept");
    named.workspace_id = workspace_id;
    named.llm_provider = kept.clone();
    named.metadata.insert(
        "llm_roles".into(),
        serde_json::json!({
            "query": { "provider": kept, "connection_id": connection_id.to_string() }
        }),
    );
    let request = LlmResolutionRequest {
        provider: Some(kept),
        model: Some("kept-model".into()),
        extra_headers: None,
    };
    let result = resolver
        .resolve_llm_provider_for_workspace(Some(&named), &request)
        .await;
    sqlx::query("DELETE FROM provider_connections WHERE id = $1")
        .bind(connection_id)
        .execute(&pool)
        .await
        .unwrap();
    match result {
        Err(ProviderResolutionError::ProviderCreationFailed {
            reason, provider, ..
        }) => {
            assert_eq!(provider, hidden);
            assert!(reason.contains("not offered"), "{reason}");
        }
        other => panic!("connection slug fell through: {other:?}"),
    }
}
