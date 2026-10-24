//! In-memory ledger for the copy/verify/clear engine.
//!
//! Postgres uses the same engine through [`OriginalsLedger`]. The claim removes
//! the row from the available set before the caller PUTs, matching
//! `FOR UPDATE SKIP LOCKED` plus a commit before the network call.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::format::{ClearView, PlanView};
use super::io::OriginalsError;
use super::key::OwnerKind;

/// Which owners a verb touches.
///
/// `All` is the default and `--kind all`: pdf, original, image, and markdown.
/// `Both` is `--kind both`: pdf and original only, so older flags keep their meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KindFilter {
    #[default]
    All,
    Both,
    One(OwnerKind),
}

impl KindFilter {
    pub fn includes(self, kind: OwnerKind) -> bool {
        match self {
            Self::All => kind != OwnerKind::Upload,
            Self::Both => matches!(kind, OwnerKind::Pdf | OwnerKind::Original),
            Self::One(want) => want == kind,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Scope {
    pub kind: KindFilter,
    pub workspace: Option<Uuid>,
    pub limit: Option<u64>,
}

impl Default for Scope {
    fn default() -> Self {
        Self {
            kind: KindFilter::All,
            workspace: None,
            limit: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OwnerRow {
    pub kind: OwnerKind,
    pub owner_id: Uuid,
    pub tenant_id: Uuid,
    pub workspace_id: Uuid,
    pub filename: String,
    pub content_type: String,
    pub bytes: Option<Vec<u8>>,
    /// Retrieval text. Copy and clear do not write this field.
    pub content: String,
    /// `octet_length(document_mm_assets.asset_data)`. The engine does not write it.
    pub asset_len: i64,
}

#[derive(Debug, Clone)]
pub struct RefView {
    pub kind: OwnerKind,
    pub owner_id: Uuid,
    pub key: String,
    pub sha256: String,
    pub byte_length: i64,
    pub verified_at: Option<DateTime<Utc>>,
    pub etag: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Claim {
    pub kind: OwnerKind,
    pub owner_id: Uuid,
    pub tenant_id: Uuid,
    pub workspace_id: Uuid,
    pub filename: String,
    pub content_type: String,
    pub bytes: Vec<u8>,
    pub sha256: String,
}

struct State {
    rows: Vec<OwnerRow>,
    refs: Vec<RefView>,
    /// Owners handed to a caller that has not inserted a ref yet.
    claimed: HashSet<(OwnerKind, Uuid)>,
    /// Resume pass already considered this ref.
    resume_seen: HashSet<(OwnerKind, Uuid)>,
    /// Packed PDF window digests, keyed by the PDF owner id.
    chunks: HashMap<Uuid, Vec<u8>>,
}

pub struct MemoryLedger {
    inner: Mutex<State>,
    pub pdf_relation_bytes: u64,
    pub original_relation_bytes: u64,
    pub asset_relation_bytes: u64,
    pub markdown_relation_bytes: u64,
}

impl MemoryLedger {
    pub fn new(rows: Vec<OwnerRow>) -> Self {
        let pdf_relation_bytes = rows
            .iter()
            .filter(|r| r.kind == OwnerKind::Pdf)
            .map(|r| r.bytes.as_ref().map(|b| b.len()).unwrap_or(0) as u64)
            .sum();
        let original_relation_bytes = rows
            .iter()
            .filter(|r| r.kind == OwnerKind::Original)
            .map(|r| r.bytes.as_ref().map(|b| b.len()).unwrap_or(0) as u64)
            .sum();
        let asset_relation_bytes = rows
            .iter()
            .filter(|r| r.kind == OwnerKind::Asset)
            .map(|r| r.bytes.as_ref().map(|b| b.len()).unwrap_or(0) as u64)
            .sum();
        let markdown_relation_bytes = rows
            .iter()
            .filter(|r| r.kind == OwnerKind::Markdown)
            .map(|r| r.bytes.as_ref().map(|b| b.len()).unwrap_or(0) as u64)
            .sum();
        Self {
            inner: Mutex::new(State {
                rows,
                refs: Vec::new(),
                claimed: HashSet::new(),
                resume_seen: HashSet::new(),
                chunks: HashMap::new(),
            }),
            pdf_relation_bytes,
            original_relation_bytes,
            asset_relation_bytes,
            markdown_relation_bytes,
        }
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, State>, OriginalsError> {
        self.inner
            .lock()
            .map_err(|_| OriginalsError::Database("ledger lock poisoned".into()))
    }
}

#[async_trait]
pub trait OriginalsLedger: Send + Sync {
    async fn plan_view(
        &self,
        scope: &Scope,
        database_url: &str,
        store_url: &str,
        mode: &str,
    ) -> Result<PlanView, OriginalsError>;
    async fn next_unreferenced(&self, scope: &Scope) -> Result<Option<Claim>, OriginalsError>;
    async fn next_resume(&self, scope: &Scope) -> Result<Option<(Claim, RefView)>, OriginalsError>;
    async fn insert_ref(
        &self,
        claim: &Claim,
        key: &str,
        sha256: &str,
        byte_length: i64,
        etag: Option<String>,
        verified: bool,
    ) -> Result<(), OriginalsError>;
    async fn release_claim(&self, kind: OwnerKind, owner: Uuid) -> Result<(), OriginalsError>;
    async fn mark_verified(&self, kind: OwnerKind, owner: Uuid) -> Result<(), OriginalsError>;
    async fn refs(&self, scope: &Scope) -> Result<Vec<RefView>, OriginalsError>;
    async fn clear_verified(&self, scope: &Scope) -> Result<ClearView, OriginalsError>;
    async fn unverified_count(&self, scope: &Scope) -> Result<u64, OriginalsError>;
    async fn write_restored(
        &self,
        kind: OwnerKind,
        owner: Uuid,
        bytes: &[u8],
    ) -> Result<(), OriginalsError>;
    async fn bytea_of(
        &self,
        kind: OwnerKind,
        owner: Uuid,
    ) -> Result<Option<Vec<u8>>, OriginalsError>;
    async fn content_of(&self, kind: OwnerKind, owner: Uuid) -> Result<String, OriginalsError>;
    async fn asset_len_of(&self, kind: OwnerKind, owner: Uuid) -> Result<i64, OriginalsError>;
    async fn put_chunk_sha(&self, owner: Uuid, packed: &[u8]) -> Result<(), OriginalsError>;
    async fn chunk_sha_of(&self, owner: Uuid) -> Result<Option<Vec<u8>>, OriginalsError>;
    /// `first` replaces the column. Later calls append. An empty `first` chunk clears it.
    async fn write_restored_chunk(
        &self,
        kind: OwnerKind,
        owner: Uuid,
        chunk: &[u8],
        first: bool,
    ) -> Result<(), OriginalsError>;
}

fn in_scope(scope: &Scope, kind: OwnerKind, workspace: Uuid) -> bool {
    if !scope.kind.includes(kind) {
        return false;
    }
    if let Some(ws) = scope.workspace {
        if ws != workspace {
            return false;
        }
    }
    true
}

fn claim_from(row: &OwnerRow) -> Result<Claim, OriginalsError> {
    let bytes = row
        .bytes
        .clone()
        .ok_or_else(|| OriginalsError::Database(format!("owner {} has no BYTEA", row.owner_id)))?;
    Ok(Claim {
        kind: row.kind,
        owner_id: row.owner_id,
        tenant_id: row.tenant_id,
        workspace_id: row.workspace_id,
        filename: row.filename.clone(),
        content_type: row.content_type.clone(),
        sha256: super::io::sha256_hex(&bytes),
        bytes,
    })
}

#[async_trait]
impl OriginalsLedger for MemoryLedger {
    async fn plan_view(
        &self,
        scope: &Scope,
        database_url: &str,
        store_url: &str,
        mode: &str,
    ) -> Result<PlanView, OriginalsError> {
        let state = self.lock()?;
        let mut pdf_rows = 0u64;
        let mut pdf_bytes = 0u64;
        let mut original_rows = 0u64;
        let mut original_bytes = 0u64;
        let mut asset_rows = 0u64;
        let mut asset_bytes = 0u64;
        let mut markdown_rows = 0u64;
        let mut markdown_bytes = 0u64;
        for row in &state.rows {
            if !in_scope(scope, row.kind, row.workspace_id) {
                continue;
            }
            let len = row.bytes.as_ref().map(|b| b.len() as u64).unwrap_or(0);
            if len == 0 && row.bytes.is_none() {
                continue;
            }
            match row.kind {
                OwnerKind::Pdf => {
                    pdf_rows += 1;
                    pdf_bytes += len;
                }
                OwnerKind::Original => {
                    original_rows += 1;
                    original_bytes += len;
                }
                OwnerKind::Asset => {
                    asset_rows += 1;
                    asset_bytes += len;
                }
                OwnerKind::Markdown => {
                    markdown_rows += 1;
                    markdown_bytes += len;
                }
                OwnerKind::Upload => {}
            }
        }
        let pdf_verified = state
            .refs
            .iter()
            .filter(|r| r.kind == OwnerKind::Pdf && r.verified_at.is_some())
            .count() as u64;
        let original_verified = state
            .refs
            .iter()
            .filter(|r| r.kind == OwnerKind::Original && r.verified_at.is_some())
            .count() as u64;
        let asset_verified = state
            .refs
            .iter()
            .filter(|r| r.kind == OwnerKind::Asset && r.verified_at.is_some())
            .count() as u64;
        let markdown_verified = state
            .refs
            .iter()
            .filter(|r| r.kind == OwnerKind::Markdown && r.verified_at.is_some())
            .count() as u64;
        let blocked = state
            .refs
            .iter()
            .filter(|r| {
                r.verified_at.is_none() && in_scope(scope, r.kind, Uuid::nil())
                    || r.verified_at.is_none()
            })
            .count() as u64;
        let _ = in_scope;
        Ok(PlanView {
            database_url: database_url.to_string(),
            store_url: store_url.to_string(),
            mode: mode.to_string(),
            role: "edgequake_originals_maint".into(),
            pdf_rows,
            pdf_bytes,
            pdf_verified,
            original_rows,
            original_bytes,
            original_verified,
            asset_rows,
            asset_bytes,
            asset_verified,
            markdown_rows,
            markdown_bytes,
            markdown_verified,
            pdf_relation_bytes: self.pdf_relation_bytes,
            original_relation_bytes: self.original_relation_bytes,
            asset_relation_bytes: self.asset_relation_bytes,
            markdown_relation_bytes: self.markdown_relation_bytes,
            blocked,
            next: "edgequake originals migrate".into(),
        })
    }

    async fn next_unreferenced(&self, scope: &Scope) -> Result<Option<Claim>, OriginalsError> {
        let mut state = self.lock()?;
        let found = state.rows.iter().find(|row| {
            row.bytes.is_some()
                && in_scope(scope, row.kind, row.workspace_id)
                && !state.claimed.contains(&(row.kind, row.owner_id))
                && !state
                    .refs
                    .iter()
                    .any(|r| r.kind == row.kind && r.owner_id == row.owner_id)
        });
        let Some(row) = found else {
            return Ok(None);
        };
        let claim = claim_from(row)?;
        state.claimed.insert((claim.kind, claim.owner_id));
        Ok(Some(claim))
    }

    async fn next_resume(&self, scope: &Scope) -> Result<Option<(Claim, RefView)>, OriginalsError> {
        let mut state = self.lock()?;
        let found = state.rows.iter().find_map(|row| {
            if row.bytes.is_none() || !in_scope(scope, row.kind, row.workspace_id) {
                return None;
            }
            if state.resume_seen.contains(&(row.kind, row.owner_id)) {
                return None;
            }
            let reference = state
                .refs
                .iter()
                .find(|r| r.kind == row.kind && r.owner_id == row.owner_id)?
                .clone();
            Some((row.kind, row.owner_id, reference))
        });
        let Some((kind, owner, reference)) = found else {
            return Ok(None);
        };
        state.resume_seen.insert((kind, owner));
        let row = state
            .rows
            .iter()
            .find(|r| r.kind == kind && r.owner_id == owner)
            .expect("row present");
        Ok(Some((claim_from(row)?, reference)))
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
        let mut state = self.lock()?;
        if state
            .refs
            .iter()
            .any(|r| r.kind == claim.kind && r.owner_id == claim.owner_id)
        {
            return Err(OriginalsError::Database(format!(
                "ref already exists for {}",
                claim.owner_id
            )));
        }
        state.refs.push(RefView {
            kind: claim.kind,
            owner_id: claim.owner_id,
            key: key.to_string(),
            sha256: sha256.to_string(),
            byte_length,
            verified_at: verified.then(Utc::now),
            etag,
        });
        state.claimed.remove(&(claim.kind, claim.owner_id));
        Ok(())
    }

    async fn release_claim(&self, kind: OwnerKind, owner: Uuid) -> Result<(), OriginalsError> {
        self.lock()?.claimed.remove(&(kind, owner));
        Ok(())
    }

    async fn mark_verified(&self, kind: OwnerKind, owner: Uuid) -> Result<(), OriginalsError> {
        let mut state = self.lock()?;
        let row = state
            .refs
            .iter_mut()
            .find(|r| r.kind == kind && r.owner_id == owner)
            .ok_or_else(|| OriginalsError::Database(format!("no ref for {owner}")))?;
        row.verified_at = Some(Utc::now());
        Ok(())
    }

    async fn refs(&self, scope: &Scope) -> Result<Vec<RefView>, OriginalsError> {
        let state = self.lock()?;
        Ok(state
            .refs
            .iter()
            .filter(|r| scope.kind.includes(r.kind))
            .cloned()
            .collect())
    }

    async fn clear_verified(&self, scope: &Scope) -> Result<ClearView, OriginalsError> {
        let mut state = self.lock()?;
        let mut pdf = 0u64;
        let mut original = 0u64;
        let mut asset = 0u64;
        let mut markdown = 0u64;
        let mut bytes = 0u64;
        let verified: Vec<(OwnerKind, Uuid)> = state
            .refs
            .iter()
            .filter(|r| r.verified_at.is_some())
            .map(|r| (r.kind, r.owner_id))
            .collect();
        for (kind, owner) in verified {
            let Some(row) = state.rows.iter_mut().find(|r| {
                r.kind == kind && r.owner_id == owner && in_scope(scope, r.kind, r.workspace_id)
            }) else {
                continue;
            };
            let len = row.bytes.as_ref().map(|b| b.len() as u64).unwrap_or(0);
            row.bytes = None;
            bytes += len;
            match kind {
                OwnerKind::Pdf => pdf += 1,
                OwnerKind::Original => original += 1,
                OwnerKind::Asset => asset += 1,
                OwnerKind::Markdown => markdown += 1,
                OwnerKind::Upload => {}
            }
        }
        Ok(ClearView {
            pdf,
            original,
            asset,
            markdown,
            bytes,
            pdf_relation_bytes: self.pdf_relation_bytes,
            original_relation_bytes: self.original_relation_bytes,
            asset_relation_bytes: self.asset_relation_bytes,
            markdown_relation_bytes: self.markdown_relation_bytes,
        })
    }

    async fn unverified_count(&self, scope: &Scope) -> Result<u64, OriginalsError> {
        let state = self.lock()?;
        Ok(state
            .refs
            .iter()
            .filter(|r| r.verified_at.is_none() && scope.kind.includes(r.kind))
            .count() as u64)
    }

    async fn write_restored(
        &self,
        kind: OwnerKind,
        owner: Uuid,
        bytes: &[u8],
    ) -> Result<(), OriginalsError> {
        let mut state = self.lock()?;
        let row = state
            .rows
            .iter_mut()
            .find(|r| r.kind == kind && r.owner_id == owner)
            .ok_or_else(|| OriginalsError::Database(format!("no owner {owner}")))?;
        row.bytes = Some(bytes.to_vec());
        Ok(())
    }

    async fn bytea_of(
        &self,
        kind: OwnerKind,
        owner: Uuid,
    ) -> Result<Option<Vec<u8>>, OriginalsError> {
        let state = self.lock()?;
        Ok(state
            .rows
            .iter()
            .find(|r| r.kind == kind && r.owner_id == owner)
            .and_then(|r| r.bytes.clone()))
    }

    async fn content_of(&self, kind: OwnerKind, owner: Uuid) -> Result<String, OriginalsError> {
        let state = self.lock()?;
        state
            .rows
            .iter()
            .find(|r| r.kind == kind && r.owner_id == owner)
            .map(|r| r.content.clone())
            .ok_or_else(|| OriginalsError::Database(format!("no owner {owner}")))
    }

    async fn asset_len_of(&self, kind: OwnerKind, owner: Uuid) -> Result<i64, OriginalsError> {
        let state = self.lock()?;
        state
            .rows
            .iter()
            .find(|r| r.kind == kind && r.owner_id == owner)
            .map(|r| r.asset_len)
            .ok_or_else(|| OriginalsError::Database(format!("no owner {owner}")))
    }

    async fn put_chunk_sha(&self, owner: Uuid, packed: &[u8]) -> Result<(), OriginalsError> {
        self.lock()?.chunks.insert(owner, packed.to_vec());
        Ok(())
    }

    async fn chunk_sha_of(&self, owner: Uuid) -> Result<Option<Vec<u8>>, OriginalsError> {
        Ok(self.lock()?.chunks.get(&owner).cloned())
    }

    async fn write_restored_chunk(
        &self,
        kind: OwnerKind,
        owner: Uuid,
        chunk: &[u8],
        first: bool,
    ) -> Result<(), OriginalsError> {
        let mut state = self.lock()?;
        let row = state
            .rows
            .iter_mut()
            .find(|r| r.kind == kind && r.owner_id == owner)
            .ok_or_else(|| OriginalsError::Database(format!("no owner {owner}")))?;
        if first {
            row.bytes = if chunk.is_empty() {
                None
            } else {
                Some(chunk.to_vec())
            };
        } else if let Some(existing) = row.bytes.as_mut() {
            existing.extend_from_slice(chunk);
        } else {
            row.bytes = Some(chunk.to_vec());
        }
        Ok(())
    }
}

impl MemoryLedger {
    /// Drop a stored window pack so verify can fill it again.
    pub fn forget_chunk(&self, owner: Uuid) {
        if let Ok(mut state) = self.lock() {
            state.chunks.remove(&owner);
        }
    }
}
