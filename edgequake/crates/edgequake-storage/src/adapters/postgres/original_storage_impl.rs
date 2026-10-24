//! PostgreSQL implementation of document original storage.

use async_trait::async_trait;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{Result, StorageError};
use crate::original_storage::*;
use crate::originals::{
    column_matches_ref, delete_loser, get_inclusive, insert_inflight, max_upload_bytes,
    original_pointer, prefer_object, publish_bytes, read_verified, remove_delete_queue,
    remove_inflight, retire_previous_key, scoped_object_key, sha256_hex, upload_key,
    workspace_tenant, OriginalsClient, OriginalsError, OwnerKind,
};

pub struct PostgresOriginalStorage {
    pool: PgPool,
    objects: Option<Arc<OriginalsClient>>,
}

impl PostgresOriginalStorage {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            objects: None,
        }
    }

    pub fn with_objects(mut self, objects: Option<Arc<OriginalsClient>>) -> Self {
        self.objects = objects;
        self
    }

    async fn original_column(&self, workspace_id: &Uuid, document_id: &Uuid) -> Result<Vec<u8>> {
        sqlx::query_scalar(
            "SELECT original_data FROM document_originals WHERE document_id = $1 AND workspace_id = $2",
        )
        .bind(document_id)
        .bind(workspace_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to get original bytes: {e}")))
    }
}

#[async_trait]
impl DocumentOriginalStorage for PostgresOriginalStorage {
    async fn store_original(&self, request: StoreOriginalRequest) -> Result<()> {
        validate_original_data(&request.original_data, max_upload_bytes())?;
        if let Some(client) = &self.objects {
            return store_object(&self.pool, client, request).await;
        }
        sqlx::query(
            r#"
            WITH written AS (
                INSERT INTO document_originals (
                    document_id, workspace_id, filename, content_type,
                    file_size_bytes, original_data
                ) VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT (document_id) DO UPDATE SET
                    filename = EXCLUDED.filename,
                    content_type = EXCLUDED.content_type,
                    file_size_bytes = EXCLUDED.file_size_bytes,
                    original_data = EXCLUDED.original_data,
                    byte_location = 'database'
                RETURNING document_id
            ),
            dropped AS (
                DELETE FROM original_object_refs r
                USING written w
                WHERE r.document_id = w.document_id
                RETURNING r.document_id
            )
            SELECT document_id FROM written
            "#,
        )
        .bind(request.document_id)
        .bind(request.workspace_id)
        .bind(&request.filename)
        .bind(&request.content_type)
        .bind(request.original_data.len() as i64)
        .bind(&request.original_data)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to store original: {e}")))?;
        Ok(())
    }

    async fn get_original(
        &self,
        workspace_id: &Uuid,
        document_id: &Uuid,
    ) -> Result<Option<DocumentOriginal>> {
        let row = sqlx::query(
            r#"
            SELECT document_id, workspace_id, filename, content_type,
                   file_size_bytes, byte_location, created_at
            FROM document_originals
            WHERE document_id = $1 AND workspace_id = $2
            "#,
        )
        .bind(document_id)
        .bind(workspace_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to get original: {e}")))?;
        let Some(row) = row else {
            return Ok(None);
        };
        let location: String = row
            .try_get("byte_location")
            .unwrap_or_else(|_| "database".into());
        let pointer = if self.objects.is_some() {
            original_pointer(&self.pool, *document_id)
                .await
                .map_err(|err| StorageError::Database(format!("original ref: {err}")))?
        } else {
            None
        };
        let tenant = workspace_tenant(&self.pool, *workspace_id)
            .await
            .map_err(|err| StorageError::Database(format!("original tenant: {err}")))?;
        let scoped = pointer.as_ref().and_then(|pointer| {
            self.objects.as_ref().and_then(|client| {
                scoped_object_key(
                    &client.prefix,
                    OwnerKind::Original,
                    tenant,
                    *workspace_id,
                    pointer.tenant_id,
                    pointer.workspace_id,
                    &pointer.key,
                )
            })
        });
        let verified = scoped.is_some() && pointer.as_ref().is_some_and(|p| p.verified);
        if location == "object" && scoped.is_none() {
            return Ok(None);
        }
        let original_data = if prefer_object(self.objects.is_some(), &location, verified) {
            let Some(key) = scoped else {
                return Ok(None);
            };
            let Some(pointer) = pointer else {
                return Ok(None);
            };
            let client = self.objects.as_ref().expect("object client");
            if location != "object" {
                let column = self.original_column(workspace_id, document_id).await?;
                if !column_matches_ref(&column, &pointer.sha256, pointer.byte_length) {
                    tracing::warn!(
                        "original column differs from the verified ref; serving database bytes"
                    );
                    column
                } else {
                    match read_verified(
                        client.blobs.as_ref(),
                        &key,
                        &pointer.sha256,
                        pointer.byte_length,
                    )
                    .await
                    {
                        Ok(bytes) => bytes,
                        Err(err) => {
                            tracing::warn!(error = %err, "original object read failed; serving database bytes");
                            column
                        }
                    }
                }
            } else {
                match read_verified(
                    client.blobs.as_ref(),
                    &key,
                    &pointer.sha256,
                    pointer.byte_length,
                )
                .await
                {
                    Ok(bytes) => bytes,
                    Err(err) => {
                        let message = if matches!(err, OriginalsError::Checksum(_)) {
                            "original object bytes do not match the verified ref".to_string()
                        } else {
                            format!("original object read failed: {err}")
                        };
                        return Err(StorageError::Unavailable(message));
                    }
                }
            }
        } else {
            self.original_column(workspace_id, document_id).await?
        };
        Ok(Some(DocumentOriginal {
            document_id: row.get("document_id"),
            workspace_id: row.get("workspace_id"),
            filename: row.get("filename"),
            content_type: row.get("content_type"),
            file_size_bytes: row.get("file_size_bytes"),
            original_data,
            created_at: row.get("created_at"),
        }))
    }

    async fn get_original_info(
        &self,
        workspace_id: &Uuid,
        document_id: &Uuid,
    ) -> Result<Option<OriginalBlobInfo>> {
        let row = sqlx::query(
            r#"
            SELECT filename, content_type, byte_location, file_size_bytes,
                   CASE WHEN byte_location = 'database'
                        THEN octet_length(original_data)::bigint
                        ELSE file_size_bytes
                   END AS bytea_len
            FROM document_originals
            WHERE document_id = $1 AND workspace_id = $2
            "#,
        )
        .bind(document_id)
        .bind(workspace_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to read original info: {e}")))?;
        let Some(row) = row else {
            return Ok(None);
        };
        let location: String = row.get("byte_location");
        let total = if location == "object" {
            if let Some(pointer) = original_pointer(&self.pool, *document_id)
                .await
                .map_err(|err| StorageError::Database(format!("original ref: {err}")))?
            {
                pointer.byte_length.max(0) as u64
            } else {
                row.get::<i64, _>("file_size_bytes").max(0) as u64
            }
        } else {
            row.get::<i64, _>("bytea_len").max(0) as u64
        };
        Ok(Some(OriginalBlobInfo {
            filename: row.get("filename"),
            content_type: row.get("content_type"),
            total_bytes: total,
        }))
    }

    async fn get_original_bytes_range(
        &self,
        workspace_id: &Uuid,
        document_id: &Uuid,
        start: u64,
        end: u64,
    ) -> Result<Option<Vec<u8>>> {
        let row = sqlx::query(
            "SELECT byte_location FROM document_originals WHERE document_id = $1 AND workspace_id = $2",
        )
        .bind(document_id)
        .bind(workspace_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to read original location: {e}")))?;
        let Some(row) = row else {
            return Ok(None);
        };
        let location: String = row.get("byte_location");
        if location == "object" {
            if let Some(client) = &self.objects {
                let pointer = original_pointer(&self.pool, *document_id)
                    .await
                    .map_err(|err| StorageError::Database(format!("original ref: {err}")))?;
                let tenant = workspace_tenant(&self.pool, *workspace_id)
                    .await
                    .map_err(|err| StorageError::Database(format!("original tenant: {err}")))?;
                let scoped = pointer.as_ref().and_then(|pointer| {
                    scoped_object_key(
                        &client.prefix,
                        OwnerKind::Original,
                        tenant,
                        *workspace_id,
                        pointer.tenant_id,
                        pointer.workspace_id,
                        &pointer.key,
                    )
                });
                let Some(key) = scoped else {
                    return Ok(None);
                };
                if !pointer.as_ref().is_some_and(|p| p.verified) {
                    return Ok(None);
                }
                let bytes = get_inclusive(client.blobs.as_ref(), &key, start, end)
                    .await
                    .map_err(|err| {
                        StorageError::Unavailable(format!("original range read failed: {err}"))
                    })?;
                return Ok(Some(bytes));
            }
        }
        let from = i32::try_from(start + 1)
            .map_err(|_| StorageError::Database("original range start out of bounds".into()))?;
        let len = i32::try_from(end - start + 1)
            .map_err(|_| StorageError::Database("original range length out of bounds".into()))?;
        let chunk: Option<Vec<u8>> = sqlx::query_scalar(
            "SELECT substring(original_data FROM $3 FOR $4) FROM document_originals WHERE document_id = $1 AND workspace_id = $2",
        )
        .bind(document_id)
        .bind(workspace_id)
        .bind(from)
        .bind(len)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to fetch original range: {e}")))?;
        Ok(chunk)
    }

    async fn delete_original(&self, workspace_id: &Uuid, document_id: &Uuid) -> Result<bool> {
        let pointer = if self.objects.is_some() {
            original_pointer(&self.pool, *document_id)
                .await
                .map_err(|err| StorageError::Database(format!("original ref: {err}")))?
        } else {
            None
        };
        let scoped = if let Some(pointer) = &pointer {
            let tenant = workspace_tenant(&self.pool, *workspace_id)
                .await
                .map_err(|err| StorageError::Database(format!("original tenant: {err}")))?;
            let key = self.objects.as_ref().and_then(|client| {
                scoped_object_key(
                    &client.prefix,
                    OwnerKind::Original,
                    tenant,
                    *workspace_id,
                    pointer.tenant_id,
                    pointer.workspace_id,
                    &pointer.key,
                )
            });
            if key.is_some() {
                sqlx::query(
                    "DELETE FROM original_object_refs \
                     WHERE document_id = $1 AND workspace_id = $2 AND tenant_id = $3",
                )
                .bind(document_id)
                .bind(workspace_id)
                .bind(tenant)
                .execute(&self.pool)
                .await
                .map_err(|e| {
                    StorageError::Database(format!("Failed to delete original ref: {e}"))
                })?;
            }
            key
        } else {
            None
        };
        let result = sqlx::query(
            "DELETE FROM document_originals WHERE document_id = $1 AND workspace_id = $2",
        )
        .bind(document_id)
        .bind(workspace_id)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to delete original: {e}")))?;
        if let (Some(client), Some(key)) = (&self.objects, scoped) {
            match client.blobs.delete_key(&key).await {
                Ok(()) => {
                    let _ = remove_delete_queue(&self.pool, &key).await;
                }
                Err(err) => {
                    tracing::warn!(error = %err, "original object delete queued for the reaper");
                }
            }
        }
        Ok(result.rows_affected() > 0)
    }
}

async fn store_object(
    pool: &PgPool,
    client: &OriginalsClient,
    request: StoreOriginalRequest,
) -> Result<()> {
    let sha = sha256_hex(&request.original_data);
    let tenant_id = workspace_tenant(pool, request.workspace_id)
        .await
        .map_err(|err| StorageError::Database(err.to_string()))?;
    let previous = original_pointer(pool, request.document_id)
        .await
        .map_err(|err| StorageError::Database(err.to_string()))?;
    let key = upload_key(
        &client.prefix,
        OwnerKind::Original,
        tenant_id,
        request.workspace_id,
        if previous.is_some() {
            Uuid::new_v4()
        } else {
            request.document_id
        },
    );
    insert_inflight(pool, &key)
        .await
        .map_err(|err| StorageError::Unavailable(err.to_string()))?;
    let meta = match publish_bytes(
        client.blobs.as_ref(),
        &key,
        &request.original_data,
        client.sse,
        client.store_checks_body,
    )
    .await
    {
        Ok(meta) => meta,
        Err(err) => {
            let _ = remove_inflight(pool, &key).await;
            return Err(StorageError::Unavailable(err.to_string()));
        }
    };
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| StorageError::Database(format!("Failed to begin original store: {e}")))?;
    if let Err(err) = sqlx::query(
        r#"
        INSERT INTO document_originals (
            document_id, workspace_id, filename, content_type,
            file_size_bytes, original_data, byte_location
        ) VALUES ($1, $2, $3, $4, $5, NULL, 'object')
        ON CONFLICT (document_id) DO UPDATE SET
            filename = EXCLUDED.filename,
                content_type = EXCLUDED.content_type,
                file_size_bytes = EXCLUDED.file_size_bytes,
                original_data = NULL,
                byte_location = 'object',
                workspace_id = EXCLUDED.workspace_id
        "#,
    )
    .bind(request.document_id)
    .bind(request.workspace_id)
    .bind(&request.filename)
    .bind(&request.content_type)
    .bind(request.original_data.len() as i64)
    .execute(&mut *tx)
    .await
    {
        drop(tx);
        let _ = delete_loser(client.blobs.as_ref(), &key).await;
        let _ = remove_inflight(pool, &key).await;
        return Err(StorageError::Database(format!(
            "Failed to store original: {err}"
        )));
    }
    if let Err(err) = sqlx::query(
        r#"
        INSERT INTO original_object_refs (
            document_id, tenant_id, workspace_id, object_key, sha256, byte_length,
            content_type, etag, verified_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, now())
        ON CONFLICT (document_id) WHERE document_id IS NOT NULL DO UPDATE SET
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
    .bind(request.document_id)
    .bind(tenant_id)
    .bind(request.workspace_id)
    .bind(&key)
    .bind(&sha)
    .bind(request.original_data.len() as i64)
    .bind(&request.content_type)
    .bind(&meta.etag)
    .execute(&mut *tx)
    .await
    {
        drop(tx);
        let _ = delete_loser(client.blobs.as_ref(), &key).await;
        let _ = remove_inflight(pool, &key).await;
        return Err(StorageError::Database(format!(
            "Failed to store original ref: {err}"
        )));
    }
    sqlx::query("DELETE FROM original_object_inflight WHERE object_key = $1")
        .bind(&key)
        .execute(&mut *tx)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to clear original inflight: {e}")))?;
    if let Err(err) = tx.commit().await {
        let _ = remove_inflight(pool, &key).await;
        let _ = delete_loser(client.blobs.as_ref(), &key).await;
        return Err(StorageError::Database(format!(
            "Failed to commit original: {err}"
        )));
    }
    if let Some(old) = previous.as_ref().map(|pointer| pointer.key.clone()) {
        if old != key {
            if let Err(err) = retire_previous_key(pool, client.blobs.as_ref(), &old).await {
                tracing::warn!(error = %err, "previous original key stayed in the bucket");
            }
        }
    }
    Ok(())
}
