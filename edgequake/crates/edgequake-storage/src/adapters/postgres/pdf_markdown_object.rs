//! Extracted PDF markdown in the originals bucket.
//!
//! The bucket is the caller's `OriginalsClient`. Database mode never constructs
//! that client (`EDGEQUAKE_ORIGINAL_STORE=database`), so this module is not called.

use chrono::Utc;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::{Result, StorageError};
use crate::originals::{
    delete_loser, insert_inflight, markdown_pointer, publish_bytes, remove_delete_queue,
    remove_inflight, retire_previous_key, scoped_object_key, sha256_hex, upload_key,
    workspace_tenant, OriginalsClient, OwnerKind,
};
use crate::pdf_storage::UpdatePdfProcessingRequest;

use super::pdf_object::is_unique_violation;

fn map_store(err: impl std::fmt::Display) -> StorageError {
    StorageError::Unavailable(err.to_string())
}

/// PUT markdown, then null `markdown_content` and record a verified ref.
pub(super) async fn store_markdown(
    pool: &PgPool,
    client: &OriginalsClient,
    request: &UpdatePdfProcessingRequest,
    markdown: &str,
) -> Result<()> {
    let bytes = markdown.as_bytes();
    let located =
        sqlx::query("SELECT workspace_id, markdown_object_id FROM pdf_documents WHERE pdf_id = $1")
            .bind(request.pdf_id)
            .fetch_optional(pool)
            .await
            .map_err(|err| {
                StorageError::Database(format!("Failed to read markdown owner: {err}"))
            })?;
    let Some(located) = located else {
        return Err(StorageError::NotFound(format!(
            "PDF {} not found",
            request.pdf_id
        )));
    };
    let workspace_id: Uuid = located.get("workspace_id");
    let owner: Uuid = located.get("markdown_object_id");
    let tenant_id = workspace_tenant(pool, workspace_id)
        .await
        .map_err(map_store)?;
    let previous = markdown_pointer(pool, owner).await.map_err(map_store)?;
    let had_ref = previous.is_some();
    let key = upload_key(
        &client.prefix,
        OwnerKind::Markdown,
        tenant_id,
        workspace_id,
        if had_ref { Uuid::new_v4() } else { owner },
    );
    insert_inflight(pool, &key).await.map_err(map_store)?;
    let meta = match publish_bytes(
        client.blobs.as_ref(),
        &key,
        bytes,
        client.sse,
        client.store_checks_body,
    )
    .await
    {
        Ok(meta) => meta,
        Err(err) => {
            let _ = remove_inflight(pool, &key).await;
            return Err(map_store(err));
        }
    };
    let sha = sha256_hex(bytes);
    let status = request.processing_status.as_str();
    let method = request.extraction_method.map(|m| m.as_str().to_string());
    let processed_at = request.processing_status.is_terminal().then(Utc::now);
    let written = write_markdown_ref(
        pool,
        request,
        owner,
        tenant_id,
        workspace_id,
        &key,
        &sha,
        bytes.len() as i64,
        meta.etag.as_deref(),
        status,
        method.as_deref(),
        processed_at,
    )
    .await;
    if let Err(err) = written {
        let _ = remove_inflight(pool, &key).await;
        let _ = delete_loser(client.blobs.as_ref(), &key).await;
        return Err(err);
    }
    if let Some(old) = previous.as_ref().map(|pointer| pointer.key.clone()) {
        if old != key {
            if let Err(err) = retire_previous_key(pool, client.blobs.as_ref(), &old).await {
                tracing::warn!(error = %err, "previous markdown key stayed in the bucket");
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn write_markdown_ref(
    pool: &PgPool,
    request: &UpdatePdfProcessingRequest,
    owner: Uuid,
    tenant_id: Uuid,
    workspace_id: Uuid,
    key: &str,
    sha: &str,
    byte_length: i64,
    etag: Option<&str>,
    status: &str,
    method: Option<&str>,
    processed_at: Option<chrono::DateTime<Utc>>,
) -> Result<()> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|err| StorageError::Database(format!("Failed to begin markdown store: {err}")))?;
    if let Err(err) = sqlx::query(
        r#"
        UPDATE pdf_documents
        SET processing_status = $1,
            extraction_method = COALESCE($2, extraction_method),
            markdown_content = NULL,
            markdown_location = 'object',
            extraction_errors = COALESCE($3, extraction_errors),
            document_id = COALESCE($4, document_id),
            processed_at = COALESCE($5, processed_at),
            vision_model = COALESCE($6, vision_model)
        WHERE pdf_id = $7
        "#,
    )
    .bind(status)
    .bind(method)
    .bind(&request.extraction_errors)
    .bind(request.document_id)
    .bind(processed_at)
    .bind(&request.vision_model)
    .bind(request.pdf_id)
    .execute(&mut *tx)
    .await
    {
        drop(tx);
        return Err(StorageError::Database(format!(
            "Failed to update PDF markdown location: {err}"
        )));
    }
    if let Err(err) = sqlx::query(
        r#"
        INSERT INTO original_object_refs (
            markdown_object_id, tenant_id, workspace_id, object_key, sha256,
            byte_length, content_type, etag, verified_at
        ) VALUES ($1, $2, $3, $4, $5, $6, 'text/markdown', $7, now())
        ON CONFLICT (markdown_object_id) WHERE markdown_object_id IS NOT NULL
        DO UPDATE SET
            tenant_id = EXCLUDED.tenant_id,
            workspace_id = EXCLUDED.workspace_id,
            object_key = EXCLUDED.object_key,
            sha256 = EXCLUDED.sha256,
            byte_length = EXCLUDED.byte_length,
            content_type = EXCLUDED.content_type,
            etag = EXCLUDED.etag,
            verified_at = now()
        "#,
    )
    .bind(owner)
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(key)
    .bind(sha)
    .bind(byte_length)
    .bind(etag)
    .execute(&mut *tx)
    .await
    {
        drop(tx);
        if is_unique_violation(&err) {
            return Err(StorageError::Conflict(format!(
                "markdown object for {owner} lost a concurrent insert"
            )));
        }
        return Err(StorageError::Database(format!(
            "Failed to write markdown object ref: {err}"
        )));
    }
    if let Err(err) = sqlx::query("DELETE FROM original_object_inflight WHERE object_key = $1")
        .bind(key)
        .execute(&mut *tx)
        .await
    {
        drop(tx);
        return Err(StorageError::Database(format!(
            "Failed to clear markdown inflight: {err}"
        )));
    }
    tx.commit()
        .await
        .map_err(|err| StorageError::Database(format!("Failed to commit markdown store: {err}")))?;
    Ok(())
}

/// Drop the markdown ref and return the key when it belongs to this workspace.
/// A mismatched key is not returned, so the caller does not touch the bucket.
pub(super) async fn take_markdown_key(
    pool: &PgPool,
    client: Option<&OriginalsClient>,
    pdf_id: Uuid,
) -> Result<Option<String>> {
    let located =
        sqlx::query("SELECT workspace_id, markdown_object_id FROM pdf_documents WHERE pdf_id = $1")
            .bind(pdf_id)
            .fetch_optional(pool)
            .await
            .map_err(|err| {
                StorageError::Database(format!("Failed to read markdown owner: {err}"))
            })?;
    let Some(located) = located else {
        return Ok(None);
    };
    let workspace_id: Uuid = located.get("workspace_id");
    let owner: Uuid = located.get("markdown_object_id");
    let pointer = markdown_pointer(pool, owner).await.map_err(map_store)?;
    let scoped = match (client, pointer.as_ref()) {
        (Some(client), Some(pointer)) => {
            let tenant = workspace_tenant(pool, workspace_id)
                .await
                .map_err(map_store)?;
            scoped_object_key(
                &client.prefix,
                OwnerKind::Markdown,
                tenant,
                workspace_id,
                pointer.tenant_id,
                pointer.workspace_id,
                &pointer.key,
            )
        }
        _ => None,
    };
    sqlx::query("DELETE FROM original_object_refs WHERE markdown_object_id = $1")
        .bind(owner)
        .execute(pool)
        .await
        .map_err(|err| StorageError::Database(format!("Failed to delete markdown ref: {err}")))?;
    Ok(scoped)
}

pub(super) async fn delete_markdown_key(pool: &PgPool, client: &OriginalsClient, key: &str) {
    match client.blobs.delete_key(key).await {
        Ok(()) => {
            let _ = remove_delete_queue(pool, key).await;
        }
        Err(err) => {
            tracing::warn!(error = %err, "markdown object delete queued for the reaper");
        }
    }
}
