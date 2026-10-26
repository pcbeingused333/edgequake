//! SPEC-176 secret backend and secret metadata.
//!
//! Admin only. Values are write-only. The database backend is the default, so a
//! deployment with no Infisical settings stores keys locally.

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::error::{ApiError, ApiResult};
use crate::handlers::auth::ApiRequireAdmin;
use crate::handlers::connections::scoped_tenant;
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct PutSecretBackend {
    pub kind: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub client_secret: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub environment: Option<String>,
    #[serde(default)]
    pub secret_path: Option<String>,
    #[serde(default)]
    pub allow_private_network: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PutSecret {
    pub name: String,
    pub value: String,
}

#[cfg(feature = "postgres")]
fn pool(state: &AppState) -> Result<sqlx::PgPool, ApiError> {
    state
        .pg_pool
        .clone()
        .ok_or_else(|| ApiError::ServiceUnavailable {
            message: "PostgreSQL required".into(),
            retry_after_secs: 5,
        })
}

#[utoipa::path(get, path = "/api/v1/secret-backend", tag = "Secrets")]
pub async fn get_secret_backend(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
    tenant_ctx: crate::middleware::TenantContext,
) -> ApiResult<Json<serde_json::Value>> {
    #[cfg(feature = "postgres")]
    {
        let view = crate::providers::secret_manager::SecretService::new(pool(&state)?)
            .view(scoped_tenant(&tenant_ctx)?)
            .await?;
        Ok(Json(serde_json::to_value(view).unwrap_or_default()))
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, tenant_ctx);
        Err(ApiError::ServiceUnavailable {
            message: "PostgreSQL required".into(),
            retry_after_secs: 5,
        })
    }
}

#[utoipa::path(put, path = "/api/v1/secret-backend", tag = "Secrets")]
pub async fn put_secret_backend(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
    tenant_ctx: crate::middleware::TenantContext,
    Json(body): Json<PutSecretBackend>,
) -> ApiResult<Json<serde_json::Value>> {
    #[cfg(feature = "postgres")]
    {
        let view = crate::providers::secret_manager::SecretService::new(pool(&state)?)
            .set_backend(
                scoped_tenant(&tenant_ctx)?,
                crate::providers::secret_manager::SetBackend {
                    kind: body.kind,
                    base_url: body.base_url,
                    client_id: body.client_id,
                    client_secret: body.client_secret,
                    project_id: body.project_id,
                    environment: body.environment,
                    secret_path: body.secret_path,
                    allow_private_network: body.allow_private_network.unwrap_or(false),
                },
            )
            .await?;
        Ok(Json(serde_json::to_value(view).unwrap_or_default()))
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, tenant_ctx, body);
        Err(ApiError::ServiceUnavailable {
            message: "PostgreSQL required".into(),
            retry_after_secs: 5,
        })
    }
}

#[utoipa::path(post, path = "/api/v1/secret-backend/test", tag = "Secrets")]
pub async fn test_secret_backend(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
    tenant_ctx: crate::middleware::TenantContext,
) -> ApiResult<Json<serde_json::Value>> {
    #[cfg(feature = "postgres")]
    {
        crate::providers::secret_manager::SecretService::new(pool(&state)?)
            .test(scoped_tenant(&tenant_ctx)?)
            .await?;
        Ok(Json(serde_json::json!({ "ok": true })))
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, tenant_ctx);
        Err(ApiError::ServiceUnavailable {
            message: "PostgreSQL required".into(),
            retry_after_secs: 5,
        })
    }
}

#[utoipa::path(get, path = "/api/v1/secrets", tag = "Secrets")]
pub async fn list_secrets(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
    tenant_ctx: crate::middleware::TenantContext,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    #[cfg(feature = "postgres")]
    {
        let rows = crate::providers::secret_manager::SecretService::new(pool(&state)?)
            .list(scoped_tenant(&tenant_ctx)?)
            .await?;
        let body = rows
            .into_iter()
            .map(|row| {
                serde_json::json!({
                    "name": row.name,
                    "fingerprint": row.fingerprint,
                    "backend": row.backend,
                })
            })
            .collect();
        Ok(Json(body))
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, tenant_ctx);
        Err(ApiError::ServiceUnavailable {
            message: "PostgreSQL required".into(),
            retry_after_secs: 5,
        })
    }
}

#[utoipa::path(post, path = "/api/v1/secrets", tag = "Secrets")]
pub async fn create_secret(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
    tenant_ctx: crate::middleware::TenantContext,
    Json(body): Json<PutSecret>,
) -> ApiResult<Json<serde_json::Value>> {
    #[cfg(feature = "postgres")]
    {
        let stored = crate::providers::secret_manager::SecretService::new(pool(&state)?)
            .put(scoped_tenant(&tenant_ctx)?, &body.name, &body.value)
            .await?;
        Ok(Json(serde_json::json!({
            "name": body.name,
            "backend": stored.backend_name(),
        })))
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, tenant_ctx, body);
        Err(ApiError::ServiceUnavailable {
            message: "PostgreSQL required".into(),
            retry_after_secs: 5,
        })
    }
}

#[utoipa::path(delete, path = "/api/v1/secrets/{name}", tag = "Secrets")]
pub async fn delete_secret(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
    tenant_ctx: crate::middleware::TenantContext,
    Path(name): Path<String>,
) -> Result<axum::http::StatusCode, ApiError> {
    #[cfg(feature = "postgres")]
    {
        crate::providers::secret_manager::SecretService::new(pool(&state)?)
            .delete_named(scoped_tenant(&tenant_ctx)?, &name)
            .await?;
        Ok(axum::http::StatusCode::NO_CONTENT)
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, tenant_ctx, name);
        Err(ApiError::ServiceUnavailable {
            message: "PostgreSQL required".into(),
            retry_after_secs: 5,
        })
    }
}
