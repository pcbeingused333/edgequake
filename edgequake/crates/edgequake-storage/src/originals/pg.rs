//! Postgres claim, pointer, and reaper queries.
//!
//! The CLI pool sets `ROLE edgequake_originals_maint` (`BYPASSRLS`). The API
//! pool does not. Copy takes a session advisory lock so two migrators do not
//! PUT the same owner, and it releases the row lock before the PUT.

use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use chrono::Utc;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use tokio::sync::Mutex;
use uuid::Uuid;

use super::boot::{boot_decision, BootDecision};
use super::config::OriginalsOpened;
use super::format::{ClearView, PlanView};
use super::io::{BlobStore, OriginalsError};
use super::key::OwnerKind;
use super::ledger::{Claim, OriginalsLedger, RefView, Scope};
use super::reaper::{reap, InflightRow, INFLIGHT_TTL};

pub const CLAIM_STATEMENT_TIMEOUT: &str = "120s";
pub const MAINT_ROLE: &str = "edgequake_originals_maint";

fn map_db(err: sqlx::Error) -> OriginalsError {
    if err
        .to_string()
        .to_ascii_lowercase()
        .contains("edgequake_originals_maint")
    {
        return OriginalsError::Database(format!(
            "{err}; the maintenance role is missing. Apply migration 215 as a superuser."
        ));
    }
    OriginalsError::Database(err.to_string())
}

fn undefined_table(err: &sqlx::Error) -> bool {
    match err {
        sqlx::Error::Database(db) => db.code().as_deref() == Some("42P01"),
        _ => false,
    }
}

pub async fn connect_maint_pool(database_url: &str) -> Result<PgPool, OriginalsError> {
    PgPoolOptions::new()
        .max_connections(8)
        .after_connect(|conn, _| {
            Box::pin(async move {
                sqlx::query("SET ROLE edgequake_originals_maint")
                    .execute(&mut *conn)
                    .await?;
                Ok(())
            })
        })
        .connect(database_url)
        .await
        .map_err(map_db)
}

pub async fn assert_maint_role(pool: &PgPool) -> Result<(), OriginalsError> {
    let row = sqlx::query(
        "SELECT current_user::text AS name, rolbypassrls \
         FROM pg_roles WHERE rolname = current_user",
    )
    .fetch_one(pool)
    .await
    .map_err(map_db)?;
    let name: String = row.try_get("name").map_err(map_db)?;
    let bypass: bool = row.try_get("rolbypassrls").map_err(map_db)?;
    if name != MAINT_ROLE || !bypass {
        return Err(OriginalsError::Database(format!(
            "originals connection is {name} (bypass={bypass}); expected {MAINT_ROLE} with BYPASSRLS"
        )));
    }
    Ok(())
}

/// `SET LOCAL statement_timeout` inside the claim transaction, then the previous value.
pub async fn claim_statement_timeout(pool: &PgPool) -> Result<(String, String), OriginalsError> {
    let before: String = sqlx::query_scalar("SELECT current_setting('statement_timeout')")
        .fetch_one(pool)
        .await
        .map_err(map_db)?;
    let mut tx = pool.begin().await.map_err(map_db)?;
    sqlx::query("SELECT set_config('statement_timeout', $1, true)")
        .bind(CLAIM_STATEMENT_TIMEOUT)
        .execute(&mut *tx)
        .await
        .map_err(map_db)?;
    let during: String = sqlx::query_scalar("SELECT current_setting('statement_timeout')")
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db)?;
    tx.commit().await.map_err(map_db)?;
    let after: String = sqlx::query_scalar("SELECT current_setting('statement_timeout')")
        .fetch_one(pool)
        .await
        .map_err(map_db)?;
    if after != before {
        return Err(OriginalsError::Database(format!(
            "statement_timeout stayed at {after} after the claim transaction (was {before})"
        )));
    }
    Ok((during, after))
}

pub async fn count_refs(pool: &PgPool) -> Result<i64, OriginalsError> {
    match sqlx::query_scalar::<_, i64>("SELECT count(*)::bigint FROM original_object_refs")
        .fetch_one(pool)
        .await
    {
        Ok(n) => Ok(n),
        Err(err) if undefined_table(&err) => Ok(0),
        Err(err) => Err(map_db(err)),
    }
}

pub async fn count_holes(pool: &PgPool) -> Result<i64, OriginalsError> {
    let sql = r#"
        SELECT (
            SELECT count(*) FROM pdf_document_blobs b
            WHERE b.byte_location = 'object'
              AND NOT EXISTS (SELECT 1 FROM original_object_refs r WHERE r.pdf_id = b.pdf_id)
        ) + (
            SELECT count(*) FROM document_originals o
            WHERE o.byte_location = 'object'
              AND NOT EXISTS (SELECT 1 FROM original_object_refs r WHERE r.document_id = o.document_id)
        )
    "#;
    match sqlx::query_scalar::<_, i64>(sql).fetch_one(pool).await {
        Ok(n) => Ok(n),
        Err(err) if undefined_table(&err) => Ok(0),
        Err(err) => Err(map_db(err)),
    }
}

pub async fn assess_boot(
    pool: &PgPool,
    opened: &OriginalsOpened,
) -> Result<BootDecision, OriginalsError> {
    if opened.mode == super::config::StoreMode::Object && count_refs(pool).await.is_err() {
        return Err(OriginalsError::Config(
            "object mode requires migration 215 (original_object_refs)".into(),
        ));
    }
    let refs = count_refs(pool).await?;
    let holes = count_holes(pool).await?;
    Ok(boot_decision(opened.mode, refs, holes, opened.bucket_ready))
}

pub async fn enforce_boot(
    pool: &PgPool,
    opened: &OriginalsOpened,
) -> Result<BootDecision, OriginalsError> {
    let decision = assess_boot(pool, opened).await?;
    if let BootDecision::Exit(message) = &decision {
        return Err(OriginalsError::Mode(message.clone()));
    }
    Ok(decision)
}

pub async fn insert_inflight(pool: &PgPool, key: &str) -> Result<(), OriginalsError> {
    sqlx::query(
        "INSERT INTO original_object_inflight (object_key) VALUES ($1) \
         ON CONFLICT (object_key) DO NOTHING",
    )
    .bind(key)
    .execute(pool)
    .await
    .map_err(map_db)?;
    Ok(())
}

pub async fn remove_inflight(pool: &PgPool, key: &str) -> Result<(), OriginalsError> {
    sqlx::query("DELETE FROM original_object_inflight WHERE object_key = $1")
        .bind(key)
        .execute(pool)
        .await
        .map_err(map_db)?;
    Ok(())
}

pub async fn remove_delete_queue(pool: &PgPool, key: &str) -> Result<(), OriginalsError> {
    sqlx::query("DELETE FROM original_object_deletes WHERE object_key = $1")
        .bind(key)
        .execute(pool)
        .await
        .map_err(map_db)?;
    Ok(())
}

/// Delete a key that a committed ref no longer names. A failed delete is queued.
pub async fn retire_previous_key(
    pool: &PgPool,
    blob: &dyn BlobStore,
    key: &str,
) -> Result<(), OriginalsError> {
    match blob.delete_key(key).await {
        Ok(()) => remove_delete_queue(pool, key).await,
        Err(err) => {
            tracing::warn!(error = %err, "replaced object delete queued for the reaper");
            sqlx::query(
                "INSERT INTO original_object_deletes (object_key, enqueued_at) \
                 VALUES ($1, now()) ON CONFLICT (object_key) DO NOTHING",
            )
            .bind(key)
            .execute(pool)
            .await
            .map_err(map_db)?;
            Ok(())
        }
    }
}

pub async fn workspace_tenant(pool: &PgPool, workspace_id: Uuid) -> Result<Uuid, OriginalsError> {
    sqlx::query_scalar("SELECT tenant_id FROM workspaces WHERE workspace_id = $1")
        .bind(workspace_id)
        .fetch_optional(pool)
        .await
        .map_err(map_db)?
        .ok_or_else(|| OriginalsError::Database(format!("workspace {workspace_id} has no tenant")))
}

#[derive(Debug, Clone)]
pub struct ObjectPointer {
    pub key: String,
    pub sha256: String,
    pub byte_length: i64,
    pub verified: bool,
    pub tenant_id: Uuid,
    pub workspace_id: Uuid,
}

pub async fn pdf_pointer(
    pool: &PgPool,
    pdf_id: Uuid,
) -> Result<Option<ObjectPointer>, OriginalsError> {
    pointer(pool, "pdf_id", pdf_id).await
}

pub async fn original_pointer(
    pool: &PgPool,
    document_id: Uuid,
) -> Result<Option<ObjectPointer>, OriginalsError> {
    pointer(pool, "document_id", document_id).await
}

pub async fn asset_pointer(
    pool: &PgPool,
    object_id: Uuid,
) -> Result<Option<ObjectPointer>, OriginalsError> {
    pointer(pool, "asset_object_id", object_id).await
}

pub async fn markdown_pointer(
    pool: &PgPool,
    markdown_object_id: Uuid,
) -> Result<Option<ObjectPointer>, OriginalsError> {
    pointer(pool, "markdown_object_id", markdown_object_id).await
}

fn owner_column(kind: OwnerKind) -> &'static str {
    match kind {
        OwnerKind::Pdf => "pdf_id",
        OwnerKind::Original => "document_id",
        OwnerKind::Asset => "asset_object_id",
        OwnerKind::Markdown => "markdown_object_id",
        OwnerKind::Upload => "upload_id",
    }
}

async fn pointer(
    pool: &PgPool,
    column: &str,
    id: Uuid,
) -> Result<Option<ObjectPointer>, OriginalsError> {
    let sql = format!(
        "SELECT object_key, sha256::text AS sha256, byte_length, \
                verified_at IS NOT NULL AS verified, tenant_id, workspace_id \
         FROM original_object_refs WHERE {column} = $1"
    );
    let row = sqlx::query(&sql)
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(map_db)?;
    Ok(row.map(|row| ObjectPointer {
        key: row.get("object_key"),
        sha256: row.get::<String, _>("sha256").trim().to_string(),
        byte_length: row.get("byte_length"),
        verified: row.get("verified"),
        tenant_id: row.get("tenant_id"),
        workspace_id: row.get("workspace_id"),
    }))
}

pub async fn reap_postgres(pool: &PgPool, blob: &dyn BlobStore) -> Result<(), OriginalsError> {
    let cutoff = Utc::now() - INFLIGHT_TTL;
    let inflight_rows = sqlx::query(
        "SELECT object_key, created_at FROM original_object_inflight WHERE created_at < $1",
    )
    .bind(cutoff)
    .fetch_all(pool)
    .await
    .map_err(map_db)?;
    let queued = sqlx::query("SELECT object_key FROM original_object_deletes")
        .fetch_all(pool)
        .await
        .map_err(map_db)?;
    let expired: Vec<InflightRow> = inflight_rows
        .iter()
        .map(|row| InflightRow {
            key: row.get("object_key"),
            created_at: row.get("created_at"),
        })
        .collect();
    let expired_keys: Vec<&str> = expired.iter().map(|row| row.key.as_str()).collect();
    let queued_keys: Vec<String> = queued.iter().map(|row| row.get("object_key")).collect();
    let queued_refs: Vec<&str> = queued_keys.iter().map(String::as_str).collect();
    let stats = reap(blob, &expired_keys, &queued_refs).await?;
    for key in &stats.removed_inflight {
        remove_inflight(pool, key).await?;
    }
    for key in &stats.removed_queue {
        remove_delete_queue(pool, key).await?;
    }
    Ok(())
}

fn lock_key(id: Uuid) -> i64 {
    i64::from_le_bytes(id.as_bytes()[..8].try_into().expect("uuid prefix"))
}

pub struct PgLedger {
    pool: PgPool,
    locks: Mutex<HashMap<Uuid, sqlx::pool::PoolConnection<sqlx::Postgres>>>,
    resume_seen: Mutex<HashSet<Uuid>>,
}

impl PgLedger {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            locks: Mutex::new(HashMap::new()),
            resume_seen: Mutex::new(HashSet::new()),
        }
    }

    async fn unlock(&self, kind: OwnerKind, owner: Uuid) {
        let _ = kind;
        let mut locks = self.locks.lock().await;
        if let Some(mut conn) = locks.remove(&owner) {
            let _ = sqlx::query("SELECT pg_advisory_unlock($1)")
                .bind(lock_key(owner))
                .execute(&mut *conn)
                .await;
        }
    }
}

#[async_trait]
impl OriginalsLedger for PgLedger {
    async fn plan_view(
        &self,
        scope: &Scope,
        database_url: &str,
        store_url: &str,
        mode: &str,
    ) -> Result<PlanView, OriginalsError> {
        let pdf = gated_count(&self.pool, OwnerKind::Pdf, scope).await?;
        let original = gated_count(&self.pool, OwnerKind::Original, scope).await?;
        let asset = gated_count(&self.pool, OwnerKind::Asset, scope).await?;
        let markdown = gated_count(&self.pool, OwnerKind::Markdown, scope).await?;
        let pdf_rel: i64 = sqlx::query_scalar(
            "SELECT public.original_relation_bytes('pdf_document_blobs'::regclass)",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_db)?;
        let orig_rel: i64 = sqlx::query_scalar(
            "SELECT public.original_relation_bytes('document_originals'::regclass)",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_db)?;
        let asset_rel: i64 = sqlx::query_scalar(
            "SELECT public.original_relation_bytes('document_mm_assets'::regclass)",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_db)?;
        let markdown_rel: i64 =
            sqlx::query_scalar("SELECT public.original_relation_bytes('pdf_documents'::regclass)")
                .fetch_one(&self.pool)
                .await
                .map_err(map_db)?;
        let pdf_verified = verified_count(&self.pool, OwnerKind::Pdf).await?;
        let original_verified = verified_count(&self.pool, OwnerKind::Original).await?;
        let asset_verified = verified_count(&self.pool, OwnerKind::Asset).await?;
        let markdown_verified = verified_count(&self.pool, OwnerKind::Markdown).await?;
        let blocked = sqlx::query_scalar::<_, i64>(
            "SELECT count(*)::bigint FROM original_object_refs WHERE verified_at IS NULL",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_db)?;
        Ok(PlanView {
            database_url: super::format::redact_database_url(database_url),
            store_url: store_url.to_string(),
            mode: mode.to_string(),
            role: MAINT_ROLE.into(),
            pdf_rows: pdf.0,
            pdf_bytes: pdf.1,
            pdf_verified,
            original_rows: original.0,
            original_bytes: original.1,
            original_verified,
            asset_rows: asset.0,
            asset_bytes: asset.1,
            asset_verified,
            markdown_rows: markdown.0,
            markdown_bytes: markdown.1,
            markdown_verified,
            pdf_relation_bytes: pdf_rel.max(0) as u64,
            original_relation_bytes: orig_rel.max(0) as u64,
            asset_relation_bytes: asset_rel.max(0) as u64,
            markdown_relation_bytes: markdown_rel.max(0) as u64,
            blocked: blocked.max(0) as u64,
            next: "edgequake originals migrate".into(),
        })
    }

    async fn next_unreferenced(&self, scope: &Scope) -> Result<Option<Claim>, OriginalsError> {
        let mut skip: Vec<Uuid> = Vec::new();
        loop {
            let Some(claim) = claim_one(&self.pool, scope, &skip).await? else {
                return Ok(None);
            };
            let mut conn = self.pool.acquire().await.map_err(map_db)?;
            let locked: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")
                .bind(lock_key(claim.owner_id))
                .fetch_one(&mut *conn)
                .await
                .map_err(map_db)?;
            if !locked {
                skip.push(claim.owner_id);
                continue;
            }
            if ref_exists(&self.pool, claim.kind, claim.owner_id).await? {
                let _ = sqlx::query("SELECT pg_advisory_unlock($1)")
                    .bind(lock_key(claim.owner_id))
                    .execute(&mut *conn)
                    .await;
                skip.push(claim.owner_id);
                continue;
            }
            self.locks.lock().await.insert(claim.owner_id, conn);
            return Ok(Some(claim));
        }
    }

    async fn next_resume(&self, scope: &Scope) -> Result<Option<(Claim, RefView)>, OriginalsError> {
        let seen = self.resume_seen.lock().await.clone();
        let found = resume_one(&self.pool, scope, &seen).await?;
        if let Some((claim, _)) = &found {
            self.resume_seen.lock().await.insert(claim.owner_id);
        }
        Ok(found)
    }

    async fn insert_ref(
        &self,
        claim: &Claim,
        key: &str,
        sha256: &str,
        byte_length: i64,
        etag: Option<String>,
        verified: bool,
    ) -> Result<(), OriginalsError> {
        let verified_at = verified.then(Utc::now);
        let (pdf_id, document_id, asset_object_id, markdown_object_id) = match claim.kind {
            OwnerKind::Pdf => (Some(claim.owner_id), None, None, None),
            OwnerKind::Original => (None, Some(claim.owner_id), None, None),
            OwnerKind::Asset => (None, None, Some(claim.owner_id), None),
            OwnerKind::Markdown => (None, None, None, Some(claim.owner_id)),
            OwnerKind::Upload => {
                return Err(OriginalsError::Usage(
                    "upload staging is not an original".into(),
                ));
            }
        };
        let result = sqlx::query(
            r#"
            INSERT INTO original_object_refs (
                pdf_id, document_id, asset_object_id, markdown_object_id,
                tenant_id, workspace_id, object_key,
                sha256, byte_length, content_type, etag, verified_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(pdf_id)
        .bind(document_id)
        .bind(asset_object_id)
        .bind(markdown_object_id)
        .bind(claim.tenant_id)
        .bind(claim.workspace_id)
        .bind(key)
        .bind(sha256)
        .bind(byte_length)
        .bind(&claim.content_type)
        .bind(etag)
        .bind(verified_at)
        .execute(&self.pool)
        .await
        .map_err(map_db);
        self.unlock(claim.kind, claim.owner_id).await;
        result?;
        Ok(())
    }

    async fn release_claim(&self, kind: OwnerKind, owner: Uuid) -> Result<(), OriginalsError> {
        self.unlock(kind, owner).await;
        Ok(())
    }

    async fn mark_verified(&self, kind: OwnerKind, owner: Uuid) -> Result<(), OriginalsError> {
        let column = owner_column(kind);
        let sql = format!(
            "UPDATE original_object_refs SET verified_at = now() WHERE {column} = $1 AND verified_at IS NULL"
        );
        sqlx::query(&sql)
            .bind(owner)
            .execute(&self.pool)
            .await
            .map_err(map_db)?;
        Ok(())
    }

    async fn refs(&self, scope: &Scope) -> Result<Vec<RefView>, OriginalsError> {
        let rows = sqlx::query(
            r#"
            SELECT pdf_id, document_id, asset_object_id, markdown_object_id,
                   object_key, sha256::text AS sha256, byte_length, etag, verified_at
            FROM original_object_refs
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_db)?;
        let mut out = Vec::new();
        for row in rows {
            let pdf_id: Option<Uuid> = row.try_get("pdf_id").map_err(map_db)?;
            let document_id: Option<Uuid> = row.try_get("document_id").map_err(map_db)?;
            let asset_object_id: Option<Uuid> = row.try_get("asset_object_id").map_err(map_db)?;
            let markdown_object_id: Option<Uuid> =
                row.try_get("markdown_object_id").map_err(map_db)?;
            let (kind, owner) = if let Some(id) = pdf_id {
                (OwnerKind::Pdf, id)
            } else if let Some(id) = document_id {
                (OwnerKind::Original, id)
            } else if let Some(id) = asset_object_id {
                (OwnerKind::Asset, id)
            } else if let Some(id) = markdown_object_id {
                (OwnerKind::Markdown, id)
            } else {
                continue;
            };
            if !scope.kind.includes(kind) {
                continue;
            }
            out.push(RefView {
                kind,
                owner_id: owner,
                key: row.get("object_key"),
                sha256: row.get::<String, _>("sha256").trim().to_string(),
                byte_length: row.get("byte_length"),
                verified_at: row.try_get("verified_at").map_err(map_db)?,
                etag: row.try_get("etag").map_err(map_db)?,
            });
        }
        Ok(out)
    }

    async fn clear_verified(&self, scope: &Scope) -> Result<ClearView, OriginalsError> {
        let pdf = if scope.kind.includes(OwnerKind::Pdf) {
            null_pdf(&self.pool, scope.workspace).await?
        } else {
            (0, 0)
        };
        let original = if scope.kind.includes(OwnerKind::Original) {
            null_original(&self.pool, scope.workspace).await?
        } else {
            (0, 0)
        };
        let asset = if scope.kind.includes(OwnerKind::Asset) {
            null_asset(&self.pool, scope.workspace).await?
        } else {
            (0, 0)
        };
        let markdown = if scope.kind.includes(OwnerKind::Markdown) {
            null_markdown(&self.pool, scope.workspace).await?
        } else {
            (0, 0)
        };
        let pdf_rel: i64 = sqlx::query_scalar(
            "SELECT public.original_relation_bytes('pdf_document_blobs'::regclass)",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_db)?;
        let orig_rel: i64 = sqlx::query_scalar(
            "SELECT public.original_relation_bytes('document_originals'::regclass)",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_db)?;
        let asset_rel: i64 = sqlx::query_scalar(
            "SELECT public.original_relation_bytes('document_mm_assets'::regclass)",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_db)?;
        let markdown_rel: i64 =
            sqlx::query_scalar("SELECT public.original_relation_bytes('pdf_documents'::regclass)")
                .fetch_one(&self.pool)
                .await
                .map_err(map_db)?;
        Ok(ClearView {
            pdf: pdf.0,
            original: original.0,
            asset: asset.0,
            markdown: markdown.0,
            bytes: pdf.1 + original.1 + asset.1 + markdown.1,
            pdf_relation_bytes: pdf_rel.max(0) as u64,
            original_relation_bytes: orig_rel.max(0) as u64,
            asset_relation_bytes: asset_rel.max(0) as u64,
            markdown_relation_bytes: markdown_rel.max(0) as u64,
        })
    }

    async fn unverified_count(&self, _scope: &Scope) -> Result<u64, OriginalsError> {
        let n: i64 = sqlx::query_scalar(
            "SELECT count(*)::bigint FROM original_object_refs WHERE verified_at IS NULL",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_db)?;
        Ok(n.max(0) as u64)
    }

    async fn write_restored(
        &self,
        kind: OwnerKind,
        owner: Uuid,
        bytes: &[u8],
    ) -> Result<(), OriginalsError> {
        match kind {
            OwnerKind::Pdf => {
                sqlx::query(
                    "UPDATE pdf_document_blobs SET pdf_data = $2, byte_location = 'database', updated_at = now() WHERE pdf_id = $1",
                )
                .bind(owner)
                .bind(bytes)
                .execute(&self.pool)
                .await
                .map_err(map_db)?;
            }
            OwnerKind::Original => {
                sqlx::query(
                    "UPDATE document_originals SET original_data = $2, byte_location = 'database' WHERE document_id = $1",
                )
                .bind(owner)
                .bind(bytes)
                .execute(&self.pool)
                .await
                .map_err(map_db)?;
            }
            OwnerKind::Asset => {
                sqlx::query(
                    "UPDATE document_mm_assets SET asset_data = $2, byte_location = 'database' WHERE object_id = $1",
                )
                .bind(owner)
                .bind(bytes)
                .execute(&self.pool)
                .await
                .map_err(map_db)?;
            }
            OwnerKind::Markdown => {
                let text = String::from_utf8(bytes.to_vec()).map_err(|err| {
                    OriginalsError::Database(format!("markdown {owner} is not utf-8: {err}"))
                })?;
                sqlx::query(
                    "UPDATE pdf_documents SET markdown_content = $2, markdown_location = 'database' WHERE markdown_object_id = $1",
                )
                .bind(owner)
                .bind(text)
                .execute(&self.pool)
                .await
                .map_err(map_db)?;
            }
            OwnerKind::Upload => {
                return Err(OriginalsError::Usage(
                    "upload staging is not an original".into(),
                ));
            }
        }
        Ok(())
    }

    async fn put_chunk_sha(&self, owner: Uuid, packed: &[u8]) -> Result<(), OriginalsError> {
        sqlx::query("UPDATE original_object_refs SET chunk_sha256 = $2 WHERE pdf_id = $1")
            .bind(owner)
            .bind(packed)
            .execute(&self.pool)
            .await
            .map_err(map_db)?;
        Ok(())
    }

    async fn chunk_sha_of(&self, owner: Uuid) -> Result<Option<Vec<u8>>, OriginalsError> {
        let value: Option<Option<Vec<u8>>> =
            sqlx::query_scalar("SELECT chunk_sha256 FROM original_object_refs WHERE pdf_id = $1")
                .bind(owner)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_db)?;
        Ok(value.flatten())
    }

    async fn write_restored_chunk(
        &self,
        kind: OwnerKind,
        owner: Uuid,
        chunk: &[u8],
        first: bool,
    ) -> Result<(), OriginalsError> {
        if first && chunk.is_empty() {
            let clear = match kind {
                OwnerKind::Pdf => {
                    "UPDATE pdf_document_blobs SET pdf_data = NULL, byte_location = 'object' WHERE pdf_id = $1"
                }
                OwnerKind::Original => {
                    "UPDATE document_originals SET original_data = NULL, byte_location = 'object' WHERE document_id = $1"
                }
                OwnerKind::Asset => {
                    "UPDATE document_mm_assets SET asset_data = NULL, byte_location = 'object' WHERE object_id = $1"
                }
                OwnerKind::Markdown => {
                    "UPDATE pdf_documents SET markdown_content = NULL, markdown_location = 'object' WHERE markdown_object_id = $1"
                }
                OwnerKind::Upload => {
                    return Err(OriginalsError::Usage(
                        "upload staging is not an original".into(),
                    ));
                }
            };
            sqlx::query(clear)
                .bind(owner)
                .execute(&self.pool)
                .await
                .map_err(map_db)?;
            return Ok(());
        }
        let sql = match kind {
            OwnerKind::Pdf => {
                "UPDATE pdf_document_blobs SET pdf_data = CASE WHEN $3 THEN $2 ELSE pdf_data || $2 END, byte_location = 'database', updated_at = now() WHERE pdf_id = $1"
            }
            OwnerKind::Original => {
                "UPDATE document_originals SET original_data = CASE WHEN $3 THEN $2 ELSE original_data || $2 END, byte_location = 'database' WHERE document_id = $1"
            }
            OwnerKind::Asset => {
                "UPDATE document_mm_assets SET asset_data = CASE WHEN $3 THEN $2 ELSE asset_data || $2 END, byte_location = 'database' WHERE object_id = $1"
            }
            OwnerKind::Markdown => {
                "UPDATE pdf_documents SET markdown_content = CASE WHEN $3 THEN convert_from($2, 'UTF8') ELSE COALESCE(markdown_content, '') || convert_from($2, 'UTF8') END, markdown_location = 'database' WHERE markdown_object_id = $1"
            }
            OwnerKind::Upload => {
                return Err(OriginalsError::Usage(
                    "upload staging is not an original".into(),
                ));
            }
        };
        sqlx::query(sql)
            .bind(owner)
            .bind(chunk)
            .bind(first)
            .execute(&self.pool)
            .await
            .map_err(map_db)?;
        Ok(())
    }

    async fn bytea_of(
        &self,
        kind: OwnerKind,
        owner: Uuid,
    ) -> Result<Option<Vec<u8>>, OriginalsError> {
        let sql = match kind {
            OwnerKind::Pdf => "SELECT pdf_data FROM pdf_document_blobs WHERE pdf_id = $1",
            OwnerKind::Original => {
                "SELECT original_data FROM document_originals WHERE document_id = $1"
            }
            OwnerKind::Asset => "SELECT asset_data FROM document_mm_assets WHERE object_id = $1",
            OwnerKind::Markdown => {
                "SELECT convert_to(markdown_content, 'UTF8') FROM pdf_documents WHERE markdown_object_id = $1"
            }
            OwnerKind::Upload => {
                return Err(OriginalsError::Usage(
                    "upload staging is not an original".into(),
                ));
            }
        };
        let value: Option<Option<Vec<u8>>> = sqlx::query_scalar(sql)
            .bind(owner)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_db)?;
        Ok(value.flatten())
    }

    async fn content_of(&self, kind: OwnerKind, owner: Uuid) -> Result<String, OriginalsError> {
        let sql = match kind {
            OwnerKind::Pdf => {
                "SELECT COALESCE(doc.content, '') FROM pdf_documents d \
                 LEFT JOIN documents doc ON doc.id = d.document_id WHERE d.pdf_id = $1"
            }
            OwnerKind::Original => "SELECT COALESCE(content, '') FROM documents WHERE id = $1",
            OwnerKind::Asset => {
                "SELECT COALESCE(doc.content, '') FROM document_mm_assets a \
                 LEFT JOIN documents doc ON doc.id = a.document_id WHERE a.object_id = $1"
            }
            OwnerKind::Markdown => {
                "SELECT COALESCE(doc.content, '') FROM pdf_documents d \
                 LEFT JOIN documents doc ON doc.id = d.document_id WHERE d.markdown_object_id = $1"
            }
            OwnerKind::Upload => {
                return Err(OriginalsError::Usage(
                    "upload staging is not an original".into(),
                ));
            }
        };
        sqlx::query_scalar(sql)
            .bind(owner)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_db)?
            .ok_or_else(|| OriginalsError::Database(format!("no content for {owner}")))
    }

    async fn asset_len_of(&self, _kind: OwnerKind, _owner: Uuid) -> Result<i64, OriginalsError> {
        // Derived page images are out of scope. Callers that need the length
        // read that table themselves; this engine does not.
        Ok(0)
    }
}

async fn ref_exists(pool: &PgPool, kind: OwnerKind, owner: Uuid) -> Result<bool, OriginalsError> {
    let sql = match kind {
        OwnerKind::Pdf => "SELECT 1 FROM original_object_refs WHERE pdf_id = $1",
        OwnerKind::Original => "SELECT 1 FROM original_object_refs WHERE document_id = $1",
        OwnerKind::Asset => "SELECT 1 FROM original_object_refs WHERE asset_object_id = $1",
        OwnerKind::Markdown => "SELECT 1 FROM original_object_refs WHERE markdown_object_id = $1",
        OwnerKind::Upload => {
            return Err(OriginalsError::Usage(
                "upload staging is not an original".into(),
            ));
        }
    };
    let row = sqlx::query(sql)
        .bind(owner)
        .fetch_optional(pool)
        .await
        .map_err(map_db)?;
    Ok(row.is_some())
}

async fn claim_one(
    pool: &PgPool,
    scope: &Scope,
    skip: &[Uuid],
) -> Result<Option<Claim>, OriginalsError> {
    if scope.kind.includes(OwnerKind::Pdf) {
        if let Some(claim) = claim_pdf(pool, scope.workspace, skip).await? {
            return Ok(Some(claim));
        }
    }
    if scope.kind.includes(OwnerKind::Original) {
        if let Some(claim) = claim_original(pool, scope.workspace, skip).await? {
            return Ok(Some(claim));
        }
    }
    if scope.kind.includes(OwnerKind::Asset) {
        if let Some(claim) = claim_asset(pool, scope.workspace, skip).await? {
            return Ok(Some(claim));
        }
    }
    if scope.kind.includes(OwnerKind::Markdown) {
        if let Some(claim) = claim_markdown(pool, scope.workspace, skip).await? {
            return Ok(Some(claim));
        }
    }
    Ok(None)
}

async fn claim_pdf(
    pool: &PgPool,
    workspace: Option<Uuid>,
    skip: &[Uuid],
) -> Result<Option<Claim>, OriginalsError> {
    let mut tx = pool.begin().await.map_err(map_db)?;
    sqlx::query("SELECT set_config('statement_timeout', $1, true)")
        .bind(CLAIM_STATEMENT_TIMEOUT)
        .execute(&mut *tx)
        .await
        .map_err(map_db)?;
    let row = sqlx::query(
        r#"
        SELECT d.pdf_id AS owner_id, d.workspace_id, w.tenant_id, d.filename, d.content_type, b.pdf_data AS bytes
        FROM pdf_document_blobs b
        JOIN pdf_documents d ON d.pdf_id = b.pdf_id
        JOIN workspaces w ON w.workspace_id = d.workspace_id
        WHERE b.byte_location = 'database'
          AND b.pdf_data IS NOT NULL
          AND NOT EXISTS (SELECT 1 FROM original_object_refs r WHERE r.pdf_id = b.pdf_id)
          AND ($1::uuid IS NULL OR d.workspace_id = $1)
          AND NOT (d.pdf_id = ANY($2))
        ORDER BY d.pdf_id
        FOR UPDATE OF b SKIP LOCKED
        LIMIT 1
        "#,
    )
    .bind(workspace)
    .bind(skip)
    .fetch_optional(&mut *tx)
    .await
    .map_err(map_db)?;
    let Some(row) = row else {
        tx.commit().await.map_err(map_db)?;
        return Ok(None);
    };
    let bytes: Vec<u8> = row.try_get("bytes").map_err(map_db)?;
    let claim = Claim {
        kind: OwnerKind::Pdf,
        owner_id: row.try_get("owner_id").map_err(map_db)?,
        tenant_id: row.try_get("tenant_id").map_err(map_db)?,
        workspace_id: row.try_get("workspace_id").map_err(map_db)?,
        filename: row.try_get("filename").map_err(map_db)?,
        content_type: row.try_get("content_type").map_err(map_db)?,
        sha256: super::io::sha256_hex(&bytes),
        bytes,
    };
    tx.commit().await.map_err(map_db)?;
    Ok(Some(claim))
}

async fn claim_original(
    pool: &PgPool,
    workspace: Option<Uuid>,
    skip: &[Uuid],
) -> Result<Option<Claim>, OriginalsError> {
    let mut tx = pool.begin().await.map_err(map_db)?;
    sqlx::query("SELECT set_config('statement_timeout', $1, true)")
        .bind(CLAIM_STATEMENT_TIMEOUT)
        .execute(&mut *tx)
        .await
        .map_err(map_db)?;
    let row = sqlx::query(
        r#"
        SELECT o.document_id AS owner_id, o.workspace_id, w.tenant_id, o.filename, o.content_type, o.original_data AS bytes
        FROM document_originals o
        JOIN workspaces w ON w.workspace_id = o.workspace_id
        WHERE o.byte_location = 'database'
          AND o.original_data IS NOT NULL
          AND NOT EXISTS (SELECT 1 FROM original_object_refs r WHERE r.document_id = o.document_id)
          AND ($1::uuid IS NULL OR o.workspace_id = $1)
          AND NOT (o.document_id = ANY($2))
        ORDER BY o.document_id
        FOR UPDATE OF o SKIP LOCKED
        LIMIT 1
        "#,
    )
    .bind(workspace)
    .bind(skip)
    .fetch_optional(&mut *tx)
    .await
    .map_err(map_db)?;
    let Some(row) = row else {
        tx.commit().await.map_err(map_db)?;
        return Ok(None);
    };
    let bytes: Vec<u8> = row.try_get("bytes").map_err(map_db)?;
    let claim = Claim {
        kind: OwnerKind::Original,
        owner_id: row.try_get("owner_id").map_err(map_db)?,
        tenant_id: row.try_get("tenant_id").map_err(map_db)?,
        workspace_id: row.try_get("workspace_id").map_err(map_db)?,
        filename: row.try_get("filename").map_err(map_db)?,
        content_type: row.try_get("content_type").map_err(map_db)?,
        sha256: super::io::sha256_hex(&bytes),
        bytes,
    };
    tx.commit().await.map_err(map_db)?;
    Ok(Some(claim))
}

async fn resume_one(
    pool: &PgPool,
    scope: &Scope,
    seen: &HashSet<Uuid>,
) -> Result<Option<(Claim, RefView)>, OriginalsError> {
    let seen: Vec<Uuid> = seen.iter().copied().collect();
    if scope.kind.includes(OwnerKind::Pdf) {
        if let Some(pair) = resume_pdf(pool, scope.workspace, &seen).await? {
            return Ok(Some(pair));
        }
    }
    if scope.kind.includes(OwnerKind::Original) {
        if let Some(pair) = resume_original(pool, scope.workspace, &seen).await? {
            return Ok(Some(pair));
        }
    }
    if scope.kind.includes(OwnerKind::Asset) {
        if let Some(pair) = resume_asset(pool, scope.workspace, &seen).await? {
            return Ok(Some(pair));
        }
    }
    if scope.kind.includes(OwnerKind::Markdown) {
        if let Some(pair) = resume_markdown(pool, scope.workspace, &seen).await? {
            return Ok(Some(pair));
        }
    }
    Ok(None)
}

async fn resume_pdf(
    pool: &PgPool,
    workspace: Option<Uuid>,
    seen: &[Uuid],
) -> Result<Option<(Claim, RefView)>, OriginalsError> {
    let row = sqlx::query(
        r#"
        SELECT d.pdf_id AS owner_id, d.workspace_id, w.tenant_id, d.filename, d.content_type,
               b.pdf_data AS bytes, r.object_key, r.sha256::text AS sha256, r.byte_length, r.etag, r.verified_at
        FROM pdf_document_blobs b
        JOIN pdf_documents d ON d.pdf_id = b.pdf_id
        JOIN workspaces w ON w.workspace_id = d.workspace_id
        JOIN original_object_refs r ON r.pdf_id = b.pdf_id
        WHERE b.pdf_data IS NOT NULL
          AND ($1::uuid IS NULL OR d.workspace_id = $1)
          AND NOT (d.pdf_id = ANY($2))
        ORDER BY d.pdf_id
        LIMIT 1
        "#,
    )
    .bind(workspace)
    .bind(seen)
    .fetch_optional(pool)
    .await
    .map_err(map_db)?;
    Ok(row.map(|row| pair_from_row(OwnerKind::Pdf, row)))
}

async fn resume_original(
    pool: &PgPool,
    workspace: Option<Uuid>,
    seen: &[Uuid],
) -> Result<Option<(Claim, RefView)>, OriginalsError> {
    let row = sqlx::query(
        r#"
        SELECT o.document_id AS owner_id, o.workspace_id, w.tenant_id, o.filename, o.content_type,
               o.original_data AS bytes, r.object_key, r.sha256::text AS sha256, r.byte_length, r.etag, r.verified_at
        FROM document_originals o
        JOIN workspaces w ON w.workspace_id = o.workspace_id
        JOIN original_object_refs r ON r.document_id = o.document_id
        WHERE o.original_data IS NOT NULL
          AND ($1::uuid IS NULL OR o.workspace_id = $1)
          AND NOT (o.document_id = ANY($2))
        ORDER BY o.document_id
        LIMIT 1
        "#,
    )
    .bind(workspace)
    .bind(seen)
    .fetch_optional(pool)
    .await
    .map_err(map_db)?;
    Ok(row.map(|row| pair_from_row(OwnerKind::Original, row)))
}

fn pair_from_row(kind: OwnerKind, row: sqlx::postgres::PgRow) -> (Claim, RefView) {
    let bytes: Vec<u8> = row.get("bytes");
    let owner_id: Uuid = row.get("owner_id");
    let claim = Claim {
        kind,
        owner_id,
        tenant_id: row.get("tenant_id"),
        workspace_id: row.get("workspace_id"),
        filename: row.get("filename"),
        content_type: row.get("content_type"),
        sha256: super::io::sha256_hex(&bytes),
        bytes,
    };
    let reference = RefView {
        kind,
        owner_id,
        key: row.get("object_key"),
        sha256: row.get::<String, _>("sha256").trim().to_string(),
        byte_length: row.get("byte_length"),
        verified_at: row.get("verified_at"),
        etag: row.get("etag"),
    };
    (claim, reference)
}

async fn count_bytea(
    pool: &PgPool,
    kind: OwnerKind,
    scope: &Scope,
) -> Result<(u64, u64), OriginalsError> {
    let (sql_rows, sql_bytes) = match kind {
        OwnerKind::Pdf => (
            "SELECT count(*)::bigint FROM pdf_document_blobs b WHERE b.pdf_data IS NOT NULL AND ($1::uuid IS NULL OR EXISTS (SELECT 1 FROM pdf_documents d WHERE d.pdf_id = b.pdf_id AND d.workspace_id = $1))",
            "SELECT COALESCE(sum(octet_length(b.pdf_data)), 0)::bigint FROM pdf_document_blobs b WHERE b.pdf_data IS NOT NULL AND ($1::uuid IS NULL OR EXISTS (SELECT 1 FROM pdf_documents d WHERE d.pdf_id = b.pdf_id AND d.workspace_id = $1))",
        ),
        OwnerKind::Original => (
            "SELECT count(*)::bigint FROM document_originals WHERE original_data IS NOT NULL AND ($1::uuid IS NULL OR workspace_id = $1)",
            "SELECT COALESCE(sum(octet_length(original_data)), 0)::bigint FROM document_originals WHERE original_data IS NOT NULL AND ($1::uuid IS NULL OR workspace_id = $1)",
        ),
        OwnerKind::Asset => (
            "SELECT count(*)::bigint FROM document_mm_assets WHERE asset_data IS NOT NULL AND ($1::uuid IS NULL OR workspace_id = $1)",
            "SELECT COALESCE(sum(octet_length(asset_data)), 0)::bigint FROM document_mm_assets WHERE asset_data IS NOT NULL AND ($1::uuid IS NULL OR workspace_id = $1)",
        ),
        OwnerKind::Markdown => (
            "SELECT count(*)::bigint FROM pdf_documents WHERE markdown_content IS NOT NULL AND ($1::uuid IS NULL OR workspace_id = $1)",
            "SELECT COALESCE(sum(octet_length(convert_to(markdown_content, 'UTF8'))), 0)::bigint FROM pdf_documents WHERE markdown_content IS NOT NULL AND ($1::uuid IS NULL OR workspace_id = $1)",
        ),
        OwnerKind::Upload => {
            return Err(OriginalsError::Usage(
                "upload staging is not an original".into(),
            ));
        }
    };
    let _ = scope.kind;
    let rows: i64 = sqlx::query_scalar(sql_rows)
        .bind(scope.workspace)
        .fetch_one(pool)
        .await
        .map_err(map_db)?;
    let bytes: i64 = sqlx::query_scalar(sql_bytes)
        .bind(scope.workspace)
        .fetch_one(pool)
        .await
        .map_err(map_db)?;
    Ok((rows.max(0) as u64, bytes.max(0) as u64))
}

async fn gated_count(
    pool: &PgPool,
    kind: OwnerKind,
    scope: &Scope,
) -> Result<(u64, u64), OriginalsError> {
    if scope.kind.includes(kind) {
        count_bytea(pool, kind, scope).await
    } else {
        Ok((0, 0))
    }
}

/// Owners that `copy` will PUT: bytes present, no ref yet.
pub async fn pending_copy_count(pool: &PgPool, scope: &Scope) -> Result<u64, OriginalsError> {
    let pdf = if scope.kind.includes(OwnerKind::Pdf) {
        let n: i64 = sqlx::query_scalar(
            "SELECT count(*)::bigint FROM pdf_document_blobs b \
             JOIN pdf_documents d ON d.pdf_id = b.pdf_id \
             WHERE b.pdf_data IS NOT NULL \
               AND NOT EXISTS (SELECT 1 FROM original_object_refs r WHERE r.pdf_id = b.pdf_id) \
               AND ($1::uuid IS NULL OR d.workspace_id = $1)",
        )
        .bind(scope.workspace)
        .fetch_one(pool)
        .await
        .map_err(map_db)?;
        n.max(0) as u64
    } else {
        0
    };
    let original = if scope.kind.includes(OwnerKind::Original) {
        let n: i64 = sqlx::query_scalar(
            "SELECT count(*)::bigint FROM document_originals o \
             WHERE o.original_data IS NOT NULL \
               AND NOT EXISTS (SELECT 1 FROM original_object_refs r WHERE r.document_id = o.document_id) \
               AND ($1::uuid IS NULL OR o.workspace_id = $1)",
        )
        .bind(scope.workspace)
        .fetch_one(pool)
        .await
        .map_err(map_db)?;
        n.max(0) as u64
    } else {
        0
    };
    let asset = if scope.kind.includes(OwnerKind::Asset) {
        let n: i64 = sqlx::query_scalar(
            "SELECT count(*)::bigint FROM document_mm_assets a \
             WHERE a.asset_data IS NOT NULL \
               AND NOT EXISTS (SELECT 1 FROM original_object_refs r WHERE r.asset_object_id = a.object_id) \
               AND ($1::uuid IS NULL OR a.workspace_id = $1)",
        )
        .bind(scope.workspace)
        .fetch_one(pool)
        .await
        .map_err(map_db)?;
        n.max(0) as u64
    } else {
        0
    };
    let markdown = if scope.kind.includes(OwnerKind::Markdown) {
        let n: i64 = sqlx::query_scalar(
            "SELECT count(*)::bigint FROM pdf_documents d \
             WHERE d.markdown_content IS NOT NULL \
               AND NOT EXISTS (SELECT 1 FROM original_object_refs r WHERE r.markdown_object_id = d.markdown_object_id) \
               AND ($1::uuid IS NULL OR d.workspace_id = $1)",
        )
        .bind(scope.workspace)
        .fetch_one(pool)
        .await
        .map_err(map_db)?;
        n.max(0) as u64
    } else {
        0
    };
    Ok(pdf + original + asset + markdown)
}

async fn verified_count(pool: &PgPool, kind: OwnerKind) -> Result<u64, OriginalsError> {
    let sql = match kind {
        OwnerKind::Pdf => {
            "SELECT count(*)::bigint FROM original_object_refs WHERE pdf_id IS NOT NULL AND verified_at IS NOT NULL"
        }
        OwnerKind::Original => {
            "SELECT count(*)::bigint FROM original_object_refs WHERE document_id IS NOT NULL AND verified_at IS NOT NULL"
        }
        OwnerKind::Asset => {
            "SELECT count(*)::bigint FROM original_object_refs WHERE asset_object_id IS NOT NULL AND verified_at IS NOT NULL"
        }
        OwnerKind::Markdown => {
            "SELECT count(*)::bigint FROM original_object_refs WHERE markdown_object_id IS NOT NULL AND verified_at IS NOT NULL"
        }
        OwnerKind::Upload => {
            return Err(OriginalsError::Usage(
                "upload staging is not an original".into(),
            ));
        }
    };
    let n: i64 = sqlx::query_scalar(sql)
        .fetch_one(pool)
        .await
        .map_err(map_db)?;
    Ok(n.max(0) as u64)
}

async fn null_pdf(pool: &PgPool, workspace: Option<Uuid>) -> Result<(u64, u64), OriginalsError> {
    // Capture octet_length before the assignment. RETURNING pdf_data after
    // SET NULL is 0 and would make `edgequake originals clear` lie.
    let lengths = sqlx::query_scalar::<_, i64>(
        r#"
        WITH doomed AS (
            SELECT b.pdf_id, octet_length(b.pdf_data)::bigint AS n
            FROM pdf_document_blobs b
            JOIN original_object_refs r ON r.pdf_id = b.pdf_id
            JOIN pdf_documents d ON d.pdf_id = b.pdf_id
            WHERE r.verified_at IS NOT NULL
              AND b.pdf_data IS NOT NULL
              AND ($1::uuid IS NULL OR d.workspace_id = $1)
            FOR UPDATE OF b
        )
        UPDATE pdf_document_blobs b
        SET pdf_data = NULL, byte_location = 'object', updated_at = now()
        FROM doomed
        WHERE b.pdf_id = doomed.pdf_id
        RETURNING doomed.n
        "#,
    )
    .bind(workspace)
    .fetch_all(pool)
    .await
    .map_err(map_db)?;
    let bytes: u64 = lengths.iter().map(|n| (*n).max(0) as u64).sum();
    Ok((lengths.len() as u64, bytes))
}

async fn null_original(
    pool: &PgPool,
    workspace: Option<Uuid>,
) -> Result<(u64, u64), OriginalsError> {
    let lengths = sqlx::query_scalar::<_, i64>(
        r#"
        WITH doomed AS (
            SELECT o.document_id, octet_length(o.original_data)::bigint AS n
            FROM document_originals o
            JOIN original_object_refs r ON r.document_id = o.document_id
            WHERE r.verified_at IS NOT NULL
              AND o.original_data IS NOT NULL
              AND ($1::uuid IS NULL OR o.workspace_id = $1)
            FOR UPDATE OF o
        )
        UPDATE document_originals o
        SET original_data = NULL, byte_location = 'object'
        FROM doomed
        WHERE o.document_id = doomed.document_id
        RETURNING doomed.n
        "#,
    )
    .bind(workspace)
    .fetch_all(pool)
    .await
    .map_err(map_db)?;
    let bytes: u64 = lengths.iter().map(|n| (*n).max(0) as u64).sum();
    Ok((lengths.len() as u64, bytes))
}

async fn claim_asset(
    pool: &PgPool,
    workspace: Option<Uuid>,
    skip: &[Uuid],
) -> Result<Option<Claim>, OriginalsError> {
    let mut tx = pool.begin().await.map_err(map_db)?;
    sqlx::query("SELECT set_config('statement_timeout', $1, true)")
        .bind(CLAIM_STATEMENT_TIMEOUT)
        .execute(&mut *tx)
        .await
        .map_err(map_db)?;
    let row = sqlx::query(
        r#"
        SELECT a.object_id AS owner_id, a.workspace_id, w.tenant_id,
               ''::text AS filename, a.content_type, a.asset_data AS bytes
        FROM document_mm_assets a
        JOIN workspaces w ON w.workspace_id = a.workspace_id
        WHERE a.byte_location = 'database'
          AND a.asset_data IS NOT NULL
          AND NOT EXISTS (SELECT 1 FROM original_object_refs r WHERE r.asset_object_id = a.object_id)
          AND ($1::uuid IS NULL OR a.workspace_id = $1)
          AND NOT (a.object_id = ANY($2))
        ORDER BY a.object_id
        FOR UPDATE OF a SKIP LOCKED
        LIMIT 1
        "#,
    )
    .bind(workspace)
    .bind(skip)
    .fetch_optional(&mut *tx)
    .await
    .map_err(map_db)?;
    let Some(row) = row else {
        tx.commit().await.map_err(map_db)?;
        return Ok(None);
    };
    let bytes: Vec<u8> = row.try_get("bytes").map_err(map_db)?;
    let claim = Claim {
        kind: OwnerKind::Asset,
        owner_id: row.try_get("owner_id").map_err(map_db)?,
        tenant_id: row.try_get("tenant_id").map_err(map_db)?,
        workspace_id: row.try_get("workspace_id").map_err(map_db)?,
        filename: row.try_get("filename").map_err(map_db)?,
        content_type: row.try_get("content_type").map_err(map_db)?,
        sha256: super::io::sha256_hex(&bytes),
        bytes,
    };
    tx.commit().await.map_err(map_db)?;
    Ok(Some(claim))
}

async fn claim_markdown(
    pool: &PgPool,
    workspace: Option<Uuid>,
    skip: &[Uuid],
) -> Result<Option<Claim>, OriginalsError> {
    let mut tx = pool.begin().await.map_err(map_db)?;
    sqlx::query("SELECT set_config('statement_timeout', $1, true)")
        .bind(CLAIM_STATEMENT_TIMEOUT)
        .execute(&mut *tx)
        .await
        .map_err(map_db)?;
    let row = sqlx::query(
        r#"
        SELECT d.markdown_object_id AS owner_id, d.workspace_id, w.tenant_id,
               ''::text AS filename, 'text/markdown'::text AS content_type,
               convert_to(d.markdown_content, 'UTF8') AS bytes
        FROM pdf_documents d
        JOIN workspaces w ON w.workspace_id = d.workspace_id
        WHERE d.markdown_location = 'database'
          AND d.markdown_content IS NOT NULL
          AND NOT EXISTS (
              SELECT 1 FROM original_object_refs r WHERE r.markdown_object_id = d.markdown_object_id
          )
          AND ($1::uuid IS NULL OR d.workspace_id = $1)
          AND NOT (d.markdown_object_id = ANY($2))
        ORDER BY d.markdown_object_id
        FOR UPDATE OF d SKIP LOCKED
        LIMIT 1
        "#,
    )
    .bind(workspace)
    .bind(skip)
    .fetch_optional(&mut *tx)
    .await
    .map_err(map_db)?;
    let Some(row) = row else {
        tx.commit().await.map_err(map_db)?;
        return Ok(None);
    };
    let bytes: Vec<u8> = row.try_get("bytes").map_err(map_db)?;
    let claim = Claim {
        kind: OwnerKind::Markdown,
        owner_id: row.try_get("owner_id").map_err(map_db)?,
        tenant_id: row.try_get("tenant_id").map_err(map_db)?,
        workspace_id: row.try_get("workspace_id").map_err(map_db)?,
        filename: row.try_get("filename").map_err(map_db)?,
        content_type: row.try_get("content_type").map_err(map_db)?,
        sha256: super::io::sha256_hex(&bytes),
        bytes,
    };
    tx.commit().await.map_err(map_db)?;
    Ok(Some(claim))
}

async fn resume_asset(
    pool: &PgPool,
    workspace: Option<Uuid>,
    seen: &[Uuid],
) -> Result<Option<(Claim, RefView)>, OriginalsError> {
    let row = sqlx::query(
        r#"
        SELECT a.object_id AS owner_id, a.workspace_id, w.tenant_id,
               ''::text AS filename, a.content_type, a.asset_data AS bytes,
               r.object_key, r.sha256::text AS sha256, r.byte_length, r.etag, r.verified_at
        FROM document_mm_assets a
        JOIN workspaces w ON w.workspace_id = a.workspace_id
        JOIN original_object_refs r ON r.asset_object_id = a.object_id
        WHERE a.asset_data IS NOT NULL
          AND ($1::uuid IS NULL OR a.workspace_id = $1)
          AND NOT (a.object_id = ANY($2))
        ORDER BY a.object_id
        LIMIT 1
        "#,
    )
    .bind(workspace)
    .bind(seen)
    .fetch_optional(pool)
    .await
    .map_err(map_db)?;
    Ok(row.map(|row| pair_from_row(OwnerKind::Asset, row)))
}

async fn resume_markdown(
    pool: &PgPool,
    workspace: Option<Uuid>,
    seen: &[Uuid],
) -> Result<Option<(Claim, RefView)>, OriginalsError> {
    let row = sqlx::query(
        r#"
        SELECT d.markdown_object_id AS owner_id, d.workspace_id, w.tenant_id,
               ''::text AS filename, 'text/markdown'::text AS content_type,
               convert_to(d.markdown_content, 'UTF8') AS bytes,
               r.object_key, r.sha256::text AS sha256, r.byte_length, r.etag, r.verified_at
        FROM pdf_documents d
        JOIN workspaces w ON w.workspace_id = d.workspace_id
        JOIN original_object_refs r ON r.markdown_object_id = d.markdown_object_id
        WHERE d.markdown_content IS NOT NULL
          AND ($1::uuid IS NULL OR d.workspace_id = $1)
          AND NOT (d.markdown_object_id = ANY($2))
        ORDER BY d.markdown_object_id
        LIMIT 1
        "#,
    )
    .bind(workspace)
    .bind(seen)
    .fetch_optional(pool)
    .await
    .map_err(map_db)?;
    Ok(row.map(|row| pair_from_row(OwnerKind::Markdown, row)))
}

async fn null_asset(pool: &PgPool, workspace: Option<Uuid>) -> Result<(u64, u64), OriginalsError> {
    let lengths = sqlx::query_scalar::<_, i64>(
        r#"
        WITH doomed AS (
            SELECT a.object_id, octet_length(a.asset_data)::bigint AS n
            FROM document_mm_assets a
            JOIN original_object_refs r ON r.asset_object_id = a.object_id
            WHERE r.verified_at IS NOT NULL
              AND a.asset_data IS NOT NULL
              AND ($1::uuid IS NULL OR a.workspace_id = $1)
            FOR UPDATE OF a
        )
        UPDATE document_mm_assets a
        SET asset_data = NULL, byte_location = 'object'
        FROM doomed
        WHERE a.object_id = doomed.object_id
        RETURNING doomed.n
        "#,
    )
    .bind(workspace)
    .fetch_all(pool)
    .await
    .map_err(map_db)?;
    let bytes: u64 = lengths.iter().map(|n| (*n).max(0) as u64).sum();
    Ok((lengths.len() as u64, bytes))
}

async fn null_markdown(
    pool: &PgPool,
    workspace: Option<Uuid>,
) -> Result<(u64, u64), OriginalsError> {
    let lengths = sqlx::query_scalar::<_, i64>(
        r#"
        WITH doomed AS (
            SELECT d.markdown_object_id,
                   octet_length(convert_to(d.markdown_content, 'UTF8'))::bigint AS n
            FROM pdf_documents d
            JOIN original_object_refs r ON r.markdown_object_id = d.markdown_object_id
            WHERE r.verified_at IS NOT NULL
              AND d.markdown_content IS NOT NULL
              AND ($1::uuid IS NULL OR d.workspace_id = $1)
            FOR UPDATE OF d
        )
        UPDATE pdf_documents d
        SET markdown_content = NULL, markdown_location = 'object'
        FROM doomed
        WHERE d.markdown_object_id = doomed.markdown_object_id
        RETURNING doomed.n
        "#,
    )
    .bind(workspace)
    .fetch_all(pool)
    .await
    .map_err(map_db)?;
    let bytes: u64 = lengths.iter().map(|n| (*n).max(0) as u64).sum();
    Ok((lengths.len() as u64, bytes))
}
