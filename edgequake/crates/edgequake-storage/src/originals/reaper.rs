//! Deletes queued keys only. It does not list the bucket (LAW-172-3).

use chrono::{DateTime, Duration, Utc};

use super::io::{BlobStore, OriginalsError};

/// Inflight rows younger than this stay. Older rows are abandoned uploads.
pub const INFLIGHT_TTL: Duration = Duration::hours(1);

#[derive(Debug, Clone)]
pub struct InflightRow {
    pub key: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReapStats {
    pub inflight_deleted: usize,
    pub queue_deleted: usize,
    pub failed: usize,
    /// Keys whose object delete succeeded. The caller drops only these rows.
    pub removed_inflight: Vec<String>,
    pub removed_queue: Vec<String>,
}

/// Keys whose inflight row is older than `ttl`. Younger keys are kept.
pub fn expired_inflight(rows: &[InflightRow], now: DateTime<Utc>, ttl: Duration) -> Vec<&str> {
    rows.iter()
        .filter(|row| now.signed_duration_since(row.created_at) >= ttl)
        .map(|row| row.key.as_str())
        .collect()
}

/// Delete the keys the caller already selected. `list_calls` on the store stays put.
///
/// One failed delete is counted and skipped. Later keys still run, and the
/// failed key is absent from `removed_*` so its queue row stays.
pub async fn reap(
    blob: &dyn BlobStore,
    expired_keys: &[&str],
    queued_deletes: &[&str],
) -> Result<ReapStats, OriginalsError> {
    let mut stats = ReapStats::default();
    for key in expired_keys {
        match blob.delete_key(key).await {
            Ok(()) => {
                stats.inflight_deleted += 1;
                stats.removed_inflight.push((*key).to_string());
            }
            Err(err) => {
                tracing::warn!(key, error = %err, "original inflight delete failed; row kept");
                stats.failed += 1;
            }
        }
    }
    for key in queued_deletes {
        match blob.delete_key(key).await {
            Ok(()) => {
                stats.queue_deleted += 1;
                stats.removed_queue.push((*key).to_string());
            }
            Err(err) => {
                tracing::warn!(key, error = %err, "original queue delete failed; row kept");
                stats.failed += 1;
            }
        }
    }
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::originals::io::RecordingBlob;
    use crate::originals::{put_bytes, Sse};

    #[test]
    fn young_inflight_is_kept_and_old_inflight_is_selected() {
        let now = Utc::now();
        let rows = vec![
            InflightRow {
                key: "young".into(),
                created_at: now - Duration::minutes(1),
            },
            InflightRow {
                key: "old".into(),
                created_at: now - Duration::hours(2),
            },
        ];
        let expired = expired_inflight(&rows, now, INFLIGHT_TTL);
        assert_eq!(expired, vec!["old"]);
    }

    #[tokio::test]
    async fn reaper_does_not_list() {
        let blob = RecordingBlob::memory();
        put_bytes(&blob, "old", b"abc", Sse::Off).await.unwrap();
        put_bytes(&blob, "queued", b"xyz", Sse::Off).await.unwrap();
        let before = blob.list_calls();
        let stats = reap(&blob, &["old"], &["queued"]).await.unwrap();
        assert_eq!(stats.inflight_deleted, 1);
        assert_eq!(stats.queue_deleted, 1);
        assert_eq!(blob.list_calls(), before);
        assert!(blob.get_all("old").await.is_err());
        assert!(blob.get_all("queued").await.is_err());
    }

    #[tokio::test]
    async fn one_failed_delete_keeps_that_key_and_continues() {
        use std::ops::Range;
        use std::sync::atomic::{AtomicUsize, Ordering};

        use async_trait::async_trait;
        use bytes::Bytes;

        use crate::originals::io::{BlobStore, PutMeta, StoreFault};

        struct FailFirst {
            inner: RecordingBlob,
            n: AtomicUsize,
        }

        #[async_trait]
        impl BlobStore for FailFirst {
            async fn put(
                &self,
                key: &str,
                body: Bytes,
                headers: &[(&str, &str)],
            ) -> Result<PutMeta, StoreFault> {
                self.inner.put(key, body, headers).await
            }

            async fn get_all(&self, key: &str) -> Result<Bytes, StoreFault> {
                self.inner.get_all(key).await
            }

            async fn get_range(&self, key: &str, range: Range<u64>) -> Result<Bytes, StoreFault> {
                self.inner.get_range(key, range).await
            }

            async fn delete_key(&self, key: &str) -> Result<(), StoreFault> {
                let i = self.n.fetch_add(1, Ordering::Relaxed);
                if i == 0 {
                    return Err(StoreFault::Other("injected delete failure".into()));
                }
                self.inner.delete_key(key).await
            }
        }

        let blob = FailFirst {
            inner: RecordingBlob::memory(),
            n: AtomicUsize::new(0),
        };
        put_bytes(&blob, "keep", b"aaa", Sse::Off).await.unwrap();
        put_bytes(&blob, "drop", b"bbb", Sse::Off).await.unwrap();
        let stats = reap(&blob, &[], &["keep", "drop"]).await.unwrap();
        assert_eq!(stats.failed, 1);
        assert_eq!(stats.removed_queue, vec!["drop".to_string()]);
        assert_eq!(stats.removed_inflight, Vec::<String>::new());
        assert!(blob.get_all("keep").await.is_ok());
        assert!(blob.get_all("drop").await.is_err());
    }
}
