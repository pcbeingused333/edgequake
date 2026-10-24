//! Original document binary storage for non-PDF uploads.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{Result, StorageError};

/// Stored original upload bytes linked to a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentOriginal {
    pub document_id: Uuid,
    pub workspace_id: Uuid,
    pub filename: String,
    pub content_type: String,
    pub file_size_bytes: i64,
    pub original_data: Vec<u8>,
    pub created_at: DateTime<Utc>,
}

/// Request to persist original upload bytes.
#[derive(Debug, Clone)]
pub struct StoreOriginalRequest {
    pub document_id: Uuid,
    pub workspace_id: Uuid,
    pub filename: String,
    pub content_type: String,
    pub original_data: Vec<u8>,
}

pub struct OriginalBlobInfo {
    pub filename: String,
    pub content_type: String,
    pub total_bytes: u64,
}

/// Storage trait for non-PDF original uploads.
#[async_trait]
pub trait DocumentOriginalStorage: Send + Sync {
    async fn store_original(&self, request: StoreOriginalRequest) -> Result<()>;

    async fn get_original(
        &self,
        workspace_id: &Uuid,
        document_id: &Uuid,
    ) -> Result<Option<DocumentOriginal>>;

    /// Size and name without the bytes. The default reads the whole original.
    async fn get_original_info(
        &self,
        workspace_id: &Uuid,
        document_id: &Uuid,
    ) -> Result<Option<OriginalBlobInfo>> {
        Ok(self
            .get_original(workspace_id, document_id)
            .await?
            .map(|original| OriginalBlobInfo {
                filename: original.filename,
                content_type: original.content_type,
                total_bytes: original.original_data.len() as u64,
            }))
    }

    /// Inclusive window. The default slices a full read.
    async fn get_original_bytes_range(
        &self,
        workspace_id: &Uuid,
        document_id: &Uuid,
        start: u64,
        end: u64,
    ) -> Result<Option<Vec<u8>>> {
        Ok(self
            .get_original(workspace_id, document_id)
            .await?
            .and_then(|original| {
                let (s, e) = (start as usize, end as usize);
                (e < original.original_data.len() && s <= e)
                    .then(|| original.original_data[s..=e].to_vec())
            }))
    }

    async fn delete_original(&self, workspace_id: &Uuid, document_id: &Uuid) -> Result<bool>;
}

/// Validate original payload size before persistence.
pub fn validate_original_data(data: &[u8], max_bytes: usize) -> Result<()> {
    if data.is_empty() {
        return Err(StorageError::InvalidData(
            "Original upload data cannot be empty".into(),
        ));
    }
    if data.len() > max_bytes {
        return Err(StorageError::InvalidData(format!(
            "Original upload exceeds max size ({} > {})",
            data.len(),
            max_bytes
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_original() {
        assert!(validate_original_data(&[], 1024).is_err());
    }

    #[tokio::test]
    async fn a_range_is_a_slice_of_the_original() {
        let storage = crate::adapters::memory::MemoryOriginalStorage::new();
        let document_id = Uuid::nil();
        let workspace_id = Uuid::from_u128(1);
        storage
            .store_original(StoreOriginalRequest {
                document_id,
                workspace_id,
                filename: "notes.txt".into(),
                content_type: "text/plain".into(),
                original_data: b"abcdefghij".to_vec(),
            })
            .await
            .unwrap();
        let info = storage
            .get_original_info(&workspace_id, &document_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(info.total_bytes, 10);
        let slice = storage
            .get_original_bytes_range(&workspace_id, &document_id, 2, 4)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(slice, b"cde");
    }
}
