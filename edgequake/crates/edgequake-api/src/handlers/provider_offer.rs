//! SPEC-177 provider offer HTTP API. Eligibility rules stay in `edgequake-core`.

use std::collections::{BTreeSet, HashSet};

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use edgequake_core::provider_offer::parse_slug;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::handlers::auth::{ApiAuthenticated, ApiRequireAdmin};
use crate::handlers::connections::env_connections;
use crate::handlers::model_policy::workspace_or_404;
use crate::handlers::workspaces::tenant_access::{require_tenant_access, TenantAccess};
use crate::services::model_policy_catalog::load_catalog;
use crate::services::provider_offer_rows::{
    load_global_offer, load_tenant_offer, load_workspace_offer, replace_offer, set_withdrawn,
    ScopeOffer,
};
use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct OfferModelDto {
    pub id: String,
    pub usages: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeploymentProviderDto {
    pub slug: String,
    pub display_name: String,
    pub origin: String,
    pub api_shape: String,
    pub offered: bool,
    pub withdrawn: bool,
    pub probe: String,
    pub key_configured: bool,
    pub models: Vec<OfferModelDto>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeploymentProvidersResponse {
    pub providers: Vec<DeploymentProviderDto>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct WithdrawRequest {
    pub withdrawn: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OfferProviderDto {
    pub slug: String,
    pub display_name: String,
    pub offered_by_parent: bool,
    pub enabled: bool,
    pub models: Vec<OfferModelDto>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProviderOfferResponse {
    pub tenant_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    pub offer_mode: String,
    pub parent_count: usize,
    pub enabled_count: usize,
    pub can_manage: bool,
    pub providers: Vec<OfferProviderDto>,
    pub legacy: Vec<LegacyConnectionDto>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LegacyConnectionDto {
    pub slug: String,
    pub display_name: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PutOfferRequest {
    pub offer_mode: String,
    #[serde(default)]
    pub slugs: Vec<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/deployment/providers",
    tag = "Providers",
    responses((status = 200, description = "Deployment provider offer", body = DeploymentProvidersResponse))
)]
pub async fn get_deployment_providers(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
) -> ApiResult<Json<DeploymentProvidersResponse>> {
    let offered = load_global_offer(&state).await?;
    let catalog = load_catalog(&state).await;
    let mut providers = Vec::new();
    let mut seen = HashSet::new();
    for row in env_connections() {
        let Some(slug) = parse_slug(&row.slug) else {
            continue;
        };
        if !seen.insert(slug.clone()) {
            continue;
        }
        let withdrawn = !offered.contains(&slug);
        providers.push(DeploymentProviderDto {
            models: models_for(&catalog, &slug),
            slug,
            display_name: row.display_name,
            origin: "env".into(),
            api_shape: row.api_shape,
            offered: offered.contains(&row.slug) || offered.iter().any(|s| s == &row.slug),
            withdrawn,
            probe: "ok".into(),
            key_configured: row.key_configured,
        });
    }
    // The offered flag above used the slug before move. Recompute from the set.
    for provider in &mut providers {
        provider.offered = offered.contains(&provider.slug);
        provider.withdrawn = !provider.offered && provider.origin == "env";
    }
    #[cfg(feature = "postgres")]
    if let Some(pool) = state.optional_pg_pool() {
        let rows: Vec<(String, String, String, Option<bool>, bool)> = sqlx::query_as(
            "SELECT slug, display_name, api_shape, last_test_ok, \
                    (api_key_ciphertext IS NOT NULL OR secret_ref IS NOT NULL) AS key_configured \
             FROM provider_connections WHERE tenant_id IS NULL ORDER BY slug",
        )
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
        let withdrawn: Vec<(String,)> = sqlx::query_as(
            "SELECT provider_slug FROM deployment_provider_overrides WHERE withdrawn",
        )
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
        let withdrawn: BTreeSet<String> = withdrawn.into_iter().map(|(s,)| s).collect();
        for (slug, display_name, api_shape, last_test_ok, key_configured) in rows {
            let Some(slug) = parse_slug(&slug) else {
                continue;
            };
            if !seen.insert(slug.clone()) {
                continue;
            }
            let probe = match last_test_ok {
                Some(true) => "ok",
                Some(false) => "failed",
                None => "unchecked",
            };
            providers.push(DeploymentProviderDto {
                offered: offered.contains(&slug),
                withdrawn: withdrawn.contains(&slug),
                models: models_for(&catalog, &slug),
                slug,
                display_name,
                origin: "fleet".into(),
                api_shape,
                probe: probe.into(),
                key_configured,
            });
        }
    }
    providers.sort_by(|a, b| a.slug.cmp(&b.slug));
    Ok(Json(DeploymentProvidersResponse { providers }))
}

#[utoipa::path(
    put,
    path = "/api/v1/deployment/providers/{slug}",
    tag = "Providers",
    params(("slug" = String, Path, description = "Provider slug")),
    request_body = WithdrawRequest,
    responses((status = 200, description = "Withdrawal updated", body = DeploymentProviderDto))
)]
pub async fn put_deployment_provider(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
    Path(slug): Path<String>,
    Json(body): Json<WithdrawRequest>,
) -> ApiResult<Json<DeploymentProviderDto>> {
    let slug = parse_slug(&slug).ok_or_else(|| {
        ApiError::unprocessable_coded("slug_invalid", "provider slug is not valid")
    })?;
    let known = env_connections()
        .iter()
        .any(|row| parse_slug(&row.slug).as_deref() == Some(slug.as_str()));
    #[cfg(feature = "postgres")]
    let known = known
        || state.optional_pg_pool().is_some_and(|_| true)
            && fleet_slug_exists(&state, &slug).await?;
    if !known {
        return Err(ApiError::NotFound(format!("provider {slug}")));
    }
    set_withdrawn(&state, &slug, body.withdrawn).await?;
    let list = get_deployment_providers(State(state), _admin).await?;
    list.0
        .providers
        .into_iter()
        .find(|row| row.slug == slug)
        .map(Json)
        .ok_or_else(|| ApiError::NotFound(format!("provider {slug}")))
}

#[utoipa::path(
    get,
    path = "/api/v1/tenants/{tenant_id}/provider-offer",
    tag = "Providers",
    params(("tenant_id" = Uuid, Path, description = "Tenant id")),
    responses((status = 200, description = "Tenant provider offer", body = ProviderOfferResponse))
)]
pub async fn get_tenant_provider_offer(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(tenant_id): Path<Uuid>,
) -> ApiResult<Json<ProviderOfferResponse>> {
    require_tenant_access(&state, &auth, tenant_id, TenantAccess::Manage).await?;
    let parent = load_global_offer(&state).await?;
    let offer = load_tenant_offer(&state, tenant_id).await?;
    let legacy = legacy_for_tenant(&state, tenant_id).await?;
    Ok(Json(
        offer_body(&state, tenant_id, None, true, &parent, &offer, legacy).await?,
    ))
}

#[utoipa::path(
    put,
    path = "/api/v1/tenants/{tenant_id}/provider-offer",
    tag = "Providers",
    params(("tenant_id" = Uuid, Path, description = "Tenant id")),
    request_body = PutOfferRequest,
    responses((status = 200, description = "Tenant provider offer replaced", body = ProviderOfferResponse))
)]
pub async fn put_tenant_provider_offer(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(tenant_id): Path<Uuid>,
    Json(body): Json<PutOfferRequest>,
) -> ApiResult<Json<ProviderOfferResponse>> {
    require_tenant_access(&state, &auth, tenant_id, TenantAccess::Manage).await?;
    let parent = load_global_offer(&state).await?;
    let (constrained, slugs) = parse_put(&body, &parent)?;
    replace_offer(&state, tenant_id, None, constrained, &slugs).await?;
    get_tenant_provider_offer(State(state), ApiAuthenticated(auth), Path(tenant_id)).await
}

#[utoipa::path(
    get,
    path = "/api/v1/workspaces/{workspace_id}/provider-offer",
    tag = "Providers",
    params(("workspace_id" = Uuid, Path, description = "Workspace id")),
    responses((status = 200, description = "Workspace provider offer", body = ProviderOfferResponse))
)]
pub async fn get_workspace_provider_offer(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(workspace_id): Path<Uuid>,
) -> ApiResult<Json<ProviderOfferResponse>> {
    let workspace = workspace_or_404(&state, workspace_id).await?;
    let scope = crate::handlers::me_settings_access::caller_scope(
        &state,
        &auth,
        workspace.tenant_id,
        Some(workspace_id),
    )
    .await?;
    if !scope.member {
        return Err(ApiError::forbidden_reason("workspace_access_denied"));
    }
    let parent = load_tenant_offer(&state, workspace.tenant_id).await?.slugs;
    let offer = load_workspace_offer(&state, workspace.tenant_id, workspace_id).await?;
    Ok(Json(
        offer_body(
            &state,
            workspace.tenant_id,
            Some(workspace_id),
            scope.grants.workspace_manage,
            &parent,
            &offer,
            Vec::new(),
        )
        .await?,
    ))
}

#[utoipa::path(
    put,
    path = "/api/v1/workspaces/{workspace_id}/provider-offer",
    tag = "Providers",
    params(("workspace_id" = Uuid, Path, description = "Workspace id")),
    request_body = PutOfferRequest,
    responses((status = 200, description = "Workspace provider offer replaced", body = ProviderOfferResponse))
)]
pub async fn put_workspace_provider_offer(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(workspace_id): Path<Uuid>,
    Json(body): Json<PutOfferRequest>,
) -> ApiResult<Json<ProviderOfferResponse>> {
    let workspace = workspace_or_404(&state, workspace_id).await?;
    let scope = crate::handlers::me_settings_access::caller_scope(
        &state,
        &auth,
        workspace.tenant_id,
        Some(workspace_id),
    )
    .await?;
    if !scope.grants.workspace_manage {
        return Err(ApiError::forbidden_reason("workspace_manage_required"));
    }
    let parent = load_tenant_offer(&state, workspace.tenant_id).await?.slugs;
    let (constrained, slugs) = parse_put(&body, &parent)?;
    replace_offer(
        &state,
        workspace.tenant_id,
        Some(workspace_id),
        constrained,
        &slugs,
    )
    .await?;
    get_workspace_provider_offer(State(state), ApiAuthenticated(auth), Path(workspace_id)).await
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ConnectionModelBody {
    pub id: String,
    #[serde(default)]
    pub supports_chat: bool,
    #[serde(default)]
    pub supports_vision: bool,
    #[serde(default)]
    pub supports_embedding: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PutConnectionModelsRequest {
    pub models: Vec<ConnectionModelBody>,
}

#[utoipa::path(
    put,
    path = "/api/v1/deployment/providers/{slug}/models",
    tag = "Providers",
    params(("slug" = String, Path, description = "Fleet provider slug")),
    request_body = PutConnectionModelsRequest,
    responses((status = 204, description = "Models replaced"))
)]
pub async fn put_connection_models(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
    Path(slug): Path<String>,
    Json(body): Json<PutConnectionModelsRequest>,
) -> Result<StatusCode, ApiError> {
    let slug = parse_slug(&slug).ok_or_else(|| {
        ApiError::unprocessable_coded("slug_invalid", "provider slug is not valid")
    })?;
    let env_only = env_connections()
        .iter()
        .any(|row| parse_slug(&row.slug).as_deref() == Some(slug.as_str()));
    #[cfg(feature = "postgres")]
    {
        let fleet = fleet_slug_exists(&state, &slug).await?;
        if env_only && !fleet {
            return Err(ApiError::unprocessable_coded(
                "provider_not_fleet",
                "environment providers use the catalog cards",
            ));
        }
        let pool = state
            .optional_pg_pool()
            .ok_or_else(|| ApiError::Internal("PostgreSQL is required".into()))?;
        let models: Vec<(String, bool, bool, bool)> = body
            .models
            .into_iter()
            .map(|model| {
                (
                    model.id,
                    model.supports_chat,
                    model.supports_vision,
                    model.supports_embedding,
                )
            })
            .collect();
        crate::services::provider_offer_rows::replace_connection_models(pool, &slug, &models)
            .await?;
        Ok(StatusCode::NO_CONTENT)
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (env_only, body);
        Err(ApiError::Internal("PostgreSQL is required".into()))
    }
}

fn parse_put(body: &PutOfferRequest, parent: &BTreeSet<String>) -> ApiResult<(bool, Vec<String>)> {
    match body.offer_mode.as_str() {
        "unconstrained" => Ok((false, Vec::new())),
        "constrained" => {
            let mut slugs = Vec::new();
            let mut seen = HashSet::new();
            for raw in &body.slugs {
                let slug = parse_slug(raw).ok_or_else(|| {
                    ApiError::unprocessable_coded("slug_invalid", "provider slug is not valid")
                })?;
                if !seen.insert(slug.clone()) {
                    return Err(ApiError::unprocessable_coded(
                        "duplicate_slug",
                        "duplicate provider",
                    ));
                }
                if !parent.contains(&slug) {
                    return Err(ApiError::unprocessable_coded(
                        "provider_not_in_parent",
                        format!("{slug} is not offered by the parent"),
                    ));
                }
                slugs.push(slug);
            }
            Ok((true, slugs))
        }
        _ => Err(ApiError::unprocessable_coded(
            "slug_invalid",
            "offer_mode must be constrained or unconstrained",
        )),
    }
}

async fn offer_body(
    state: &AppState,
    tenant_id: Uuid,
    workspace_id: Option<Uuid>,
    can_manage: bool,
    parent: &BTreeSet<String>,
    offer: &ScopeOffer,
    legacy: Vec<LegacyConnectionDto>,
) -> ApiResult<ProviderOfferResponse> {
    let mut catalog = load_catalog(state).await;
    crate::services::model_policy_catalog::append_connection_models(state, &mut catalog).await?;
    let providers = parent
        .iter()
        .map(|slug| OfferProviderDto {
            display_name: slug.clone(),
            offered_by_parent: true,
            enabled: offer.slugs.contains(slug),
            models: models_for(&catalog, slug),
            slug: slug.clone(),
        })
        .collect();
    Ok(ProviderOfferResponse {
        tenant_id: tenant_id.to_string(),
        workspace_id: workspace_id.map(|id| id.to_string()),
        offer_mode: if offer.constrained {
            "constrained"
        } else {
            "unconstrained"
        }
        .into(),
        parent_count: parent.len(),
        enabled_count: offer.slugs.len(),
        can_manage,
        providers,
        legacy,
    })
}

fn models_for(
    catalog: &[crate::services::model_policy_catalog::CatalogItem],
    slug: &str,
) -> Vec<OfferModelDto> {
    catalog
        .iter()
        .filter(|item| item.entry.provider == slug && item.entry.facts.registered)
        .map(|item| OfferModelDto {
            id: item.entry.model.clone(),
            usages: usage_names(&item.entry.facts),
        })
        .collect()
}

fn usage_names(facts: &edgequake_core::model_policy::ModelFacts) -> Vec<String> {
    let mut names = Vec::new();
    if facts.chat {
        names.extend(
            ["extract", "query", "summary", "keyword"]
                .into_iter()
                .map(str::to_string),
        );
    }
    if facts.chat && facts.vision {
        names.push("vlm".into());
    }
    if facts.embedding {
        names.push("embedding".into());
    }
    names
}

#[cfg(feature = "postgres")]
async fn fleet_slug_exists(state: &AppState, slug: &str) -> ApiResult<bool> {
    let Some(pool) = state.optional_pg_pool() else {
        return Ok(false);
    };
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM provider_connections WHERE tenant_id IS NULL AND slug = $1)",
    )
    .bind(slug)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(exists)
}

#[cfg(feature = "postgres")]
async fn legacy_for_tenant(state: &AppState, tenant: Uuid) -> ApiResult<Vec<LegacyConnectionDto>> {
    let Some(pool) = state.optional_pg_pool() else {
        return Ok(Vec::new());
    };
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT slug, display_name FROM provider_connections WHERE tenant_id = $1 ORDER BY slug",
    )
    .bind(tenant)
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(rows
        .into_iter()
        .map(|(slug, display_name)| LegacyConnectionDto { slug, display_name })
        .collect())
}

#[cfg(not(feature = "postgres"))]
async fn legacy_for_tenant(
    _state: &AppState,
    _tenant: Uuid,
) -> ApiResult<Vec<LegacyConnectionDto>> {
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parent(slugs: &[&str]) -> BTreeSet<String> {
        slugs.iter().map(|slug| (*slug).to_string()).collect()
    }

    fn code(result: ApiResult<(bool, Vec<String>)>) -> Option<&'static str> {
        result.err().map(|err| err.code())
    }

    #[test]
    fn tenant_slug_outside_global_is_422() {
        let body = PutOfferRequest {
            offer_mode: "constrained".into(),
            slugs: vec!["acme".into()],
        };
        assert_eq!(
            code(parse_put(&body, &parent(&["openai"]))),
            Some("provider_not_in_parent")
        );
    }

    #[test]
    fn workspace_slug_outside_tenant_is_422() {
        let body = PutOfferRequest {
            offer_mode: "constrained".into(),
            slugs: vec!["acme".into()],
        };
        assert_eq!(
            code(parse_put(&body, &parent(&["openai"]))),
            Some("provider_not_in_parent")
        );
    }

    #[test]
    fn slug_invalid_is_422() {
        let body = PutOfferRequest {
            offer_mode: "constrained".into(),
            slugs: vec!["Has Space".into()],
        };
        assert_eq!(
            code(parse_put(&body, &parent(&["openai"]))),
            Some("slug_invalid")
        );
    }

    #[test]
    fn duplicate_slug_is_422() {
        let body = PutOfferRequest {
            offer_mode: "constrained".into(),
            slugs: vec!["openai".into(), "OpenAI".into()],
        };
        assert_eq!(
            code(parse_put(&body, &parent(&["openai"]))),
            Some("duplicate_slug")
        );
    }

    #[test]
    fn unconstrained_ignores_slugs() {
        let body = PutOfferRequest {
            offer_mode: "unconstrained".into(),
            slugs: vec!["missing".into()],
        };
        let (constrained, slugs) = parse_put(&body, &parent(&["openai"])).unwrap();
        assert!(!constrained);
        assert!(slugs.is_empty());
    }

    #[test]
    fn offer_json_shape_has_no_secret_field() {
        let body = serde_json::to_string(&ProviderOfferResponse {
            tenant_id: "t".into(),
            workspace_id: None,
            offer_mode: "unconstrained".into(),
            parent_count: 0,
            enabled_count: 0,
            can_manage: true,
            providers: vec![],
            legacy: vec![],
        })
        .unwrap();
        assert!(!body.contains("api_key"));
        assert!(!body.contains("secret_ref"));
        assert!(!body.contains("ciphertext"));
    }
}
