//! Put, ranged get, SHA-256, and delete. One implementation for every backend.

use async_trait::async_trait;
use bytes::Bytes;
use futures::StreamExt;
use object_store::path::Path;
use object_store::{GetOptions, GetRange, ObjectStore, ObjectStoreExt, PutPayload};
use std::ops::Range;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use uuid::Uuid;

use super::cap::max_upload_bytes;
use super::key::{object_key, OwnerKind};

/// Objects at or under this size use one `put`. Above it, multipart.
pub const SINGLE_PUT_MAX: u64 = 100 * 1024 * 1024;
/// S3 rejects a non-final multipart part smaller than this. GCS and Azure use the same floor.
pub const MIN_MULTIPART_PART: u64 = 5 * 1024 * 1024;
/// Window used to hash an object larger than the 50 MiB product cap.
const STREAM_CHUNK: u64 = 8 * 1024 * 1024;
/// In-flight requests against one bucket. Copy workers and multipart parts share it.
pub const STORE_REQUEST_LIMIT: usize = 8;
/// Multipart parts sent together. The store limit still applies on top.
const PARTS_IN_FLIGHT: usize = 4;

/// Server-side encryption EdgeQuake will ask for. KMS is a later spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sse {
    Off,
    Aes256,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PutMeta {
    pub etag: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum OriginalsError {
    #[error("config: {0}")]
    Config(String),
    #[error("credentials: {0}")]
    Credentials(String),
    #[error("checksum: {0}")]
    Checksum(String),
    #[error("mode: {0}")]
    Mode(String),
    #[error("blocked: {0}")]
    Blocked(String),
    #[error("database: {0}")]
    Database(String),
    #[error("usage: {0}")]
    Usage(String),
    #[error("store: {0}")]
    Store(String),
    #[error("not supported: {0}")]
    NotSupported(String),
}

impl OriginalsError {
    pub fn class_name(&self) -> &'static str {
        match self {
            Self::Config(_) => "config",
            Self::Credentials(_) => "credentials",
            Self::Checksum(_) => "checksum",
            Self::Mode(_) => "mode",
            Self::Blocked(_) => "blocked",
            Self::Database(_) => "database",
            Self::Usage(_) => "usage",
            Self::Store(_) => "store",
            Self::NotSupported(_) => "not_supported",
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Usage(_) => 2,
            _ => 1,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StoreFault {
    #[error("credentials: {0}")]
    Credentials(String),
    #[error("{0}")]
    Other(String),
}

impl From<StoreFault> for OriginalsError {
    fn from(value: StoreFault) -> Self {
        match value {
            StoreFault::Credentials(m) => Self::Credentials(m),
            StoreFault::Other(m) => classify_store_message(&m),
        }
    }
}

fn classify_store_message(message: &str) -> OriginalsError {
    let lower = message.to_ascii_lowercase();
    if lower.contains("accessdenied")
        || lower.contains("invalidaccesskey")
        || lower.contains("expiredtoken")
        || lower.contains("signaturedoesnotmatch")
        || lower.contains("credentials")
    {
        OriginalsError::Credentials(message.to_string())
    } else {
        OriginalsError::Store(message.to_string())
    }
}

/// How a payload is written. 50 MiB is one `put`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteStrategy {
    Single,
    Multipart,
}

pub fn write_strategy(len: u64) -> WriteStrategy {
    if len <= SINGLE_PUT_MAX {
        WriteStrategy::Single
    } else {
        WriteStrategy::Multipart
    }
}

/// Non-final parts must be at least 5 MiB. Checked before any request.
pub fn reject_short_parts(part_lens: &[u64]) -> Result<(), OriginalsError> {
    if part_lens.len() <= 1 {
        return Ok(());
    }
    let last = part_lens.len() - 1;
    for (index, len) in part_lens.iter().enumerate() {
        if index != last && *len < MIN_MULTIPART_PART {
            return Err(OriginalsError::Config(format!(
                "multipart part {index} is {len} bytes; a non-final part must be at least {MIN_MULTIPART_PART} bytes"
            )));
        }
    }
    Ok(())
}

pub fn sse_headers(sse: Sse) -> Vec<(&'static str, &'static str)> {
    match sse {
        Sse::Off => Vec::new(),
        Sse::Aes256 => vec![("x-amz-server-side-encryption", "AES256")],
    }
}

/// Inclusive window used by `get_pdf_bytes_range` (`start..=end`) to the
/// half-open range `object_store` expects.
pub fn inclusive_to_get_range(start: u64, end: u64) -> Result<Range<u64>, OriginalsError> {
    if end < start {
        return Err(OriginalsError::Config(format!(
            "range end {end} is before start {start}"
        )));
    }
    let end_exclusive = end.checked_add(1).ok_or_else(|| {
        OriginalsError::Config("range end does not fit in a half-open bound".into())
    })?;
    Ok(start..end_exclusive)
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}

/// The byte store the upload path, the reaper, and the CLI share.
#[async_trait]
pub trait BlobStore: Send + Sync {
    async fn put(
        &self,
        key: &str,
        body: Bytes,
        headers: &[(&str, &str)],
    ) -> Result<PutMeta, StoreFault>;
    async fn get_all(&self, key: &str) -> Result<Bytes, StoreFault>;
    async fn get_range(&self, key: &str, range: Range<u64>) -> Result<Bytes, StoreFault>;
    async fn delete_key(&self, key: &str) -> Result<(), StoreFault>;
    /// Size of the stored object. The default reads the body; the cloud client uses HEAD.
    async fn content_length(&self, key: &str) -> Result<u64, StoreFault> {
        let bytes = self.get_all(key).await?;
        Ok(bytes.len() as u64)
    }
    /// Upload `parts` in order. Every part except the last must already be at least 5 MiB.
    async fn put_multipart(
        &self,
        key: &str,
        parts: &[Bytes],
        headers: &[(&str, &str)],
    ) -> Result<PutMeta, StoreFault> {
        let _ = (key, parts, headers);
        Err(StoreFault::Other(
            "this store has no multipart upload".into(),
        ))
    }
    fn list_calls(&self) -> u64 {
        0
    }
    fn put_calls(&self) -> u64 {
        0
    }
    fn multipart_calls(&self) -> u64 {
        0
    }

    /// Presign capability, when this backend can sign a PUT. Memory stores return `None`.
    fn signer(&self) -> Option<std::sync::Arc<dyn object_store::signer::Signer>> {
        None
    }

    /// Header the backend checks as a SHA-256 checksum on PUT. GCS signs `content-type` only.
    fn checksum_header(&self) -> Option<&'static str> {
        None
    }

    fn allow_http_presign(&self) -> bool {
        false
    }
}

/// `object_store` backend. `probe` is the only list. The reaper does not call it.
pub struct ObjectBlob {
    store: Arc<dyn ObjectStore>,
    /// URL prefix. Stored keys already include it. An empty prefix is the
    /// in-memory test client and allows any key `Path` accepts.
    prefix: String,
    list_calls: AtomicU64,
    put_calls: AtomicU64,
    multipart_calls: AtomicU64,
    signer: Option<Arc<dyn object_store::signer::Signer>>,
    checksum_header: Option<&'static str>,
    allow_http_presign: bool,
}

impl ObjectBlob {
    pub fn new(store: Arc<dyn ObjectStore>) -> Self {
        Self::prefixed(store, "")
    }

    pub fn prefixed(store: Arc<dyn ObjectStore>, prefix: impl Into<String>) -> Self {
        Self {
            store,
            prefix: prefix.into(),
            list_calls: AtomicU64::new(0),
            put_calls: AtomicU64::new(0),
            multipart_calls: AtomicU64::new(0),
            signer: None,
            checksum_header: None,
            allow_http_presign: false,
        }
    }

    pub fn with_signer(
        mut self,
        signer: Arc<dyn object_store::signer::Signer>,
        checksum_header: Option<&'static str>,
        allow_http_presign: bool,
    ) -> Self {
        self.signer = Some(signer);
        self.checksum_header = checksum_header;
        self.allow_http_presign = allow_http_presign;
        self
    }

    fn require_key(&self, key: &str) -> Result<(), StoreFault> {
        let prefix = self.prefix.trim_matches('/');
        if prefix.is_empty() || key.starts_with(&format!("{prefix}/")) {
            return Ok(());
        }
        Err(StoreFault::Other(
            "object key is outside the configured prefix".into(),
        ))
    }

    fn require_probe(&self, prefix: &str) -> bool {
        let configured = self.prefix.trim_matches('/');
        if configured.is_empty() {
            return true;
        }
        let asked = prefix.trim_matches('/');
        asked == configured || asked.starts_with(&format!("{configured}/"))
    }

    /// One list of `prefix`, stopped after the first name. An empty prefix
    /// lists the bucket. A sibling prefix is not requested.
    pub async fn probe(&self, prefix: &str) -> bool {
        if !self.require_probe(prefix) {
            tracing::warn!("original store probe is outside the configured prefix");
            return false;
        }
        let path = match probe_prefix(prefix) {
            Ok(path) => path,
            Err(err) => {
                tracing::warn!(error = %err, "original store prefix is not a valid object path");
                return false;
            }
        };
        self.list_calls.fetch_add(1, Ordering::Relaxed);
        match self.store.list(path.as_ref()).next().await {
            None => true,
            Some(Ok(_)) => true,
            Some(Err(err)) => {
                tracing::warn!(error = %err, "original store probe failed");
                false
            }
        }
    }
}

fn probe_prefix(prefix: &str) -> Result<Option<Path>, StoreFault> {
    let trimmed = prefix.trim_matches('/');
    if trimmed.is_empty() {
        Ok(None)
    } else {
        Path::parse(trimmed)
            .map(Some)
            .map_err(|err| StoreFault::Other(format!("object prefix: {err}")))
    }
}

fn path_for(key: &str) -> Result<Path, StoreFault> {
    Path::parse(key).map_err(|err| StoreFault::Other(format!("object key: {err}")))
}

#[async_trait]
impl BlobStore for ObjectBlob {
    async fn put(
        &self,
        key: &str,
        body: Bytes,
        _headers: &[(&str, &str)],
    ) -> Result<PutMeta, StoreFault> {
        self.require_key(key)?;
        self.put_calls.fetch_add(1, Ordering::Relaxed);
        let path = path_for(key)?;
        let result = self
            .store
            .put(&path, PutPayload::from(body))
            .await
            .map_err(|err| StoreFault::Other(err.to_string()))?;
        Ok(PutMeta { etag: result.e_tag })
    }

    async fn put_multipart(
        &self,
        key: &str,
        parts: &[Bytes],
        _headers: &[(&str, &str)],
    ) -> Result<PutMeta, StoreFault> {
        if parts.is_empty() {
            return Err(StoreFault::Other("multipart upload has no parts".into()));
        }
        self.require_key(key)?;
        self.multipart_calls.fetch_add(1, Ordering::Relaxed);
        let path = path_for(key)?;
        let mut upload = self
            .store
            .put_multipart(&path)
            .await
            .map_err(|err| StoreFault::Other(err.to_string()))?;
        let mut inflight = Vec::with_capacity(PARTS_IN_FLIGHT);
        for part in parts {
            inflight.push(upload.put_part(PutPayload::from(part.clone())));
            if inflight.len() == PARTS_IN_FLIGHT {
                if let Err(err) = futures::future::try_join_all(inflight.drain(..)).await {
                    let _ = upload.abort().await;
                    return Err(StoreFault::Other(err.to_string()));
                }
            }
        }
        if let Err(err) = futures::future::try_join_all(inflight).await {
            let _ = upload.abort().await;
            return Err(StoreFault::Other(err.to_string()));
        }
        match upload.complete().await {
            Ok(result) => Ok(PutMeta { etag: result.e_tag }),
            Err(err) => {
                let _ = upload.abort().await;
                Err(StoreFault::Other(err.to_string()))
            }
        }
    }

    async fn get_all(&self, key: &str) -> Result<Bytes, StoreFault> {
        self.require_key(key)?;
        let path = path_for(key)?;
        let got = self
            .store
            .get(&path)
            .await
            .map_err(|err| StoreFault::Other(err.to_string()))?;
        got.bytes()
            .await
            .map_err(|err| StoreFault::Other(err.to_string()))
    }

    async fn get_range(&self, key: &str, range: Range<u64>) -> Result<Bytes, StoreFault> {
        self.require_key(key)?;
        let path = path_for(key)?;
        let opts = GetOptions {
            range: Some(GetRange::Bounded(range)),
            ..Default::default()
        };
        let got = self
            .store
            .get_opts(&path, opts)
            .await
            .map_err(|err| StoreFault::Other(err.to_string()))?;
        got.bytes()
            .await
            .map_err(|err| StoreFault::Other(err.to_string()))
    }

    async fn delete_key(&self, key: &str) -> Result<(), StoreFault> {
        self.require_key(key)?;
        let path = path_for(key)?;
        self.store
            .delete(&path)
            .await
            .map_err(|err| StoreFault::Other(err.to_string()))
    }

    async fn content_length(&self, key: &str) -> Result<u64, StoreFault> {
        self.require_key(key)?;
        let path = path_for(key)?;
        let meta = self
            .store
            .head(&path)
            .await
            .map_err(|err| StoreFault::Other(err.to_string()))?;
        Ok(meta.size)
    }

    fn list_calls(&self) -> u64 {
        self.list_calls.load(Ordering::Relaxed)
    }

    fn put_calls(&self) -> u64 {
        self.put_calls.load(Ordering::Relaxed)
    }

    fn multipart_calls(&self) -> u64 {
        self.multipart_calls.load(Ordering::Relaxed)
    }

    fn signer(&self) -> Option<Arc<dyn object_store::signer::Signer>> {
        self.signer.clone()
    }

    fn checksum_header(&self) -> Option<&'static str> {
        self.checksum_header
    }

    fn allow_http_presign(&self) -> bool {
        self.allow_http_presign
    }
}

/// Rejects an empty body before any PUT. One `put` at or under 100 MiB.
pub async fn put_bytes(
    blob: &dyn BlobStore,
    key: &str,
    body: &[u8],
    sse: Sse,
) -> Result<PutMeta, OriginalsError> {
    put_at_threshold(blob, key, body, sse, max_upload_bytes(), SINGLE_PUT_MAX).await
}

/// `single_max` is 100 MiB in production. Tests pass a lower threshold so the
/// multipart branch runs without allocating 100 MiB.
async fn put_at_threshold(
    blob: &dyn BlobStore,
    key: &str,
    body: &[u8],
    sse: Sse,
    cap: usize,
    single_max: u64,
) -> Result<PutMeta, OriginalsError> {
    if body.is_empty() {
        return Err(OriginalsError::Config(
            "original payload is empty; put was not called".into(),
        ));
    }
    if body.len() > cap {
        return Err(OriginalsError::Config(format!(
            "payload {} exceeds upload cap {cap}",
            body.len()
        )));
    }
    let strategy = if (body.len() as u64) <= single_max {
        WriteStrategy::Single
    } else {
        WriteStrategy::Multipart
    };
    match strategy {
        WriteStrategy::Single => Ok(blob
            .put(key, Bytes::copy_from_slice(body), &sse_headers(sse))
            .await?),
        WriteStrategy::Multipart => {
            let plan = multipart_plan(body.len() as u64);
            reject_short_parts(&plan)?;
            let mut parts = Vec::with_capacity(plan.len());
            let mut offset = 0usize;
            for len in plan {
                let end = offset + len as usize;
                parts.push(Bytes::copy_from_slice(&body[offset..end]));
                offset = end;
            }
            Ok(blob.put_multipart(key, &parts, &sse_headers(sse)).await?)
        }
    }
}

fn multipart_plan(len: u64) -> Vec<u64> {
    let part = 8 * 1024 * 1024;
    let mut out = Vec::new();
    let mut left = len;
    while left > 0 {
        let n = left.min(part);
        out.push(n);
        left -= n;
    }
    out
}

pub async fn get_inclusive(
    blob: &dyn BlobStore,
    key: &str,
    start: u64,
    end: u64,
) -> Result<Vec<u8>, OriginalsError> {
    let range = inclusive_to_get_range(start, end)?;
    let bytes = blob.get_range(key, range).await?;
    Ok(bytes.to_vec())
}

pub async fn verify_sha256(
    blob: &dyn BlobStore,
    key: &str,
    expected: &str,
    byte_length: i64,
) -> Result<bool, OriginalsError> {
    match sha256_object(blob, key, byte_length).await {
        Ok(sha) => Ok(sha == expected),
        Err(OriginalsError::Checksum(_)) => Ok(false),
        Err(err) => Err(err),
    }
}

/// Hash `byte_length` bytes. An object larger than 50 MiB is read in 8 MiB
/// windows and those windows are dropped. The returned digest is the only copy.
pub async fn sha256_object(
    blob: &dyn BlobStore,
    key: &str,
    byte_length: i64,
) -> Result<String, OriginalsError> {
    let (sha, _) = hash_object(blob, key, byte_length, false, STREAM_CHUNK).await?;
    Ok(sha)
}

/// Full object only when the bytes match the ref. The error names neither the
/// key nor the digest, so a caller can show it to a client.
///
/// At or under 50 MiB this is one GET. Above that, 8 MiB ranges are appended
/// to a single buffer while the hash is updated, so the cloud response is not
/// held beside a second copy.
pub async fn read_verified(
    blob: &dyn BlobStore,
    key: &str,
    expected_sha: &str,
    byte_length: i64,
) -> Result<Vec<u8>, OriginalsError> {
    if byte_length > crate::originals::MAX_UPLOAD_BYTES as i64 {
        let (sha, bytes) = hash_object(blob, key, byte_length, true, STREAM_CHUNK).await?;
        if sha != expected_sha {
            return Err(OriginalsError::Checksum(
                "object bytes do not match the verified ref".into(),
            ));
        }
        return Ok(bytes);
    }
    let bytes = blob.get_all(key).await?;
    if byte_length < 0 || bytes.len() as i64 != byte_length || sha256_hex(&bytes) != expected_sha {
        return Err(OriginalsError::Checksum(
            "object bytes do not match the verified ref".into(),
        ));
    }
    Ok(bytes.to_vec())
}

async fn hash_object(
    blob: &dyn BlobStore,
    key: &str,
    byte_length: i64,
    keep: bool,
    chunk: u64,
) -> Result<(String, Vec<u8>), OriginalsError> {
    if byte_length < 0 || chunk == 0 {
        return Err(OriginalsError::Checksum(
            "object bytes do not match the verified ref".into(),
        ));
    }
    let stored = blob.content_length(key).await?;
    if stored != byte_length as u64 {
        return Err(OriginalsError::Checksum(
            "object bytes do not match the verified ref".into(),
        ));
    }
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    let mut out = if keep {
        Vec::with_capacity(stored as usize)
    } else {
        Vec::new()
    };
    let mut offset = 0u64;
    while offset < stored {
        let end = offset.saturating_add(chunk - 1).min(stored - 1);
        let range = inclusive_to_get_range(offset, end)?;
        let part = blob.get_range(key, range).await?;
        if part.len() as u64 != end - offset + 1 {
            return Err(OriginalsError::Checksum(
                "object bytes do not match the verified ref".into(),
            ));
        }
        hasher.update(&part);
        if keep {
            out.extend_from_slice(&part);
        }
        offset = end + 1;
    }
    Ok((format!("{:x}", hasher.finalize()), out))
}

/// Put `body` at `key`. When `store_checks_body` is set, a successful put is
/// the check and the object is not read back. Otherwise the read-back must
/// match, and a failed read-back deletes `key`.
pub async fn publish_bytes(
    blob: &dyn BlobStore,
    key: &str,
    body: &[u8],
    sse: Sse,
    store_checks_body: bool,
) -> Result<PutMeta, OriginalsError> {
    let meta = put_bytes(blob, key, body, sse).await?;
    if store_checks_body {
        return Ok(meta);
    }
    if let Err(err) = confirm_put(blob, key, body).await {
        let _ = blob.delete_key(key).await;
        return Err(err);
    }
    Ok(meta)
}

/// Put `body` at a fresh uuid under the kind prefix, then read it back.
/// A failed read-back deletes that new key and leaves every other key alone.
pub async fn publish_new_key(
    blob: &dyn BlobStore,
    prefix: &str,
    kind: OwnerKind,
    tenant: Uuid,
    workspace: Uuid,
    body: &[u8],
    sse: Sse,
) -> Result<(String, PutMeta), OriginalsError> {
    let key = object_key(prefix, kind, tenant, workspace, Uuid::new_v4());
    let meta = publish_bytes(blob, &key, body, sse, false).await?;
    Ok((key, meta))
}

/// Read the object back and require it to be the bytes just written.
pub async fn confirm_put(
    blob: &dyn BlobStore,
    key: &str,
    body: &[u8],
) -> Result<(), OriginalsError> {
    let sha = sha256_hex(body);
    let got = sha256_object(blob, key, body.len() as i64).await?;
    if got != sha {
        return Err(OriginalsError::Checksum(
            "object bytes do not match the verified ref".into(),
        ));
    }
    Ok(())
}

/// Records headers so tests can see SSE without a cloud.
pub struct RecordingBlob {
    inner: ObjectBlob,
    pub last_headers: std::sync::Mutex<Vec<(String, String)>>,
}

impl RecordingBlob {
    pub fn memory() -> Self {
        Self {
            inner: ObjectBlob::new(Arc::new(object_store::memory::InMemory::new())),
            last_headers: std::sync::Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl BlobStore for RecordingBlob {
    async fn put(
        &self,
        key: &str,
        body: Bytes,
        headers: &[(&str, &str)],
    ) -> Result<PutMeta, StoreFault> {
        {
            let mut guard = self
                .last_headers
                .lock()
                .map_err(|_| StoreFault::Other("header lock poisoned".into()))?;
            guard.clear();
            guard.extend(
                headers
                    .iter()
                    .map(|(k, v)| ((*k).to_string(), (*v).to_string())),
            );
        }
        self.inner.put(key, body, headers).await
    }

    async fn put_multipart(
        &self,
        key: &str,
        parts: &[Bytes],
        headers: &[(&str, &str)],
    ) -> Result<PutMeta, StoreFault> {
        {
            let mut guard = self
                .last_headers
                .lock()
                .map_err(|_| StoreFault::Other("header lock poisoned".into()))?;
            guard.clear();
            guard.extend(
                headers
                    .iter()
                    .map(|(k, v)| ((*k).to_string(), (*v).to_string())),
            );
        }
        self.inner.put_multipart(key, parts, headers).await
    }

    async fn get_all(&self, key: &str) -> Result<Bytes, StoreFault> {
        self.inner.get_all(key).await
    }

    async fn get_range(&self, key: &str, range: Range<u64>) -> Result<Bytes, StoreFault> {
        self.inner.get_range(key, range).await
    }

    async fn delete_key(&self, key: &str) -> Result<(), StoreFault> {
        self.inner.delete_key(key).await
    }

    fn list_calls(&self) -> u64 {
        self.inner.list_calls()
    }

    fn put_calls(&self) -> u64 {
        self.inner.put_calls()
    }

    fn multipart_calls(&self) -> u64 {
        self.inner.multipart_calls()
    }
}

/// Fails the Nth put (1-based) with a credentials error. Earlier puts succeed.
pub struct FailNth {
    inner: RecordingBlob,
    n: std::sync::atomic::AtomicUsize,
    fail_on: usize,
}

impl FailNth {
    pub fn new(fail_on: usize) -> Self {
        Self {
            inner: RecordingBlob::memory(),
            n: std::sync::atomic::AtomicUsize::new(0),
            fail_on,
        }
    }

    pub fn inner(&self) -> &RecordingBlob {
        &self.inner
    }
}

#[async_trait]
impl BlobStore for FailNth {
    async fn put(
        &self,
        key: &str,
        body: Bytes,
        headers: &[(&str, &str)],
    ) -> Result<PutMeta, StoreFault> {
        let i = self.n.fetch_add(1, Ordering::Relaxed);
        if i + 1 == self.fail_on {
            return Err(StoreFault::Credentials("injected credential expiry".into()));
        }
        self.inner.put(key, body, headers).await
    }

    async fn get_all(&self, key: &str) -> Result<Bytes, StoreFault> {
        self.inner.get_all(key).await
    }

    async fn get_range(&self, key: &str, range: Range<u64>) -> Result<Bytes, StoreFault> {
        self.inner.get_range(key, range).await
    }

    async fn delete_key(&self, key: &str) -> Result<(), StoreFault> {
        self.inner.delete_key(key).await
    }

    fn put_calls(&self) -> u64 {
        self.inner.put_calls()
    }

    fn list_calls(&self) -> u64 {
        self.inner.list_calls()
    }
}

/// Panics if `put` is called. Used to prove an empty payload never reaches the store.
pub struct PanicOnPut;

#[async_trait]
impl BlobStore for PanicOnPut {
    async fn put(
        &self,
        _key: &str,
        _body: Bytes,
        _headers: &[(&str, &str)],
    ) -> Result<PutMeta, StoreFault> {
        panic!("put must not be called");
    }
    async fn get_all(&self, _key: &str) -> Result<Bytes, StoreFault> {
        Err(StoreFault::Other("no object".into()))
    }
    async fn get_range(&self, _key: &str, _range: Range<u64>) -> Result<Bytes, StoreFault> {
        Err(StoreFault::Other("no object".into()))
    }
    async fn delete_key(&self, _key: &str) -> Result<(), StoreFault> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    #[tokio::test]
    async fn read_verified_rejects_a_replaced_object() {
        let blob = RecordingBlob::memory();
        let body = b"page-png";
        put_bytes(&blob, "k", body, Sse::Off).await.unwrap();
        let sha = sha256_hex(body);
        let got = read_verified(&blob, "k", &sha, body.len() as i64)
            .await
            .unwrap();
        assert_eq!(got, body);
        put_bytes(&blob, "k", b"tampered", Sse::Off).await.unwrap();
        let err = read_verified(&blob, "k", &sha, body.len() as i64)
            .await
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "checksum: object bytes do not match the verified ref"
        );
    }

    struct LieGet;

    #[async_trait]
    impl BlobStore for LieGet {
        async fn put(
            &self,
            _key: &str,
            _body: Bytes,
            _headers: &[(&str, &str)],
        ) -> Result<PutMeta, StoreFault> {
            Ok(PutMeta { etag: None })
        }
        async fn get_all(&self, _key: &str) -> Result<Bytes, StoreFault> {
            Ok(Bytes::from_static(b"not-what-was-put"))
        }
        async fn get_range(&self, _key: &str, _range: Range<u64>) -> Result<Bytes, StoreFault> {
            Err(StoreFault::Other("no range".into()))
        }
        async fn delete_key(&self, _key: &str) -> Result<(), StoreFault> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn confirm_put_fails_when_the_object_differs() {
        let err = confirm_put(&LieGet, "k", b"page-png").await.unwrap_err();
        assert_eq!(err.class_name(), "checksum");
        let blob = RecordingBlob::memory();
        put_bytes(&blob, "k", b"page-png", Sse::Off).await.unwrap();
        confirm_put(&blob, "k", b"page-png").await.unwrap();
    }

    #[tokio::test]
    async fn empty_payload_does_not_put() {
        let err = put_bytes(&PanicOnPut, "k", b"", Sse::Off)
            .await
            .unwrap_err();
        assert_eq!(err.class_name(), "config");
    }

    #[tokio::test]
    async fn range_matches_source_slice_including_last_byte() {
        let blob = RecordingBlob::memory();
        let body: Vec<u8> = (0u8..32).collect();
        put_bytes(&blob, "k", &body, Sse::Off).await.unwrap();
        for (start, end) in [(0u64, 0u64), (0, 9), (31, 31)] {
            let got = get_inclusive(&blob, "k", start, end).await.unwrap();
            let expect = &body[start as usize..=end as usize];
            assert_eq!(got, expect, "{start}..={end}");
        }
    }

    #[tokio::test]
    async fn sse_header_absent_by_default_and_set_for_aes256() {
        let blob = RecordingBlob::memory();
        put_bytes(&blob, "a", b"hello", Sse::Off).await.unwrap();
        assert!(blob.last_headers.lock().unwrap().is_empty());
        put_bytes(&blob, "b", b"hello", Sse::Aes256).await.unwrap();
        let headers = blob.last_headers.lock().unwrap().clone();
        assert_eq!(
            headers,
            vec![("x-amz-server-side-encryption".into(), "AES256".into())]
        );
    }

    #[test]
    fn fifty_mib_is_one_put_and_short_part_is_rejected() {
        assert_eq!(write_strategy(50 * 1024 * 1024), WriteStrategy::Single);
        assert_eq!(write_strategy(SINGLE_PUT_MAX), WriteStrategy::Single);
        assert_eq!(write_strategy(SINGLE_PUT_MAX + 1), WriteStrategy::Multipart);
        let err = reject_short_parts(&[1024 * 1024, 1024 * 1024]).unwrap_err();
        assert!(err.to_string().contains("5"));
        assert!(reject_short_parts(&[8 * 1024 * 1024, 1024]).is_ok());
    }

    #[tokio::test]
    async fn fifty_mib_boundary_calls_put_once_and_rejects_one_extra_byte() {
        if std::env::var(crate::originals::MAX_UPLOAD_BYTES_ENV).is_ok() {
            return;
        }
        let blob = RecordingBlob::memory();
        let ok = vec![b'a'; crate::originals::MAX_UPLOAD_BYTES];
        put_bytes(&blob, "cap", &ok, Sse::Off).await.unwrap();
        assert_eq!(blob.put_calls(), 1);
        assert_eq!(blob.multipart_calls(), 0);
        let mut over = ok;
        over.push(b'b');
        let err = put_bytes(&blob, "over", &over, Sse::Off).await.unwrap_err();
        assert_eq!(err.class_name(), "config");
        assert_eq!(blob.put_calls(), 1);
    }

    #[tokio::test]
    async fn above_the_single_put_max_uses_multipart_and_round_trips() {
        let blob = RecordingBlob::memory();
        let single_max = 8 * 1024 * 1024;
        let len = single_max + 64;
        let body: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
        put_at_threshold(
            &blob,
            "wide",
            &body,
            Sse::Aes256,
            len + 1,
            single_max as u64,
        )
        .await
        .unwrap();
        assert_eq!(blob.multipart_calls(), 1);
        assert_eq!(blob.put_calls(), 0);
        let headers = blob.last_headers.lock().unwrap().clone();
        assert_eq!(
            headers,
            vec![("x-amz-server-side-encryption".into(), "AES256".into())]
        );
        let got = blob.get_all("wide").await.unwrap();
        assert_eq!(got.as_ref(), body.as_slice());
    }

    #[test]
    fn inclusive_window_is_half_open() {
        assert_eq!(inclusive_to_get_range(0, 99).unwrap(), 0..100);
        assert_eq!(inclusive_to_get_range(5, 5).unwrap(), 5..6);
    }

    #[tokio::test]
    async fn probe_lists_only_after_the_prefix_parses() {
        let blob = ObjectBlob::new(Arc::new(object_store::memory::InMemory::new()));
        assert!(blob.probe("dev").await);
        assert!(blob.probe("").await);
        assert_eq!(blob.list_calls(), 2);
        assert!(!blob.probe("..").await);
        assert_eq!(blob.list_calls(), 2);
    }

    #[tokio::test]
    async fn nine_parts_round_trip_through_the_batch() {
        let blob = ObjectBlob::new(Arc::new(object_store::memory::InMemory::new()));
        let parts: Vec<Bytes> = (0..9).map(|i| Bytes::from(vec![i; 8])).collect();
        blob.put_multipart("k", &parts, &[]).await.unwrap();
        let got = blob.get_all("k").await.unwrap();
        assert_eq!(got.len(), 72);
        assert_eq!(got[0], 0);
        assert_eq!(got[64], 8);
    }

    #[tokio::test]
    async fn past_one_window_the_hash_drops_each_chunk() {
        let blob = ObjectBlob::new(Arc::new(object_store::memory::InMemory::new()));
        let body: Vec<u8> = (0..20).collect();
        blob.put("k", Bytes::from(body.clone()), &[]).await.unwrap();
        let (sha, kept) = hash_object(&blob, "k", 20, false, 8).await.unwrap();
        assert!(kept.is_empty());
        assert_eq!(sha, sha256_hex(&body));
        let (sha, kept) = hash_object(&blob, "k", 20, true, 8).await.unwrap();
        assert_eq!(kept, body);
        assert_eq!(sha, sha256_hex(&body));
        let err = hash_object(&blob, "k", 19, false, 8).await.unwrap_err();
        assert_eq!(err.class_name(), "checksum");
    }

    #[tokio::test]
    async fn a_store_checksum_skips_the_read_back() {
        let blob = NoRead::new();
        publish_bytes(&blob, "dev/checked", b"body", Sse::Off, true)
            .await
            .unwrap();
        assert_eq!(blob.reads(), 0);
        let err = publish_bytes(&blob, "dev/checked", b"body", Sse::Off, false)
            .await
            .unwrap_err();
        assert_eq!(err.class_name(), "checksum");
        assert!(blob.reads() > 0);
    }

    struct NoRead {
        inner: ObjectBlob,
        reads: std::sync::atomic::AtomicU64,
    }

    impl NoRead {
        fn new() -> Self {
            Self {
                inner: ObjectBlob::new(Arc::new(object_store::memory::InMemory::new())),
                reads: std::sync::atomic::AtomicU64::new(0),
            }
        }

        fn reads(&self) -> u64 {
            self.reads.load(Ordering::Relaxed)
        }
    }

    #[async_trait]
    impl BlobStore for NoRead {
        async fn put(
            &self,
            key: &str,
            body: Bytes,
            headers: &[(&str, &str)],
        ) -> Result<PutMeta, StoreFault> {
            self.inner.put(key, body, headers).await
        }

        async fn get_all(&self, key: &str) -> Result<Bytes, StoreFault> {
            self.reads.fetch_add(1, Ordering::Relaxed);
            self.inner.get_all(key).await
        }

        async fn get_range(&self, key: &str, range: Range<u64>) -> Result<Bytes, StoreFault> {
            self.reads.fetch_add(1, Ordering::Relaxed);
            let bytes = self.inner.get_range(key, range).await?;
            let mut flipped = bytes.to_vec();
            if let Some(first) = flipped.first_mut() {
                *first ^= 0xff;
            }
            Ok(Bytes::from(flipped))
        }

        async fn delete_key(&self, key: &str) -> Result<(), StoreFault> {
            self.inner.delete_key(key).await
        }

        async fn content_length(&self, key: &str) -> Result<u64, StoreFault> {
            self.reads.fetch_add(1, Ordering::Relaxed);
            self.inner.content_length(key).await
        }
    }

    #[tokio::test]
    async fn a_key_outside_the_prefix_is_refused() {
        let blob = ObjectBlob::prefixed(Arc::new(object_store::memory::InMemory::new()), "dev");
        let err = blob
            .put("other/object", Bytes::from_static(b"no"), &[])
            .await
            .unwrap_err();
        assert!(err.to_string().contains("outside the configured prefix"));
        blob.put("dev/t/object", Bytes::from_static(b"yes"), &[])
            .await
            .unwrap();
        assert_eq!(blob.get_all("dev/t/object").await.unwrap().as_ref(), b"yes");
        assert!(blob.get_all("other/object").await.is_err());
        assert!(blob.get_range("other/object", 0..1).await.is_err());
        assert!(blob.delete_key("other/object").await.is_err());
        assert!(blob.content_length("other/object").await.is_err());
        assert!(!blob.probe("other").await);
        assert!(blob.probe("dev").await);
    }

    #[tokio::test]
    async fn confirm_failure_on_replace_leaves_the_old_object() {
        let old_key = "dev/t/00000000-0000-0000-0000-000000000001/w/00000000-0000-0000-0000-000000000002/asset/00000000-0000-0000-0000-000000000003";
        let blob = FlipRead::new(old_key);
        let old_body = Bytes::from_static(b"old-asset");
        blob.put(old_key, old_body.clone(), &[]).await.unwrap();
        let mut ref_key = old_key.to_string();
        let err = publish_new_key(
            &blob,
            "dev",
            OwnerKind::Asset,
            Uuid::nil(),
            Uuid::from_u128(2),
            b"new-asset",
            Sse::Off,
        )
        .await
        .unwrap_err();
        assert_eq!(err.class_name(), "checksum");
        assert_eq!(ref_key, old_key);
        assert_eq!(blob.get_all(old_key).await.unwrap(), old_body);
        assert_eq!(blob.keys(), vec![old_key.to_string()]);

        let honest = ObjectBlob::prefixed(Arc::new(object_store::memory::InMemory::new()), "dev");
        honest.put(old_key, old_body.clone(), &[]).await.unwrap();
        let (key, _) = publish_new_key(
            &honest,
            "dev",
            OwnerKind::Asset,
            Uuid::from_u128(1),
            Uuid::from_u128(2),
            b"new-asset",
            Sse::Off,
        )
        .await
        .unwrap();
        assert_ne!(key, old_key);
        ref_key = key.clone();
        honest.delete_key(old_key).await.unwrap();
        assert!(honest.get_all(old_key).await.is_err());
        assert_eq!(
            honest.get_all(&ref_key).await.unwrap().as_ref(),
            b"new-asset"
        );
    }

    struct FlipRead {
        inner: ObjectBlob,
        keep: String,
        keys: std::sync::Mutex<Vec<String>>,
    }

    impl FlipRead {
        fn new(keep: &str) -> Self {
            Self {
                inner: ObjectBlob::new(Arc::new(object_store::memory::InMemory::new())),
                keep: keep.to_string(),
                keys: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn keys(&self) -> Vec<String> {
            self.keys.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl BlobStore for FlipRead {
        async fn put(
            &self,
            key: &str,
            body: Bytes,
            headers: &[(&str, &str)],
        ) -> Result<PutMeta, StoreFault> {
            self.keys.lock().unwrap().push(key.to_string());
            self.inner.put(key, body, headers).await
        }

        async fn get_all(&self, key: &str) -> Result<Bytes, StoreFault> {
            self.inner.get_all(key).await
        }

        async fn get_range(&self, key: &str, range: Range<u64>) -> Result<Bytes, StoreFault> {
            let bytes = self.inner.get_range(key, range).await?;
            if key == self.keep {
                return Ok(bytes);
            }
            let mut flipped = bytes.to_vec();
            if let Some(first) = flipped.first_mut() {
                *first ^= 0xff;
            }
            Ok(Bytes::from(flipped))
        }

        async fn delete_key(&self, key: &str) -> Result<(), StoreFault> {
            self.keys.lock().unwrap().retain(|item| item != key);
            self.inner.delete_key(key).await
        }

        async fn content_length(&self, key: &str) -> Result<u64, StoreFault> {
            self.inner.content_length(key).await
        }
    }
}
