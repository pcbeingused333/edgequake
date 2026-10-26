//! Construct LLM/embedding providers from an explicit Connection (no env mutation).
//!
//! Uses public constructors from `edgequake-llm` 0.10.9.

use std::sync::Arc;

use edgequake_llm::traits::{EmbeddingProvider, LLMProvider};
use edgequake_llm::{AnthropicProvider, GeminiProvider, OllamaProvider, OpenAIProvider};

use crate::error::ApiError;
use crate::providers::connection_shapes::{is_openai_wire, requires_server_credentials};
use crate::safety_limits::{
    SafetyLimitedEmbeddingProviderWrapper, SafetyLimitedProviderWrapper, SafetyLimitsConfig,
};

#[derive(Clone)]
pub struct ConnectionSpec {
    pub shape: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
    pub embedding_model: Option<String>,
    pub embedding_dimension: Option<usize>,
}

pub fn llm_from_connection(spec: &ConnectionSpec) -> Result<Arc<dyn LLMProvider>, ApiError> {
    let shape = spec.shape.to_ascii_lowercase();
    if requires_server_credentials(&shape) {
        return Err(server_credential_error(&shape));
    }
    let key = spec.api_key.clone().unwrap_or_default();
    let inner: Arc<dyn LLMProvider> = match shape.as_str() {
        "ollama" => Arc::new(build_ollama(spec)?),
        "anthropic_messages" | "anthropic" | "claude" => Arc::new(
            AnthropicProvider::new(key)
                .with_base_url(&spec.base_url)
                .with_model(&spec.model),
        ),
        "gemini" | "google" => {
            if key.is_empty() {
                return Err(ApiError::BadRequest(
                    "Gemini requires an API key on the connection".into(),
                ));
            }
            Arc::new(GeminiProvider::new(key).with_model(&spec.model))
        }
        other if is_openai_wire(other) => {
            Arc::new(OpenAIProvider::compatible(key, spec.base_url.clone()).with_model(&spec.model))
        }
        other => {
            return Err(ApiError::BadRequest(format!(
                "provider shape '{other}' is not supported"
            )));
        }
    };
    Ok(wrap_llm(&shape, inner))
}

pub fn embedding_from_connection(
    spec: &ConnectionSpec,
) -> Result<Arc<dyn EmbeddingProvider>, ApiError> {
    let shape = spec.shape.to_ascii_lowercase();
    if requires_server_credentials(&shape) {
        return Err(server_credential_error(&shape));
    }
    let key = spec.api_key.clone().unwrap_or_default();
    let model = spec
        .embedding_model
        .clone()
        .unwrap_or_else(|| spec.model.clone());
    let inner: Arc<dyn EmbeddingProvider> = match shape.as_str() {
        "ollama" => {
            let mut adjusted = spec.clone();
            adjusted.embedding_model = Some(model.clone());
            Arc::new(build_ollama(&adjusted)?)
        }
        "gemini" | "google" => {
            if key.is_empty() {
                return Err(ApiError::BadRequest(
                    "Gemini requires an API key on the connection".into(),
                ));
            }
            Arc::new(GeminiProvider::new(key).with_embedding_model(&model))
        }
        other if is_openai_wire(other) => Arc::new(
            OpenAIProvider::compatible(key, spec.base_url.clone()).with_embedding_model(&model),
        ),
        other => {
            return Err(ApiError::BadRequest(format!(
                "provider shape '{other}' is not supported for embeddings"
            )));
        }
    };
    Ok(wrap_embed(&shape, inner))
}

fn server_credential_error(shape: &str) -> ApiError {
    ApiError::BadRequest(format!(
        "{shape} is available to add, but a saved URL and key cannot build it. Configure server credentials for this provider."
    ))
}

fn build_ollama(spec: &ConnectionSpec) -> Result<OllamaProvider, ApiError> {
    let mut builder = OllamaProvider::builder()
        .host(spec.base_url.clone())
        .model(spec.model.clone());
    if let Some(key) = spec.api_key.as_deref().filter(|s| !s.is_empty()) {
        builder = builder.api_key(key);
    }
    if let Some(model) = spec.embedding_model.as_deref().filter(|s| !s.is_empty()) {
        builder = builder.embedding_model(model);
    }
    if let Some(dim) = spec.embedding_dimension.filter(|d| *d > 0) {
        builder = builder.embedding_dimension(dim);
    }
    builder
        .build()
        .map_err(|e| ApiError::BadRequest(e.to_string()))
}

fn wrap_llm(shape: &str, inner: Arc<dyn LLMProvider>) -> Arc<dyn LLMProvider> {
    let key = if crate::locality::is_slow_local_provider(shape) {
        shape
    } else {
        "openai"
    };
    Arc::new(SafetyLimitedProviderWrapper::new(
        inner,
        SafetyLimitsConfig::from_env_for_extraction(key),
    ))
}

fn wrap_embed(shape: &str, inner: Arc<dyn EmbeddingProvider>) -> Arc<dyn EmbeddingProvider> {
    let key = if crate::locality::is_slow_local_provider(shape) {
        shape
    } else {
        "openai"
    };
    Arc::new(SafetyLimitedEmbeddingProviderWrapper::new(
        inner,
        SafetyLimitsConfig::from_env_for_extraction(key),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openai_compatible_constructs() {
        let spec = ConnectionSpec {
            shape: "openai_chat".into(),
            base_url: "http://127.0.0.1:9050".into(),
            api_key: Some("k".into()),
            model: "default".into(),
            embedding_model: None,
            embedding_dimension: None,
        };
        assert!(llm_from_connection(&spec).is_ok());
        assert!(embedding_from_connection(&spec).is_ok());
    }

    #[test]
    fn ollama_uses_the_saved_host() {
        let spec = ConnectionSpec {
            shape: "ollama".into(),
            base_url: "http://127.0.0.1:19999".into(),
            api_key: None,
            model: "custom-chat".into(),
            embedding_model: Some("custom-embed".into()),
            embedding_dimension: Some(32),
        };
        let built = build_ollama(&spec).expect("builder");
        assert_eq!(built.host(), "http://127.0.0.1:19999");
        assert_eq!(LLMProvider::model(&built), "custom-chat");
        assert!(llm_from_connection(&spec).is_ok());
        assert!(embedding_from_connection(&spec).is_ok());
    }

    #[test]
    fn mistral_and_openrouter_use_the_saved_base_url() {
        for shape in ["mistral", "openrouter"] {
            let spec = ConnectionSpec {
                shape: shape.into(),
                base_url: "https://example.test/v1".into(),
                api_key: Some("k".into()),
                model: "default".into(),
                embedding_model: None,
                embedding_dimension: None,
            };
            assert!(llm_from_connection(&spec).is_ok(), "{shape}");
            assert!(embedding_from_connection(&spec).is_ok(), "{shape}");
        }
    }

    #[test]
    fn vertex_does_not_fall_back_to_env() {
        let spec = ConnectionSpec {
            shape: "vertexai".into(),
            base_url: "https://aiplatform.googleapis.com".into(),
            api_key: Some("k".into()),
            model: "gemini".into(),
            embedding_model: None,
            embedding_dimension: None,
        };
        match llm_from_connection(&spec) {
            Err(err) => assert!(err.to_string().contains("server credentials"), "{err}"),
            Ok(_) => panic!("vertexai must not build from a saved key"),
        }
    }
}
