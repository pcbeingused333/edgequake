//! PostgreSQL implementation of document multimodal asset storage.
//!
//! Object mode is `with_objects(Some(client))`, which the API passes only when
//! `EDGEQUAKE_ORIGINAL_STORE=object`. Database mode leaves the client unset
//! and writes `asset_data` without opening the bucket.

use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{Result, StorageError};
use crate::mm_asset_storage::*;
use crate::originals::{
    after_object_checksum, asset_pointer, column_matches_ref, delete_loser, insert_inflight,
    prefer_object, publish_bytes, read_verified, remove_inflight, retire_previous_key,
    scoped_object_key, sha256_hex, upload_key, workspace_tenant, ObjectServe, OriginalsClient,
    OriginalsError, OwnerKind,
};

use super::pdf_object::is_unique_violation;

pub struct PostgresMmAssetStorage {
    pool: PgPool,
    objects: Option<Arc<OriginalsClient>>,
}

impl PostgresMmAssetStorage {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            objects: None,
        }
    }

    /// `None` keeps BYTEA. The API passes the originals client only in object mode.
    pub fn with_objects(mut self, objects: Option<Arc<OriginalsClient>>) -> Self {
        self.objects = objects;
        self
    }
}

#[async_trait]
impl DocumentMmAssetStorage for PostgresMmAssetStorage {
    async fn store_asset(&self, request: StoreMmAssetRequest) -> Result<()> {
        let path = normalize_mm_asset_path(&request.asset_path)?;
        let asset_id = normalize_mm_asset_id(&request.asset_id)?;
        validate_mm_asset_data(&request.asset_data)?;
        document_in_workspace(&self.pool, request.document_id, request.workspace_id).await?;
        if let Some(client) = &self.objects {
            return store_object(&self.pool, client, request, &path, &asset_id).await;
        }
        let stored: Option<Uuid> = sqlx::query_scalar(
            r#"
            WITH written AS (
                INSERT INTO document_mm_assets (
                    document_id, workspace_id, asset_id, asset_path, content_type,
                    file_size_bytes, asset_data, asset_kind, page_num, byte_location
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'database')
                ON CONFLICT (document_id, asset_path) DO UPDATE SET
                    asset_id = EXCLUDED.asset_id,
                    content_type = EXCLUDED.content_type,
                    file_size_bytes = EXCLUDED.file_size_bytes,
                    asset_data = EXCLUDED.asset_data,
                    asset_kind = EXCLUDED.asset_kind,
                    page_num = EXCLUDED.page_num,
                    byte_location = 'database'
                WHERE document_mm_assets.workspace_id = EXCLUDED.workspace_id
                RETURNING object_id
            ),
            dropped AS (
                DELETE FROM original_object_refs r
                USING written w
                WHERE r.asset_object_id = w.object_id
                RETURNING r.asset_object_id
            )
            SELECT object_id FROM written
            "#,
        )
        .bind(request.document_id)
        .bind(request.workspace_id)
        .bind(&asset_id)
        .bind(&path)
        .bind(&request.content_type)
        .bind(request.asset_data.len() as i64)
        .bind(&request.asset_data)
        .bind(&request.asset_kind)
        .bind(request.page_num)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to store mm-asset: {e}")))?;
        if stored.is_none() {
            return Err(StorageError::NotFound(format!(
                "document {} is not in this workspace",
                request.document_id
            )));
        }
        Ok(())
    }

    async fn get_asset(
        &self,
        workspace_id: &Uuid,
        document_id: &Uuid,
        asset_path: &str,
    ) -> Result<Option<DocumentMmAsset>> {
        let path = normalize_mm_asset_path(asset_path)?;
        let row = sqlx::query_as::<_, MmAssetRow>(
            r#"
            SELECT document_id, workspace_id, asset_id, asset_path, content_type,
                   file_size_bytes, asset_data, asset_kind, page_num, created_at,
                   object_id, byte_location
            FROM document_mm_assets
            WHERE document_id = $1 AND workspace_id = $2 AND asset_path = $3
            "#,
        )
        .bind(document_id)
        .bind(workspace_id)
        .bind(&path)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to get mm-asset: {e}")))?;

        match row {
            Some(row) => self.hydrate(row).await,
            None => Ok(None),
        }
    }

    async fn get_asset_by_id(
        &self,
        workspace_id: &Uuid,
        document_id: &Uuid,
        asset_id: &str,
    ) -> Result<Option<DocumentMmAsset>> {
        let id = normalize_mm_asset_id(asset_id)?;
        let row = sqlx::query_as::<_, MmAssetRow>(
            r#"
            SELECT document_id, workspace_id, asset_id, asset_path, content_type,
                   file_size_bytes, asset_data, asset_kind, page_num, created_at,
                   object_id, byte_location
            FROM document_mm_assets
            WHERE document_id = $1 AND workspace_id = $2 AND asset_id = $3
            "#,
        )
        .bind(document_id)
        .bind(workspace_id)
        .bind(&id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to get mm-asset by id: {e}")))?;

        match row {
            Some(row) => self.hydrate(row).await,
            None => Ok(None),
        }
    }

    async fn list_asset_paths(
        &self,
        workspace_id: &Uuid,
        document_id: &Uuid,
    ) -> Result<Vec<String>> {
        let rows = sqlx::query_scalar::<_, String>(
            r#"
            SELECT asset_path FROM document_mm_assets
            WHERE document_id = $1 AND workspace_id = $2
            ORDER BY asset_path
            "#,
        )
        .bind(document_id)
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to list mm-assets: {e}")))?;
        Ok(rows)
    }

    async fn list_asset_summaries(
        &self,
        workspace_id: &Uuid,
        document_id: &Uuid,
    ) -> Result<Vec<DocumentMmAssetSummary>> {
        let rows = sqlx::query_as::<_, MmAssetSummaryRow>(
            r#"
            SELECT asset_id, asset_path, content_type, file_size_bytes, asset_kind, page_num
            FROM document_mm_assets
            WHERE document_id = $1 AND workspace_id = $2
            ORDER BY page_num NULLS LAST, asset_path
            "#,
        )
        .bind(document_id)
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to list mm-asset summaries: {e}")))?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn delete_assets_for_document(
        &self,
        workspace_id: &Uuid,
        document_id: &Uuid,
    ) -> Result<u64> {
        let result = sqlx::query(
            "DELETE FROM document_mm_assets WHERE document_id = $1 AND workspace_id = $2",
        )
        .bind(document_id)
        .bind(workspace_id)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to delete mm-assets: {e}")))?;
        Ok(result.rows_affected())
    }
}

impl PostgresMmAssetStorage {
    async fn hydrate(&self, row: MmAssetRow) -> Result<Option<DocumentMmAsset>> {
        let Some(asset_data) = self.read_bytes(&row).await? else {
            return Ok(None);
        };
        Ok(Some(DocumentMmAsset {
            document_id: row.document_id,
            workspace_id: row.workspace_id,
            asset_id: row.asset_id,
            asset_path: row.asset_path,
            content_type: row.content_type,
            file_size_bytes: row.file_size_bytes,
            asset_data,
            asset_kind: row.asset_kind,
            page_num: row.page_num,
            created_at: row.created_at,
        }))
    }

    async fn read_bytes(&self, row: &MmAssetRow) -> Result<Option<Vec<u8>>> {
        let Some(client) = &self.objects else {
            return Ok(row.asset_data.clone());
        };
        let pointer = asset_pointer(&self.pool, row.object_id)
            .await
            .map_err(|err| StorageError::Database(format!("mm-asset ref: {err}")))?;
        let tenant = workspace_tenant(&self.pool, row.workspace_id)
            .await
            .map_err(|err| StorageError::Database(format!("mm-asset tenant: {err}")))?;
        let scoped = pointer.as_ref().and_then(|pointer| {
            scoped_object_key(
                &client.prefix,
                OwnerKind::Asset,
                tenant,
                row.workspace_id,
                pointer.tenant_id,
                pointer.workspace_id,
                &pointer.key,
            )
        });
        let verified = scoped.is_some() && pointer.as_ref().is_some_and(|p| p.verified);
        if row.byte_location == "object" && scoped.is_none() {
            return Ok(None);
        }
        if !prefer_object(true, &row.byte_location, verified) {
            return Ok(row.asset_data.clone());
        }
        let Some(key) = scoped else {
            return Ok(None);
        };
        let Some(pointer) = pointer else {
            return Ok(None);
        };
        serve_checked(
            client.blobs.as_ref(),
            &key,
            &pointer.sha256,
            pointer.byte_length,
            &row.byte_location,
            row.asset_data.clone(),
            "mm-asset",
        )
        .await
    }
}

async fn serve_checked(
    blob: &dyn crate::originals::BlobStore,
    key: &str,
    sha: &str,
    byte_length: i64,
    location: &str,
    column: Option<Vec<u8>>,
    label: &str,
) -> Result<Option<Vec<u8>>> {
    if location != "object" {
        if let Some(local) = &column {
            if !column_matches_ref(local, sha, byte_length) {
                tracing::warn!(%label, "database bytes differ from the verified ref; serving the column");
                return Ok(column);
            }
        }
    }
    match read_verified(blob, key, sha, byte_length).await {
        Ok(bytes) => Ok(Some(bytes)),
        Err(err) => match after_object_checksum(location, column.is_some(), false) {
            ObjectServe::Column => {
                tracing::warn!(%label, error = %err, "object read failed; serving database bytes");
                Ok(column)
            }
            ObjectServe::Refuse | ObjectServe::Object => {
                let message = if matches!(err, OriginalsError::Checksum(_)) {
                    format!("{label} object bytes do not match the verified ref")
                } else {
                    format!("{label} object read failed: {err}")
                };
                Err(StorageError::Unavailable(message))
            }
        },
    }
}

async fn document_in_workspace(pool: &PgPool, document_id: Uuid, workspace_id: Uuid) -> Result<()> {
    let found: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM documents WHERE id = $1 AND workspace_id = $2)",
    )
    .bind(document_id)
    .bind(workspace_id)
    .fetch_one(pool)
    .await
    .map_err(|err| StorageError::Database(format!("Failed to check document workspace: {err}")))?;
    if found {
        Ok(())
    } else {
        Err(StorageError::NotFound(format!(
            "document {document_id} is not in this workspace"
        )))
    }
}

async fn store_object(
    pool: &PgPool,
    client: &OriginalsClient,
    request: StoreMmAssetRequest,
    path: &str,
    asset_id: &str,
) -> Result<()> {
    let object_id = Uuid::new_v4();
    let tenant_id = workspace_tenant(pool, request.workspace_id)
        .await
        .map_err(|err| StorageError::Unavailable(err.to_string()))?;
    let key = upload_key(
        &client.prefix,
        OwnerKind::Asset,
        tenant_id,
        request.workspace_id,
        object_id,
    );
    insert_inflight(pool, &key)
        .await
        .map_err(|err| StorageError::Unavailable(err.to_string()))?;
    let meta = match publish_bytes(
        client.blobs.as_ref(),
        &key,
        &request.asset_data,
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
    let sha = sha256_hex(&request.asset_data);
    match insert_asset_object(
        pool,
        &request,
        path,
        asset_id,
        object_id,
        tenant_id,
        &key,
        &sha,
        meta.etag.as_deref(),
    )
    .await
    {
        Ok(()) => Ok(()),
        Err(err) => {
            let _ = remove_inflight(pool, &key).await;
            let _ = delete_loser(client.blobs.as_ref(), &key).await;
            if matches!(&err, StorageError::Conflict(_)) {
                return replace_asset_object(pool, client, request, path, asset_id).await;
            }
            Err(err)
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn insert_asset_object(
    pool: &PgPool,
    request: &StoreMmAssetRequest,
    path: &str,
    asset_id: &str,
    object_id: Uuid,
    tenant_id: Uuid,
    key: &str,
    sha: &str,
    etag: Option<&str>,
) -> Result<()> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|err| StorageError::Database(format!("Failed to begin mm-asset store: {err}")))?;
    if let Err(err) = sqlx::query(
        r#"
        INSERT INTO document_mm_assets (
            document_id, workspace_id, asset_id, asset_path, content_type,
            file_size_bytes, asset_data, asset_kind, page_num, object_id, byte_location
        ) VALUES ($1, $2, $3, $4, $5, $6, NULL, $7, $8, $9, 'object')
        "#,
    )
    .bind(request.document_id)
    .bind(request.workspace_id)
    .bind(asset_id)
    .bind(path)
    .bind(&request.content_type)
    .bind(request.asset_data.len() as i64)
    .bind(&request.asset_kind)
    .bind(request.page_num)
    .bind(object_id)
    .execute(&mut *tx)
    .await
    {
        drop(tx);
        if is_unique_violation(&err) {
            return Err(StorageError::Conflict(
                "mm-asset path already stored".into(),
            ));
        }
        return Err(StorageError::Database(format!(
            "Failed to store mm-asset: {err}"
        )));
    }
    if let Err(err) = sqlx::query(
        r#"
        INSERT INTO original_object_refs (
            asset_object_id, tenant_id, workspace_id, object_key, sha256,
            byte_length, content_type, etag, verified_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, now())
        "#,
    )
    .bind(object_id)
    .bind(tenant_id)
    .bind(request.workspace_id)
    .bind(key)
    .bind(sha)
    .bind(request.asset_data.len() as i64)
    .bind(&request.content_type)
    .bind(etag)
    .execute(&mut *tx)
    .await
    {
        drop(tx);
        if is_unique_violation(&err) {
            return Err(StorageError::Conflict(
                "mm-asset object ref already stored".into(),
            ));
        }
        return Err(StorageError::Database(format!(
            "Failed to write mm-asset object ref: {err}"
        )));
    }
    if let Err(err) = sqlx::query("DELETE FROM original_object_inflight WHERE object_key = $1")
        .bind(key)
        .execute(&mut *tx)
        .await
    {
        drop(tx);
        return Err(StorageError::Database(format!(
            "Failed to clear mm-asset inflight: {err}"
        )));
    }
    tx.commit()
        .await
        .map_err(|err| StorageError::Database(format!("Failed to commit mm-asset store: {err}")))?;
    Ok(())
}

async fn replace_asset_object(
    pool: &PgPool,
    client: &OriginalsClient,
    request: StoreMmAssetRequest,
    path: &str,
    asset_id: &str,
) -> Result<()> {
    let existing: Option<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT object_id, workspace_id FROM document_mm_assets WHERE document_id = $1 AND asset_path = $2",
    )
    .bind(request.document_id)
    .bind(path)
    .fetch_optional(pool)
    .await
    .map_err(|err| StorageError::Database(format!("Failed to read mm-asset owner: {err}")))?;
    let Some((object_id, row_workspace)) = existing else {
        return Err(StorageError::Conflict(
            "mm-asset path disappeared during replace".into(),
        ));
    };
    if row_workspace != request.workspace_id {
        return Err(StorageError::NotFound(format!(
            "document {} is not in this workspace",
            request.document_id
        )));
    }
    let tenant_id = workspace_tenant(pool, request.workspace_id)
        .await
        .map_err(|err| StorageError::Unavailable(err.to_string()))?;
    let previous = asset_pointer(pool, object_id)
        .await
        .map_err(|err| StorageError::Database(err.to_string()))?;
    let key = upload_key(
        &client.prefix,
        OwnerKind::Asset,
        tenant_id,
        request.workspace_id,
        if previous.is_some() {
            Uuid::new_v4()
        } else {
            object_id
        },
    );
    insert_inflight(pool, &key)
        .await
        .map_err(|err| StorageError::Unavailable(err.to_string()))?;
    let meta = match publish_bytes(
        client.blobs.as_ref(),
        &key,
        &request.asset_data,
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
    let sha = sha256_hex(&request.asset_data);
    let mut tx = pool.begin().await.map_err(|err| {
        StorageError::Database(format!("Failed to begin mm-asset replace: {err}"))
    })?;
    let updated = sqlx::query(
        r#"
        UPDATE document_mm_assets
        SET asset_id = $3,
            content_type = $4,
            file_size_bytes = $5,
            asset_data = NULL,
            asset_kind = $6,
            page_num = $7,
            byte_location = 'object'
        WHERE document_id = $1 AND asset_path = $2 AND workspace_id = $8
        "#,
    )
    .bind(request.document_id)
    .bind(path)
    .bind(asset_id)
    .bind(&request.content_type)
    .bind(request.asset_data.len() as i64)
    .bind(&request.asset_kind)
    .bind(request.page_num)
    .bind(request.workspace_id)
    .execute(&mut *tx)
    .await;
    let updated = match updated {
        Ok(done) => done,
        Err(err) => {
            drop(tx);
            let _ = remove_inflight(pool, &key).await;
            let _ = delete_loser(client.blobs.as_ref(), &key).await;
            return Err(StorageError::Database(format!(
                "Failed to replace mm-asset: {err}"
            )));
        }
    };
    if updated.rows_affected() == 0 {
        drop(tx);
        let _ = remove_inflight(pool, &key).await;
        let _ = delete_loser(client.blobs.as_ref(), &key).await;
        return Err(StorageError::NotFound(format!(
            "document {} is not in this workspace",
            request.document_id
        )));
    }
    if let Err(err) = sqlx::query(
        r#"
        INSERT INTO original_object_refs (
            asset_object_id, tenant_id, workspace_id, object_key, sha256,
            byte_length, content_type, etag, verified_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, now())
        ON CONFLICT (asset_object_id) WHERE asset_object_id IS NOT NULL
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
    .bind(object_id)
    .bind(tenant_id)
    .bind(request.workspace_id)
    .bind(&key)
    .bind(&sha)
    .bind(request.asset_data.len() as i64)
    .bind(&request.content_type)
    .bind(meta.etag.as_deref())
    .execute(&mut *tx)
    .await
    {
        drop(tx);
        let _ = remove_inflight(pool, &key).await;
        let _ = delete_loser(client.blobs.as_ref(), &key).await;
        return Err(StorageError::Database(format!(
            "Failed to replace mm-asset ref: {err}"
        )));
    }
    if let Err(err) = sqlx::query("DELETE FROM original_object_inflight WHERE object_key = $1")
        .bind(&key)
        .execute(&mut *tx)
        .await
    {
        drop(tx);
        let _ = remove_inflight(pool, &key).await;
        let _ = delete_loser(client.blobs.as_ref(), &key).await;
        return Err(StorageError::Database(format!(
            "Failed to clear mm-asset inflight: {err}"
        )));
    }
    if let Err(err) = tx.commit().await {
        let _ = remove_inflight(pool, &key).await;
        let _ = delete_loser(client.blobs.as_ref(), &key).await;
        return Err(StorageError::Database(format!(
            "Failed to commit mm-asset replace: {err}"
        )));
    }
    if let Some(old) = previous.as_ref().map(|pointer| pointer.key.clone()) {
        if old != key {
            if let Err(err) = retire_previous_key(pool, client.blobs.as_ref(), &old).await {
                tracing::warn!(error = %err, "previous asset key stayed in the bucket");
            }
        }
    }
    Ok(())
}

#[derive(sqlx::FromRow)]
struct MmAssetSummaryRow {
    asset_id: String,
    asset_path: String,
    content_type: String,
    file_size_bytes: i64,
    asset_kind: String,
    page_num: Option<i32>,
}

impl From<MmAssetSummaryRow> for DocumentMmAssetSummary {
    fn from(row: MmAssetSummaryRow) -> Self {
        Self {
            asset_id: row.asset_id,
            asset_path: row.asset_path,
            content_type: row.content_type,
            file_size_bytes: row.file_size_bytes,
            asset_kind: row.asset_kind,
            page_num: row.page_num,
        }
    }
}

#[derive(sqlx::FromRow)]
struct MmAssetRow {
    document_id: Uuid,
    workspace_id: Uuid,
    asset_id: String,
    asset_path: String,
    content_type: String,
    file_size_bytes: i64,
    asset_data: Option<Vec<u8>>,
    asset_kind: String,
    page_num: Option<i32>,
    created_at: chrono::DateTime<chrono::Utc>,
    object_id: Uuid,
    byte_location: String,
}
