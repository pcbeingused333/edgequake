//! Document multimodal asset binary storage (page PNGs / chart crops).
//!
//! Mirrors [`crate::original_storage`] — BYTEA in Postgres, not KV JSON.
//! Identity: filename stem (`page-0001-chart`); keep in sync with
//! `edgequake_pdf::asset_id_from_rel_path`.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

use crate::error::{Result, StorageError};

/// Canonical asset kinds persisted with vision page renders.
pub const ASSET_KIND_PAGE_FULL: &str = "page_full";
pub const ASSET_KIND_PAGE_CHART_CROP: &str = "page_chart_crop";
pub const ASSET_KIND_EMBEDDED_FIGURE: &str = "embedded_figure";
pub const ASSET_KIND_TABLE_CROP: &str = "table_crop";

/// One stored multimodal asset linked to a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMmAsset {
    pub document_id: Uuid,
    pub workspace_id: Uuid,
    /// Stable REST id (filename stem), e.g. `page-0001-chart`.
    pub asset_id: String,
    pub asset_path: String,
    pub content_type: String,
    pub file_size_bytes: i64,
    pub asset_data: Vec<u8>,
    pub asset_kind: String,
    pub page_num: Option<i32>,
    pub created_at: DateTime<Utc>,
}

/// Request to upsert one mm-asset row.
#[derive(Debug, Clone)]
pub struct StoreMmAssetRequest {
    pub document_id: Uuid,
    pub workspace_id: Uuid,
    pub asset_id: String,
    pub asset_path: String,
    pub content_type: String,
    pub asset_data: Vec<u8>,
    pub asset_kind: String,
    pub page_num: Option<i32>,
}

/// Lightweight asset row for lineage / list APIs (no BYTEA).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocumentMmAssetSummary {
    pub asset_id: String,
    pub asset_path: String,
    pub content_type: String,
    pub file_size_bytes: i64,
    pub asset_kind: String,
    pub page_num: Option<i32>,
}

/// Storage trait for document multimodal assets (SPEC-047 durable mm-assets).
#[async_trait]
pub trait DocumentMmAssetStorage: Send + Sync {
    async fn store_asset(&self, request: StoreMmAssetRequest) -> Result<()>;

    async fn store_assets(&self, requests: &[StoreMmAssetRequest]) -> Result<()> {
        for request in requests {
            self.store_asset(request.clone()).await?;
        }
        Ok(())
    }

    async fn get_asset(
        &self,
        workspace_id: &Uuid,
        document_id: &Uuid,
        asset_path: &str,
    ) -> Result<Option<DocumentMmAsset>>;

    /// First-principles lookup: document + stable asset id.
    async fn get_asset_by_id(
        &self,
        workspace_id: &Uuid,
        document_id: &Uuid,
        asset_id: &str,
    ) -> Result<Option<DocumentMmAsset>>;

    async fn list_asset_paths(
        &self,
        workspace_id: &Uuid,
        document_id: &Uuid,
    ) -> Result<Vec<String>>;

    /// Page-linked asset summaries for document lineage / list (no binary payload).
    async fn list_asset_summaries(
        &self,
        workspace_id: &Uuid,
        document_id: &Uuid,
    ) -> Result<Vec<DocumentMmAssetSummary>>;

    async fn delete_assets_for_document(
        &self,
        workspace_id: &Uuid,
        document_id: &Uuid,
    ) -> Result<u64>;
}

/// Stable REST asset id from relative path (filename stem).
/// Must match `edgequake_pdf::asset_id_from_rel_path`.
pub fn asset_id_from_path(asset_path: &str) -> String {
    let name = asset_path.rsplit('/').next().unwrap_or(asset_path).trim();
    Path::new(name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(name)
        .to_string()
}

/// Infer kind + page number from relative path (`assets/page-0001-fig-01.png`).
pub fn classify_mm_asset_path(asset_path: &str) -> (String, Option<i32>) {
    let name = asset_path.rsplit('/').next().unwrap_or(asset_path).trim();
    let page_num = name.strip_prefix("page-").and_then(|rest| {
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        digits.parse::<i32>().ok()
    });
    let kind = if name.contains("-fig-") {
        ASSET_KIND_EMBEDDED_FIGURE
    } else if name.contains("-table-") {
        ASSET_KIND_TABLE_CROP
    } else if name.contains("-chart.") {
        ASSET_KIND_PAGE_CHART_CROP
    } else {
        ASSET_KIND_PAGE_FULL
    };
    (kind.to_string(), page_num)
}

/// Normalize and validate a relative asset path (no traversal).
pub fn normalize_mm_asset_path(asset_path: &str) -> Result<String> {
    let trimmed = asset_path.trim().trim_start_matches('/');
    if trimmed.is_empty() {
        return Err(StorageError::InvalidData("empty asset path".into()));
    }
    if trimmed.contains("..") || trimmed.starts_with('/') {
        return Err(StorageError::InvalidData(format!(
            "invalid asset path: {asset_path}"
        )));
    }
    Ok(trimmed.to_string())
}

/// Normalize asset id (no path separators / traversal).
pub fn normalize_mm_asset_id(asset_id: &str) -> Result<String> {
    let trimmed = asset_id.trim().trim_start_matches('/');
    if trimmed.is_empty() {
        return Err(StorageError::InvalidData("empty asset id".into()));
    }
    if trimmed.contains('/') || trimmed.contains("..") || trimmed.contains('\\') {
        return Err(StorageError::InvalidData(format!(
            "invalid asset id: {asset_id}"
        )));
    }
    Ok(trimmed.to_string())
}

/// Content type safe to put on an image response. Anything else, including
/// HTML and SVG, is an opaque download so the API origin cannot render it.
pub fn safe_mm_asset_content_type(content_type: &str) -> &'static str {
    match content_type.trim().to_ascii_lowercase().as_str() {
        "image/png" => "image/png",
        "image/jpeg" => "image/jpeg",
        "image/webp" => "image/webp",
        "image/gif" => "image/gif",
        _ => "application/octet-stream",
    }
}

/// Guess content-type from path extension.
pub fn guess_mm_asset_content_type(asset_path: &str) -> &'static str {
    let lower = asset_path.to_ascii_lowercase();
    if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else if lower.ends_with(".gif") {
        "image/gif"
    } else {
        "application/octet-stream"
    }
}

/// Reject empty payloads.
pub fn validate_mm_asset_data(data: &[u8]) -> Result<()> {
    if data.is_empty() {
        return Err(StorageError::InvalidData(
            "mm-asset data cannot be empty".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_chart_crop_path() {
        let (kind, page) = classify_mm_asset_path("assets/page-0006-table-01.png");
        assert_eq!(kind, ASSET_KIND_TABLE_CROP);
        assert_eq!(page, Some(6));

        let (kind, page) = classify_mm_asset_path("assets/page-0002-chart.png");
        assert_eq!(kind, ASSET_KIND_PAGE_CHART_CROP);
        assert_eq!(page, Some(2));
        assert_eq!(
            asset_id_from_path("assets/page-0002-chart.png"),
            "page-0002-chart"
        );
    }

    #[test]
    fn content_type_allowlist_blocks_html_and_svg() {
        assert_eq!(safe_mm_asset_content_type("image/png"), "image/png");
        assert_eq!(safe_mm_asset_content_type("IMAGE/JPEG"), "image/jpeg");
        assert_eq!(
            safe_mm_asset_content_type("text/html"),
            "application/octet-stream"
        );
        assert_eq!(
            safe_mm_asset_content_type("image/svg+xml"),
            "application/octet-stream"
        );
        assert_eq!(
            safe_mm_asset_content_type("image/png\r\nX-Evil: 1"),
            "application/octet-stream"
        );
    }

    #[test]
    fn classifies_full_page_path() {
        let (kind, page) = classify_mm_asset_path("assets/page-0042.png");
        assert_eq!(kind, ASSET_KIND_PAGE_FULL);
        assert_eq!(page, Some(42));
        assert_eq!(asset_id_from_path("assets/page-0042.png"), "page-0042");
    }

    #[test]
    fn rejects_traversal() {
        assert!(normalize_mm_asset_path("../x").is_err());
        assert!(normalize_mm_asset_path("assets/page-0001.png").is_ok());
        assert!(normalize_mm_asset_id("page-0001").is_ok());
        assert!(normalize_mm_asset_id("a/b").is_err());
    }
}
