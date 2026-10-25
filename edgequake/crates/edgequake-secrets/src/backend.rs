//! Secret backends. The database backend is the default. Infisical is optional
//! and is only constructed when a tenant selects it.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::SecretsError;

/// Shown when a database-backed key cannot be written. The server still boots.
pub const KEY_REQUIRED_MESSAGE: &str = "EDGEQUAKE_SECRETS_KEY is required to store API keys";

/// Where a secret value lives. Stored on the provider connection; the value is not.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "backend", rename_all = "snake_case")]
pub enum SecretRef {
    Database {
        id: String,
    },
    Infisical {
        project_id: String,
        environment: String,
        secret_path: String,
        name: String,
    },
}

impl SecretRef {
    pub fn backend_name(&self) -> &'static str {
        match self {
            Self::Database { .. } => "database",
            Self::Infisical { .. } => "infisical",
        }
    }
}

/// A secret the caller may list. Never includes the value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretMeta {
    pub name: String,
    pub fingerprint: Option<String>,
    pub backend: String,
}

/// `put` writes a value and returns a reference. `get` reads it back.
/// Implementations must not log the value.
#[async_trait]
pub trait SecretBackend: Send + Sync {
    fn kind(&self) -> &'static str;

    async fn put(&self, name: &str, value: &crate::SecretString)
        -> Result<SecretRef, SecretsError>;

    async fn get(&self, secret_ref: &SecretRef) -> Result<crate::SecretString, SecretsError>;

    async fn delete(&self, secret_ref: &SecretRef) -> Result<(), SecretsError>;

    async fn list(&self) -> Result<Vec<SecretMeta>, SecretsError>;
}

/// Write a throwaway value, read it back, and delete it.
pub async fn probe_backend(backend: &dyn SecretBackend) -> Result<(), SecretsError> {
    let name = format!("eq-probe-{}", &uuid_token()[..12]);
    let value = crate::SecretString::new("edgequake-probe");
    let stored = backend.put(&name, &value).await?;
    let got = backend.get(&stored).await?;
    let deleted = backend.delete(&stored).await;
    if got.expose() != value.expose() {
        return Err(SecretsError::Decrypt);
    }
    deleted
}

/// Letters, digits, and `._-`. Rejects path separators so a name cannot escape a URL segment.
pub fn validate_secret_name(name: &str) -> Result<(), SecretsError> {
    let ok = !name.is_empty()
        && name.len() <= 128
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    if ok {
        Ok(())
    } else {
        Err(SecretsError::InvalidName)
    }
}

pub(crate) fn uuid_token() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
