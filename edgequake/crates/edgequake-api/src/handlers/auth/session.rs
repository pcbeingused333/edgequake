//! Authentication session handlers: login, refresh, logout, get_me.
//!
//! @implements FEAT0802 (JWT Token Support)
//! @implements FEAT0804 (JWT login with access and refresh tokens)
//! @implements FEAT0805 (Token refresh without re-authentication)

use axum::{
    extract::State,
    http::{
        header::{AUTHORIZATION, SET_COOKIE},
        HeaderMap, StatusCode,
    },
    response::{IntoResponse, Response},
    Json,
};
use tracing::info;

use edgequake_audit::{AuditEventType, AuditResult};
use edgequake_auth::Role;

use crate::error::ApiError;
use crate::services::federation::scope_guard::revalidate_scope;
use crate::services::login_tokens::{
    issue_session, mint_access_token, persist_new_refresh, SessionContext, SessionScope,
};
use crate::services::record_compliance_event_runtime;
use crate::state::{
    ApiSecurityConfig, AuthRuntime, ComplianceRuntime, OperationalStores, PostgresRuntime,
    StorageRuntime,
};

use super::refresh_cookie::{
    clear_refresh_cookie_header, cookie_secure_from_headers, refresh_token_for_json_body,
    resolve_refresh_token, set_refresh_cookie_header,
};
use super::{find_user_by_login, get_record_by_id, get_user_by_id, UserRecord};
pub use crate::handlers::auth_types::{
    GetMeResponse, LoginRequest, LoginResponse, RefreshTokenRequest, RefreshTokenResponse, UserInfo,
};
use crate::oauth::session_cookie::{
    clear_oauth_session_cookie_header, set_oauth_session_cookie_header,
};

/// Login endpoint.
///
/// POST /api/v1/auth/login
#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    tag = "Authentication",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = LoginResponse),
        (status = 401, description = "Invalid credentials"),
        (status = 423, description = "Account locked")
    )
)]
#[allow(clippy::too_many_arguments)] // Axum extractor fan-in
pub async fn login(
    State(auth): State<AuthRuntime>,
    State(storage): State<StorageRuntime>,
    State(pg_runtime): State<PostgresRuntime>,
    State(security): State<ApiSecurityConfig>,
    State(compliance): State<ComplianceRuntime>,
    State(stores): State<OperationalStores>,
    headers: HeaderMap,
    Json(request): Json<LoginRequest>,
) -> Result<Response, ApiError> {
    info!("Login attempt for user: {}", request.username);

    let user =
        find_user_by_login(&storage, Some(&pg_runtime), &security, &request.username).await?;

    let user = match user {
        Some(u) => u,
        None => {
            record_compliance_event_runtime(
                &compliance,
                "default",
                AuditEventType::Authentication,
                "login",
                AuditResult::Failure,
                None,
                None,
                None,
            );
            return Err(ApiError::auth_unauthorized(
                "login",
                "user_not_found",
                Some(&request.username),
            ));
        }
    };

    let mut record = get_record_by_id(
        &storage,
        Some(&pg_runtime),
        &security,
        stores.identity.as_deref(),
        &user.user_id,
    )
    .await?
    .ok_or_else(|| ApiError::Internal("User record missing after lookup".into()))?;

    crate::services::login_lockout::ensure_login_allowed(&record)?;

    if !record.is_active {
        return Err(ApiError::forbidden_reason("account_inactive"));
    }

    let password_valid = auth
        .password
        .verify_password(&request.password, &record.password_hash)
        .map_err(|e| ApiError::Internal(format!("password_verify failed: {e}")))?;

    if !password_valid {
        record_compliance_event_runtime(
            &compliance,
            "default",
            AuditEventType::Authentication,
            "login",
            AuditResult::Failure,
            None,
            Some(record.user_id.clone()),
            None,
        );
        crate::services::login_lockout::record_failed_login(
            &storage,
            Some(&pg_runtime),
            &security,
            stores.identity.as_deref(),
            &auth.config,
            &mut record,
        )
        .await?;
        return Err(ApiError::auth_unauthorized(
            "login",
            "invalid_password",
            Some(&request.username),
        ));
    }

    crate::services::login_lockout::record_successful_login(
        &storage,
        Some(&pg_runtime),
        &security,
        stores.identity.as_deref(),
        &mut record,
    )
    .await?;

    let response = open_session(
        &auth,
        &storage,
        &pg_runtime,
        &security,
        &stores,
        &headers,
        &record,
    )
    .await?;

    info!("Login successful for user: {}", record.username);

    record_compliance_event_runtime(
        &compliance,
        "default",
        AuditEventType::Authentication,
        "login",
        AuditResult::Success,
        None,
        Some(record.user_id.clone()),
        None,
    );

    Ok(response)
}

/// Mint a session for an already-authenticated account: tokens + HttpOnly refresh cookie.
///
/// Shared by password login and the one-time access links (SPEC-169) so a session is always
/// created the same way.
pub(crate) async fn open_session(
    auth: &AuthRuntime,
    storage: &StorageRuntime,
    pg: &PostgresRuntime,
    security: &ApiSecurityConfig,
    stores: &OperationalStores,
    headers: &HeaderMap,
    record: &UserRecord,
) -> Result<Response, ApiError> {
    let ctx = SessionContext {
        auth,
        storage,
        pg,
        security,
        stores,
    };
    let issued = issue_session(
        &ctx,
        &record.user_id,
        Role::parse(&record.role),
        &SessionScope::default_scope(),
    )
    .await?;

    let secure = cookie_secure_from_headers(headers);
    Ok(with_session_cookies(
        LoginResponse {
            access_token: issued.access_token.clone(),
            token_type: "Bearer".to_string(),
            expires_in: issued.expires_in,
            refresh_token: refresh_token_for_json_body(headers, &issued.refresh_token),
            user: UserInfo::from(record),
        },
        &issued.access_token,
        issued.expires_in,
        &issued.refresh_token,
        secure,
    ))
}

/// Refresh access token.
///
/// POST /api/v1/auth/refresh
#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    tag = "Authentication",
    request_body = RefreshTokenRequest,
    responses(
        (status = 200, description = "Token refreshed", body = RefreshTokenResponse),
        (status = 401, description = "Invalid or expired refresh token")
    )
)]
#[allow(clippy::too_many_arguments)]
pub async fn refresh_token(
    State(auth): State<AuthRuntime>,
    State(storage): State<StorageRuntime>,
    State(pg_runtime): State<PostgresRuntime>,
    State(security): State<ApiSecurityConfig>,
    State(stores): State<OperationalStores>,
    State(workspaces): State<crate::state::SharedWorkspaceService>,
    headers: HeaderMap,
    Json(request): Json<RefreshTokenRequest>,
) -> Result<Response, ApiError> {
    use crate::services::auth_memory_store::TakeWebRefreshOutcome;

    let presented = resolve_refresh_token(request.refresh_token.as_deref(), &headers)
        .ok_or_else(|| ApiError::auth_unauthorized("refresh", "token_missing", None))?;

    let outcome = crate::services::session_storage::take_web_refresh(
        &storage,
        Some(&pg_runtime),
        &security,
        stores.sessions.as_deref(),
        &presented,
    )
    .await?;

    let record = match outcome {
        TakeWebRefreshOutcome::Consumed(r) => *r,
        TakeWebRefreshOutcome::Invalid => {
            return Err(ApiError::auth_unauthorized(
                "refresh",
                "token_invalid",
                None,
            ));
        }
        TakeWebRefreshOutcome::ReuseDetected { family_id: _ } => {
            return Err(ApiError::auth_unauthorized("refresh", "token_reuse", None));
        }
    };

    let user = get_user_by_id(
        &storage,
        Some(&pg_runtime),
        &security,
        stores.identity.as_deref(),
        &record.user_id,
    )
    .await?
    .ok_or(ApiError::auth_unauthorized(
        "refresh",
        "user_not_found",
        None,
    ))?;

    let user_uuid = uuid::Uuid::parse_str(&user.user_id)
        .map_err(|_| ApiError::Internal("Invalid user ID format".to_string()))?;

    // SPEC-158: SSO sessions keep their tenant scope and are re-validated on every renewal.
    let (scope, role) = refresh_scope(
        &auth,
        workspaces.as_ref(),
        record.family_id,
        user_uuid,
        &user.role,
    )
    .await?;

    // SPEC-154 Wave 4: rotate — issue successor in the same family.
    let ctx = SessionContext {
        auth: &auth,
        storage: &storage,
        pg: &pg_runtime,
        security: &security,
        stores: &stores,
    };
    let new_refresh = persist_new_refresh(&ctx, &record.user_id, record.family_id).await?;
    let (access_token, expires_in) = mint_access_token(&auth, user_uuid, role, &scope)?;
    if auth
        .federation
        .get_session(record.family_id)
        .await?
        .is_some()
    {
        crate::services::federation::access_jti::remember_access_token(
            auth.federation.as_ref(),
            record.family_id,
            &access_token,
        )
        .await?;
    }

    let secure = cookie_secure_from_headers(&headers);
    Ok(with_session_cookies(
        RefreshTokenResponse {
            access_token: access_token.clone(),
            token_type: "Bearer".to_string(),
            expires_in,
            refresh_token: refresh_token_for_json_body(&headers, &new_refresh),
        },
        &access_token,
        expires_in,
        &new_refresh,
        secure,
    ))
}

/// Logout endpoint (revoke refresh token).
///
/// POST /api/v1/auth/logout
#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    tag = "Authentication",
    request_body = RefreshTokenRequest,
    responses(
        (status = 204, description = "Logout successful"),
        (status = 401, description = "Invalid token")
    )
)]
#[allow(clippy::too_many_arguments)] // Axum extractor fan-in
pub async fn logout(
    State(auth): State<AuthRuntime>,
    State(storage): State<StorageRuntime>,
    State(pg_runtime): State<PostgresRuntime>,
    State(security): State<ApiSecurityConfig>,
    State(compliance): State<ComplianceRuntime>,
    State(stores): State<OperationalStores>,
    headers: HeaderMap,
    Json(request): Json<RefreshTokenRequest>,
) -> Result<Response, ApiError> {
    let presented = resolve_refresh_token(request.refresh_token.as_deref(), &headers);

    let user_id = if let Some(ref token) = presented {
        crate::services::session_storage::load_refresh_token(
            &storage,
            Some(&pg_runtime),
            &security,
            stores.sessions.as_deref(),
            token,
        )
        .await?
        .map(|record| record.user_id)
    } else {
        None
    };

    if let Some(ref token) = presented {
        let _ = crate::services::session_storage::revoke_refresh_token(
            &storage,
            Some(&pg_runtime),
            &security,
            stores.sessions.as_deref(),
            token,
        )
        .await?;
    }

    // SPEC-083 / SPEC-154: durable denylist access-token jti when Bearer is presented.
    if let Some(header) = headers.get(AUTHORIZATION).and_then(|v| v.to_str().ok()) {
        if let Some(token) = header.strip_prefix("Bearer ") {
            let claims = auth
                .jwt
                .verify_token(token)
                .or_else(|_| auth.jwt.decode_unverified(token));
            if let Ok(claims) = claims {
                let expires_at = crate::services::jti_denylist::exp_claim_to_utc(claims.exp);
                crate::services::jti_denylist::revoke_jti_parts(
                    &auth.jwt,
                    pg_runtime.optional_pg_pool(),
                    &claims.jti,
                    expires_at,
                    "logout",
                )
                .await?;
            }
        }
    }

    record_compliance_event_runtime(
        &compliance,
        "default",
        AuditEventType::Authentication,
        "logout",
        AuditResult::Success,
        None,
        user_id,
        None,
    );

    let secure = cookie_secure_from_headers(&headers);
    let mut response = StatusCode::NO_CONTENT.into_response();
    let response_headers = response.headers_mut();
    response_headers.insert(SET_COOKIE, clear_refresh_cookie_header(secure));
    response_headers.append(SET_COOKIE, clear_oauth_session_cookie_header(secure));
    Ok(response)
}

/// Tenant scope + role for a refreshed access token (default scope for non-SSO sessions).
async fn refresh_scope(
    auth: &AuthRuntime,
    workspaces: &dyn edgequake_core::WorkspaceService,
    family_id: uuid::Uuid,
    user_id: uuid::Uuid,
    local_role: &Role,
) -> Result<(SessionScope, Role), ApiError> {
    let Some(session) = auth.federation.get_session(family_id).await? else {
        return Ok((SessionScope::default_scope(), local_role.clone()));
    };
    if session.revoked_at.is_some() {
        return Err(ApiError::auth_unauthorized(
            "refresh",
            "session_revoked",
            None,
        ));
    }
    let scope = SessionScope {
        tenant_id: session.tenant_id,
        workspace_id: session.workspace_id,
    };
    let role = revalidate_scope(workspaces, user_id, &scope, &local_role.to_string()).await?;
    Ok((scope, role))
}

fn with_session_cookies<T: serde::Serialize>(
    body: T,
    access_token: &str,
    expires_in: i64,
    refresh_token: &str,
    secure: bool,
) -> Response {
    let mut response = Json(body).into_response();
    let headers = response.headers_mut();
    // Refresh first so callers that read a single Set-Cookie still see `eq_refresh`.
    headers.insert(SET_COOKIE, set_refresh_cookie_header(refresh_token, secure));
    headers.append(
        SET_COOKIE,
        set_oauth_session_cookie_header(access_token, expires_in, secure),
    );
    response
}

/// Get current user information.
///
/// GET /api/v1/auth/me
#[utoipa::path(
    get,
    path = "/api/v1/auth/me",
    tag = "Authentication",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "User information", body = GetMeResponse),
        (status = 401, description = "Not authenticated")
    )
)]
pub async fn get_me(
    State(state): State<crate::state::AppState>,
    headers: HeaderMap,
) -> Result<Json<GetMeResponse>, ApiError> {
    let credential = crate::middleware::extract_token_from_headers(&headers)
        .ok_or_else(ApiError::unauthorized)?;
    let principal = crate::services::auth_validation::decide(
        &state,
        &credential,
        crate::services::auth_validation::TokenProfile::WebSession,
        &headers,
    )
    .await?
    .ok_or_else(ApiError::unauthorized)?;
    #[cfg(feature = "postgres")]
    let pg = state.pg_pool.clone().map(|pool| PostgresRuntime {
        pool: Some(pool),
        capabilities: None,
    });
    #[cfg(feature = "postgres")]
    let pg = pg.as_ref();
    #[cfg(not(feature = "postgres"))]
    let pg = None;
    let user_record = get_record_by_id(
        &state.storage,
        pg,
        &state.security,
        state.operational_stores.identity.as_deref(),
        &principal.authenticated.auth.user_id,
    )
    .await?
    .ok_or_else(ApiError::unauthorized)?;

    if !user_record.is_active {
        return Err(ApiError::forbidden_reason("account_inactive"));
    }

    Ok(Json(GetMeResponse {
        user: UserInfo::from(&user_record),
    }))
}
