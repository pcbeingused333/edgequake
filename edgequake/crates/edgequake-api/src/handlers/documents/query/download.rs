//! Document download handlers (original bytes + markdown).

use axum::body::{Body, Bytes};
use axum::extract::{Path, State};
use axum::http::{header, HeaderValue};
use axum::response::IntoResponse;
use futures::stream::unfold;
use serde_json::Value;
use tracing::info;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::handlers::pdf_upload::byte_range::range_windows;
use crate::handlers::pdf_upload::content::pdf_byte_stream;
use crate::middleware::TenantContext;
use crate::services::document_body_loader::load_document_body;
use crate::services::document_metadata_scan::metadata_key_for_document;
use crate::state::AppState;
use edgequake_storage::PdfDocumentStorage;

#[cfg(feature = "postgres")]
fn get_pdf_storage(state: &AppState) -> ApiResult<std::sync::Arc<dyn PdfDocumentStorage>> {
    if state.storage.is_postgresql() {
        state
            .storage
            .validate_postgres_adapters()
            .map_err(ApiError::Internal)?;
    }
    state
        .storage
        .pdf_storage
        .as_ref()
        .cloned()
        .ok_or_else(|| ApiError::Internal("PDF storage not initialized".into()))
}

#[cfg(not(feature = "postgres"))]
fn get_pdf_storage(_state: &AppState) -> ApiResult<std::sync::Arc<dyn PdfDocumentStorage>> {
    Err(ApiError::Internal(
        "PDF storage not available (postgres feature disabled)".into(),
    ))
}

#[cfg(feature = "postgres")]
fn get_original_storage(
    state: &AppState,
) -> ApiResult<std::sync::Arc<dyn edgequake_storage::DocumentOriginalStorage>> {
    if state.storage.is_postgresql() {
        state
            .storage
            .validate_postgres_adapters()
            .map_err(ApiError::Internal)?;
    }
    state
        .storage
        .original_storage
        .as_ref()
        .cloned()
        .ok_or_else(|| ApiError::Internal("Original storage not initialized".into()))
}

pub(crate) async fn load_document_metadata(
    state: &AppState,
    document_id: &str,
) -> ApiResult<Value> {
    let metadata_key = metadata_key_for_document(document_id);

    #[cfg(feature = "postgres")]
    if let Some(pool) = state.optional_pg_pool() {
        if let Some(value) =
            edgequake_storage::adapters::postgres::document_shell::shell_value_by_key(
                pool,
                &metadata_key,
            )
            .await
            .map_err(ApiError::from)?
        {
            return Ok(value);
        }
    }

    let metadata = state
        .storage
        .kv_storage
        .get_by_ids(&[metadata_key])
        .await
        .map_err(ApiError::from)?
        .into_iter()
        .next()
        .ok_or_else(|| ApiError::NotFound(format!("Document not found: {document_id}")))?;
    Ok(metadata)
}

fn metadata_pdf_id(metadata: &Value) -> Option<Uuid> {
    metadata
        .get("pdf_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
}

fn attachment_filename(name: &str) -> String {
    format!("attachment; filename=\"{}\"", name.replace('"', ""))
}

fn original_byte_stream(
    storage: std::sync::Arc<dyn edgequake_storage::DocumentOriginalStorage>,
    workspace_id: Uuid,
    document_id: Uuid,
    start: u64,
    end: u64,
) -> Body {
    let windows = range_windows(start, end).into_iter();
    Body::from_stream(unfold(
        (storage, workspace_id, document_id, windows),
        |(storage, workspace_id, document_id, mut windows)| async move {
            let (s, e) = windows.next()?;
            let item = match storage
                .get_original_bytes_range(&workspace_id, &document_id, s, e)
                .await
            {
                Ok(Some(chunk)) => Ok(Bytes::from(chunk)),
                Ok(None) => Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "original not found",
                )),
                Err(err) => Err(std::io::Error::other(err.to_string())),
            };
            Some((item, (storage, workspace_id, document_id, windows)))
        },
    ))
}

async fn assemble_pdf_bytes(
    storage: std::sync::Arc<dyn edgequake_storage::PdfDocumentStorage>,
    pdf_id: Uuid,
    total: u64,
) -> ApiResult<Vec<u8>> {
    let mut out = Vec::with_capacity(total as usize);
    if total == 0 {
        return Ok(out);
    }
    for (start, end) in range_windows(0, total - 1) {
        let chunk = storage
            .get_pdf_bytes_range(&pdf_id, start, end)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to get PDF: {e}")))?
            .ok_or_else(|| ApiError::NotFound("PDF not found".into()))?;
        out.extend(chunk);
    }
    Ok(out)
}

async fn assemble_original_bytes(
    storage: std::sync::Arc<dyn edgequake_storage::DocumentOriginalStorage>,
    workspace_id: Uuid,
    document_id: Uuid,
    total: u64,
) -> ApiResult<Vec<u8>> {
    let mut out = Vec::with_capacity(total as usize);
    if total == 0 {
        return Ok(out);
    }
    for (start, end) in range_windows(0, total - 1) {
        let chunk = storage
            .get_original_bytes_range(&workspace_id, &document_id, start, end)
            .await
            .map_err(ApiError::from)?
            .ok_or_else(|| ApiError::NotFound("Original file not found".into()))?;
        out.extend(chunk);
    }
    Ok(out)
}

/// Download the original document bytes (PDF delegation or stored upload).
#[utoipa::path(
    get,
    path = "/api/v1/documents/{document_id}/download/original",
    params(("document_id" = String, Path, description = "Document identifier")),
    responses(
        (status = 200, description = "Original file bytes"),
        (status = 404, description = "Document or original not found"),
        (status = 403, description = "Not authorized"),
    ),
    tag = "Documents"
)]
pub async fn download_document_original(
    State(state): State<AppState>,
    context: TenantContext,
    Path(document_id): Path<String>,
) -> ApiResult<axum::response::Response<axum::body::Body>> {
    let workspace_id = Uuid::parse_str(&context.workspace_id_or_default())
        .map_err(|_| ApiError::BadRequest("Invalid workspace id".into()))?;
    #[cfg(feature = "postgres")]
    if let Some(pool) = state.optional_pg_pool() {
        let tenant = context
            .tenant_id_uuid()
            .unwrap_or_else(crate::middleware::default_tenant_uuid);
        crate::services::tenant_isolation::ensure_caller_tenant(pool, tenant, workspace_id).await?;
    }

    let metadata = load_document_metadata(&state, &document_id).await?;

    if let Some(pdf_id) = metadata_pdf_id(&metadata) {
        let pdf_storage = get_pdf_storage(&state)?;
        let info = pdf_storage
            .get_pdf_blob_info(&pdf_id)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to get PDF: {e}")))?
            .ok_or_else(|| ApiError::NotFound("PDF not found".into()))?;

        if info.workspace_id != workspace_id {
            return Err(ApiError::forbidden());
        }

        info!(
            document_id = %document_id,
            pdf_id = %pdf_id,
            filename = %info.filename,
            bytes = info.total_bytes,
            "Document original download streams the PDF"
        );

        let body = if info.total_bytes == 0 {
            Body::empty()
        } else {
            pdf_byte_stream(pdf_storage, pdf_id, 0, info.total_bytes - 1)
        };
        let content_length = HeaderValue::from_str(&info.total_bytes.to_string())
            .unwrap_or_else(|_| HeaderValue::from_static("0"));
        return Ok(axum::http::Response::builder()
            .header(header::CONTENT_TYPE, "application/pdf")
            .header(
                header::CONTENT_DISPOSITION,
                attachment_filename(&info.filename),
            )
            .header(header::CACHE_CONTROL, "private, max-age=3600")
            .header(header::ACCEPT_RANGES, "bytes")
            .header(header::CONTENT_LENGTH, content_length)
            .body(body)
            .unwrap_or_else(|_| {
                axum::http::Response::builder()
                    .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from("failed to build PDF response"))
                    .expect("error response")
            }));
    }

    #[cfg(feature = "postgres")]
    {
        let document_uuid = Uuid::parse_str(&document_id)
            .map_err(|_| ApiError::BadRequest("Invalid document id".into()))?;
        let original_storage = get_original_storage(&state)?;
        let info = original_storage
            .get_original_info(&workspace_id, &document_uuid)
            .await
            .map_err(ApiError::from)?
            .ok_or_else(|| ApiError::NotFound("Original file not found".into()))?;

        info!(
            document_id = %document_id,
            filename = %info.filename,
            bytes = info.total_bytes,
            "Document original download streams the file"
        );

        let body = if info.total_bytes == 0 {
            Body::empty()
        } else {
            original_byte_stream(
                original_storage,
                workspace_id,
                document_uuid,
                0,
                info.total_bytes - 1,
            )
        };
        let content_length = HeaderValue::from_str(&info.total_bytes.to_string())
            .unwrap_or_else(|_| HeaderValue::from_static("0"));
        let content_type = HeaderValue::from_str(&info.content_type)
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream"));
        Ok(axum::http::Response::builder()
            .header(header::CONTENT_TYPE, content_type)
            .header(
                header::CONTENT_DISPOSITION,
                attachment_filename(&info.filename),
            )
            .header(header::CACHE_CONTROL, "private, max-age=3600")
            .header(header::ACCEPT_RANGES, "bytes")
            .header(header::CONTENT_LENGTH, content_length)
            .body(body)
            .unwrap_or_else(|_| {
                axum::http::Response::builder()
                    .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from("failed to build original response"))
                    .expect("error response")
            }))
    }

    #[cfg(not(feature = "postgres"))]
    Err(ApiError::NotFound("Original file not found".into()))
}

/// Bytes + MIME for REST and MCP. Does not wait for pipeline completion.
pub(crate) async fn load_original_bytes(
    state: &AppState,
    tenant_ctx: &TenantContext,
    document_id: &str,
) -> ApiResult<(Vec<u8>, String)> {
    let workspace_id = Uuid::parse_str(&tenant_ctx.workspace_id_or_default())
        .map_err(|_| ApiError::BadRequest("Invalid workspace id".into()))?;
    #[cfg(feature = "postgres")]
    if let Some(pool) = state.optional_pg_pool() {
        let tenant = tenant_ctx
            .tenant_id_uuid()
            .unwrap_or_else(crate::middleware::default_tenant_uuid);
        crate::services::tenant_isolation::ensure_caller_tenant(pool, tenant, workspace_id).await?;
    }
    let metadata = load_document_metadata(state, document_id).await?;

    if let Some(pdf_id) = metadata_pdf_id(&metadata) {
        let pdf_storage = get_pdf_storage(state)?;
        let info = pdf_storage
            .get_pdf_blob_info(&pdf_id)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to get PDF: {e}")))?
            .ok_or_else(|| ApiError::NotFound("PDF not found".into()))?;
        if info.workspace_id != workspace_id {
            return Err(ApiError::forbidden());
        }
        let bytes = assemble_pdf_bytes(pdf_storage, pdf_id, info.total_bytes).await?;
        return Ok((bytes, info.content_type));
    }

    #[cfg(feature = "postgres")]
    {
        let document_uuid = Uuid::parse_str(document_id)
            .map_err(|_| ApiError::BadRequest("Invalid document id".into()))?;
        let original_storage = get_original_storage(state)?;
        let info = original_storage
            .get_original_info(&workspace_id, &document_uuid)
            .await
            .map_err(ApiError::from)?
            .ok_or_else(|| ApiError::NotFound("Original file not found".into()))?;
        let bytes = assemble_original_bytes(
            original_storage,
            workspace_id,
            document_uuid,
            info.total_bytes,
        )
        .await?;
        Ok((bytes, info.content_type))
    }

    #[cfg(not(feature = "postgres"))]
    Err(ApiError::NotFound("Original file not found".into()))
}

/// Download extracted markdown for a document.
#[utoipa::path(
    get,
    path = "/api/v1/documents/{document_id}/download/markdown",
    params(("document_id" = String, Path, description = "Document identifier")),
    responses(
        (status = 200, description = "Markdown file", content_type = "text/markdown"),
        (status = 404, description = "Document or markdown not found"),
    ),
    tag = "Documents"
)]
pub async fn download_document_markdown(
    State(state): State<AppState>,
    _context: TenantContext,
    Path(document_id): Path<String>,
) -> ApiResult<axum::response::Response<axum::body::Body>> {
    let metadata = load_document_metadata(&state, &document_id).await?;
    let body = load_document_body(&state.storage, &document_id, &metadata)
        .await
        .ok_or_else(|| ApiError::NotFound("Markdown content not found".into()))?;

    let filename = metadata
        .get("file_name")
        .or_else(|| metadata.get("title"))
        .and_then(|v| v.as_str())
        .map(|name| {
            if name.to_lowercase().ends_with(".md") {
                name.to_string()
            } else {
                format!("{name}.md")
            }
        })
        .unwrap_or_else(|| format!("{document_id}.md"));

    info!(document_id = %document_id, filename = %filename, "Document markdown download");

    Ok((
        [
            (header::CONTENT_TYPE, "text/markdown; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                attachment_filename(&filename).as_str(),
            ),
            (header::CACHE_CONTROL, "private, max-age=3600"),
        ],
        body.markdown,
    )
        .into_response())
}
