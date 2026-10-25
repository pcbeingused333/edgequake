//! SPEC-175 W2: persistence for `model_catalog_policy` (tenant ceiling and workspace
//! narrowing). Reads and full-scope replaces only. Rules live in
//! `edgequake_core::model_policy`.

use edgequake_core::model_policy::PolicyRow;
use edgequake_core::Usage;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

/// Load rows for one scope. `workspace = None` returns the tenant ceiling.
#[cfg(feature = "postgres")]
pub async fn load_rows(
    state: &AppState,
    tenant: Uuid,
    workspace: Option<Uuid>,
) -> ApiResult<Vec<PolicyRow>> {
    let pool = state
        .optional_pg_pool()
        .ok_or_else(|| ApiError::Internal("PostgreSQL is required".into()))?;
    let rows: Vec<(String, String, String, bool)> = sqlx::query_as(
        "SELECT provider, model, usage, enabled FROM public.model_catalog_policy \
         WHERE tenant_id = $1 AND workspace_id IS NOT DISTINCT FROM $2 \
         ORDER BY usage, provider, model",
    )
    .bind(tenant)
    .bind(workspace)
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;
    rows.into_iter().map(stored_row).collect()
}

#[cfg(not(feature = "postgres"))]
pub async fn load_rows(
    _state: &AppState,
    _tenant: Uuid,
    _workspace: Option<Uuid>,
) -> ApiResult<Vec<PolicyRow>> {
    Err(ApiError::Internal("PostgreSQL is required".into()))
}

/// Replace every row of one scope in a single transaction. A unique or foreign-key
/// violation rolls back, so the previous rows stay (SPEC-175 `08-api-contract.md`).
#[cfg(feature = "postgres")]
pub async fn replace_rows(
    state: &AppState,
    tenant: Uuid,
    workspace: Option<Uuid>,
    rows: &[PolicyRow],
) -> ApiResult<()> {
    let pool = state
        .optional_pg_pool()
        .ok_or_else(|| ApiError::Internal("PostgreSQL is required".into()))?;
    let mut tx = pool.begin().await.map_err(internal)?;
    sqlx::query(
        "DELETE FROM public.model_catalog_policy \
         WHERE tenant_id = $1 AND workspace_id IS NOT DISTINCT FROM $2",
    )
    .bind(tenant)
    .bind(workspace)
    .execute(&mut *tx)
    .await
    .map_err(internal)?;
    for row in rows {
        sqlx::query(
            "INSERT INTO public.model_catalog_policy \
             (tenant_id, workspace_id, provider, model, usage, enabled) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(tenant)
        .bind(workspace)
        .bind(&row.provider)
        .bind(&row.model)
        .bind(row.usage.as_str())
        .bind(row.enabled)
        .execute(&mut *tx)
        .await
        .map_err(write_error)?;
    }
    tx.commit().await.map_err(internal)
}

#[cfg(not(feature = "postgres"))]
pub async fn replace_rows(
    _state: &AppState,
    _tenant: Uuid,
    _workspace: Option<Uuid>,
    _rows: &[PolicyRow],
) -> ApiResult<()> {
    Err(ApiError::Internal("PostgreSQL is required".into()))
}

#[cfg(feature = "postgres")]
fn stored_row(
    (provider, model, usage, enabled): (String, String, String, bool),
) -> ApiResult<PolicyRow> {
    let usage = Usage::from_wire(&usage)
        .ok_or_else(|| ApiError::Internal(format!("unknown stored usage '{usage}'")))?;
    Ok(PolicyRow {
        provider,
        model,
        usage,
        enabled,
    })
}

#[cfg(feature = "postgres")]
fn internal(error: sqlx::Error) -> ApiError {
    ApiError::Internal(error.to_string())
}

/// 23503 (foreign key): the tenant or workspace does not exist, or the workspace belongs
/// to another tenant. 23505 (unique): a concurrent write. Both fail closed instead of
/// returning a raw database error.
#[cfg(feature = "postgres")]
fn write_error(error: sqlx::Error) -> ApiError {
    match error.as_database_error().and_then(|db| db.code()) {
        Some(code) if code == "23503" => ApiError::NotFound("tenant or workspace".into()),
        Some(code) if code == "23505" => {
            ApiError::Conflict("model policy changed concurrently; retry".into())
        }
        _ => internal(error),
    }
}
