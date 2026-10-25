//! SPEC-175 W2: model catalog policy HTTP API.
//!
//! Handlers authorize, validate, then persist. Eligibility and widening checks are
//! pure functions in `edgequake_core::model_policy`.
//!
//! `/api/v1/tenants` and `/api/v1/models` are global resources (`bind_request` does
//! not bind them), so the tenant and eligible handlers call their own checks.

use std::collections::{BTreeSet, HashMap};

use axum::extract::{Path, Query, State};
use axum::Json;
use edgequake_core::model_policy::{
    eligible_models, policy_mode, usages_left_empty, widening_rows, CatalogEntry, PolicyRow,
};
use edgequake_core::Workspace;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::handlers::auth::ApiAuthenticated;
use crate::handlers::me_settings_access::{caller_scope, CallerScope};
use crate::handlers::model_policy_types::{
    catalog_view, parse_configurable_usage, parse_policy_rows, EligibleModelDto,
    EligibleModelsResponse, EligibleQuery, PolicyRowDto, PutPolicyRequest, TenantCatalogResponse,
    TenantPolicyResponse, WorkspacePolicyResponse,
};
use crate::handlers::workspaces::tenant_access::{require_tenant_access, TenantAccess};
use crate::services::model_policy_catalog::{load_catalog, CatalogItem};
use crate::services::model_policy_rows::{load_rows, replace_rows};
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Tenant ceiling
// ---------------------------------------------------------------------------

/// `GET /api/v1/tenants/{tenant_id}/model-policy`: ceiling rows (Super Admin only).
#[utoipa::path(
    get,
    path = "/api/v1/tenants/{tenant_id}/model-policy",
    tag = "Models",
    params(("tenant_id" = Uuid, Path, description = "Tenant id")),
    responses(
        (status = 200, description = "Tenant model ceiling", body = TenantPolicyResponse),
        (status = 403, description = "Tenant owner or admin required"),
    )
)]
pub async fn get_tenant_model_policy(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(tenant_id): Path<Uuid>,
) -> ApiResult<Json<TenantPolicyResponse>> {
    require_tenant_access(&state, &auth, tenant_id, TenantAccess::Manage).await?;
    let rows = load_rows(&state, tenant_id, None).await?;
    Ok(Json(tenant_view(tenant_id, &rows)))
}

/// `GET /api/v1/tenants/{tenant_id}/model-catalog`: registered models for the ceiling
/// editor (tenant managers only). Member pickers use `/models/eligible` instead.
#[utoipa::path(
    get,
    path = "/api/v1/tenants/{tenant_id}/model-catalog",
    tag = "Models",
    params(("tenant_id" = Uuid, Path, description = "Tenant id")),
    responses(
        (status = 200, description = "Registered models and the tenant ceiling mode", body = TenantCatalogResponse),
        (status = 403, description = "Tenant owner or admin required"),
    )
)]
pub async fn get_tenant_model_catalog(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(tenant_id): Path<Uuid>,
) -> ApiResult<Json<TenantCatalogResponse>> {
    require_tenant_access(&state, &auth, tenant_id, TenantAccess::Manage).await?;
    let ceiling = load_rows(&state, tenant_id, None).await?;
    let catalog = load_catalog(&state).await;
    Ok(Json(catalog_view(&catalog, &ceiling)))
}

/// `PUT /api/v1/tenants/{tenant_id}/model-policy`: replace the ceiling (Super Admin only).
/// An empty `rows` array returns the tenant to unconstrained.
#[utoipa::path(
    put,
    path = "/api/v1/tenants/{tenant_id}/model-policy",
    tag = "Models",
    params(("tenant_id" = Uuid, Path, description = "Tenant id")),
    request_body = PutPolicyRequest,
    responses(
        (status = 200, description = "Ceiling replaced", body = TenantPolicyResponse),
        (status = 403, description = "Tenant owner or admin required"),
        (status = 422, description = "Invalid row, duplicate row, or rerank usage"),
    )
)]
pub async fn put_tenant_model_policy(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(tenant_id): Path<Uuid>,
    Json(body): Json<PutPolicyRequest>,
) -> ApiResult<Json<TenantPolicyResponse>> {
    require_tenant_access(&state, &auth, tenant_id, TenantAccess::Manage).await?;
    let rows = parse_policy_rows(&body.rows)?;
    let catalog = load_catalog(&state).await;
    ensure_usable(&rows, &catalog, &rows, &[])?;
    replace_rows(&state, tenant_id, None, &rows).await?;
    Ok(Json(tenant_view(tenant_id, &rows)))
}

fn tenant_view(tenant_id: Uuid, rows: &[PolicyRow]) -> TenantPolicyResponse {
    let constrained: BTreeSet<&str> = rows.iter().map(|row| row.usage.as_str()).collect();
    TenantPolicyResponse {
        tenant_id: tenant_id.to_string(),
        constrained_usages: constrained.into_iter().map(str::to_string).collect(),
        rows: rows.iter().map(PolicyRowDto::from_row).collect(),
    }
}

// ---------------------------------------------------------------------------
// Workspace narrowing
// ---------------------------------------------------------------------------

/// `GET /api/v1/workspaces/{workspace_id}/model-policy`: narrowing rows (members read).
#[utoipa::path(
    get,
    path = "/api/v1/workspaces/{workspace_id}/model-policy",
    tag = "Models",
    params(("workspace_id" = Uuid, Path, description = "Workspace id")),
    responses(
        (status = 200, description = "Workspace narrowing", body = WorkspacePolicyResponse),
        (status = 403, description = "Not a member of this workspace"),
        (status = 404, description = "Workspace not found"),
    )
)]
pub async fn get_workspace_model_policy(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(workspace_id): Path<Uuid>,
) -> ApiResult<Json<WorkspacePolicyResponse>> {
    let workspace = workspace_or_404(&state, workspace_id).await?;
    let scope = caller_scope(&state, &auth, workspace.tenant_id, Some(workspace_id)).await?;
    require_member(scope)?;
    let rows = load_rows(&state, workspace.tenant_id, Some(workspace_id)).await?;
    Ok(Json(workspace_view(workspace_id, scope, &rows)))
}

/// `PUT /api/v1/workspaces/{workspace_id}/model-policy`: replace narrowing rows
/// (workspace owner or admin). A row that enables a pair the ceiling omits is 422.
#[utoipa::path(
    put,
    path = "/api/v1/workspaces/{workspace_id}/model-policy",
    tag = "Models",
    params(("workspace_id" = Uuid, Path, description = "Workspace id")),
    request_body = PutPolicyRequest,
    responses(
        (status = 200, description = "Narrowing replaced", body = WorkspacePolicyResponse),
        (status = 403, description = "Workspace owner or admin required"),
        (status = 422, description = "Invalid row or row widens the tenant ceiling"),
    )
)]
pub async fn put_workspace_model_policy(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(workspace_id): Path<Uuid>,
    Json(body): Json<PutPolicyRequest>,
) -> ApiResult<Json<WorkspacePolicyResponse>> {
    let workspace = workspace_or_404(&state, workspace_id).await?;
    let scope = caller_scope(&state, &auth, workspace.tenant_id, Some(workspace_id)).await?;
    if !scope.grants.workspace_manage {
        return Err(ApiError::forbidden_reason("workspace_manage_required"));
    }
    let rows = parse_policy_rows(&body.rows)?;
    let ceiling = load_rows(&state, workspace.tenant_id, None).await?;
    if !widening_rows(&ceiling, &rows).is_empty() {
        return Err(ApiError::unprocessable_coded(
            "policy_widens_ceiling",
            "a workspace row cannot enable a model the tenant ceiling omits",
        ));
    }
    let catalog = load_catalog(&state).await;
    ensure_usable(&rows, &catalog, &ceiling, &rows)?;
    replace_rows(&state, workspace.tenant_id, Some(workspace_id), &rows).await?;
    Ok(Json(workspace_view(workspace_id, scope, &rows)))
}

fn workspace_view(
    workspace_id: Uuid,
    scope: CallerScope,
    rows: &[PolicyRow],
) -> WorkspacePolicyResponse {
    WorkspacePolicyResponse {
        workspace_id: workspace_id.to_string(),
        can_manage: scope.grants.workspace_manage,
        rows: rows.iter().map(PolicyRowDto::from_row).collect(),
    }
}

// ---------------------------------------------------------------------------
// Member picker
// ---------------------------------------------------------------------------

/// `GET /api/v1/models/eligible?workspace_id=&usage=`: models a member may pick.
#[utoipa::path(
    get,
    path = "/api/v1/models/eligible",
    tag = "Models",
    params(
        ("workspace_id" = Uuid, Query, description = "Workspace id"),
        ("usage" = String, Query, description = "extract | query | summary | vlm | keyword | embedding | decision"),
    ),
    responses(
        (status = 200, description = "Eligible models for this usage", body = EligibleModelsResponse),
        (status = 403, description = "Not a member of this workspace"),
        (status = 422, description = "Unknown usage or rerank"),
    )
)]
pub async fn get_eligible_models(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Query(query): Query<EligibleQuery>,
) -> ApiResult<Json<EligibleModelsResponse>> {
    let usage = parse_configurable_usage(&query.usage)?;
    let workspace = workspace_or_404(&state, query.workspace_id).await?;
    let scope = caller_scope(&state, &auth, workspace.tenant_id, Some(query.workspace_id)).await?;
    require_member(scope)?;

    let ceiling = load_rows(&state, workspace.tenant_id, None).await?;
    let narrowing = load_rows(&state, workspace.tenant_id, Some(query.workspace_id)).await?;
    let catalog = load_catalog(&state).await;
    let entries = catalog_entries(&catalog);
    let eligible = eligible_models(usage, &entries, &ceiling, &narrowing);

    Ok(Json(EligibleModelsResponse {
        usage: usage.as_str().to_string(),
        policy_mode: policy_mode(&ceiling, usage).as_str().to_string(),
        models: eligible_dtos(&eligible, &catalog),
    }))
}

fn catalog_entries(catalog: &[CatalogItem]) -> Vec<CatalogEntry> {
    catalog.iter().map(|item| item.entry.clone()).collect()
}

fn eligible_dtos(eligible: &[&CatalogEntry], catalog: &[CatalogItem]) -> Vec<EligibleModelDto> {
    let by_pair: HashMap<(&str, &str), &CatalogItem> = catalog
        .iter()
        .map(|item| {
            (
                (item.entry.provider.as_str(), item.entry.model.as_str()),
                item,
            )
        })
        .collect();
    eligible
        .iter()
        .filter_map(|entry| by_pair.get(&(entry.provider.as_str(), entry.model.as_str())))
        .map(|item| EligibleModelDto {
            provider: item.entry.provider.clone(),
            provider_display_name: item.provider_display_name.clone(),
            model: item.entry.model.clone(),
            display_name: item.display_name.clone(),
            supports_vision: item.supports_vision,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Shared checks
// ---------------------------------------------------------------------------

/// Load an active workspace; an unknown or inactive id is 404 (never another tenant's).
pub(crate) async fn workspace_or_404(state: &AppState, workspace_id: Uuid) -> ApiResult<Workspace> {
    state
        .workspace_service
        .get_workspace(workspace_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .filter(|workspace| workspace.is_active)
        .ok_or_else(|| ApiError::NotFound(format!("workspace {workspace_id}")))
}

/// Refuse a write that would leave a written usage with no model to pick (`policy_empty_usage`).
fn ensure_usable(
    written: &[PolicyRow],
    catalog: &[CatalogItem],
    ceiling: &[PolicyRow],
    narrowing: &[PolicyRow],
) -> ApiResult<()> {
    let empty = usages_left_empty(written, &catalog_entries(catalog), ceiling, narrowing);
    if empty.is_empty() {
        return Ok(());
    }
    let names: Vec<&str> = empty.iter().map(|usage| usage.as_str()).collect();
    Err(ApiError::unprocessable_coded(
        "policy_empty_usage",
        format!(
            "no catalog model would remain selectable for: {}",
            names.join(", ")
        ),
    ))
}

pub(crate) fn require_member(scope: CallerScope) -> ApiResult<()> {
    if scope.member {
        Ok(())
    } else {
        Err(ApiError::forbidden_reason("workspace_access_denied"))
    }
}
