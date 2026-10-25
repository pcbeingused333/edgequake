//! Infisical secret backend (optional).
//!
//! Universal Auth: `POST /api/v1/auth/universal-auth/login`.
//! Secrets: `POST|GET|PATCH|DELETE /api/v4/secrets/{name}` (Infisical API v4).
//!
//! Nothing in this module runs at process start. Callers build a client only
//! when a tenant's backend is Infisical. The product works with no Infisical
//! configuration at all.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::backend::{validate_secret_name, SecretBackend, SecretMeta, SecretRef};
use crate::{SecretString, SecretsError};

/// Non-secret coordinates plus the Universal Auth client secret (never logged).
pub struct InfisicalConfig {
    pub base_url: String,
    pub client_id: String,
    pub client_secret: SecretString,
    pub project_id: String,
    pub environment: String,
    pub secret_path: String,
}

struct CachedToken {
    value: String,
    refresh_at: Instant,
}

/// Talks to one Infisical project. Token refresh is lazy, on the next call.
pub struct InfisicalSecretBackend {
    http: reqwest::Client,
    config: InfisicalConfig,
    token: Mutex<Option<CachedToken>>,
}

impl InfisicalSecretBackend {
    pub fn new(config: InfisicalConfig) -> Result<Self, SecretsError> {
        let base = config.base_url.trim().trim_end_matches('/');
        if !(base.starts_with("https://") || base.starts_with("http://")) || base.len() < 8 {
            return Err(SecretsError::Infisical {
                status: 0,
                message: "base URL must be http or https".into(),
            });
        }
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| SecretsError::Infisical {
                status: 0,
                message: e.to_string(),
            })?;
        Ok(Self {
            http,
            config: InfisicalConfig {
                base_url: base.to_string(),
                ..config
            },
            token: Mutex::new(None),
        })
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.config.base_url)
    }

    async fn login(&self) -> Result<String, SecretsError> {
        let response = self
            .http
            .post(self.url("/api/v1/auth/universal-auth/login"))
            .json(&json!({
                "clientId": self.config.client_id,
                "clientSecret": self.config.client_secret.expose(),
            }))
            .send()
            .await
            .map_err(transport_error)?;
        let status = response.status().as_u16();
        let body = response.text().await.map_err(transport_error)?;
        if !(200..300).contains(&status) {
            return Err(SecretsError::Infisical {
                status,
                message: clip_message(&body),
            });
        }
        let parsed: LoginResponse =
            serde_json::from_str(&body).map_err(|_| SecretsError::Infisical {
                status,
                message: "login response was not a token".into(),
            })?;
        let expires = parsed.expires_in.max(0) as u64;
        let skew = 30.min(expires / 2);
        let refresh_at = Instant::now() + Duration::from_secs(expires.saturating_sub(skew));
        *self.token.lock().expect("token") = Some(CachedToken {
            value: parsed.access_token.clone(),
            refresh_at,
        });
        Ok(parsed.access_token)
    }

    async fn bearer(&self) -> Result<String, SecretsError> {
        let current = self
            .token
            .lock()
            .expect("token")
            .as_ref()
            .filter(|t| Instant::now() < t.refresh_at)
            .map(|t| t.value.clone());
        if let Some(token) = current {
            return Ok(token);
        }
        self.login().await
    }

    fn location_query(&self, reference: &SecretRef) -> Result<Vec<(&str, String)>, SecretsError> {
        let SecretRef::Infisical {
            project_id,
            environment,
            secret_path,
            ..
        } = reference
        else {
            return Err(SecretsError::WrongBackend);
        };
        Ok(vec![
            ("projectId", project_id.clone()),
            ("environment", environment.clone()),
            ("secretPath", secret_path.clone()),
        ])
    }

    fn write_body(&self, value: &str) -> serde_json::Value {
        json!({
            "projectId": self.config.project_id,
            "environment": self.config.environment,
            "secretPath": self.config.secret_path,
            "secretValue": value,
            "type": "shared",
        })
    }

    async fn send(
        &self,
        method: reqwest::Method,
        url: String,
        query: Vec<(&str, String)>,
        body: Option<serde_json::Value>,
        retry_unauthorized: bool,
    ) -> Result<(u16, String), SecretsError> {
        let token = self.bearer().await?;
        let mut request = self.http.request(method.clone(), &url).bearer_auth(&token);
        if !query.is_empty() {
            request = request.query(&query);
        }
        if let Some(body) = body.clone() {
            request = request.json(&body);
        }
        let response = request.send().await.map_err(transport_error)?;
        let status = response.status().as_u16();
        if status == 401 && retry_unauthorized {
            *self.token.lock().expect("token") = None;
            return Box::pin(self.send(method, url, query, body, false)).await;
        }
        let text = response.text().await.map_err(transport_error)?;
        Ok((status, text))
    }
}

#[async_trait]
impl SecretBackend for InfisicalSecretBackend {
    fn kind(&self) -> &'static str {
        "infisical"
    }

    async fn put(&self, name: &str, value: &SecretString) -> Result<SecretRef, SecretsError> {
        validate_secret_name(name)?;
        let url = self.url(&format!("/api/v4/secrets/{}", urlencoding_segment(name)));
        let body = self.write_body(value.expose());
        let (status, text) = self
            .send(
                reqwest::Method::POST,
                url.clone(),
                vec![],
                Some(body.clone()),
                true,
            )
            .await?;
        if !(200..300).contains(&status) {
            // A second save updates the existing secret instead of failing the connection.
            let (patched, patched_body) = self
                .send(reqwest::Method::PATCH, url, vec![], Some(body), true)
                .await?;
            if !(200..300).contains(&patched) {
                return Err(SecretsError::Infisical {
                    status: patched,
                    message: clip_message(&patched_body),
                });
            }
            let _ = text;
        }
        Ok(SecretRef::Infisical {
            project_id: self.config.project_id.clone(),
            environment: self.config.environment.clone(),
            secret_path: self.config.secret_path.clone(),
            name: name.to_string(),
        })
    }

    async fn get(&self, secret_ref: &SecretRef) -> Result<SecretString, SecretsError> {
        let SecretRef::Infisical { name, .. } = secret_ref else {
            return Err(SecretsError::WrongBackend);
        };
        let url = self.url(&format!("/api/v4/secrets/{}", urlencoding_segment(name)));
        let query = self.location_query(secret_ref)?;
        let (status, text) = self
            .send(reqwest::Method::GET, url, query, None, true)
            .await?;
        if status == 404 {
            return Err(SecretsError::NotFound);
        }
        if !(200..300).contains(&status) {
            return Err(SecretsError::Infisical {
                status,
                message: clip_message(&text),
            });
        }
        let parsed: SecretEnvelope =
            serde_json::from_str(&text).map_err(|_| SecretsError::Infisical {
                status,
                message: "secret response had no value".into(),
            })?;
        Ok(SecretString::new(parsed.secret.secret_value))
    }

    async fn delete(&self, secret_ref: &SecretRef) -> Result<(), SecretsError> {
        let SecretRef::Infisical { name, .. } = secret_ref else {
            return Err(SecretsError::WrongBackend);
        };
        let url = self.url(&format!("/api/v4/secrets/{}", urlencoding_segment(name)));
        let query = self.location_query(secret_ref)?;
        let (status, text) = self
            .send(reqwest::Method::DELETE, url, query, None, true)
            .await?;
        if status == 404 || (200..300).contains(&status) {
            return Ok(());
        }
        Err(SecretsError::Infisical {
            status,
            message: clip_message(&text),
        })
    }

    async fn list(&self) -> Result<Vec<SecretMeta>, SecretsError> {
        let url = self.url("/api/v4/secrets");
        let query = vec![
            ("projectId", self.config.project_id.clone()),
            ("environment", self.config.environment.clone()),
            ("secretPath", self.config.secret_path.clone()),
        ];
        let (status, text) = self
            .send(reqwest::Method::GET, url, query, None, true)
            .await?;
        if !(200..300).contains(&status) {
            return Err(SecretsError::Infisical {
                status,
                message: clip_message(&text),
            });
        }
        let parsed: SecretList =
            serde_json::from_str(&text).unwrap_or(SecretList { secrets: vec![] });
        Ok(parsed
            .secrets
            .into_iter()
            .map(|row| SecretMeta {
                name: row.secret_key,
                fingerprint: None,
                backend: "infisical".into(),
            })
            .collect())
    }
}

#[derive(Deserialize)]
struct LoginResponse {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "expiresIn")]
    expires_in: i64,
}

#[derive(Deserialize)]
struct SecretEnvelope {
    secret: SecretBody,
}

#[derive(Deserialize)]
struct SecretBody {
    #[serde(rename = "secretValue")]
    secret_value: String,
}

#[derive(Deserialize, Default)]
struct SecretList {
    #[serde(default)]
    secrets: Vec<SecretListItem>,
}

#[derive(Deserialize)]
struct SecretListItem {
    #[serde(rename = "secretKey")]
    secret_key: String,
}

fn transport_error(err: reqwest::Error) -> SecretsError {
    SecretsError::Infisical {
        status: 0,
        message: format!("Infisical is unreachable ({err})"),
    }
}

/// Keep upstream error text short and free of any echoed secret value we might have sent.
fn clip_message(body: &str) -> String {
    let message = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| {
            v.get("message")
                .and_then(|m| m.as_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| body.chars().take(180).collect());
    message.chars().take(180).collect()
}

fn urlencoding_segment(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' => c.to_string(),
            other => format!("%{:02X}", other as u32),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;

    fn config(base: &str) -> InfisicalConfig {
        InfisicalConfig {
            base_url: base.into(),
            client_id: "client".into(),
            client_secret: SecretString::new("client-secret"),
            project_id: "proj".into(),
            environment: "dev".into(),
            secret_path: "/".into(),
        }
    }

    type FakeHandler = Arc<dyn Fn(&str, &str, &str) -> (u16, String) + Send + Sync>;

    /// One HTTP/1.1 request/response per connection. The handler sees method, path, and body.
    fn serve(handler: FakeHandler) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let handler = handler.clone();
                thread::spawn(move || {
                    let mut stream = stream;
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                    if let Some((method, path, body)) = read_request(&mut stream) {
                        let (status, response) = handler(&method, &path, &body);
                        let raw = format!(
                            "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}",
                            response.len()
                        );
                        let _ = stream.write_all(raw.as_bytes());
                    }
                });
            }
        });
        format!("http://{addr}")
    }

    fn read_request(stream: &mut std::net::TcpStream) -> Option<(String, String, String)> {
        let mut buf = Vec::new();
        let mut tmp = [0u8; 2048];
        let header_end;
        loop {
            let n = stream.read(&mut tmp).ok()?;
            if n == 0 {
                return None;
            }
            buf.extend_from_slice(&tmp[..n]);
            if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                header_end = pos;
                break;
            }
            if buf.len() > 64 * 1024 {
                return None;
            }
        }
        let head = String::from_utf8_lossy(&buf[..header_end]).to_string();
        let mut lines = head.lines();
        let request = lines.next().unwrap_or("");
        let mut parts = request.split_whitespace();
        let method = parts.next()?.to_string();
        let path = parts.next()?.to_string();
        let length = head
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                if name.eq_ignore_ascii_case("content-length") {
                    value.trim().parse::<usize>().ok()
                } else {
                    None
                }
            })
            .unwrap_or(0);
        while buf.len() < header_end + 4 + length {
            let n = stream.read(&mut tmp).ok()?;
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&tmp[..n]);
        }
        let body = String::from_utf8_lossy(&buf[header_end + 4..]).to_string();
        Some((method, path, body))
    }

    #[tokio::test]
    async fn round_trip_against_a_fake_infisical() {
        let store = Arc::new(Mutex::new(
            std::collections::HashMap::<String, String>::new(),
        ));
        let logins = Arc::new(AtomicUsize::new(0));
        let store_h = store.clone();
        let logins_h = logins.clone();
        let base = serve(Arc::new(move |method, path, body| {
            if path.starts_with("/api/v1/auth/universal-auth/login") {
                logins_h.fetch_add(1, Ordering::SeqCst);
                assert!(body.contains("client-secret"));
                return (
                    200,
                    r#"{"accessToken":"tok","expiresIn":3600,"accessTokenMaxTTL":7200,"tokenType":"Bearer"}"#.into(),
                );
            }
            let name = path.trim_start_matches("/api/v4/secrets/");
            let name = name.split('?').next().unwrap_or(name);
            if name.is_empty() || path.starts_with("/api/v4/secrets?") {
                let secrets: Vec<_> = store_h
                    .lock()
                    .unwrap()
                    .keys()
                    .map(|k| json!({ "secretKey": k }))
                    .collect();
                return (200, json!({ "secrets": secrets }).to_string());
            }
            let name = name.to_string();
            match method {
                "POST" => {
                    if store_h.lock().unwrap().contains_key(&name) {
                        return (400, r#"{"message":"Secret already exists"}"#.into());
                    }
                    let value = serde_json::from_str::<serde_json::Value>(body)
                        .ok()
                        .and_then(|v| {
                            v.get("secretValue")
                                .and_then(|s| s.as_str())
                                .map(str::to_string)
                        })
                        .unwrap_or_default();
                    store_h.lock().unwrap().insert(name.clone(), value.clone());
                    (
                        200,
                        json!({ "secret": { "secretKey": name, "secretValue": value } })
                            .to_string(),
                    )
                }
                "PATCH" => {
                    let value = serde_json::from_str::<serde_json::Value>(body)
                        .ok()
                        .and_then(|v| {
                            v.get("secretValue")
                                .and_then(|s| s.as_str())
                                .map(str::to_string)
                        })
                        .unwrap_or_default();
                    store_h.lock().unwrap().insert(name.clone(), value.clone());
                    (
                        200,
                        json!({ "secret": { "secretKey": name, "secretValue": value } })
                            .to_string(),
                    )
                }
                "GET" => match store_h.lock().unwrap().get(&name) {
                    Some(value) => (
                        200,
                        json!({ "secret": { "secretKey": name, "secretValue": value } })
                            .to_string(),
                    ),
                    None => (404, r#"{"message":"not found"}"#.into()),
                },
                "DELETE" => {
                    store_h.lock().unwrap().remove(&name);
                    (200, "{}".into())
                }
                _ => (404, "{}".into()),
            }
        }));

        let backend = InfisicalSecretBackend::new(config(&base)).unwrap();
        let stored = backend
            .put("openai-prod", &SecretString::new("sk-test"))
            .await
            .unwrap();
        assert_eq!(backend.get(&stored).await.unwrap().expose(), "sk-test");
        // A second put updates instead of failing.
        backend
            .put("openai-prod", &SecretString::new("sk-rotated"))
            .await
            .unwrap();
        assert_eq!(backend.get(&stored).await.unwrap().expose(), "sk-rotated");
        let names: Vec<_> = backend
            .list()
            .await
            .unwrap()
            .into_iter()
            .map(|m| m.name)
            .collect();
        assert!(names.contains(&"openai-prod".to_string()));
        backend.delete(&stored).await.unwrap();
        assert!(matches!(
            backend.get(&stored).await.unwrap_err(),
            SecretsError::NotFound
        ));
        assert_eq!(logins.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn expired_token_and_unauthorized_login_again() {
        let logins = Arc::new(AtomicUsize::new(0));
        let calls = Arc::new(AtomicUsize::new(0));
        let logins_h = logins.clone();
        let calls_h = calls.clone();
        let base = serve(Arc::new(move |method, path, _| {
            if path.contains("universal-auth/login") {
                logins_h.fetch_add(1, Ordering::SeqCst);
                return (200, r#"{"accessToken":"tok","expiresIn":0,"accessTokenMaxTTL":0,"tokenType":"Bearer"}"#.into());
            }
            let _ = method;
            let n = calls_h.fetch_add(1, Ordering::SeqCst);
            if n == 0 {
                return (401, r#"{"message":"Unauthorized"}"#.into());
            }
            (
                200,
                r#"{"secret":{"secretKey":"k","secretValue":"v"}}"#.into(),
            )
        }));
        let backend = InfisicalSecretBackend::new(config(&base)).unwrap();
        let value = backend
            .get(&SecretRef::Infisical {
                project_id: "proj".into(),
                environment: "dev".into(),
                secret_path: "/".into(),
                name: "k".into(),
            })
            .await
            .unwrap();
        assert_eq!(value.expose(), "v");
        assert!(logins.load(Ordering::SeqCst) >= 2);
    }

    #[tokio::test]
    async fn unreachable_host_is_an_error_and_names_no_secret() {
        let backend = InfisicalSecretBackend::new(config("http://127.0.0.1:1")).unwrap();
        let err = backend
            .put("openai-prod", &SecretString::new("sk-should-not-leak"))
            .await
            .unwrap_err();
        let text = err.to_string();
        assert!(text.contains("unreachable") || text.contains("Infisical"));
        assert!(!text.contains("sk-should-not-leak"));
    }
}
