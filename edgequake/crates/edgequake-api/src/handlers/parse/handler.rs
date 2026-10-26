//! POST /api/v1/parse — sync or async PDF → Markdown.

use axum::body::Bytes;
use axum::extract::{FromRequest, Query, Request, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use axum_extra::extract::Multipart;
use tracing::info;
use uuid::Uuid;

use edgequake_pdf::{resolve_pdf_page_count, PdfParserBackend};

use super::errors::ParseErrorCode;
use super::intake::{intake_multipart, intake_raw_pdf, prefer_respond_async, ParsedIntake};
use super::options::resolve_options;
use super::service::{run_parse, ParseLimits};
use super::types::{ParseAsyncAccepted, ParseOptions, ParseResponse};
use crate::error::ApiError;
use crate::middleware::TenantContext;
use crate::state::AppState;

/// Parse a document to Markdown without ingestion (SPEC-094).
#[utoipa::path(
    post,
    path = "/api/v1/parse",
    params(
        ("Prefer" = Option<String>, Header, description = "Send `respond-async` to force a 202 job"),
        ("X-Filename" = Option<String>, Header, description = "Filename for raw application/pdf bodies"),
    ),
    request_body(content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Parse completed", body = ParseResponse),
        (status = 202, description = "Async job accepted", body = ParseAsyncAccepted),
        (status = 400, description = "Invalid request"),
        (status = 413, description = "Payload or page count too large"),
        (status = 415, description = "Unsupported media type"),
        (status = 422, description = "Document unreadable"),
        (status = 502, description = "Backend unavailable"),
        (status = 504, description = "Parse timeout")
    ),
    tag = "Parse"
)]
pub async fn parse_document(
    State(state): State<AppState>,
    _context: TenantContext,
    Query(query_options): Query<ParseOptions>,
    req: Request,
) -> Result<Response, ApiError> {
    let headers = req.headers().clone();
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();

    let intake: ParsedIntake = if content_type.starts_with("multipart/") {
        let multipart = Multipart::from_request(req, &state).await.map_err(|e| {
            ParseErrorCode::InvalidRequest.into_api_error(format!("Failed to parse multipart: {e}"))
        })?;
        intake_multipart(multipart).await?
    } else if content_type.contains("application/pdf")
        || content_type.contains("application/octet-stream")
        || content_type.is_empty()
    {
        let max = state.resource_budget().max_upload_bytes;
        let body = Bytes::from_request(req, &state).await.map_err(|e| {
            ParseErrorCode::TooLarge
                .into_api_error(format!("Failed to read body (limit {max}): {e}"))
        })?;
        intake_raw_pdf(&headers, body, query_options)?
    } else {
        return Err(ParseErrorCode::UnsupportedMediaType.into_api_error(format!(
            "Unsupported Content-Type '{content_type}'; expected multipart/form-data or application/pdf"
        )));
    };

    dispatch_parse(&state, &headers, intake).await
}

async fn dispatch_parse(
    state: &AppState,
    headers: &HeaderMap,
    intake: ParsedIntake,
) -> Result<Response, ApiError> {
    let request_id = format!("pr_{}", Uuid::new_v4().simple());
    let default_backend = PdfParserBackend::from_env().unwrap_or(PdfParserBackend::Vision);
    let default_provider = std::env::var("EDGEQUAKE_LLM_PROVIDER")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| state.query.llm_provider.name().to_string());
    let default_concurrency = (state.parse_jobs.max_concurrent()).clamp(1, 4);

    let resolved = resolve_options(
        &intake.options,
        &intake.filename,
        default_backend,
        &default_provider,
        default_concurrency,
    )?;
    if let (Some(provider), Some(model)) = (
        intake.options.provider.as_deref(),
        intake.options.model.as_deref(),
    ) {
        let provider = provider.trim();
        let model = model.trim();
        if !provider.is_empty() && !model.is_empty() && !provider.eq_ignore_ascii_case("mock") {
            if resolved.backend == PdfParserBackend::Vision {
                crate::services::model_policy_enforcement::ensure_deployment_choice(
                    state,
                    None,
                    None,
                    None,
                    None,
                    Some(provider),
                    Some(model),
                )
                .await?;
            } else {
                crate::services::model_policy_enforcement::ensure_deployment_choice(
                    state,
                    Some(provider),
                    Some(model),
                    None,
                    None,
                    None,
                    None,
                )
                .await?;
            }
        }
    }

    let limits = ParseLimits::from_env();
    let bytes_len = intake.bytes.len() as u64;
    let page_count = resolve_pdf_page_count(&intake.bytes, None)
        .await
        .unwrap_or(0) as u32;

    if limits.exceeds_async(page_count, bytes_len) {
        return Err(ParseErrorCode::TooLarge.into_api_error(format!(
            "Document has {page_count} pages / {bytes_len} bytes, exceeding async limits of {} pages / {} bytes",
            limits.async_max_pages, limits.async_max_bytes
        )));
    }

    let want_async = resolved.force_async
        || prefer_respond_async(headers)
        || limits.exceeds_sync(page_count, bytes_len);

    info!(
        request_id = %request_id,
        backend = resolved.backend.as_str(),
        page_count,
        bytes = bytes_len,
        async_mode = want_async,
        "Parse request admitted"
    );

    if want_async {
        let accepted = state
            .parse_jobs
            .enqueue(intake.bytes, resolved, request_id)
            .await?;
        return Ok((StatusCode::ACCEPTED, Json(accepted)).into_response());
    }

    let _permit = state.parse_jobs.acquire_permit().await.ok_or_else(|| {
        ParseErrorCode::BackendUnavailable.into_api_error("Parse admission semaphore closed")
    })?;

    let result = run_parse(&intake.bytes, &resolved, Some(request_id)).await?;
    Ok((StatusCode::OK, Json::<ParseResponse>(result)).into_response())
}
