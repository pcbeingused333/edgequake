//! PUT presign for agent uploads (SPEC-173). One method: `PUT`.

use std::time::Duration;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use object_store::path::Path;
use object_store::signer::{HeaderName, HeaderValue, Method, SignedUrlOptions, Signer, Url};

use super::io::OriginalsError;

pub const S3_CHECKSUM_HEADER: &str = "x-amz-checksum-sha256";

/// A presigned PUT whose signature includes the content type and, when the
/// backend supports it, the SHA-256 checksum header.
#[derive(Debug)]
pub struct PresignedPut {
    pub url: Url,
    pub headers: Vec<(String, String)>,
}

/// Sign one PUT. Callers map [`OriginalsError::NotSupported`] to the relay.
/// This function does not retry with an unsigned URL.
pub async fn presign_put(
    signer: &dyn Signer,
    key: &str,
    expires_in: Duration,
    content_type: &str,
    sha256_hex: &str,
    checksum_header: Option<&str>,
) -> Result<PresignedPut, OriginalsError> {
    if expires_in.as_secs() == 0 || expires_in.as_secs() > 900 {
        return Err(OriginalsError::Config(
            "presign lifetime must be 1..=900 seconds".into(),
        ));
    }
    let mut options = SignedUrlOptions::default().with_signed_header(
        HeaderName::from_static("content-type"),
        HeaderValue::from_str(content_type).map_err(|_| {
            OriginalsError::Config(format!(
                "content-type is not a header value: {content_type}"
            ))
        })?,
    );
    let mut headers = vec![("content-type".to_string(), content_type.to_string())];
    if let Some(name) = checksum_header {
        let raw = hex_sha256(sha256_hex)?;
        let value = STANDARD.encode(raw);
        let header_name = HeaderName::from_bytes(name.as_bytes())
            .map_err(|_| OriginalsError::Config(format!("checksum header name: {name}")))?;
        let header_value = HeaderValue::from_str(&value)
            .map_err(|_| OriginalsError::Config("checksum header value".into()))?;
        options = options.with_signed_header(header_name, header_value);
        headers.push((name.to_string(), value));
    }
    let path = Path::from(key);
    let url = signer
        .signed_url_opts(Method::PUT, &path, expires_in, &options)
        .await
        .map_err(map_sign_err)?;
    Ok(PresignedPut { url, headers })
}

/// Refuse an `http` URL unless the store was opened for local MinIO.
pub fn reject_insecure_presign(url: &Url, allow_http: bool) -> Result<(), OriginalsError> {
    if url.scheme() == "https" || (allow_http && url.scheme() == "http") {
        return Ok(());
    }
    Err(OriginalsError::Config(format!(
        "presigned URL scheme `{}` is not allowed",
        url.scheme()
    )))
}

fn hex_sha256(hex_str: &str) -> Result<[u8; 32], OriginalsError> {
    if hex_str.len() != 64 || !hex_str.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(OriginalsError::Config(
            "sha256 must be 64 hex characters".into(),
        ));
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = u8::from_str_radix(&hex_str[i * 2..i * 2 + 2], 16)
            .map_err(|_| OriginalsError::Config("sha256 is not hex".into()))?;
    }
    Ok(out)
}

fn map_sign_err(err: object_store::Error) -> OriginalsError {
    match err {
        object_store::Error::NotSupported { source } => {
            OriginalsError::NotSupported(source.to_string())
        }
        other => OriginalsError::Config(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use object_store::signer::HeaderMap;
    use std::sync::Mutex;

    #[derive(Debug)]
    struct RecordSigner {
        method: Mutex<Option<Method>>,
        headers: Mutex<HeaderMap>,
        url: &'static str,
        unsupported: bool,
    }

    #[async_trait]
    impl Signer for RecordSigner {
        async fn signed_url(
            &self,
            method: Method,
            _path: &Path,
            _expires_in: Duration,
        ) -> object_store::Result<Url> {
            *self.method.lock().unwrap() = Some(method);
            Url::parse(self.url).map_err(|err| object_store::Error::Generic {
                store: "test",
                source: err.to_string().into(),
            })
        }

        async fn signed_url_opts(
            &self,
            method: Method,
            path: &Path,
            expires_in: Duration,
            options: &SignedUrlOptions,
        ) -> object_store::Result<Url> {
            if self.unsupported {
                return Err(object_store::Error::NotSupported {
                    source: "test signer does not bind headers".into(),
                });
            }
            *self.headers.lock().unwrap() = options.signed_headers.clone();
            self.signed_url(method, path, expires_in).await
        }
    }

    #[tokio::test]
    async fn presign_is_put_and_binds_type_and_checksum() {
        let signer = RecordSigner {
            method: Mutex::new(None),
            headers: Mutex::new(HeaderMap::new()),
            url: "https://bucket.example/obj",
            unsupported: false,
        };
        let sha = "ab".repeat(32);
        let signed = presign_put(
            &signer,
            "dev/t/x/w/y/upload/z",
            Duration::from_secs(60),
            "application/pdf",
            &sha,
            Some(S3_CHECKSUM_HEADER),
        )
        .await
        .unwrap();
        assert_eq!(signer.method.lock().unwrap().clone(), Some(Method::PUT));
        assert_eq!(signed.url.scheme(), "https");
        let headers = signer.headers.lock().unwrap();
        assert_eq!(
            headers
                .get(HeaderName::from_static("content-type"))
                .unwrap(),
            "application/pdf"
        );
        assert!(headers.get(S3_CHECKSUM_HEADER).is_some());
        reject_insecure_presign(&signed.url, false).unwrap();
    }

    #[tokio::test]
    async fn unsupported_signer_is_not_an_unsigned_url() {
        let signer = RecordSigner {
            method: Mutex::new(None),
            headers: Mutex::new(HeaderMap::new()),
            url: "https://bucket.example/obj",
            unsupported: true,
        };
        let err = presign_put(
            &signer,
            "k",
            Duration::from_secs(60),
            "application/pdf",
            &"ab".repeat(32),
            Some(S3_CHECKSUM_HEADER),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, OriginalsError::NotSupported(_)));
        assert!(signer.method.lock().unwrap().is_none());
    }

    #[test]
    fn http_presign_needs_the_local_flag() {
        let url = Url::parse("http://127.0.0.1:9000/obj").unwrap();
        assert!(reject_insecure_presign(&url, false).is_err());
        assert!(reject_insecure_presign(&url, true).is_ok());
    }
}
