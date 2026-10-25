//! Database secret backend. Values are AES-256-GCM envelopes. The store is injected
//! so unit tests use memory and the server uses PostgreSQL.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Mutex;

use crate::backend::{validate_secret_name, SecretBackend, SecretMeta, SecretRef};
use crate::{decrypt, encrypt, Envelope, SecretString, SecretsError};

/// Persistence for envelopes. One store is scoped to a single tenant.
#[async_trait]
pub trait EnvelopeStore: Send + Sync {
    async fn upsert(
        &self,
        name: &str,
        envelope: &Envelope,
        fingerprint: &str,
    ) -> Result<String, SecretsError>;

    async fn load(&self, id: &str) -> Result<Option<Envelope>, SecretsError>;

    async fn remove_by_id(&self, id: &str) -> Result<(), SecretsError>;

    async fn list(&self) -> Result<Vec<(String, String)>, SecretsError>;
}

/// In-memory store for tests and for callers that have no database.
#[derive(Default)]
pub struct MemoryEnvelopeStore {
    rows: Mutex<HashMap<String, MemoryRow>>,
}

struct MemoryRow {
    name: String,
    envelope: Envelope,
    fingerprint: String,
}

impl MemoryEnvelopeStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.rows.lock().expect("store").len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[async_trait]
impl EnvelopeStore for MemoryEnvelopeStore {
    async fn upsert(
        &self,
        name: &str,
        envelope: &Envelope,
        fingerprint: &str,
    ) -> Result<String, SecretsError> {
        let mut rows = self.rows.lock().expect("store");
        if let Some((id, row)) = rows.iter_mut().find(|(_, row)| row.name == name) {
            row.envelope = envelope.clone();
            row.fingerprint = fingerprint.to_string();
            return Ok(id.clone());
        }
        let id = crate::backend::uuid_token();
        rows.insert(
            id.clone(),
            MemoryRow {
                name: name.to_string(),
                envelope: envelope.clone(),
                fingerprint: fingerprint.to_string(),
            },
        );
        Ok(id)
    }

    async fn load(&self, id: &str) -> Result<Option<Envelope>, SecretsError> {
        Ok(self
            .rows
            .lock()
            .expect("store")
            .get(id)
            .map(|row| row.envelope.clone()))
    }

    async fn remove_by_id(&self, id: &str) -> Result<(), SecretsError> {
        self.rows.lock().expect("store").remove(id);
        Ok(())
    }

    async fn list(&self) -> Result<Vec<(String, String)>, SecretsError> {
        let rows = self.rows.lock().expect("store");
        Ok(rows
            .values()
            .map(|row| (row.name.clone(), row.fingerprint.clone()))
            .collect())
    }
}

/// Encrypts with the process key and delegates storage.
pub struct DatabaseSecretBackend<S: EnvelopeStore> {
    store: S,
    key: [u8; 32],
    key_id: String,
}

impl<S: EnvelopeStore> DatabaseSecretBackend<S> {
    pub fn new(store: S, key: [u8; 32], key_id: impl Into<String>) -> Self {
        Self {
            store,
            key,
            key_id: key_id.into(),
        }
    }

    /// Fails closed when `EDGEQUAKE_SECRETS_KEY` is unset. Does not generate a key.
    pub fn from_env(store: S) -> Result<Self, SecretsError> {
        let (key, key_id) = crate::key_from_env()?;
        Ok(Self::new(store, key, key_id))
    }

    pub fn store(&self) -> &S {
        &self.store
    }
}

#[async_trait]
impl<S: EnvelopeStore> SecretBackend for DatabaseSecretBackend<S> {
    fn kind(&self) -> &'static str {
        "database"
    }

    async fn put(&self, name: &str, value: &SecretString) -> Result<SecretRef, SecretsError> {
        validate_secret_name(name)?;
        let envelope = encrypt(value.expose().as_bytes(), &self.key, &self.key_id)?;
        let id = self
            .store
            .upsert(name, &envelope, &value.fingerprint())
            .await?;
        Ok(SecretRef::Database { id })
    }

    async fn get(&self, secret_ref: &SecretRef) -> Result<SecretString, SecretsError> {
        let SecretRef::Database { id } = secret_ref else {
            return Err(SecretsError::WrongBackend);
        };
        let Some(envelope) = self.store.load(id).await? else {
            return Err(SecretsError::NotFound);
        };
        let bytes = decrypt(&envelope, &self.key)?;
        let text = String::from_utf8(bytes).map_err(|_| SecretsError::Decrypt)?;
        Ok(SecretString::new(text))
    }

    async fn delete(&self, secret_ref: &SecretRef) -> Result<(), SecretsError> {
        let SecretRef::Database { id } = secret_ref else {
            return Err(SecretsError::WrongBackend);
        };
        self.store.remove_by_id(id).await
    }

    async fn list(&self) -> Result<Vec<SecretMeta>, SecretsError> {
        let rows = self.store.list().await?;
        Ok(rows
            .into_iter()
            .map(|(name, fingerprint)| SecretMeta {
                name,
                fingerprint: Some(fingerprint),
                backend: "database".into(),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::probe_backend;

    fn backend() -> DatabaseSecretBackend<MemoryEnvelopeStore> {
        DatabaseSecretBackend::new(MemoryEnvelopeStore::new(), [3u8; 32], "v1")
    }

    #[tokio::test]
    async fn round_trip_updates_the_same_name_and_lists_without_the_value() {
        let backend = backend();
        let first = backend
            .put("openai-prod", &SecretString::new("sk-one"))
            .await
            .unwrap();
        let second = backend
            .put("openai-prod", &SecretString::new("sk-two"))
            .await
            .unwrap();
        assert_eq!(first, second);
        assert_eq!(backend.get(&first).await.unwrap().expose(), "sk-two");
        assert_eq!(backend.store().len(), 1);
        let listed = backend.list().await.unwrap();
        assert_eq!(listed[0].name, "openai-prod");
        assert!(listed[0].fingerprint.is_some());
        let rendered = format!("{listed:?}");
        assert!(!rendered.contains("sk-two"));
        probe_backend(&backend).await.unwrap();
        backend.delete(&first).await.unwrap();
        assert!(matches!(
            backend.get(&first).await.unwrap_err(),
            SecretsError::NotFound
        ));
    }

    #[tokio::test]
    async fn rejects_a_reference_from_another_backend() {
        let backend = backend();
        let foreign = SecretRef::Infisical {
            project_id: "p".into(),
            environment: "dev".into(),
            secret_path: "/".into(),
            name: "k".into(),
        };
        assert!(matches!(
            backend.get(&foreign).await.unwrap_err(),
            SecretsError::WrongBackend
        ));
    }

    #[tokio::test]
    async fn missing_process_key_fails_closed() {
        std::env::remove_var(crate::ENV_SECRETS_KEY);
        match DatabaseSecretBackend::from_env(MemoryEnvelopeStore::new()) {
            Err(SecretsError::MissingKey) => {}
            Err(other) => panic!("expected a missing key, got {other}"),
            Ok(_) => panic!("expected a missing key"),
        }
    }
}
