//! `EDGEQUAKE_ORIGINAL_STORE` parsing. Database mode never builds a client.

use std::sync::Arc;

use object_store::aws::{AmazonS3Builder, AmazonS3ConfigKey, Checksum};
use object_store::gcp::GoogleCloudStorageBuilder;
use object_store::limit::LimitStore;
use object_store::ObjectStore;

use super::io::{BlobStore, ObjectBlob, OriginalsError, Sse, STORE_REQUEST_LIMIT};

/// Where original bytes live for this process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreMode {
    Database,
    Object,
}

/// Which `object_store` builder a URL selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreKind {
    AmazonS3,
    GoogleCloudStorage,
    MicrosoftAzure,
}

/// Parsed settings. Tests build this directly so they do not touch process env.
#[derive(Debug, Clone)]
pub struct Settings {
    pub mode: StoreMode,
    pub url: Option<String>,
    pub endpoint: Option<String>,
    pub region: String,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
    /// `None` means path-style when an endpoint is set, virtual-hosted otherwise.
    pub path_style: Option<bool>,
    pub allow_http: bool,
    pub create_bucket: bool,
    pub sse: Sse,
    /// True when a GCS service-account file, key, ADC file, or application-credentials path is present.
    pub gcs_credentials: bool,
    pub gcs_service_account_path: Option<String>,
    pub gcs_service_account_key: Option<String>,
    pub gcs_application_credentials: Option<String>,
    pub azure_account: Option<String>,
    pub azure_key: Option<String>,
    pub azure_sas: Option<String>,
    pub azure_token: Option<String>,
    pub azure_client_id: Option<String>,
    pub azure_client_secret: Option<String>,
    pub azure_tenant_id: Option<String>,
    pub azure_federated_token_file: Option<String>,
    /// `AWS_WEB_IDENTITY_TOKEN_FILE` for EKS IRSA. Used only when no custom endpoint is set.
    pub aws_web_identity_token_file: Option<String>,
    pub aws_role_arn: Option<String>,
    pub aws_session_token: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: StoreMode::Database,
            url: None,
            endpoint: None,
            region: "us-east-1".into(),
            access_key: None,
            secret_key: None,
            path_style: None,
            allow_http: false,
            create_bucket: false,
            sse: Sse::Off,
            gcs_credentials: false,
            gcs_service_account_path: None,
            gcs_service_account_key: None,
            gcs_application_credentials: None,
            azure_account: None,
            azure_key: None,
            azure_sas: None,
            azure_token: None,
            azure_client_id: None,
            azure_client_secret: None,
            azure_tenant_id: None,
            azure_federated_token_file: None,
            aws_web_identity_token_file: None,
            aws_role_arn: None,
            aws_session_token: None,
        }
    }
}

/// Client plus the prefix from the store URL.
pub struct OriginalsClient {
    pub blobs: Arc<dyn BlobStore>,
    pub prefix: String,
    pub bucket: String,
    pub endpoint_host: String,
    pub path_style: bool,
    pub sse: Sse,
    /// Real AWS checks SHA-256 on the put. MinIO, GCS, and Azure do not.
    pub store_checks_body: bool,
}

/// Result of opening config. `client` is absent in database mode.
pub struct OriginalsOpened {
    pub mode: StoreMode,
    pub client: Option<Arc<OriginalsClient>>,
    pub bucket_ready: bool,
}

pub fn path_style_enabled(endpoint: Option<&str>, explicit: Option<bool>) -> bool {
    match explicit {
        Some(value) => value,
        None => endpoint.map(|e| !e.is_empty()).unwrap_or(false),
    }
}

pub fn classify_url(url: &str) -> Result<(StoreKind, String, String), OriginalsError> {
    let (scheme, rest) = url.split_once("://").ok_or_else(|| {
        OriginalsError::Config(format!("original store URL `{url}` has no scheme"))
    })?;
    let kind = match scheme {
        "s3" | "s3a" => StoreKind::AmazonS3,
        "gs" => StoreKind::GoogleCloudStorage,
        "az" | "abfs" | "abfss" => StoreKind::MicrosoftAzure,
        other => {
            return Err(OriginalsError::Config(format!(
                "original store scheme `{other}` is not s3, gs, or az"
            )))
        }
    };
    let rest = rest.trim_start_matches('/');
    if kind == StoreKind::MicrosoftAzure && rest.contains('@') {
        // abfs://container@account.dfs.core.windows.net/prefix
        let (container, after) = rest
            .split_once('@')
            .ok_or_else(|| OriginalsError::Config("abfs URL is missing the account host".into()))?;
        let (host, prefix) = after.split_once('/').unwrap_or((after, ""));
        if container.is_empty() || host.is_empty() {
            return Err(OriginalsError::Config(
                "abfs URL needs a container and an account host".into(),
            ));
        }
        return Ok((
            kind,
            container.to_string(),
            prefix.trim_matches('/').to_string(),
        ));
    }
    let (bucket, prefix) = rest.split_once('/').unwrap_or((rest, ""));
    if bucket.is_empty() {
        return Err(OriginalsError::Config(
            "original store URL is missing a bucket".into(),
        ));
    }
    Ok((
        kind,
        bucket.to_string(),
        prefix.trim_matches('/').to_string(),
    ))
}

pub fn sse_from_env(raw: Option<&str>) -> Result<Sse, OriginalsError> {
    match raw.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(Sse::Off),
        Some(value) if value.eq_ignore_ascii_case("AES256") => Ok(Sse::Aes256),
        Some(value) => Err(OriginalsError::Config(format!(
            "EDGEQUAKE_OBJECT_STORE_SSE={value} is not supported; use AES256 or leave it unset"
        ))),
    }
}

fn nonempty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn env_nonempty(name: &str) -> Option<String> {
    std::env::var(name).ok().and_then(|v| nonempty(&v))
}

fn env_bool(name: &str) -> Option<bool> {
    env_nonempty(name).and_then(|v| match v.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" => Some(true),
        "0" | "false" | "no" => Some(false),
        _ => None,
    })
}

/// Read process env. Database is the default and ignores the bucket.
pub fn settings_from_env() -> Result<Settings, OriginalsError> {
    let mode = match env_nonempty("EDGEQUAKE_ORIGINAL_STORE")
        .unwrap_or_else(|| "database".into())
        .to_ascii_lowercase()
        .as_str()
    {
        "database" => StoreMode::Database,
        "object" => StoreMode::Object,
        other => {
            return Err(OriginalsError::Config(format!(
                "EDGEQUAKE_ORIGINAL_STORE={other} is not database or object"
            )))
        }
    };
    let access_key = env_nonempty("EDGEQUAKE_ORIGINAL_STORE_ACCESS_KEY_ID")
        .or_else(|| env_nonempty("AWS_ACCESS_KEY_ID"));
    let secret_key = env_nonempty("EDGEQUAKE_ORIGINAL_STORE_SECRET_ACCESS_KEY")
        .or_else(|| env_nonempty("AWS_SECRET_ACCESS_KEY"));
    let gcs_service_account_path = env_nonempty("GOOGLE_SERVICE_ACCOUNT")
        .or_else(|| env_nonempty("GOOGLE_SERVICE_ACCOUNT_PATH"))
        .or_else(|| env_nonempty("SERVICE_ACCOUNT"));
    let gcs_service_account_key = env_nonempty("GOOGLE_SERVICE_ACCOUNT_KEY");
    let gcs_application_credentials = env_nonempty("GOOGLE_APPLICATION_CREDENTIALS");
    let gcs_credentials = gcs_service_account_path.is_some()
        || gcs_service_account_key.is_some()
        || gcs_application_credentials.is_some()
        || gcs_adc_file_exists();
    let azure_key = env_nonempty("AZURE_STORAGE_ACCOUNT_KEY")
        .or_else(|| env_nonempty("AZURE_STORAGE_ACCESS_KEY"));
    Ok(Settings {
        mode,
        url: env_nonempty("EDGEQUAKE_ORIGINAL_STORE_URL"),
        endpoint: env_nonempty("EDGEQUAKE_ORIGINAL_STORE_ENDPOINT"),
        region: env_nonempty("EDGEQUAKE_ORIGINAL_STORE_REGION")
            .or_else(|| env_nonempty("AWS_REGION"))
            .or_else(|| env_nonempty("AWS_DEFAULT_REGION"))
            .unwrap_or_else(|| "us-east-1".into()),
        access_key,
        secret_key,
        path_style: env_bool("EDGEQUAKE_ORIGINAL_STORE_PATH_STYLE"),
        allow_http: env_bool("EDGEQUAKE_ORIGINAL_STORE_ALLOW_HTTP").unwrap_or(false),
        create_bucket: env_bool("EDGEQUAKE_ORIGINAL_STORE_CREATE_BUCKET").unwrap_or(false),
        sse: sse_from_env(env_nonempty("EDGEQUAKE_OBJECT_STORE_SSE").as_deref())?,
        gcs_credentials,
        gcs_service_account_path,
        gcs_service_account_key,
        gcs_application_credentials,
        azure_account: env_nonempty("AZURE_STORAGE_ACCOUNT_NAME")
            .or_else(|| env_nonempty("AZURE_STORAGE_ACCOUNT")),
        azure_key,
        azure_sas: env_nonempty("AZURE_STORAGE_SAS_KEY")
            .or_else(|| env_nonempty("AZURE_STORAGE_SAS_TOKEN")),
        azure_token: env_nonempty("AZURE_STORAGE_TOKEN"),
        azure_client_id: env_nonempty("AZURE_STORAGE_CLIENT_ID"),
        azure_client_secret: env_nonempty("AZURE_STORAGE_CLIENT_SECRET"),
        azure_tenant_id: env_nonempty("AZURE_STORAGE_TENANT_ID"),
        azure_federated_token_file: env_nonempty("AZURE_FEDERATED_TOKEN_FILE"),
        aws_web_identity_token_file: env_nonempty("AWS_WEB_IDENTITY_TOKEN_FILE"),
        aws_role_arn: env_nonempty("AWS_ROLE_ARN"),
        aws_session_token: env_nonempty("AWS_SESSION_TOKEN"),
    })
}

fn gcs_adc_file_exists() -> bool {
    let home = if cfg!(windows) {
        std::env::var_os("APPDATA")
    } else {
        std::env::var_os("HOME")
    };
    let Some(home) = home else {
        return false;
    };
    let relative = if cfg!(windows) {
        "gcloud/application_default_credentials.json"
    } else {
        ".config/gcloud/application_default_credentials.json"
    };
    std::path::Path::new(&home).join(relative).is_file()
}

fn account_from_store_url(url: &str) -> Option<String> {
    let (_, rest) = url.split_once("://")?;
    let (_, after) = rest.split_once('@')?;
    let host = after.split('/').next().unwrap_or("");
    let account = host.split('.').next().unwrap_or("");
    if account.is_empty() {
        None
    } else {
        Some(account.to_string())
    }
}

fn azure_account<'a>(settings: &'a Settings, url: &'a str) -> Option<String> {
    settings
        .azure_account
        .clone()
        .or_else(|| account_from_store_url(url))
}

fn azure_has_secret(settings: &Settings) -> bool {
    settings.azure_key.is_some()
        || settings.azure_sas.is_some()
        || settings.azure_token.is_some()
        || (settings.azure_client_id.is_some()
            && settings.azure_client_secret.is_some()
            && settings.azure_tenant_id.is_some())
        || (settings.azure_client_id.is_some()
            && settings.azure_tenant_id.is_some()
            && settings.azure_federated_token_file.is_some())
}

fn aws_has_static_keys(settings: &Settings) -> bool {
    settings.access_key.is_some() && settings.secret_key.is_some()
}

fn endpoint_host(endpoint: &str) -> String {
    endpoint
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or(endpoint)
        .to_string()
}

/// Build a client, or return database mode without constructing one.
///
/// `builder` is called only for object mode. Database mode never invokes it,
/// which is how a panicking builder proves the path is unused.
pub async fn open_settings_with<F>(
    settings: &Settings,
    builder: F,
) -> Result<OriginalsOpened, OriginalsError>
where
    F: FnOnce(&Settings) -> Result<Arc<dyn BlobStore>, OriginalsError>,
{
    if settings.mode == StoreMode::Database {
        if settings.url.is_some() {
            tracing::warn!(
                "EDGEQUAKE_ORIGINAL_STORE_URL is set while EDGEQUAKE_ORIGINAL_STORE=database; the bucket is not opened"
            );
        }
        return Ok(OriginalsOpened {
            mode: StoreMode::Database,
            client: None,
            bucket_ready: true,
        });
    }
    validate_object(settings)?;
    let (kind, bucket, prefix) = classify_url(settings.url.as_deref().unwrap_or(""))?;
    let path_style = path_style_enabled(settings.endpoint.as_deref(), settings.path_style);
    let blobs = builder(settings)?;
    let host = settings
        .endpoint
        .as_deref()
        .map(endpoint_host)
        .unwrap_or_else(|| match kind {
            StoreKind::AmazonS3 => format!("s3.{}.amazonaws.com", settings.region),
            StoreKind::GoogleCloudStorage => "storage.googleapis.com".into(),
            StoreKind::MicrosoftAzure => "blob.core.windows.net".into(),
        });
    Ok(OriginalsOpened {
        mode: StoreMode::Object,
        client: Some(Arc::new(OriginalsClient {
            blobs,
            prefix,
            bucket,
            endpoint_host: host,
            path_style,
            sse: settings.sse,
            store_checks_body: store_checks_body(kind, settings.endpoint.as_deref()),
        })),
        bucket_ready: true,
    })
}

pub async fn open_settings(settings: &Settings) -> Result<OriginalsOpened, OriginalsError> {
    if settings.mode == StoreMode::Database {
        return open_settings_with(settings, |_| {
            Err(OriginalsError::Config(
                "database mode must not build a client".into(),
            ))
        })
        .await;
    }
    validate_object(settings)?;
    let (_, _, prefix) = classify_url(settings.url.as_deref().unwrap_or(""))?;
    let built = build_object_blob(settings)?;
    let ready = built.probe(&prefix).await;
    let blobs: Arc<dyn BlobStore> = Arc::new(built);
    // Re-run the shared opener's bookkeeping without probing twice.
    let (kind, bucket, prefix) = classify_url(settings.url.as_deref().unwrap_or(""))?;
    let path_style = path_style_enabled(settings.endpoint.as_deref(), settings.path_style);
    let host = settings
        .endpoint
        .as_deref()
        .map(endpoint_host)
        .unwrap_or_else(|| match kind {
            StoreKind::AmazonS3 => format!("s3.{}.amazonaws.com", settings.region),
            StoreKind::GoogleCloudStorage => "storage.googleapis.com".into(),
            StoreKind::MicrosoftAzure => "blob.core.windows.net".into(),
        });
    if !ready {
        tracing::warn!(host = %host, bucket = %bucket, path_style, "original store bucket is unreachable");
        if !path_style && settings.endpoint.is_some() {
            tracing::warn!(
                "boot list of the prefix failed; set path-style for this endpoint (EDGEQUAKE_ORIGINAL_STORE_PATH_STYLE=true)"
            );
        }
    }
    let _ = kind;
    Ok(OriginalsOpened {
        mode: StoreMode::Object,
        client: Some(Arc::new(OriginalsClient {
            blobs,
            prefix,
            bucket,
            endpoint_host: host,
            path_style,
            sse: settings.sse,
            store_checks_body: store_checks_body(kind, settings.endpoint.as_deref()),
        })),
        bucket_ready: ready,
    })
}

/// SHA-256 is sent on the put only for Amazon S3 with no custom endpoint.
pub fn store_checks_body(kind: StoreKind, endpoint: Option<&str>) -> bool {
    matches!(kind, StoreKind::AmazonS3) && endpoint.is_none()
}

pub async fn open_from_env() -> Result<OriginalsOpened, OriginalsError> {
    open_settings(&settings_from_env()?).await
}

fn validate_object(settings: &Settings) -> Result<(), OriginalsError> {
    let url = settings.url.as_deref().ok_or_else(|| {
        OriginalsError::Config(
            "EDGEQUAKE_ORIGINAL_STORE=object requires EDGEQUAKE_ORIGINAL_STORE_URL".into(),
        )
    })?;
    let (kind, _bucket, _prefix) = classify_url(url)?;
    if settings.create_bucket {
        return Err(OriginalsError::Config(
            "EDGEQUAKE_ORIGINAL_STORE_CREATE_BUCKET=true is not supported; the process does not create a bucket"
                .into(),
        ));
    }
    if let Some(endpoint) = settings.endpoint.as_deref() {
        if endpoint.starts_with("http://") && !settings.allow_http {
            return Err(OriginalsError::Config(
                "http endpoint requires EDGEQUAKE_ORIGINAL_STORE_ALLOW_HTTP=true".into(),
            ));
        }
    }
    match kind {
        StoreKind::GoogleCloudStorage if !settings.gcs_credentials => {
            return Err(OriginalsError::Config(
                "gs:// requires GOOGLE_APPLICATION_CREDENTIALS, a service account, or application-default credentials; no connection was opened"
                    .into(),
            ));
        }
        StoreKind::MicrosoftAzure if azure_account(settings, url).is_none() => {
            return Err(OriginalsError::Config(
                "az:// requires an account in AZURE_STORAGE_ACCOUNT_NAME or in an abfs://container@account URL; no connection was opened"
                    .into(),
            ));
        }
        StoreKind::MicrosoftAzure if !azure_has_secret(settings) => {
            return Err(OriginalsError::Config(
                "Azure needs an account key, SAS, bearer token, or workload identity; no connection was opened"
                    .into(),
            ));
        }
        StoreKind::AmazonS3 if settings.endpoint.is_some() && !aws_has_static_keys(settings) => {
            return Err(OriginalsError::Config(
                "a custom S3 endpoint requires EDGEQUAKE_ORIGINAL_STORE_ACCESS_KEY_ID and EDGEQUAKE_ORIGINAL_STORE_SECRET_ACCESS_KEY; no connection was opened"
                    .into(),
            ));
        }
        _ => {}
    }
    if settings.sse == Sse::Aes256 && kind != StoreKind::AmazonS3 {
        return Err(OriginalsError::Config(
            "EDGEQUAKE_OBJECT_STORE_SSE=AES256 applies to s3:// only".into(),
        ));
    }
    let _ = &settings.secret_key;
    Ok(())
}

fn build_object_blob(settings: &Settings) -> Result<ObjectBlob, OriginalsError> {
    let url = settings.url.as_deref().unwrap_or("");
    let (kind, bucket, prefix) = classify_url(url)?;
    // Validation already refused missing cloud credentials. Building is the
    // first moment a client exists, and it does not dial.
    let allow_http = settings.allow_http;
    match kind {
        StoreKind::AmazonS3 => {
            let s3 = build_s3(settings, &bucket)?;
            let signer: Arc<dyn object_store::signer::Signer> = Arc::new(s3.clone());
            let store: Arc<dyn ObjectStore> = Arc::new(LimitStore::new(s3, STORE_REQUEST_LIMIT));
            Ok(ObjectBlob::prefixed(store, prefix).with_signer(
                signer,
                Some(super::S3_CHECKSUM_HEADER),
                allow_http,
            ))
        }
        StoreKind::GoogleCloudStorage => {
            let gcs = build_gcs(settings, &bucket)?;
            let signer: Arc<dyn object_store::signer::Signer> = Arc::new(gcs.clone());
            let store: Arc<dyn ObjectStore> = Arc::new(LimitStore::new(gcs, STORE_REQUEST_LIMIT));
            Ok(ObjectBlob::prefixed(store, prefix).with_signer(signer, None, allow_http))
        }
        StoreKind::MicrosoftAzure => {
            let azure = build_azure(settings, url)?;
            let signer: Arc<dyn object_store::signer::Signer> = Arc::new(azure.clone());
            let store: Arc<dyn ObjectStore> = Arc::new(LimitStore::new(azure, STORE_REQUEST_LIMIT));
            // Headers make `signed_url_opts` return NotSupported. The caller relays.
            Ok(ObjectBlob::prefixed(store, prefix).with_signer(
                signer,
                Some(super::S3_CHECKSUM_HEADER),
                allow_http,
            ))
        }
    }
}

fn build_s3(
    settings: &Settings,
    bucket: &str,
) -> Result<object_store::aws::AmazonS3, OriginalsError> {
    let path_style = path_style_enabled(settings.endpoint.as_deref(), settings.path_style);
    let mut builder = AmazonS3Builder::new()
        .with_bucket_name(bucket)
        .with_region(&settings.region)
        .with_virtual_hosted_style_request(!path_style);
    if let Some(endpoint) = &settings.endpoint {
        builder = builder.with_endpoint(endpoint);
    }
    if settings.allow_http {
        builder = builder.with_allow_http(true);
    }
    if let (Some(key), Some(secret)) = (&settings.access_key, &settings.secret_key) {
        builder = builder
            .with_access_key_id(key)
            .with_secret_access_key(secret);
    }
    if let Some(token) = &settings.aws_session_token {
        builder = builder.with_token(token);
    }
    if settings.endpoint.is_none() {
        builder = builder.with_checksum_algorithm(Checksum::SHA256);
        if let Some(file) = &settings.aws_web_identity_token_file {
            builder = builder.with_config(AmazonS3ConfigKey::WebIdentityTokenFile, file);
        }
        if let Some(role) = &settings.aws_role_arn {
            builder = builder.with_config(AmazonS3ConfigKey::RoleArn, role);
        }
    }
    if settings.sse == Sse::Aes256 {
        let key: AmazonS3ConfigKey = "aws_server_side_encryption"
            .parse()
            .expect("sse config key");
        builder = builder.with_config(key, "AES256");
    }
    builder
        .build()
        .map_err(|err| OriginalsError::Config(format!("s3 client: {err}")))
}

fn build_gcs(
    settings: &Settings,
    bucket: &str,
) -> Result<object_store::gcp::GoogleCloudStorage, OriginalsError> {
    if !settings.gcs_credentials {
        return Err(OriginalsError::Config(
            "gs:// credentials missing before client build".into(),
        ));
    }
    let mut builder = GoogleCloudStorageBuilder::new().with_bucket_name(bucket);
    if let Some(url) = &settings.url {
        builder = builder.with_url(url);
    }
    if let Some(path) = &settings.gcs_service_account_path {
        builder = builder.with_service_account_path(path);
    }
    if let Some(key) = &settings.gcs_service_account_key {
        builder = builder.with_service_account_key(key);
    }
    if let Some(path) = &settings.gcs_application_credentials {
        builder = builder.with_application_credentials(path);
    }
    builder
        .build()
        .map_err(|err| OriginalsError::Config(format!("gcs client: {err}")))
}

fn build_azure(
    settings: &Settings,
    url: &str,
) -> Result<object_store::azure::MicrosoftAzure, OriginalsError> {
    let account = azure_account(settings, url).ok_or_else(|| {
        OriginalsError::Config("az:// account missing before client build".into())
    })?;
    if !azure_has_secret(settings) {
        return Err(OriginalsError::Config(
            "Azure credentials missing before client build".into(),
        ));
    }
    let (_kind, container, _prefix) = classify_url(url)?;
    let mut builder = object_store::azure::MicrosoftAzureBuilder::new()
        .with_url(url)
        .with_account(account)
        .with_container_name(container);
    if let Some(key) = &settings.azure_key {
        builder = builder.with_access_key(key);
    }
    if let Some(sas) = &settings.azure_sas {
        builder = builder.with_config(object_store::azure::AzureConfigKey::SasKey, sas);
    }
    if let Some(token) = &settings.azure_token {
        builder = builder.with_bearer_token_authorization(token);
    }
    if let (Some(client_id), Some(secret), Some(tenant)) = (
        &settings.azure_client_id,
        &settings.azure_client_secret,
        &settings.azure_tenant_id,
    ) {
        builder = builder.with_client_secret_authorization(client_id, secret, tenant);
    } else if let (Some(client_id), Some(tenant), Some(file)) = (
        &settings.azure_client_id,
        &settings.azure_tenant_id,
        &settings.azure_federated_token_file,
    ) {
        builder = builder
            .with_client_id(client_id)
            .with_tenant_id(tenant)
            .with_federated_token_file(file);
    }
    builder
        .build()
        .map_err(|err| OriginalsError::Config(format!("azure client: {err}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use object_store::ObjectStoreExt;

    #[test]
    fn parses_s3_gs_az_and_abfs() {
        assert_eq!(classify_url("s3://b/p").unwrap().0, StoreKind::AmazonS3);
        assert_eq!(
            classify_url("gs://b/p").unwrap().0,
            StoreKind::GoogleCloudStorage
        );
        assert_eq!(
            classify_url("az://c/p").unwrap().0,
            StoreKind::MicrosoftAzure
        );
        let (kind, container, prefix) =
            classify_url("abfs://c@account.dfs.core.windows.net/p").unwrap();
        assert_eq!(kind, StoreKind::MicrosoftAzure);
        assert_eq!(container, "c");
        assert_eq!(prefix, "p");
    }

    #[test]
    fn http_without_allow_http_is_config() {
        let mut settings = Settings {
            mode: StoreMode::Object,
            url: Some("s3://bucket/prefix".into()),
            endpoint: Some("http://localhost:9000".into()),
            ..Settings::default()
        };
        settings.allow_http = false;
        let err = validate_object(&settings).unwrap_err();
        assert_eq!(err.class_name(), "config");
        assert!(!err.to_string().contains("secret"));
    }

    #[test]
    fn kms_sse_is_config_error_and_aes256_is_ok() {
        assert!(sse_from_env(Some("kms")).is_err());
        assert_eq!(sse_from_env(None).unwrap(), Sse::Off);
        assert_eq!(sse_from_env(Some("AES256")).unwrap(), Sse::Aes256);
    }

    #[test]
    fn missing_cloud_credentials_fail_before_a_client() {
        let gs = Settings {
            mode: StoreMode::Object,
            url: Some("gs://bucket/prefix".into()),
            gcs_credentials: false,
            ..Settings::default()
        };
        let err = validate_object(&gs).unwrap_err();
        assert_eq!(err.class_name(), "config");
        let az = Settings {
            mode: StoreMode::Object,
            url: Some("az://container/prefix".into()),
            azure_account: None,
            azure_key: None,
            ..Settings::default()
        };
        assert_eq!(validate_object(&az).unwrap_err().class_name(), "config");
    }

    #[test]
    fn checksum_is_only_for_s3_without_an_endpoint() {
        assert!(store_checks_body(StoreKind::AmazonS3, None));
        assert!(!store_checks_body(
            StoreKind::AmazonS3,
            Some("http://127.0.0.1:9000")
        ));
        assert!(!store_checks_body(StoreKind::GoogleCloudStorage, None));
        assert!(!store_checks_body(StoreKind::MicrosoftAzure, None));
    }

    #[test]
    fn create_bucket_true_is_a_config_error() {
        let bare = Settings {
            mode: StoreMode::Object,
            url: Some("s3://bucket/prefix".into()),
            endpoint: None,
            create_bucket: true,
            ..Settings::default()
        };
        let err = validate_object(&bare).unwrap_err();
        assert_eq!(err.class_name(), "config");
        assert!(err.to_string().contains("does not create a bucket"));
        let with_endpoint = Settings {
            mode: StoreMode::Object,
            url: Some("s3://bucket/prefix".into()),
            endpoint: Some("http://127.0.0.1:9000".into()),
            allow_http: true,
            create_bucket: true,
            access_key: Some("minioadmin".into()),
            secret_key: Some("minioadmin123".into()),
            ..Settings::default()
        };
        assert_eq!(
            validate_object(&with_endpoint).unwrap_err().class_name(),
            "config"
        );
    }

    #[test]
    fn endpoint_defaults_to_path_style() {
        assert!(path_style_enabled(Some("http://localhost:9000"), None));
        assert!(!path_style_enabled(None, None));
        assert!(!path_style_enabled(
            Some("http://localhost:9000"),
            Some(false)
        ));
    }

    #[tokio::test]
    async fn database_mode_does_not_call_the_builder() {
        let settings = Settings::default();
        let opened = open_settings_with(&settings, |_| panic!("builder must not run"))
            .await
            .unwrap();
        assert!(opened.client.is_none());
        assert!(opened.bucket_ready);
    }

    #[test]
    fn custom_s3_endpoint_without_keys_does_not_build() {
        let settings = Settings {
            mode: StoreMode::Object,
            url: Some("s3://bucket/prefix".into()),
            endpoint: Some("https://minio.internal:9000".into()),
            ..Settings::default()
        };
        assert_eq!(
            validate_object(&settings).unwrap_err().class_name(),
            "config"
        );
    }

    #[test]
    fn abfs_account_in_the_url_satisfies_the_account_check() {
        let settings = Settings {
            mode: StoreMode::Object,
            url: Some("abfs://c@account.dfs.core.windows.net/p".into()),
            azure_key: Some("key".into()),
            ..Settings::default()
        };
        assert!(validate_object(&settings).is_ok());
    }

    #[test]
    fn azure_workload_identity_does_not_require_an_account_key() {
        let settings = Settings {
            mode: StoreMode::Object,
            url: Some("az://container/prefix".into()),
            azure_account: Some("acct".into()),
            azure_client_id: Some("id".into()),
            azure_client_secret: Some("secret".into()),
            azure_tenant_id: Some("tenant".into()),
            ..Settings::default()
        };
        assert!(validate_object(&settings).is_ok());
    }

    #[test]
    fn sse_aes256_is_s3_only() {
        let settings = Settings {
            mode: StoreMode::Object,
            url: Some("gs://bucket/prefix".into()),
            gcs_credentials: true,
            sse: Sse::Aes256,
            ..Settings::default()
        };
        let err = validate_object(&settings).unwrap_err();
        assert!(err.to_string().contains("s3://"));
    }

    #[test]
    fn builds_aws_gcs_and_azure_clients_without_a_socket() {
        let s3 = Settings {
            mode: StoreMode::Object,
            url: Some("s3://bucket/prefix".into()),
            region: "us-east-1".into(),
            ..Settings::default()
        };
        assert!(
            build_s3(&s3, "bucket").is_ok(),
            "real AWS with no static key uses the instance credential chain"
        );

        let gcs = Settings {
            mode: StoreMode::Object,
            url: Some("gs://bucket/prefix".into()),
            gcs_credentials: true,
            gcs_service_account_key: Some("{}".into()),
            ..Settings::default()
        };
        let err = build_gcs(&gcs, "bucket").unwrap_err();
        assert_eq!(err.class_name(), "config");
        assert!(err.to_string().contains("gcs"));

        let url = "az://container/prefix";
        let azure = Settings {
            mode: StoreMode::Object,
            url: Some(url.into()),
            azure_account: Some("devstoreaccount1".into()),
            azure_key: Some(
                "Eby8vdM02xNOcqFlqUwJPLlmEtlCDXJ1OUzFT50uSRZ6IFsuFq2UVErCz4I6tq/K1SZFPTOtr/KBHBeksoGMGw=="
                    .into(),
            ),
            ..Settings::default()
        };
        build_azure(&azure, url).expect("azure account key builds without a network call");
    }

    #[tokio::test]
    async fn closed_port_is_not_ready_and_hides_the_secret() {
        let settings = Settings {
            mode: StoreMode::Object,
            url: Some("s3://eq-originals/dev".into()),
            endpoint: Some("http://127.0.0.1:9".into()),
            allow_http: true,
            path_style: Some(true),
            access_key: Some("minioadmin".into()),
            secret_key: Some("super-secret-value".into()),
            region: "us-east-1".into(),
            ..Settings::default()
        };
        let opened = open_settings(&settings).await.unwrap();
        assert!(!opened.bucket_ready);
        let rendered = format!(
            "{} {}",
            opened.client.as_ref().unwrap().endpoint_host,
            opened.client.as_ref().unwrap().bucket
        );
        assert!(!rendered.contains("super-secret-value"));
        assert!(rendered.contains("127.0.0.1:9"));
        assert!(rendered.contains("eq-originals"));
    }

    #[tokio::test]
    async fn app_identity_cannot_leave_the_prefix() {
        if std::env::var("EDGEQUAKE_ORIGINAL_STORE_E2E")
            .ok()
            .as_deref()
            != Some("1")
        {
            return;
        }
        let endpoint = std::env::var("EDGEQUAKE_ORIGINAL_STORE_ENDPOINT")
            .unwrap_or_else(|_| "http://127.0.0.1:19011".into());
        let access = std::env::var("MINIO_APP_USER").unwrap_or_else(|_| "eq-originals-app".into());
        let secret =
            std::env::var("MINIO_APP_PASSWORD").unwrap_or_else(|_| "eq-originals-app-dev".into());
        let store = AmazonS3Builder::new()
            .with_bucket_name("eq-originals")
            .with_endpoint(&endpoint)
            .with_region("us-east-1")
            .with_access_key_id(&access)
            .with_secret_access_key(&secret)
            .with_allow_http(true)
            .with_virtual_hosted_style_request(false)
            .build()
            .expect("app client");
        let outside = object_store::path::Path::from("other/policy-canary");
        let missing = store.get(&outside).await.expect_err("outside prefix");
        let missing = missing.to_string();
        assert!(
            missing.to_ascii_lowercase().contains("accessdenied")
                || missing.contains("Access Denied"),
            "{missing}"
        );
        let probe = object_store::path::Path::from("dev/policy-probe");
        store
            .put(&probe, object_store::PutPayload::from_static(b"ok"))
            .await
            .expect("put under the prefix");
        store.delete(&probe).await.expect("delete under the prefix");
        let denied = store
            .put(
                &object_store::path::Path::from("other/app-write"),
                object_store::PutPayload::from_static(b"no"),
            )
            .await
            .expect_err("put outside the prefix");
        let denied = denied.to_string();
        assert!(
            denied.to_ascii_lowercase().contains("accessdenied")
                || denied.contains("Access Denied"),
            "{denied}"
        );

        let compose = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docker/docker-compose.yml"
        );
        let project =
            std::env::var("COMPOSE_PROJECT_NAME").unwrap_or_else(|_| "edgequake-dev".into());
        let host = format!("MC_HOST_app=http://{access}:{secret}@minio:9000");
        let output = std::process::Command::new("docker")
            .args([
                "compose",
                "-p",
                &project,
                "-f",
                compose,
                "--profile",
                "minio",
                "run",
                "--rm",
                "--no-deps",
                "-e",
                &host,
                "--entrypoint",
                "mc",
                "minio-init",
                "mb",
                "app/eq-originals-denied-probe",
            ])
            .output()
            .expect("docker compose");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!output.status.success(), "{text}");
        assert!(
            text.to_ascii_lowercase().contains("access denied") || text.contains("AccessDenied"),
            "{text}"
        );
    }
}
