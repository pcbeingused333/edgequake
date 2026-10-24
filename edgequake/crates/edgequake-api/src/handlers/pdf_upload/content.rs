use axum::body::{Body, Bytes};
use axum::extract::{Path, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use futures::stream::unfold;
use serde::Serialize;
use tracing::debug;
use utoipa::ToSchema;
use uuid::Uuid;

use super::byte_range::{
    evaluate_range, pdf_body_plan, pdf_content_disposition, range_windows, ByteRange, PdfBodyPlan,
};
use super::helpers::get_pdf_storage;
use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::state::AppState;
use edgequake_storage::{PdfDocumentStorage, PdfProcessingStatus};

// ============================================================================
// PDF Content Download Endpoints (SPEC-002: Document Viewer)
// ============================================================================

/// PDF download response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PdfContentResponse {
    /// PDF ID.
    pub pdf_id: String,
    /// Linked document ID (mm-assets / markdown viewer scope).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_id: Option<String>,
    /// Original filename.
    pub filename: String,
    /// File size in bytes.
    pub file_size_bytes: i64,
    /// MIME type.
    pub content_type: String,
    /// Extracted markdown content (if processed).
    pub markdown_content: Option<String>,
    /// Whether PDF processing is complete.
    pub is_processed: bool,
}

/// Download raw PDF file data.
///
/// @implements SPEC-002: Document Viewer - PDF download endpoint
/// @implements UC0711: Download PDF for viewing
/// @enforces BR0701: Workspace isolation
///
/// Returns the raw PDF binary data with appropriate content-type headers.
/// This allows the frontend PDF viewer to render the original document.
///
/// # Arguments
///
/// * `state` - Application state with PDF storage
/// * `context` - Tenant context for workspace isolation
/// * `pdf_id` - PDF identifier
///
/// # Returns
///
/// * `Ok(Response)` - Raw PDF data with application/pdf content-type
/// * `Err(404)` - PDF not found
/// * `Err(403)` - Not authorized for this workspace
#[utoipa::path(
    get,
    path = "/api/v1/documents/pdf/{pdf_id}/download",
    params(
        ("pdf_id" = String, Path, description = "PDF identifier")
    ),
    responses(
        (status = 200, description = "Raw PDF data", content_type = "application/pdf"),
        (status = 206, description = "Partial PDF data for a `Range: bytes=` request", content_type = "application/pdf"),
        (status = 416, description = "Range not satisfiable"),
        (status = 404, description = "PDF not found"),
        (status = 403, description = "Not authorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Documents"
)]
pub async fn download_pdf(
    State(state): State<AppState>,
    context: TenantContext,
    headers: axum::http::HeaderMap,
    Path(pdf_id): Path<String>,
) -> ApiResult<axum::response::Response<axum::body::Body>> {
    let pdf_id = Uuid::parse_str(&pdf_id)
        .map_err(|_| ApiError::BadRequest("Invalid PDF ID format".to_string()))?;

    let pdf_storage = get_pdf_storage(&state)?;

    // Header-only lookup: auth + size without loading the blob, so a Range
    // request never costs a whole-file read.
    let info = pdf_storage
        .get_pdf_blob_info(&pdf_id)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get PDF: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("PDF not found".to_string()))?;

    // OODA-51: Make workspace verification optional for PDF viewer compatibility
    // WHY: react-pdf Document component loads PDFs via URL without custom headers,
    // so X-Workspace-ID header is not available. The PDF is already isolated by its
    // UUID which is unique per workspace, so access is implicitly scoped.
    // If workspace header IS provided, verify it matches for defense-in-depth.
    if let Some(workspace_id) = context.workspace_id_uuid() {
        if info.workspace_id != workspace_id {
            return Err(ApiError::forbidden());
        }
    }
    #[cfg(feature = "postgres")]
    if let Some(pool) = state.optional_pg_pool() {
        let tenant = context
            .tenant_id_uuid()
            .unwrap_or_else(crate::middleware::default_tenant_uuid);
        crate::services::tenant_isolation::ensure_caller_tenant(pool, tenant, info.workspace_id)
            .await?;
    }

    let range_header = headers
        .get(axum::http::header::RANGE)
        .and_then(|v| v.to_str().ok());
    let range = evaluate_range(range_header, info.total_bytes);
    debug!(
        "PDF download: id={}, filename={}, size={}, range={:?} -> {:?}",
        pdf_id, info.filename, info.total_bytes, range_header, range
    );

    let plan = pdf_body_plan(range, info.total_bytes);
    let (body, body_len) = match plan {
        PdfBodyPlan::Unsatisfiable | PdfBodyPlan::Empty => (Body::empty(), 0),
        PdfBodyPlan::Stream { start, end } => {
            let body_len = end.saturating_sub(start).saturating_add(1);
            (
                pdf_byte_stream(pdf_storage.clone(), pdf_id, start, end),
                body_len,
            )
        }
    };

    Ok(build_pdf_response(
        &info.filename,
        info.total_bytes,
        range,
        body,
        body_len,
    ))
}

/// Stream `start..=end` in [`super::byte_range::PDF_RANGE_CHUNK`] windows.
///
/// Each poll of the body awaits at most one storage read, so pdf.js aborting
/// the unranged probe after headers costs at most one chunk (never `get_pdf`).
pub(crate) fn pdf_byte_stream(
    storage: std::sync::Arc<dyn PdfDocumentStorage>,
    pdf_id: Uuid,
    start: u64,
    end: u64,
) -> Body {
    let windows = range_windows(start, end).into_iter();
    Body::from_stream(unfold(
        (storage, pdf_id, windows),
        |(storage, pdf_id, mut windows)| async move {
            let (s, e) = windows.next()?;
            let item = match storage.get_pdf_bytes_range(&pdf_id, s, e).await {
                Ok(Some(chunk)) => Ok(Bytes::from(chunk)),
                Ok(None) => Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "PDF not found",
                )),
                Err(err) => Err(std::io::Error::other(err.to_string())),
            };
            Some((item, (storage, pdf_id, windows)))
        },
    ))
}

/// Build the `200` / `206` / `416` PDF response for an evaluated range.
///
/// WHY range support: pdf.js reads the xref at the end of the file and then
/// only the chunks backing the pages in view, so page 1 paints without
/// transferring the whole file. `Content-Encoding: identity` is explicit because
/// pdf.js disables range requests on compressed responses (and gzip on a PDF
/// gains nothing).
fn build_pdf_response(
    filename: &str,
    total: u64,
    range: ByteRange,
    body: Body,
    body_len: u64,
) -> axum::response::Response<axum::body::Body> {
    let content_disposition = pdf_content_disposition(filename);
    let content_length = HeaderValue::from_str(&body_len.to_string())
        .unwrap_or_else(|_| HeaderValue::from_static("0"));

    let mut builder = axum::http::Response::builder()
        .header(header::CONTENT_TYPE, "application/pdf")
        .header(header::CONTENT_DISPOSITION, content_disposition)
        .header(header::CACHE_CONTROL, "private, max-age=3600")
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CONTENT_ENCODING, "identity")
        .header(header::CONTENT_LENGTH, content_length);

    match range {
        ByteRange::Full => {
            builder = builder.status(StatusCode::OK);
        }
        ByteRange::Partial { start, end } => {
            builder = builder.status(StatusCode::PARTIAL_CONTENT).header(
                header::CONTENT_RANGE,
                format!("bytes {start}-{end}/{total}"),
            );
        }
        ByteRange::Unsatisfiable => {
            builder = builder
                .status(StatusCode::RANGE_NOT_SATISFIABLE)
                .header(header::CONTENT_RANGE, format!("bytes */{total}"));
        }
    }

    builder.body(body).unwrap_or_else(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to build PDF response",
        )
            .into_response()
    })
}

/// Get PDF content metadata including markdown.
///
/// @implements SPEC-002: Document Viewer - Markdown content endpoint
/// @implements UC0712: Get PDF metadata with extracted markdown
/// @enforces BR0701: Workspace isolation
///
/// Returns PDF metadata including the extracted markdown content (if processed).
/// This allows the frontend to display both the original PDF and the extracted markdown.
///
/// # Arguments
///
/// * `state` - Application state with PDF storage
/// * `context` - Tenant context for workspace isolation
/// * `pdf_id` - PDF identifier
///
/// # Returns
///
/// * `Ok(Json(PdfContentResponse))` - PDF metadata with markdown
/// * `Err(404)` - PDF not found
/// * `Err(403)` - Not authorized for this workspace
#[utoipa::path(
    get,
    path = "/api/v1/documents/pdf/{pdf_id}/content",
    params(
        ("pdf_id" = String, Path, description = "PDF identifier")
    ),
    responses(
        (status = 200, description = "PDF content metadata", body = PdfContentResponse),
        (status = 404, description = "PDF not found"),
        (status = 403, description = "Not authorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Documents"
)]
pub async fn get_pdf_content(
    State(state): State<AppState>,
    context: TenantContext,
    Path(pdf_id): Path<String>,
) -> ApiResult<Json<PdfContentResponse>> {
    let pdf_id = Uuid::parse_str(&pdf_id)
        .map_err(|_| ApiError::BadRequest("Invalid PDF ID format".to_string()))?;

    let pdf_storage = get_pdf_storage(&state)?;

    let pdf = pdf_storage
        .get_pdf_content_view(&pdf_id)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get PDF: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("PDF not found".to_string()))?;

    // OODA-51: Make workspace verification optional for PDF viewer compatibility
    // WHY: Frontend PDF components may not have access to custom headers.
    // If workspace header IS provided, verify it matches for defense-in-depth.
    if let Some(workspace_id) = context.workspace_id_uuid() {
        if pdf.workspace_id != workspace_id {
            return Err(ApiError::forbidden());
        }
    }
    #[cfg(feature = "postgres")]
    if let Some(pool) = state.optional_pg_pool() {
        let tenant = context
            .tenant_id_uuid()
            .unwrap_or_else(crate::middleware::default_tenant_uuid);
        crate::services::tenant_isolation::ensure_caller_tenant(pool, tenant, pdf.workspace_id)
            .await?;
    }

    let is_processed = pdf.processing_status == PdfProcessingStatus::Completed;

    Ok(Json(PdfContentResponse {
        pdf_id: pdf.pdf_id.to_string(),
        document_id: pdf.document_id.map(|id| id.to_string()),
        filename: pdf.filename,
        file_size_bytes: pdf.file_size_bytes,
        content_type: pdf.content_type,
        markdown_content: pdf.markdown_content,
        is_processed,
    }))
}

#[cfg(test)]
mod range_response_tests {
    use super::{build_pdf_response, evaluate_range, pdf_body_plan, PdfBodyPlan};
    use axum::body::Body;
    use axum::http::{header, StatusCode};

    fn sample() -> Vec<u8> {
        (0u8..100).collect()
    }

    /// Mirror of the handler: evaluate the header, then emit a sized body.
    fn respond(range_header: Option<&str>) -> axum::response::Response<axum::body::Body> {
        let data = sample();
        let total = data.len() as u64;
        let range = evaluate_range(range_header, total);
        let (body, body_len) = match pdf_body_plan(range, total) {
            PdfBodyPlan::Unsatisfiable | PdfBodyPlan::Empty => (Body::empty(), 0),
            PdfBodyPlan::Stream { start, end } => {
                let slice = data[start as usize..=end as usize].to_vec();
                let len = slice.len() as u64;
                (Body::from(slice), len)
            }
        };
        build_pdf_response("a.pdf", total, range, body, body_len)
    }

    #[tokio::test]
    async fn full_response_advertises_range_support() {
        let res = respond(None);
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.headers()[header::ACCEPT_RANGES], "bytes");
        assert_eq!(res.headers()[header::CONTENT_ENCODING], "identity");
        assert_eq!(res.headers()[header::CONTENT_LENGTH], "100");
        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(body.len(), 100);
    }

    #[tokio::test]
    async fn range_request_returns_206_with_exact_slice() {
        let res = respond(Some("bytes=10-19"));
        assert_eq!(res.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(res.headers()[header::CONTENT_RANGE], "bytes 10-19/100");
        assert_eq!(res.headers()[header::CONTENT_LENGTH], "10");
        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(body.as_ref(), &sample()[10..=19]);
    }

    #[tokio::test]
    async fn suffix_and_open_ended_set_content_length() {
        let open = respond(Some("bytes=90-"));
        assert_eq!(open.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(open.headers()[header::CONTENT_RANGE], "bytes 90-99/100");
        assert_eq!(open.headers()[header::CONTENT_LENGTH], "10");

        let suffix = respond(Some("bytes=-5"));
        assert_eq!(suffix.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(suffix.headers()[header::CONTENT_RANGE], "bytes 95-99/100");
        assert_eq!(suffix.headers()[header::CONTENT_LENGTH], "5");
    }

    #[tokio::test]
    async fn out_of_bounds_range_returns_416() {
        let res = respond(Some("bytes=500-"));
        assert_eq!(res.status(), StatusCode::RANGE_NOT_SATISFIABLE);
        assert_eq!(res.headers()[header::CONTENT_RANGE], "bytes */100");
        assert_eq!(res.headers()[header::CONTENT_LENGTH], "0");
    }

    #[tokio::test]
    async fn malformed_range_returns_416_not_full_body() {
        let res = respond(Some("bytes=0-9,20-29"));
        assert_eq!(res.status(), StatusCode::RANGE_NOT_SATISFIABLE);
        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        assert!(body.is_empty());
    }

    #[tokio::test]
    async fn disposition_never_embeds_crlf() {
        let range = super::ByteRange::Full;
        let res = build_pdf_response("evil\r\nX-Injected: 1.pdf", 0, range, Body::empty(), 0);
        let disp = res.headers()[header::CONTENT_DISPOSITION].to_str().unwrap();
        assert!(!disp.contains('\r'));
        assert!(!disp.contains('\n'));
    }
}
