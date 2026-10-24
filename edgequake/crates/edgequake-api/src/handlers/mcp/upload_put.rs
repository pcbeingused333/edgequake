//! Relay PUT for SPEC-173. HMAC is the capability. This route is not an MCP method.

use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use crate::mcp::project::upload_session::{accept_relay_body, RelayReject};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct RelayQuery {
    exp: String,
    sig: String,
    sha: String,
    len: String,
    #[serde(default)]
    tid: String,
    #[serde(default)]
    wid: String,
}

pub async fn mcp_upload_put(
    State(state): State<AppState>,
    Path(upload_id): Path<String>,
    Query(query): Query<RelayQuery>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let content_type = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .trim();
    let authorization = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());
    let pairs = [
        ("exp".into(), query.exp),
        ("sig".into(), query.sig),
        ("sha".into(), query.sha),
        ("len".into(), query.len),
        ("tid".into(), query.tid),
        ("wid".into(), query.wid),
    ];
    match accept_relay_body(
        &state,
        &upload_id,
        &pairs,
        content_type,
        authorization,
        &body,
    )
    .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(RelayReject::Forbidden) => StatusCode::FORBIDDEN.into_response(),
        Err(RelayReject::NotFound) => StatusCode::NOT_FOUND.into_response(),
        Err(RelayReject::Conflict) => StatusCode::CONFLICT.into_response(),
        Err(RelayReject::TooLarge) => StatusCode::PAYLOAD_TOO_LARGE.into_response(),
    }
}
