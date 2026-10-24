//! Document markdown/body loader SSOT (SPEC-028 DRY).
//!
//! Unifies KV `{doc_id}-content` reads with PDF pipeline `markdown_content` hydration
//! and chunk-aggregation heal for truncated shell bodies.

use serde_json::Value;
use tracing::warn;
use uuid::Uuid;

use crate::state::StorageRuntime;

/// Historical finalize bug wrote only the first 500 chars into `documents.content`.
/// Used as the heal threshold when chunks hold a longer body.
const SUMMARY_TRUNCATE_CHARS: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentBodySource {
    Kv,
    PdfStorage,
    /// Rebuilt from ordered `{doc}-chunk-N` rows (heal for truncated shell).
    Chunks,
}

impl DocumentBodySource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Kv => "kv",
            Self::PdfStorage => "pdf_storage",
            Self::Chunks => "chunks",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DocumentBody {
    pub markdown: String,
    pub source: DocumentBodySource,
    pub pdf_id: Option<String>,
}

pub async fn load_document_body(
    storage: &StorageRuntime,
    document_id: &str,
    metadata: &Value,
) -> Option<DocumentBody> {
    if crate::services::clearance::must_hide(document_id)
        .await
        .unwrap_or(true)
    {
        return None;
    }
    let shell = load_kv_document_body(storage, document_id).await;
    // Avoid chunk-prefix scans on healthy full bodies (common path).
    let shell_may_need_heal = match &shell {
        None => true,
        Some(s) => s.markdown.chars().count() <= SUMMARY_TRUNCATE_CHARS,
    };
    if shell_may_need_heal {
        if let Some(chunks_body) = load_chunks_document_body(storage, document_id).await {
            let needs_heal = match &shell {
                None => true,
                Some(s) => shell_looks_truncated(&s.markdown, chunks_body.markdown.chars().count()),
            };
            if needs_heal {
                write_back_shell_content(storage, document_id, &chunks_body.markdown).await;
                return Some(chunks_body);
            }
        }
    }
    if let Some(shell) = shell {
        return Some(shell);
    }
    #[cfg(feature = "postgres")]
    return load_pdf_markdown_body(storage, metadata).await;
    #[cfg(not(feature = "postgres"))]
    {
        let _ = metadata;
        None
    }
}

/// True when shell is empty or a classic summary-length truncate vs a longer chunk body.
pub(crate) fn shell_looks_truncated(shell: &str, chunk_body_chars: usize) -> bool {
    let shell_chars = shell.chars().count();
    if shell_chars == 0 {
        return chunk_body_chars > 0;
    }
    shell_chars <= SUMMARY_TRUNCATE_CHARS && chunk_body_chars > shell_chars
}

async fn load_kv_document_body(
    storage: &StorageRuntime,
    document_id: &str,
) -> Option<DocumentBody> {
    // Final key first; staging during admit/pre-promote (SPEC-086).
    let keys = [
        format!("{document_id}-content"),
        edgequake_storage::kv_keys::staging_doc_content(document_id),
    ];
    let values = storage.kv_storage.get_by_ids_ordered(&keys).await.ok()?;
    let markdown = values.into_iter().find_map(|maybe| {
        maybe.and_then(|val| {
            val.get("content")
                .or_else(|| val.get("text"))
                .and_then(|c| c.as_str())
                .map(str::to_string)
                .filter(|s| !s.is_empty())
        })
    })?;
    Some(DocumentBody {
        markdown,
        source: DocumentBodySource::Kv,
        pdf_id: None,
    })
}

async fn load_chunks_document_body(
    storage: &StorageRuntime,
    document_id: &str,
) -> Option<DocumentBody> {
    let prefix = edgequake_storage::kv_keys::doc_chunk_prefix(document_id);
    let keys = storage
        .kv_storage
        .keys_with_prefix(&prefix)
        .await
        .ok()
        .filter(|k| !k.is_empty())?;

    let mut indexed: Vec<(usize, String)> = keys
        .into_iter()
        .filter_map(|key| {
            let (_doc, index) = edgequake_storage::kv_keys::parse_doc_chunk(&key)?;
            Some((index, key))
        })
        .collect();
    if indexed.is_empty() {
        return None;
    }
    indexed.sort_by_key(|(index, _)| *index);

    let ordered_keys: Vec<String> = indexed.into_iter().map(|(_, key)| key).collect();
    let contents =
        edgequake_storage::batch_fetch_chunk_contents(storage.kv_storage.as_ref(), &ordered_keys)
            .await
            .ok()?;

    let mut parts: Vec<String> = Vec::with_capacity(ordered_keys.len());
    for key in &ordered_keys {
        if let Some(text) = contents.get(key) {
            if !text.is_empty() {
                parts.push(text.clone());
            }
        }
    }
    if parts.is_empty() {
        return None;
    }

    Some(DocumentBody {
        markdown: parts.join("\n"),
        source: DocumentBodySource::Chunks,
        pdf_id: None,
    })
}

async fn write_back_shell_content(storage: &StorageRuntime, document_id: &str, markdown: &str) {
    let key = edgequake_storage::kv_keys::doc_content(document_id);
    let value = serde_json::json!({ "content": markdown });
    if let Err(e) = storage.kv_storage.upsert(&[(key, value)]).await {
        warn!(
            document_id = %document_id,
            error = %e,
            "Failed to write-back healed document body to shell (non-fatal)"
        );
    }
}

#[cfg(feature = "postgres")]
async fn load_pdf_markdown_body(
    storage: &StorageRuntime,
    metadata: &Value,
) -> Option<DocumentBody> {
    let obj = metadata.as_object()?;
    let is_pdf = obj
        .get("source_type")
        .and_then(|v| v.as_str())
        .is_some_and(|s| s == "pdf");
    if !is_pdf {
        return None;
    }
    let pdf_id_str = obj.get("pdf_id").and_then(|v| v.as_str())?;
    let pdf_uuid = Uuid::parse_str(pdf_id_str).ok()?;
    let pdf_storage = storage.pdf_storage.as_ref()?;
    let pdf = pdf_storage.get_pdf(&pdf_uuid).await.ok()??;
    let markdown = pdf.markdown_content.filter(|s| !s.trim().is_empty())?;
    Some(DocumentBody {
        markdown,
        source: DocumentBodySource::PdfStorage,
        pdf_id: Some(pdf_id_str.to_string()),
    })
}

pub fn pdf_api_paths(pdf_id: &str) -> (String, String) {
    (
        format!("/api/v1/documents/pdf/{pdf_id}/download"),
        format!("/api/v1/documents/pdf/{pdf_id}/content"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_storage::adapters::memory::{
        MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage, MemoryWorkspaceVectorRegistry,
    };
    use edgequake_storage::kv_keys;
    use edgequake_storage::traits::KVStorage;
    use std::sync::Arc;

    fn memory_runtime(kv: Arc<MemoryKVStorage>) -> StorageRuntime {
        let vector = Arc::new(MemoryVectorStorage::new("test", 1536));
        let graph = Arc::new(MemoryGraphStorage::new("test"));
        let registry: Arc<dyn edgequake_storage::traits::WorkspaceVectorRegistry> =
            Arc::new(MemoryWorkspaceVectorRegistry::new(
                Arc::clone(&vector) as Arc<dyn edgequake_storage::traits::VectorStorage>
            ));
        StorageRuntime::for_memory_tests(
            Arc::clone(&kv) as Arc<dyn edgequake_storage::traits::KVStorage>,
            Arc::clone(&vector) as Arc<dyn edgequake_storage::traits::VectorStorage>,
            registry,
            Arc::clone(&graph) as Arc<dyn edgequake_storage::traits::GraphStorage>,
        )
    }

    #[test]
    fn shell_truncated_when_summary_length_and_chunks_longer() {
        let shell: String = "x".repeat(500);
        assert!(shell_looks_truncated(&shell, 1200));
        assert!(!shell_looks_truncated(&shell, 500));
        assert!(!shell_looks_truncated(&"hello".repeat(200), 50));
        assert!(shell_looks_truncated("", 10));
        assert!(!shell_looks_truncated("", 0));
    }

    #[tokio::test]
    async fn heals_truncated_shell_from_chunks_and_write_back() {
        let kv = Arc::new(MemoryKVStorage::new("test"));
        let doc_id = "019fbd16-936f-7606-9dc6-a2b4468a61fb";
        let full: String = format!("{}{}", "A".repeat(600), "SENTINEL_TAIL_MARKER_XYZ");
        assert!(full.chars().count() > SUMMARY_TRUNCATE_CHARS);

        let truncated: String = full.chars().take(SUMMARY_TRUNCATE_CHARS).collect();
        kv.upsert(&[(
            kv_keys::doc_content(doc_id),
            serde_json::json!({ "content": truncated }),
        )])
        .await
        .unwrap();

        // Two chunks that concatenate (with join "\n") to more than the summary.
        let chunk0: String = full.chars().take(300).collect();
        let chunk1: String = full.chars().skip(300).collect();
        kv.upsert(&[
            (
                kv_keys::doc_chunk(doc_id, 0),
                serde_json::json!({ "content": chunk0 }),
            ),
            (
                kv_keys::doc_chunk(doc_id, 1),
                serde_json::json!({ "content": chunk1 }),
            ),
        ])
        .await
        .unwrap();

        let storage = memory_runtime(Arc::clone(&kv));
        let body = load_document_body(&storage, doc_id, &Value::Null)
            .await
            .expect("body");
        assert_eq!(body.source, DocumentBodySource::Chunks);
        assert!(body.markdown.contains("SENTINEL_TAIL_MARKER_XYZ"));
        assert!(body.markdown.chars().count() > SUMMARY_TRUNCATE_CHARS);

        // Write-back repaired shell for subsequent reads.
        let shell_again = kv.get_by_id(&kv_keys::doc_content(doc_id)).await.unwrap();
        let repaired = shell_again
            .and_then(|v| {
                v.get("content")
                    .and_then(|c| c.as_str())
                    .map(str::to_string)
            })
            .expect("write-back");
        assert!(repaired.contains("SENTINEL_TAIL_MARKER_XYZ"));
    }

    #[tokio::test]
    async fn trusts_full_shell_when_not_truncated() {
        let kv = Arc::new(MemoryKVStorage::new("test"));
        let doc_id = "doc-full-shell";
        let full = "full body that is longer than five hundred characters ".repeat(20);
        kv.upsert(&[(
            kv_keys::doc_content(doc_id),
            serde_json::json!({ "content": full.clone() }),
        )])
        .await
        .unwrap();
        kv.upsert(&[(
            kv_keys::doc_chunk(doc_id, 0),
            serde_json::json!({ "content": "chunk-only-fragment" }),
        )])
        .await
        .unwrap();

        let storage = memory_runtime(kv);
        let body = load_document_body(&storage, doc_id, &Value::Null)
            .await
            .expect("body");
        assert_eq!(body.source, DocumentBodySource::Kv);
        assert_eq!(body.markdown, full);
    }
}
