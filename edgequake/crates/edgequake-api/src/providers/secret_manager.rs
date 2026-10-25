//! Tenant secret service. The database backend is the default and needs no
//! Infisical settings. An Infisical client is built only when that tenant's
//! config says `infisical`, and a failed Infisical call is never written to
//! the database instead.

use async_trait::async_trait;
use edgequake_secrets::{
    probe_backend, DatabaseSecretBackend, Envelope, EnvelopeStore, InfisicalConfig,
    InfisicalSecretBackend, SecretBackend, SecretMeta, SecretRef, SecretString, SecretsError,
    KEY_REQUIRED_MESSAGE,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiError;
use crate::ssrf::{validate_provider_url, SsrfPolicy};

/// Requests with no tenant share this bucket. It is not a real tenant.
pub fn tenant_bucket(tenant: Option<Uuid>) -> Uuid {
    tenant.unwrap_or(Uuid::nil())
}

/// Pure gate used by saves. A missing key refuses the save and does not stop the server.
pub fn database_key_refusal(configured: bool) -> Option<&'static str> {
    if configured {
        None
    } else {
        Some(KEY_REQUIRED_MESSAGE)
    }
}

fn map_secret(err: SecretsError) -> ApiError {
    match err {
        SecretsError::MissingKey => ApiError::BadRequest(KEY_REQUIRED_MESSAGE.into()),
        SecretsError::NotFound => ApiError::NotFound("secret not found".into()),
        SecretsError::Infisical { status, message } if status == 0 || status >= 500 => {
            ApiError::ServiceUnavailable {
                message,
                retry_after_secs: 5,
            }
        }
        other => ApiError::BadRequest(other.to_string()),
    }
}

struct PgEnvelopeStore {
    pool: PgPool,
    tenant_id: Uuid,
}

#[async_trait]
impl EnvelopeStore for PgEnvelopeStore {
    async fn upsert(
        &self,
        name: &str,
        envelope: &Envelope,
        fingerprint: &str,
    ) -> Result<String, SecretsError> {
        let id: Uuid = sqlx::query_scalar(
            r#"INSERT INTO managed_secrets (tenant_id, name, ciphertext, nonce, key_id, fingerprint)
               VALUES ($1, $2, $3, $4, $5, $6)
               ON CONFLICT (tenant_id, name) DO UPDATE
                 SET ciphertext = EXCLUDED.ciphertext,
                     nonce = EXCLUDED.nonce,
                     key_id = EXCLUDED.key_id,
                     fingerprint = EXCLUDED.fingerprint,
                     updated_at = now()
               RETURNING id"#,
        )
        .bind(self.tenant_id)
        .bind(name)
        .bind(&envelope.ciphertext)
        .bind(&envelope.nonce)
        .bind(&envelope.key_id)
        .bind(fingerprint)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| SecretsError::Store(e.to_string()))?;
        Ok(id.to_string())
    }

    async fn load(&self, id: &str) -> Result<Option<Envelope>, SecretsError> {
        let Ok(id) = Uuid::parse_str(id) else {
            return Ok(None);
        };
        let row = sqlx::query_as::<_, EnvelopeRow>(
            r#"SELECT ciphertext, nonce, key_id
               FROM managed_secrets
               WHERE id = $1 AND tenant_id = $2"#,
        )
        .bind(id)
        .bind(self.tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| SecretsError::Store(e.to_string()))?;
        Ok(row.map(|row| Envelope {
            ciphertext: row.ciphertext,
            nonce: row.nonce,
            key_id: row.key_id,
        }))
    }

    async fn remove_by_id(&self, id: &str) -> Result<(), SecretsError> {
        let Ok(id) = Uuid::parse_str(id) else {
            return Ok(());
        };
        sqlx::query("DELETE FROM managed_secrets WHERE id = $1 AND tenant_id = $2")
            .bind(id)
            .bind(self.tenant_id)
            .execute(&self.pool)
            .await
            .map_err(|e| SecretsError::Store(e.to_string()))?;
        Ok(())
    }

    async fn list(&self) -> Result<Vec<(String, String)>, SecretsError> {
        let rows = sqlx::query_as::<_, NameRow>(
            r#"SELECT name, fingerprint FROM managed_secrets
               WHERE tenant_id = $1 AND name NOT LIKE 'eq-probe-%'
               ORDER BY name"#,
        )
        .bind(self.tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| SecretsError::Store(e.to_string()))?;
        Ok(rows
            .into_iter()
            .map(|row| (row.name, row.fingerprint))
            .collect())
    }
}

#[derive(sqlx::FromRow)]
struct EnvelopeRow {
    ciphertext: Vec<u8>,
    nonce: Vec<u8>,
    key_id: String,
}

#[derive(sqlx::FromRow)]
struct NameRow {
    name: String,
    fingerprint: String,
}

#[derive(sqlx::FromRow)]
struct BackendRow {
    kind: String,
    infisical_base_url: Option<String>,
    infisical_client_id: Option<String>,
    infisical_project_id: Option<String>,
    infisical_environment: Option<String>,
    infisical_secret_path: String,
    allow_private_network: bool,
    client_secret_ciphertext: Option<Vec<u8>>,
    client_secret_nonce: Option<Vec<u8>>,
    client_secret_key_id: Option<String>,
}

/// What the settings screen may show. The client secret is never included.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BackendView {
    pub kind: String,
    pub secrets_key_configured: bool,
    pub infisical: Option<InfisicalView>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct InfisicalView {
    pub base_url: String,
    pub client_id: String,
    pub project_id: String,
    pub environment: String,
    pub secret_path: String,
    pub allow_private_network: bool,
    pub client_secret_configured: bool,
}

#[derive(Debug, Clone)]
pub struct SetBackend {
    pub kind: String,
    pub base_url: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub project_id: Option<String>,
    pub environment: Option<String>,
    pub secret_path: Option<String>,
    pub allow_private_network: bool,
}

pub struct SecretService {
    pool: PgPool,
}

impl SecretService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn database(&self, tenant: Uuid) -> Result<DatabaseSecretBackend<PgEnvelopeStore>, ApiError> {
        if database_key_refusal(edgequake_secrets::secrets_configured()).is_some() {
            return Err(ApiError::BadRequest(KEY_REQUIRED_MESSAGE.into()));
        }
        DatabaseSecretBackend::from_env(PgEnvelopeStore {
            pool: self.pool.clone(),
            tenant_id: tenant,
        })
        .map_err(map_secret)
    }

    async fn load_row(&self, tenant: Uuid) -> Result<Option<BackendRow>, ApiError> {
        sqlx::query_as::<_, BackendRow>(
            r#"SELECT kind, infisical_base_url, infisical_client_id, infisical_project_id,
                      infisical_environment, infisical_secret_path, allow_private_network,
                      client_secret_ciphertext, client_secret_nonce, client_secret_key_id
               FROM secret_backend_configs WHERE tenant_id = $1"#,
        )
        .bind(tenant)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))
    }

    async fn kind_of(&self, tenant: Uuid) -> Result<String, ApiError> {
        Ok(self
            .load_row(tenant)
            .await?
            .map(|row| row.kind)
            .unwrap_or_else(|| "database".into()))
    }

    async fn infisical(&self, tenant: Uuid) -> Result<InfisicalSecretBackend, ApiError> {
        let row = self.load_row(tenant).await?.ok_or_else(|| {
            ApiError::BadRequest(
                "Infisical is not configured for this tenant. Keys stay in the database until you save Infisical settings."
                    .into(),
            )
        })?;
        let (Some(base_url), Some(client_id), Some(project_id), Some(environment)) = (
            row.infisical_base_url,
            row.infisical_client_id,
            row.infisical_project_id,
            row.infisical_environment,
        ) else {
            return Err(ApiError::BadRequest(
                "Infisical settings are incomplete".into(),
            ));
        };
        let (Some(ciphertext), Some(nonce), Some(key_id)) = (
            row.client_secret_ciphertext,
            row.client_secret_nonce,
            row.client_secret_key_id,
        ) else {
            return Err(ApiError::BadRequest(
                "Infisical client secret is not stored".into(),
            ));
        };
        let client_secret = edgequake_secrets::decrypt_string(&Envelope {
            ciphertext,
            nonce,
            key_id,
        })
        .map_err(|_| {
            ApiError::BadRequest(
                "Could not read the Infisical client secret. Check EDGEQUAKE_SECRETS_KEY.".into(),
            )
        })?;
        InfisicalSecretBackend::new(InfisicalConfig {
            base_url,
            client_id,
            client_secret,
            project_id,
            environment,
            secret_path: row.infisical_secret_path,
        })
        .map_err(map_secret)
    }

    pub async fn put(
        &self,
        tenant: Option<Uuid>,
        name: &str,
        value: &str,
    ) -> Result<SecretRef, ApiError> {
        let bucket = tenant_bucket(tenant);
        let secret = SecretString::new(value);
        match self.kind_of(bucket).await?.as_str() {
            "infisical" => self
                .infisical(bucket)
                .await?
                .put(name, &secret)
                .await
                .map_err(map_secret),
            _ => self
                .database(bucket)?
                .put(name, &secret)
                .await
                .map_err(map_secret),
        }
    }

    pub async fn get(
        &self,
        tenant: Option<Uuid>,
        secret_ref: &SecretRef,
    ) -> Result<String, ApiError> {
        let bucket = tenant_bucket(tenant);
        let secret = match secret_ref {
            SecretRef::Database { .. } => self.database(bucket)?.get(secret_ref).await,
            SecretRef::Infisical { .. } => self.infisical(bucket).await?.get(secret_ref).await,
        }
        .map_err(map_secret)?;
        Ok(secret.expose().to_string())
    }

    pub async fn delete_ref(
        &self,
        tenant: Option<Uuid>,
        secret_ref: &SecretRef,
    ) -> Result<(), ApiError> {
        let bucket = tenant_bucket(tenant);
        match secret_ref {
            SecretRef::Database { .. } => self.database(bucket)?.delete(secret_ref).await,
            SecretRef::Infisical { .. } => self.infisical(bucket).await?.delete(secret_ref).await,
        }
        .map_err(map_secret)
    }

    pub async fn list(&self, tenant: Option<Uuid>) -> Result<Vec<SecretMeta>, ApiError> {
        let bucket = tenant_bucket(tenant);
        match self.kind_of(bucket).await?.as_str() {
            "infisical" => self
                .infisical(bucket)
                .await?
                .list()
                .await
                .map_err(map_secret),
            _ => self.database(bucket)?.list().await.map_err(map_secret),
        }
    }

    pub async fn delete_named(&self, tenant: Option<Uuid>, name: &str) -> Result<(), ApiError> {
        let bucket = tenant_bucket(tenant);
        if self.kind_of(bucket).await? == "infisical" {
            let row = self
                .load_row(bucket)
                .await?
                .ok_or_else(|| ApiError::BadRequest("Infisical is not configured".into()))?;
            let secret_ref = SecretRef::Infisical {
                project_id: row.infisical_project_id.unwrap_or_default(),
                environment: row.infisical_environment.unwrap_or_default(),
                secret_path: row.infisical_secret_path,
                name: name.to_string(),
            };
            return self.delete_ref(tenant, &secret_ref).await;
        }
        let id: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM managed_secrets WHERE tenant_id = $1 AND name = $2")
                .bind(bucket)
                .bind(name)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?;
        let Some(id) = id else {
            return Err(ApiError::NotFound("secret not found".into()));
        };
        self.delete_ref(tenant, &SecretRef::Database { id: id.to_string() })
            .await
    }

    pub async fn view(&self, tenant: Option<Uuid>) -> Result<BackendView, ApiError> {
        let bucket = tenant_bucket(tenant);
        let row = self.load_row(bucket).await?;
        Ok(view_from(row.as_ref()))
    }

    pub async fn set_backend(
        &self,
        tenant: Option<Uuid>,
        body: SetBackend,
    ) -> Result<BackendView, ApiError> {
        let bucket = tenant_bucket(tenant);
        match body.kind.as_str() {
            "database" => {
                sqlx::query(
                    r#"INSERT INTO secret_backend_configs (tenant_id, kind)
                       VALUES ($1, 'database')
                       ON CONFLICT (tenant_id) DO UPDATE
                         SET kind = 'database', updated_at = now()"#,
                )
                .bind(bucket)
                .execute(&self.pool)
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?;
            }
            "infisical" => self.save_infisical(bucket, &body).await?,
            other => {
                return Err(ApiError::BadRequest(format!(
                    "unknown secret backend '{other}'"
                )));
            }
        }
        self.view(tenant).await
    }

    async fn save_infisical(&self, bucket: Uuid, body: &SetBackend) -> Result<(), ApiError> {
        let base_url = required(&body.base_url, "base URL")?;
        let client_id = required(&body.client_id, "client ID")?;
        let project_id = required(&body.project_id, "project ID")?;
        let environment = required(&body.environment, "environment")?;
        let secret_path = body
            .secret_path
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "/".into());
        validate_provider_url(
            &base_url,
            SsrfPolicy {
                allow_private: body.allow_private_network,
            },
        )
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
        crate::ssrf::enforce_resolved_addresses(
            &base_url,
            SsrfPolicy {
                allow_private: body.allow_private_network,
            },
        )
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

        let existing = self.load_row(bucket).await?;
        let (ciphertext, nonce, key_id) = if let Some(secret) =
            body.client_secret.as_deref().filter(|s| !s.is_empty())
        {
            if database_key_refusal(edgequake_secrets::secrets_configured()).is_some() {
                return Err(ApiError::BadRequest(KEY_REQUIRED_MESSAGE.into()));
            }
            let envelope = edgequake_secrets::encrypt_string(secret).map_err(map_secret)?;
            (
                Some(envelope.ciphertext),
                Some(envelope.nonce),
                Some(envelope.key_id),
            )
        } else {
            let row = existing.as_ref().ok_or_else(|| {
                ApiError::BadRequest("Infisical client secret is required the first time".into())
            })?;
            if row.client_secret_ciphertext.is_none() {
                return Err(ApiError::BadRequest(
                    "Infisical client secret is required the first time".into(),
                ));
            }
            (
                row.client_secret_ciphertext.clone(),
                row.client_secret_nonce.clone(),
                row.client_secret_key_id.clone(),
            )
        };

        sqlx::query(
            r#"INSERT INTO secret_backend_configs (
                   tenant_id, kind, infisical_base_url, infisical_client_id, infisical_project_id,
                   infisical_environment, infisical_secret_path, allow_private_network,
                   client_secret_ciphertext, client_secret_nonce, client_secret_key_id
               ) VALUES ($1, 'infisical', $2, $3, $4, $5, $6, $7, $8, $9, $10)
               ON CONFLICT (tenant_id) DO UPDATE SET
                   kind = 'infisical',
                   infisical_base_url = EXCLUDED.infisical_base_url,
                   infisical_client_id = EXCLUDED.infisical_client_id,
                   infisical_project_id = EXCLUDED.infisical_project_id,
                   infisical_environment = EXCLUDED.infisical_environment,
                   infisical_secret_path = EXCLUDED.infisical_secret_path,
                   allow_private_network = EXCLUDED.allow_private_network,
                   client_secret_ciphertext = EXCLUDED.client_secret_ciphertext,
                   client_secret_nonce = EXCLUDED.client_secret_nonce,
                   client_secret_key_id = EXCLUDED.client_secret_key_id,
                   updated_at = now()"#,
        )
        .bind(bucket)
        .bind(base_url)
        .bind(client_id)
        .bind(project_id)
        .bind(environment)
        .bind(secret_path)
        .bind(body.allow_private_network)
        .bind(ciphertext.as_deref())
        .bind(nonce.as_deref())
        .bind(key_id.as_deref())
        .execute(&self.pool)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
        Ok(())
    }

    pub async fn test(&self, tenant: Option<Uuid>) -> Result<(), ApiError> {
        let bucket = tenant_bucket(tenant);
        match self.kind_of(bucket).await?.as_str() {
            "infisical" => {
                let backend = self.infisical(bucket).await?;
                probe_backend(&backend).await.map_err(map_secret)
            }
            _ => {
                let backend = self.database(bucket)?;
                probe_backend(&backend).await.map_err(map_secret)
            }
        }
    }
}

fn required(value: &Option<String>, label: &str) -> Result<String, ApiError> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .ok_or_else(|| ApiError::BadRequest(format!("Infisical {label} is required")))
}

fn view_from(row: Option<&BackendRow>) -> BackendView {
    let infisical = row.and_then(|row| {
        Some(InfisicalView {
            base_url: row.infisical_base_url.clone()?,
            client_id: row.infisical_client_id.clone()?,
            project_id: row.infisical_project_id.clone()?,
            environment: row.infisical_environment.clone()?,
            secret_path: row.infisical_secret_path.clone(),
            allow_private_network: row.allow_private_network,
            client_secret_configured: row.client_secret_ciphertext.is_some(),
        })
    });
    BackendView {
        kind: row
            .map(|row| row.kind.clone())
            .unwrap_or_else(|| "database".into()),
        secrets_key_configured: edgequake_secrets::secrets_configured(),
        infisical,
    }
}

/// Resolve a connection key. A `secret_ref` wins. Older rows still use the inline envelope.
/// An Infisical failure is returned. It is not replaced with a database copy.
pub async fn reveal_connection_key(
    pool: &PgPool,
    tenant_id: Option<Uuid>,
    secret_ref: Option<serde_json::Value>,
    ciphertext: Option<Vec<u8>>,
    nonce: Option<Vec<u8>>,
    key_id: Option<String>,
) -> Result<Option<String>, ApiError> {
    if let Some(value) = secret_ref {
        let reference: SecretRef = serde_json::from_value(value)
            .map_err(|e| ApiError::Internal(format!("invalid secret reference: {e}")))?;
        return SecretService::new(pool.clone())
            .get(tenant_id, &reference)
            .await
            .map(Some);
    }
    let Some(kid) = key_id else {
        return Ok(None);
    };
    let (Some(ciphertext), Some(nonce)) = (ciphertext, nonce) else {
        return Ok(None);
    };
    Ok(edgequake_secrets::decrypt_string(&Envelope {
        ciphertext,
        nonce,
        key_id: kid,
    })
    .ok()
    .map(|secret| secret.expose().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_key_refuses_the_save_and_names_the_variable() {
        let message = database_key_refusal(false).expect("refused");
        assert!(message.contains("EDGEQUAKE_SECRETS_KEY"));
        assert!(database_key_refusal(true).is_none());
    }
}
