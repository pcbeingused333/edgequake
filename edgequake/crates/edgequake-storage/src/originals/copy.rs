//! Copy, verify, clear, and restore. Upload and this engine call `io`.

use std::time::Instant;

use uuid::Uuid;

use super::chunk::{chunk_digests, pack_digests, PDF_CHUNK_BYTES};
use super::config::StoreMode;
use super::format::{render_clear, render_migrate, render_plan, render_status};
use super::io::{
    get_inclusive, put_bytes, sha256_hex, sha256_object, verify_sha256, BlobStore, OriginalsError,
    Sse,
};
use super::key::{object_key, OwnerKind};
use super::ledger::{Claim, OriginalsLedger, Scope};

#[derive(Debug, Clone)]
pub struct RunOptions {
    pub prefix: String,
    pub sse: Sse,
    pub mode: StoreMode,
    pub scope: Scope,
    pub yes: bool,
    /// Parallel claim workers. One keeps the copy deterministic in tests.
    pub concurrency: usize,
}

#[derive(Debug, Clone)]
pub struct CopyReport {
    pub copied: u64,
    pub total: u64,
    pub verified: u64,
    pub skipped: u64,
    pub failed: u64,
    pub bytes: u64,
    pub elapsed: std::time::Duration,
    pub text: String,
}

fn key_for(prefix: &str, claim: &Claim) -> String {
    object_key(
        prefix,
        claim.kind,
        claim.tenant_id,
        claim.workspace_id,
        claim.owner_id,
    )
}

async fn copy_unreferenced(
    ledger: &dyn OriginalsLedger,
    blob: &dyn BlobStore,
    opts: &RunOptions,
    taken: &std::sync::atomic::AtomicU64,
    done: &std::sync::atomic::AtomicU64,
    on_copied: &(dyn Fn(u64) + Sync),
) -> Result<(u64, u64, u64), OriginalsError> {
    let mut copied = 0u64;
    let mut verified = 0u64;
    let mut bytes = 0u64;
    loop {
        let reserved = taken.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if opts.scope.limit.is_some_and(|n| reserved >= n) {
            taken.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
            break;
        }
        let Some(claim) = ledger.next_unreferenced(&opts.scope).await? else {
            taken.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
            break;
        };
        let key = key_for(&opts.prefix, &claim);
        let meta = match put_bytes(blob, &key, &claim.bytes, opts.sse).await {
            Ok(meta) => meta,
            Err(err) => {
                taken.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                ledger.release_claim(claim.kind, claim.owner_id).await?;
                return Err(match err {
                    OriginalsError::Credentials(message) => OriginalsError::Credentials(format!(
                        "{message}; next: edgequake originals copy"
                    )),
                    other => other,
                });
            }
        };
        let fresh = sha256_hex(&claim.bytes);
        let matches = verify_sha256(blob, &key, &fresh, claim.bytes.len() as i64).await?;
        ledger
            .insert_ref(
                &claim,
                &key,
                &fresh,
                claim.bytes.len() as i64,
                meta.etag,
                matches,
            )
            .await?;
        if claim.kind == OwnerKind::Pdf {
            let packed = pack_digests(&chunk_digests(&claim.bytes, PDF_CHUNK_BYTES));
            ledger.put_chunk_sha(claim.owner_id, &packed).await?;
        }
        copied += 1;
        bytes += claim.bytes.len() as u64;
        let n = done.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
        on_copied(n);
        if matches {
            verified += 1;
        } else {
            return Err(mismatch(&claim, &fresh, "object"));
        }
    }
    Ok((copied, verified, bytes))
}

pub async fn copy_owners(
    ledger: &dyn OriginalsLedger,
    blob: &dyn BlobStore,
    opts: &RunOptions,
) -> Result<CopyReport, OriginalsError> {
    copy_owners_with(ledger, blob, opts, &|_| {}).await
}

pub async fn copy_owners_with(
    ledger: &dyn OriginalsLedger,
    blob: &dyn BlobStore,
    opts: &RunOptions,
    on_copied: &(dyn Fn(u64) + Sync),
) -> Result<CopyReport, OriginalsError> {
    let started = Instant::now();
    let mut skipped = 0u64;
    let mut copied = 0u64;
    let mut verified = 0u64;
    let mut bytes = 0u64;
    let puts_before = blob.put_calls();

    loop {
        let Some((claim, reference)) = ledger.next_resume(&opts.scope).await? else {
            break;
        };
        match verify_sha256(
            blob,
            &reference.key,
            &reference.sha256,
            reference.byte_length,
        )
        .await
        {
            Ok(true) if reference.byte_length == claim.bytes.len() as i64 => {
                ledger.mark_verified(claim.kind, claim.owner_id).await?;
                skipped += 1;
                verified += 1;
            }
            Ok(_) => {
                return Err(mismatch(&claim, &reference.sha256, &claim.sha256));
            }
            Err(err) => return Err(err),
        }
    }

    let workers = opts.concurrency.max(1);
    let taken = std::sync::atomic::AtomicU64::new(0);
    let done = std::sync::atomic::AtomicU64::new(0);
    let (more_copied, more_verified, more_bytes) = if workers == 1 {
        copy_unreferenced(ledger, blob, opts, &taken, &done, on_copied).await?
    } else {
        let mut jobs = Vec::with_capacity(workers);
        for _ in 0..workers {
            jobs.push(copy_unreferenced(
                ledger, blob, opts, &taken, &done, on_copied,
            ));
        }
        let parts = futures::future::try_join_all(jobs).await?;
        parts.into_iter().fold((0u64, 0u64, 0u64), |acc, part| {
            (acc.0 + part.0, acc.1 + part.1, acc.2 + part.2)
        })
    };
    copied += more_copied;
    verified += more_verified;
    bytes += more_bytes;

    let _ = puts_before;
    let text = render_migrate(
        copied,
        copied + skipped,
        verified,
        skipped,
        0,
        bytes,
        started.elapsed(),
    );
    Ok(CopyReport {
        copied,
        total: copied + skipped,
        verified,
        skipped,
        failed: 0,
        bytes,
        elapsed: started.elapsed(),
        text,
    })
}

fn mismatch(claim: &Claim, source: &str, object: &str) -> OriginalsError {
    OriginalsError::Checksum(format!(
        "failed  kind={} owner={} sha256 source={} object={}; BYTEA for {} was not cleared",
        claim.kind.as_str(),
        claim.owner_id,
        source,
        object,
        claim.owner_id
    ))
}

pub async fn verify_owners(
    ledger: &dyn OriginalsLedger,
    blob: &dyn BlobStore,
    scope: &Scope,
) -> Result<u64, OriginalsError> {
    let mut ok = 0u64;
    let refs = ledger.refs(scope).await?;
    for reference in &refs {
        if reference.verified_at.is_some() {
            ok += 1;
            continue;
        }
        if verify_sha256(
            blob,
            &reference.key,
            &reference.sha256,
            reference.byte_length,
        )
        .await?
        {
            ledger
                .mark_verified(reference.kind, reference.owner_id)
                .await?;
            ok += 1;
        } else {
            return Err(OriginalsError::Checksum(format!(
                "failed  kind={} owner={} sha256 source={} object=mismatch",
                reference.kind.as_str(),
                reference.owner_id,
                reference.sha256
            )));
        }
    }
    fill_missing_pdf_windows(ledger, blob, &refs).await?;
    Ok(ok)
}

/// PDF copies from before `chunk_sha256` have a verified ref and a null digest.
/// Hash the object in 128 KB windows and store the pack. No second put.
async fn fill_missing_pdf_windows(
    ledger: &dyn OriginalsLedger,
    blob: &dyn BlobStore,
    refs: &[super::ledger::RefView],
) -> Result<(), OriginalsError> {
    for reference in refs {
        if reference.kind != OwnerKind::Pdf || reference.byte_length <= 0 {
            continue;
        }
        if ledger.chunk_sha_of(reference.owner_id).await?.is_some() {
            continue;
        }
        let total = reference.byte_length as u64;
        let mut digests = Vec::new();
        use sha2::Digest;
        let mut hasher = sha2::Sha256::new();
        let mut offset = 0u64;
        while offset < total {
            let end = offset.saturating_add(PDF_CHUNK_BYTES - 1).min(total - 1);
            let part = get_inclusive(blob, &reference.key, offset, end).await?;
            if part.len() as u64 != end - offset + 1 {
                return Err(OriginalsError::Checksum(format!(
                    "failed  kind=pdf owner={} sha256 source={} object=short; chunk digest was not written",
                    reference.owner_id, reference.sha256
                )));
            }
            hasher.update(&part);
            let window = chunk_digests(&part, PDF_CHUNK_BYTES);
            let Some(digest) = window.first() else {
                return Err(OriginalsError::Checksum(format!(
                    "failed  kind=pdf owner={} sha256 source={} object=empty; chunk digest was not written",
                    reference.owner_id, reference.sha256
                )));
            };
            if window.len() != 1 {
                return Err(OriginalsError::Checksum(format!(
                    "failed  kind=pdf owner={} sha256 source={} object=window; chunk digest was not written",
                    reference.owner_id, reference.sha256
                )));
            }
            digests.push(*digest);
            offset = end + 1;
        }
        let sha = format!("{:x}", hasher.finalize());
        if sha != reference.sha256 {
            return Err(OriginalsError::Checksum(format!(
                "failed  kind=pdf owner={} sha256 source={} object={}; chunk digest was not written",
                reference.owner_id, reference.sha256, sha
            )));
        }
        ledger
            .put_chunk_sha(reference.owner_id, &pack_digests(&digests))
            .await?;
    }
    Ok(())
}

pub async fn clear_owners(
    ledger: &dyn OriginalsLedger,
    blob: &dyn BlobStore,
    opts: &RunOptions,
) -> Result<String, OriginalsError> {
    if opts.mode != StoreMode::Object {
        return Err(OriginalsError::Mode(
            "clear while EDGEQUAKE_ORIGINAL_STORE is not object. Restart the server in object mode, then clear."
                .into(),
        ));
    }
    if !opts.yes {
        return Err(OriginalsError::Usage(
            "clear drops the database copy; pass --yes".into(),
        ));
    }
    let refs = ledger.refs(&opts.scope).await?;
    for reference in &refs {
        if reference.verified_at.is_none() {
            continue;
        }
        let object_sha = match sha256_object(blob, &reference.key, reference.byte_length).await {
            Ok(sha) => sha,
            Err(OriginalsError::Checksum(_)) => {
                return Err(OriginalsError::Checksum(format!(
                    "failed  kind={} owner={} sha256 source={} object=mismatch; BYTEA was not cleared",
                    reference.kind.as_str(),
                    reference.owner_id,
                    reference.sha256
                )));
            }
            Err(err) => return Err(err),
        };
        if object_sha != reference.sha256 {
            return Err(OriginalsError::Checksum(format!(
                "failed  kind={} owner={} sha256 source={} object={}; BYTEA was not cleared",
                reference.kind.as_str(),
                reference.owner_id,
                reference.sha256,
                object_sha
            )));
        }
    }
    if ledger.unverified_count(&opts.scope).await? > 0 {
        return Err(OriginalsError::Blocked(
            "clear saw unverified refs; next: edgequake originals verify".into(),
        ));
    }
    let view = ledger.clear_verified(&opts.scope).await?;
    Ok(render_clear(&view))
}

pub async fn restore_owners(
    ledger: &dyn OriginalsLedger,
    blob: &dyn BlobStore,
    scope: &Scope,
) -> Result<u64, OriginalsError> {
    let refs = ledger.refs(scope).await?;
    let mut restored = 0u64;
    for reference in refs {
        let object_sha = sha256_object(blob, &reference.key, reference.byte_length).await?;
        if object_sha != reference.sha256 {
            return Err(OriginalsError::Checksum(format!(
                "restore refused owner {} source={} object={}; nothing was written",
                reference.owner_id, reference.sha256, object_sha
            )));
        }
        let total = reference.byte_length.max(0) as u64;
        if total == 0 {
            ledger
                .write_restored_chunk(reference.kind, reference.owner_id, &[], true)
                .await?;
        } else {
            let mut offset = 0u64;
            let mut first = true;
            while offset < total {
                let end = offset.saturating_add(PDF_CHUNK_BYTES - 1).min(total - 1);
                let part = match get_inclusive(blob, &reference.key, offset, end).await {
                    Ok(part) => part,
                    Err(err) => {
                        if !first {
                            let _ = ledger
                                .write_restored_chunk(reference.kind, reference.owner_id, &[], true)
                                .await;
                        }
                        return Err(err);
                    }
                };
                if let Err(err) = ledger
                    .write_restored_chunk(reference.kind, reference.owner_id, &part, first)
                    .await
                {
                    if !first {
                        let _ = ledger
                            .write_restored_chunk(reference.kind, reference.owner_id, &[], true)
                            .await;
                    }
                    return Err(err);
                }
                first = false;
                offset = end + 1;
            }
        }
        restored += 1;
    }
    Ok(restored)
}

pub async fn plan_text(
    ledger: &dyn OriginalsLedger,
    scope: &Scope,
    database_url: &str,
    store_url: &str,
    mode: &str,
) -> Result<String, OriginalsError> {
    let view = ledger
        .plan_view(scope, database_url, store_url, mode)
        .await?;
    Ok(render_plan(&view))
}

pub async fn status_text(
    ledger: &dyn OriginalsLedger,
    scope: &Scope,
    database_url: &str,
    store_url: &str,
    mode: &str,
) -> Result<String, OriginalsError> {
    let view = ledger
        .plan_view(scope, database_url, store_url, mode)
        .await?;
    Ok(render_status(&view))
}

/// Shared by the server when a checksum conflict loses the race.
pub async fn delete_loser(blob: &dyn BlobStore, key: &str) -> Result<(), OriginalsError> {
    blob.delete_key(key).await?;
    Ok(())
}

pub fn prefer_object(client_present: bool, byte_location: &str, verified: bool) -> bool {
    if byte_location == "object" {
        return true;
    }
    client_present && verified
}

/// What a full read serves after it has decided to try the object.
/// A failed checksum with the database copy still present serves that copy.
/// A failed checksum with no database copy serves nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectServe {
    Object,
    Column,
    Refuse,
}

pub fn after_object_checksum(location: &str, column_present: bool, hash_ok: bool) -> ObjectServe {
    if hash_ok {
        ObjectServe::Object
    } else if location != "object" && column_present {
        ObjectServe::Column
    } else {
        ObjectServe::Refuse
    }
}

/// True when the database copy is still the bytes the ref names.
/// A later edit of the column makes the column authoritative.
pub fn column_matches_ref(column: &[u8], ref_sha: &str, ref_len: i64) -> bool {
    ref_len >= 0 && column.len() as i64 == ref_len && sha256_hex(column) == ref_sha
}

/// Placeholder so the upload path and the engine share the key helper.
pub fn upload_key(
    prefix: &str,
    kind: OwnerKind,
    tenant: Uuid,
    workspace: Uuid,
    owner: Uuid,
) -> String {
    object_key(prefix, kind, tenant, workspace, owner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::originals::io::{FailNth, RecordingBlob};
    use crate::originals::ledger::{KindFilter, MemoryLedger, OwnerRow};
    use std::sync::Arc;
    use uuid::Uuid;

    fn row(kind: OwnerKind, n: u128, bytes: &[u8]) -> OwnerRow {
        OwnerRow {
            kind,
            owner_id: Uuid::from_u128(n),
            tenant_id: Uuid::from_u128(10),
            workspace_id: Uuid::from_u128(20),
            filename: format!("../café {n}.pdf"),
            content_type: "application/pdf".into(),
            bytes: Some(bytes.to_vec()),
            content: format!("retrieval-{n}"),
            asset_len: 42,
        }
    }

    fn object_opts() -> RunOptions {
        RunOptions {
            prefix: "dev".into(),
            sse: Sse::Off,
            mode: StoreMode::Object,
            scope: Scope::default(),
            yes: true,
            concurrency: 1,
        }
    }

    #[tokio::test]
    async fn both_copies_stay_until_clear_and_download_uses_the_object() {
        let ledger = MemoryLedger::new(vec![row(OwnerKind::Pdf, 1, b"%PDF-hello")]);
        let blob = RecordingBlob::memory();
        let report = copy_owners(&ledger, &blob, &object_opts()).await.unwrap();
        assert_eq!(report.copied, 1);
        let bytes = ledger
            .bytea_of(OwnerKind::Pdf, Uuid::from_u128(1))
            .await
            .unwrap();
        assert_eq!(bytes.unwrap(), b"%PDF-hello");
        let reference = ledger.refs(&Scope::default()).await.unwrap();
        assert!(reference[0].verified_at.is_some());
        assert!(prefer_object(true, "database", true));
        assert_eq!(
            after_object_checksum("database", true, false),
            ObjectServe::Column
        );
        assert_eq!(
            after_object_checksum("object", false, false),
            ObjectServe::Refuse
        );
        assert_eq!(
            after_object_checksum("object", false, true),
            ObjectServe::Object
        );
        let sha = sha256_hex(b"%PDF-hello");
        assert!(column_matches_ref(b"%PDF-hello", &sha, 10));
        assert!(!column_matches_ref(b"%PDF-newer", &sha, 10));
        assert!(!column_matches_ref(b"%PDF-hello", &sha, 11));
        let object = blob.get_all(&reference[0].key).await.unwrap();
        assert_eq!(object.as_ref(), b"%PDF-hello");
        assert!(!reference[0].key.contains("café"));
    }

    #[tokio::test]
    async fn clear_refuses_a_flipped_object_and_keeps_bytea() {
        let owner = Uuid::from_u128(1);
        let ledger = MemoryLedger::new(vec![row(OwnerKind::Pdf, 1, b"%PDF-hello")]);
        let blob = RecordingBlob::memory();
        copy_owners(&ledger, &blob, &object_opts()).await.unwrap();
        let key = ledger.refs(&Scope::default()).await.unwrap()[0].key.clone();
        blob.put(&key, bytes::Bytes::from_static(b"%PDF-mutated"), &[])
            .await
            .unwrap();
        let before = ledger.bytea_of(OwnerKind::Pdf, owner).await.unwrap();
        let err = clear_owners(&ledger, &blob, &object_opts())
            .await
            .unwrap_err();
        assert_eq!(err.class_name(), "checksum");
        assert!(err.to_string().contains(&owner.to_string()));
        assert_eq!(
            ledger.bytea_of(OwnerKind::Pdf, owner).await.unwrap(),
            before
        );
    }

    #[tokio::test]
    async fn clear_in_database_mode_keeps_bytea() {
        let ledger = MemoryLedger::new(vec![row(OwnerKind::Pdf, 1, b"%PDF-hello")]);
        let blob = RecordingBlob::memory();
        copy_owners(&ledger, &blob, &object_opts()).await.unwrap();
        let mut opts = object_opts();
        opts.mode = StoreMode::Database;
        let err = clear_owners(&ledger, &blob, &opts).await.unwrap_err();
        assert_eq!(err.class_name(), "mode");
        assert!(ledger
            .bytea_of(OwnerKind::Pdf, Uuid::from_u128(1))
            .await
            .unwrap()
            .is_some());
    }

    #[tokio::test]
    async fn clear_without_verify_does_not_null_bytea() {
        let ledger = MemoryLedger::new(vec![row(OwnerKind::Pdf, 1, b"%PDF-hello")]);
        let blob = RecordingBlob::memory();
        copy_owners(&ledger, &blob, &object_opts()).await.unwrap();
        // Drop verified_at by inserting through the public mark path is one-way.
        // A second owner copied then unmarked is represented by a ref the
        // engine left unverified: simulate by clearing the flag via a fresh
        // ledger row that has a ref inserted unverified.
        let opts = object_opts();
        // Force an unverified ref by copying with a store that cannot be
        // re-read as a match is hard. Directly use insert after a manual claim.
        let claim = ledger.next_unreferenced(&Scope::default()).await.unwrap();
        assert!(claim.is_none(), "the copied owner is referenced");
        // Build an unverified sibling.
        let extra = MemoryLedger::new(vec![row(OwnerKind::Pdf, 7, b"%PDF-x")]);
        let claim = extra
            .next_unreferenced(&Scope::default())
            .await
            .unwrap()
            .unwrap();
        extra
            .insert_ref(
                &claim,
                "k",
                &claim.sha256,
                claim.bytes.len() as i64,
                None,
                false,
            )
            .await
            .unwrap();
        let err = clear_owners(&extra, &blob, &opts).await.unwrap_err();
        assert_eq!(err.class_name(), "blocked");
        assert!(extra
            .bytea_of(OwnerKind::Pdf, Uuid::from_u128(7))
            .await
            .unwrap()
            .is_some());
    }

    #[tokio::test]
    async fn clear_text_names_vacuum_full() {
        let ledger = MemoryLedger::new(vec![row(OwnerKind::Pdf, 1, b"%PDF-hello")]);
        let blob = RecordingBlob::memory();
        copy_owners(&ledger, &blob, &object_opts()).await.unwrap();
        let text = clear_owners(&ledger, &blob, &object_opts()).await.unwrap();
        assert!(text.contains("VACUUM FULL"));
        assert!(text.contains("pdf_document_blobs"));
        assert!(text.contains("document_originals"));
        assert!(ledger
            .bytea_of(OwnerKind::Pdf, Uuid::from_u128(1))
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn plan_and_clear_include_images_and_markdown_and_a_second_copy_does_not_put() {
        let png = [0x89, b'P', b'N', b'G', 1, 2, 3, 4];
        let ledger = MemoryLedger::new(vec![
            row(OwnerKind::Asset, 11, &png),
            row(OwnerKind::Markdown, 12, b"# page\n"),
        ]);
        let both = Scope {
            kind: KindFilter::Both,
            ..Scope::default()
        };
        assert!(ledger.next_unreferenced(&both).await.unwrap().is_none());
        let text = plan_text(
            &ledger,
            &Scope::default(),
            "postgres://localhost/edgequake",
            "s3://eq-originals/dev",
            "object",
        )
        .await
        .unwrap();
        assert!(text.contains("asset"));
        assert!(text.contains("markdown"));
        let blob = RecordingBlob::memory();
        let report = copy_owners(&ledger, &blob, &object_opts()).await.unwrap();
        assert_eq!(report.copied, 2);
        assert_eq!(blob.put_calls(), 2);
        for reference in ledger.refs(&Scope::default()).await.unwrap() {
            assert!(!reference.key.contains("café"));
            assert!(!reference.key.contains(".png"));
            assert!(!reference.key.contains(".md"));
        }
        let second = copy_owners(&ledger, &blob, &object_opts()).await.unwrap();
        assert_eq!(second.skipped, 2);
        assert_eq!(blob.put_calls(), 2);
        let cleared = clear_owners(&ledger, &blob, &object_opts()).await.unwrap();
        assert!(cleared.contains("asset"));
        assert!(cleared.contains("markdown"));
        assert!(ledger
            .bytea_of(OwnerKind::Asset, Uuid::from_u128(11))
            .await
            .unwrap()
            .is_none());
        assert!(ledger
            .bytea_of(OwnerKind::Markdown, Uuid::from_u128(12))
            .await
            .unwrap()
            .is_none());
        assert_eq!(
            ledger
                .content_of(OwnerKind::Asset, Uuid::from_u128(11))
                .await
                .unwrap(),
            "retrieval-11"
        );
    }

    #[tokio::test]
    async fn second_copy_skips_and_does_not_put_again() {
        let ledger = MemoryLedger::new(vec![row(OwnerKind::Pdf, 1, b"%PDF-hello")]);
        let blob = RecordingBlob::memory();
        copy_owners(&ledger, &blob, &object_opts()).await.unwrap();
        assert_eq!(blob.put_calls(), 1);
        let second = copy_owners(&ledger, &blob, &object_opts()).await.unwrap();
        assert_eq!(second.skipped, 1);
        assert_eq!(blob.put_calls(), 1);
    }

    #[tokio::test]
    async fn credential_failure_keeps_the_first_owner_verified() {
        let ledger = MemoryLedger::new(vec![
            row(OwnerKind::Pdf, 1, b"%PDF-one"),
            row(OwnerKind::Pdf, 2, b"%PDF-two"),
        ]);
        let blob = FailNth::new(2);
        let err = copy_owners(&ledger, &blob, &object_opts())
            .await
            .unwrap_err();
        assert_eq!(err.class_name(), "credentials");
        assert!(err.to_string().contains("edgequake originals copy"));
        let refs = ledger.refs(&Scope::default()).await.unwrap();
        assert_eq!(refs.len(), 1);
        assert!(refs[0].verified_at.is_some());
        assert_eq!(refs[0].owner_id, Uuid::from_u128(1));
        let again = copy_owners(&ledger, blob.inner(), &object_opts())
            .await
            .unwrap();
        assert_eq!(again.copied, 1);
        assert_eq!(again.skipped, 1);
        assert_eq!(ledger.refs(&Scope::default()).await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn restore_writes_bytea_and_a_bad_hash_writes_nothing() {
        let owner = Uuid::from_u128(1);
        let ledger = MemoryLedger::new(vec![row(OwnerKind::Pdf, 1, b"%PDF-hello")]);
        let blob = RecordingBlob::memory();
        copy_owners(&ledger, &blob, &object_opts()).await.unwrap();
        clear_owners(&ledger, &blob, &object_opts()).await.unwrap();
        assert!(ledger
            .bytea_of(OwnerKind::Pdf, owner)
            .await
            .unwrap()
            .is_none());
        let n = restore_owners(&ledger, &blob, &Scope::default())
            .await
            .unwrap();
        assert_eq!(n, 1);
        assert_eq!(
            ledger
                .bytea_of(OwnerKind::Pdf, owner)
                .await
                .unwrap()
                .unwrap(),
            b"%PDF-hello"
        );
        clear_owners(&ledger, &blob, &object_opts()).await.unwrap();
        let key = ledger.refs(&Scope::default()).await.unwrap()[0].key.clone();
        blob.put(&key, bytes::Bytes::from_static(b"nope"), &[])
            .await
            .unwrap();
        let err = restore_owners(&ledger, &blob, &Scope::default())
            .await
            .unwrap_err();
        assert_eq!(err.class_name(), "checksum");
        assert!(ledger
            .bytea_of(OwnerKind::Pdf, owner)
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn content_and_assets_survive_clear() {
        let owner = Uuid::from_u128(1);
        let ledger = MemoryLedger::new(vec![row(OwnerKind::Pdf, 1, b"%PDF-hello")]);
        let before = ledger.content_of(OwnerKind::Pdf, owner).await.unwrap();
        let assets = ledger.asset_len_of(OwnerKind::Pdf, owner).await.unwrap();
        let blob = RecordingBlob::memory();
        copy_owners(&ledger, &blob, &object_opts()).await.unwrap();
        clear_owners(&ledger, &blob, &object_opts()).await.unwrap();
        assert_eq!(
            ledger.content_of(OwnerKind::Pdf, owner).await.unwrap(),
            before
        );
        assert_eq!(
            ledger.asset_len_of(OwnerKind::Pdf, owner).await.unwrap(),
            assets
        );
        let sql = include_str!("pg.rs");
        let images = include_str!("../../../../migrations/217_spec172_mm_asset_objects.sql");
        assert!(sql.contains("document_mm_assets"));
        assert!(sql.contains("markdown_content"));
        assert!(images.contains("document_mm_assets"));
        assert!(images.contains("markdown_object_id"));
        assert!(images.contains("documents.content"));
    }

    #[tokio::test]
    async fn copy_stores_a_pdf_window_digest() {
        let owner = Uuid::from_u128(1);
        let body = b"%PDF-hello";
        let ledger = MemoryLedger::new(vec![row(OwnerKind::Pdf, 1, body)]);
        let blob = RecordingBlob::memory();
        copy_owners(&ledger, &blob, &object_opts()).await.unwrap();
        let packed = ledger.chunk_sha_of(owner).await.unwrap().unwrap();
        let digests = chunk_digests(body, PDF_CHUNK_BYTES);
        assert_eq!(packed, pack_digests(&digests));
        assert!(crate::originals::chunk_matches(
            body,
            crate::originals::digest_at(&packed, 0).unwrap()
        ));
        assert!(!crate::originals::chunk_matches(
            b"%PDF-mutated",
            crate::originals::digest_at(&packed, 0).unwrap()
        ));
    }

    #[tokio::test]
    async fn verify_fills_a_missing_pdf_digest_without_another_put() {
        let owner = Uuid::from_u128(1);
        let body = b"%PDF-hello";
        let ledger = MemoryLedger::new(vec![row(OwnerKind::Pdf, 1, body)]);
        let blob = RecordingBlob::memory();
        copy_owners(&ledger, &blob, &object_opts()).await.unwrap();
        ledger.forget_chunk(owner);
        assert!(ledger.chunk_sha_of(owner).await.unwrap().is_none());
        let puts = blob.put_calls();
        let checked = verify_owners(&ledger, &blob, &Scope::default())
            .await
            .unwrap();
        assert_eq!(checked, 1);
        assert_eq!(blob.put_calls(), puts);
        let packed = ledger.chunk_sha_of(owner).await.unwrap().unwrap();
        assert_eq!(packed, pack_digests(&chunk_digests(body, PDF_CHUNK_BYTES)));
    }

    #[tokio::test]
    async fn plan_prints_relation_size_and_free_space() {
        let ledger = MemoryLedger::new(vec![row(OwnerKind::Pdf, 1, b"%PDF-hello")]);
        let text = plan_text(
            &ledger,
            &Scope::default(),
            "postgres://edgequake:secret@localhost/edgequake",
            "s3://eq-originals/dev",
            "object",
        )
        .await
        .unwrap();
        assert!(text.contains("pg_total_relation_size"));
        assert!(text.contains("free space"));
        assert!(text.contains("VACUUM FULL"));
    }

    #[tokio::test]
    async fn two_tasks_put_each_owner_once() {
        let rows: Vec<_> = (1..=10)
            .map(|n| row(OwnerKind::Pdf, n, b"%PDF-row"))
            .collect();
        let ledger = Arc::new(MemoryLedger::new(rows));
        let blob = Arc::new(RecordingBlob::memory());
        let opts = object_opts();
        let left = {
            let ledger = Arc::clone(&ledger);
            let blob = Arc::clone(&blob);
            let opts = opts.clone();
            tokio::spawn(async move { copy_owners(ledger.as_ref(), blob.as_ref(), &opts).await })
        };
        let right = {
            let ledger = Arc::clone(&ledger);
            let blob = Arc::clone(&blob);
            tokio::spawn(async move { copy_owners(ledger.as_ref(), blob.as_ref(), &opts).await })
        };
        left.await.unwrap().unwrap();
        right.await.unwrap().unwrap();
        assert_eq!(ledger.refs(&Scope::default()).await.unwrap().len(), 10);
        assert_eq!(blob.put_calls(), 10);
    }

    #[tokio::test]
    async fn concurrency_flag_copies_every_owner_once() {
        let rows: Vec<_> = (1..=8)
            .map(|n| row(OwnerKind::Pdf, n, b"%PDF-row"))
            .collect();
        let ledger = MemoryLedger::new(rows);
        let blob = RecordingBlob::memory();
        let mut opts = object_opts();
        opts.concurrency = 4;
        let report = copy_owners(&ledger, &blob, &opts).await.unwrap();
        assert_eq!(report.copied, 8);
        assert_eq!(blob.put_calls(), 8);
        assert_eq!(ledger.refs(&Scope::default()).await.unwrap().len(), 8);
        let seen = std::sync::atomic::AtomicU64::new(0);
        let opts = object_opts();
        let ledger = MemoryLedger::new(
            (11..=12)
                .map(|n| row(OwnerKind::Pdf, n, b"%PDF-row"))
                .collect(),
        );
        let blob = RecordingBlob::memory();
        let report = copy_owners_with(&ledger, &blob, &opts, &|n| {
            seen.store(n, std::sync::atomic::Ordering::Relaxed);
        })
        .await
        .unwrap();
        assert_eq!(report.copied, 2);
        assert_eq!(seen.load(std::sync::atomic::Ordering::Relaxed), 2);
    }

    #[test]
    fn object_upload_during_claim_is_not_selected() {
        // An object-mode row has no BYTEA, so the claim predicate skips it.
        let mut object_row = row(OwnerKind::Pdf, 3, b"%PDF-new");
        object_row.bytes = None;
        let ledger = MemoryLedger::new(vec![object_row, row(OwnerKind::Pdf, 4, b"%PDF-old")]);
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let claim = ledger
                .next_unreferenced(&Scope::default())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(claim.owner_id, Uuid::from_u128(4));
            assert!(ledger
                .next_unreferenced(&Scope::default())
                .await
                .unwrap()
                .is_none());
        });
    }
}
