//! SPEC-123 model config resolution — Upload → Workspace → Tenant → Env → Default.
//!
//! Mirrors [`edgequake_pdf::resolve_pdf_parser_choice`] for LLM, embedding, and
//! vision LLM provider/model pairs. There is **no** separate “vision embedding”
//! concept — vision is a VLM; embeddings are text vectors.
//!
//! # Laws
//!
//! - **LAW-123-2**: Request/upload > Workspace > Tenant > Env > compiled default
//! - **LAW-123-5**: One pure resolver; call sites must not mutate lower layers into the request
//! - **LAW-123-1**: What you resolve is what you run

use crate::types::{Tenant, Workspace};

/// Provenance of the winning model choice (LAW-123-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelResolutionSource {
    Request,
    Workspace,
    Tenant,
    Env,
    Default,
}

impl ModelResolutionSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Request => "request",
            Self::Workspace => "workspace",
            Self::Tenant => "tenant",
            Self::Env => "env",
            Self::Default => "default",
        }
    }

    /// Prefer more specific provenance (Request > … > Default).
    #[allow(dead_code)]
    fn rank(self) -> u8 {
        match self {
            Self::Request => 4,
            Self::Workspace => 3,
            Self::Tenant => 2,
            Self::Env => 1,
            Self::Default => 0,
        }
    }
}

/// Resolved provider + model with provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedProviderModel {
    pub provider: String,
    pub model: String,
    pub source: ModelResolutionSource,
}

/// Resolved embedding stack with provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedEmbedding {
    pub provider: String,
    pub model: String,
    pub dimension: usize,
    pub source: ModelResolutionSource,
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|s| !s.is_empty())
}

/// One priority layer: (provider, model) + provenance.
type ProviderModelLayer<'a> = ((Option<&'a str>, Option<&'a str>), ModelResolutionSource);

fn first_string<'a>(
    layers: &[ProviderModelLayer<'a>],
) -> Option<(String, String, ModelResolutionSource)> {
    for ((provider, model), source) in layers {
        let p = non_empty(*provider);
        let m = non_empty(*model);
        if p.is_none() && m.is_none() {
            continue;
        }
        // Layer wins if either field is set; fill the other from lower layers later.
        return Some((
            p.unwrap_or("").to_string(),
            m.unwrap_or("").to_string(),
            *source,
        ));
    }
    None
}

fn fill_missing_provider_model(
    mut provider: String,
    mut model: String,
    source: ModelResolutionSource,
    fallbacks: &[ProviderModelLayer<'_>],
    default_provider: &str,
    default_model_for: &dyn Fn(&str) -> String,
) -> ResolvedProviderModel {
    // Keep provenance of the winning layer; lower layers only fill gaps.
    if provider.is_empty() || model.is_empty() {
        for ((p, m), _) in fallbacks {
            if provider.is_empty() {
                if let Some(v) = non_empty(*p) {
                    provider = v.to_string();
                }
            }
            if model.is_empty() {
                if let Some(v) = non_empty(*m) {
                    model = v.to_string();
                }
            }
            if !provider.is_empty() && !model.is_empty() {
                break;
            }
        }
    }
    if provider.is_empty() {
        provider = default_provider.to_string();
    }
    if model.is_empty() {
        model = default_model_for(&provider);
    }
    ResolvedProviderModel {
        provider,
        model,
        source,
    }
}

/// Env leaf for LLM (same vars as [`Workspace::default_llm_config`]).
pub fn env_llm_provider_model() -> (String, String) {
    Workspace::default_llm_config()
}

/// Env leaf for embedding.
pub fn env_embedding_provider_model() -> (String, String, usize) {
    Workspace::default_embedding_config()
}

/// Env leaf for vision provider (API `vision_env` twin — no API crate dependency).
pub fn env_vision_provider() -> String {
    std::env::var("EDGEQUAKE_VISION_PROVIDER")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| {
            std::env::var("EDGEQUAKE_VISION_LLM_PROVIDER")
                .ok()
                .filter(|s| !s.is_empty())
        })
        .or_else(|| {
            std::env::var("EDGEQUAKE_DEFAULT_LLM_PROVIDER")
                .ok()
                .filter(|s| !s.is_empty())
        })
        .or_else(|| {
            std::env::var("EDGEQUAKE_LLM_PROVIDER")
                .ok()
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| "ollama".to_string())
}

/// Env leaf for vision model candidates (first non-empty).
pub fn env_vision_model() -> Option<String> {
    for key in [
        "EDGEQUAKE_VISION_MODEL",
        "EDGEQUAKE_VISION_LLM_MODEL",
        "EDGEQUAKE_DEFAULT_LLM_MODEL",
        "EDGEQUAKE_LLM_MODEL",
    ] {
        if let Ok(v) = std::env::var(key) {
            if !v.is_empty() {
                return Some(v);
            }
        }
    }
    None
}

/// Compiled vision model default for a provider (Acc / startup leaf).
pub fn compiled_vision_model_for(provider: &str) -> String {
    match provider {
        "openai" => "gpt-5.6-luna".to_string(),
        "anthropic" => "claude-sonnet-4-20250514".to_string(),
        "mistral" => "mistral-small-latest".to_string(),
        _ => "gemma4:latest".to_string(),
    }
}

fn metadata_has_nonempty(
    metadata: &std::collections::HashMap<String, serde_json::Value>,
    key: &str,
) -> bool {
    metadata
        .get(key)
        .and_then(|v| v.as_str())
        .is_some_and(|s| !s.trim().is_empty())
}

/// LAW-123-8: workspace LLM/embedding fields count only with deliberate override metadata.
fn workspace_has_llm_override(workspace: &Workspace) -> bool {
    metadata_has_nonempty(&workspace.metadata, "llm_model")
        || metadata_has_nonempty(&workspace.metadata, "llm_provider")
}

fn workspace_has_embedding_override(workspace: &Workspace) -> bool {
    metadata_has_nonempty(&workspace.metadata, "embedding_model")
        || metadata_has_nonempty(&workspace.metadata, "embedding_provider")
}

fn workspace_has_vision_override(workspace: &Workspace) -> bool {
    // Deliberate: metadata keys and/or Option fields set by user (never inherit-paint).
    metadata_has_nonempty(&workspace.metadata, "vision_llm_model")
        || metadata_has_nonempty(&workspace.metadata, "vision_llm_provider")
        || workspace
            .vision_llm_provider
            .as_ref()
            .is_some_and(|s| !s.trim().is_empty())
        || workspace
            .vision_llm_model
            .as_ref()
            .is_some_and(|s| !s.trim().is_empty())
}

/// Resolve LLM provider/model: Request → Workspace → Tenant → Env → Default.
pub fn resolve_llm_choice(
    request_provider: Option<&str>,
    request_model: Option<&str>,
    workspace: Option<&Workspace>,
    tenant: Option<&Tenant>,
) -> ResolvedProviderModel {
    let (ws_p, ws_m) = match workspace {
        Some(w) if workspace_has_llm_override(w) => {
            (Some(w.llm_provider.as_str()), Some(w.llm_model.as_str()))
        }
        _ => (None, None),
    };
    let ten_p = tenant.map(|t| t.default_llm_provider.as_str());
    let ten_m = tenant.map(|t| t.default_llm_model.as_str());
    let (env_model, env_provider) = env_llm_provider_model();

    let layers = [
        (
            (request_provider, request_model),
            ModelResolutionSource::Request,
        ),
        ((ws_p, ws_m), ModelResolutionSource::Workspace),
        ((ten_p, ten_m), ModelResolutionSource::Tenant),
        (
            (Some(env_provider.as_str()), Some(env_model.as_str())),
            ModelResolutionSource::Env,
        ),
    ];

    let (provider, model, source) = first_string(&layers).unwrap_or_else(|| {
        (
            env_provider.clone(),
            env_model.clone(),
            ModelResolutionSource::Env,
        )
    });

    fill_missing_provider_model(provider, model, source, &layers, &env_provider, &|_| {
        env_model.clone()
    })
}

/// Resolve embedding: Request → Workspace → Tenant → Env → Default.
pub fn resolve_embedding_choice(
    request_provider: Option<&str>,
    request_model: Option<&str>,
    request_dimension: Option<usize>,
    workspace: Option<&Workspace>,
    tenant: Option<&Tenant>,
) -> ResolvedEmbedding {
    let resolved_pair = {
        let (ws_p, ws_m) = match workspace {
            Some(w) if workspace_has_embedding_override(w) => (
                Some(w.embedding_provider.as_str()),
                Some(w.embedding_model.as_str()),
            ),
            _ => (None, None),
        };
        let ten_p = tenant.map(|t| t.default_embedding_provider.as_str());
        let ten_m = tenant.map(|t| t.default_embedding_model.as_str());
        let (env_model, env_provider, _) = env_embedding_provider_model();

        let layers = [
            (
                (request_provider, request_model),
                ModelResolutionSource::Request,
            ),
            ((ws_p, ws_m), ModelResolutionSource::Workspace),
            ((ten_p, ten_m), ModelResolutionSource::Tenant),
            (
                (Some(env_provider.as_str()), Some(env_model.as_str())),
                ModelResolutionSource::Env,
            ),
        ];

        let (provider, model, source) = first_string(&layers).unwrap_or_else(|| {
            (
                env_provider.clone(),
                env_model.clone(),
                ModelResolutionSource::Env,
            )
        });

        fill_missing_provider_model(provider, model, source, &layers, &env_provider, &|_| {
            env_model.clone()
        })
    };

    let ws_dim = workspace
        .filter(|w| workspace_has_embedding_override(w))
        .map(|w| w.embedding_dimension)
        .filter(|d| *d > 0);
    let dimension = request_dimension
        .filter(|d| *d > 0)
        .or(ws_dim)
        .or_else(|| {
            tenant
                .map(|t| t.default_embedding_dimension)
                .filter(|d| *d > 0)
        })
        .unwrap_or_else(|| {
            let (_, _, dim) = env_embedding_provider_model();
            dim
        });

    ResolvedEmbedding {
        provider: resolved_pair.provider,
        model: resolved_pair.model,
        dimension,
        source: resolved_pair.source,
    }
}

/// Resolve vision LLM: Upload → Workspace vision → Tenant vision → Workspace LLM → Env → Default.
///
/// Not an embedding model — this is the PDF/VLM stack (SPEC-041 / SPEC-123).
/// LAW-123-8: vision Option / metadata must be deliberate; inherit-paint must not invent Workspace.
pub fn resolve_vision_llm_choice(
    upload_provider: Option<&str>,
    upload_model: Option<&str>,
    workspace: Option<&Workspace>,
    tenant: Option<&Tenant>,
) -> ResolvedProviderModel {
    let has_vision = workspace.is_some_and(workspace_has_vision_override);
    let ws_vision_p = if has_vision {
        workspace
            .and_then(|w| w.vision_llm_provider.as_deref())
            .filter(|s| !s.is_empty())
    } else {
        None
    };
    let ws_vision_m = if has_vision {
        workspace
            .and_then(|w| w.vision_llm_model.as_deref())
            .filter(|s| !s.is_empty())
    } else {
        None
    };
    // Workspace LLM fallback only when deliberate LLM override (not inherit-paint).
    let (ws_llm_p, ws_llm_m) = match workspace {
        Some(w) if workspace_has_llm_override(w) => {
            (Some(w.llm_provider.as_str()), Some(w.llm_model.as_str()))
        }
        _ => (None, None),
    };
    let ten_p = tenant
        .and_then(|t| t.default_vision_llm_provider.as_deref())
        .filter(|s| !s.is_empty());
    let ten_m = tenant
        .and_then(|t| t.default_vision_llm_model.as_deref())
        .filter(|s| !s.is_empty());
    let env_p = env_vision_provider();
    let env_m = env_vision_model();

    let layers = [
        (
            (upload_provider, upload_model),
            ModelResolutionSource::Request,
        ),
        ((ws_vision_p, ws_vision_m), ModelResolutionSource::Workspace),
        // Tenant vision before workspace LLM fallback (SPEC-041 / SPEC-123).
        ((ten_p, ten_m), ModelResolutionSource::Tenant),
        ((ws_llm_p, ws_llm_m), ModelResolutionSource::Workspace),
        (
            (Some(env_p.as_str()), env_m.as_deref()),
            ModelResolutionSource::Env,
        ),
    ];

    let (provider, model, source) = first_string(&layers)
        .unwrap_or_else(|| (env_p.clone(), String::new(), ModelResolutionSource::Env));

    fill_missing_provider_model(provider, model, source, &layers, &env_p, &|p| {
        env_m
            .clone()
            .unwrap_or_else(|| compiled_vision_model_for(p))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn llm_request_wins_over_workspace_tenant_env() {
        let mut ws = Workspace::new(Uuid::nil(), "ws", "ws");
        ws.llm_provider = "ollama".into();
        ws.llm_model = "gemma4:latest".into();
        ws.metadata
            .insert("llm_provider".into(), serde_json::json!("ollama"));
        ws.metadata
            .insert("llm_model".into(), serde_json::json!("gemma4:latest"));
        let mut tenant = Tenant::new("t", "t");
        tenant.default_llm_provider = "mistral".into();
        tenant.default_llm_model = "mistral-small-latest".into();

        let resolved = resolve_llm_choice(
            Some("openai"),
            Some("gpt-4.1-mini"),
            Some(&ws),
            Some(&tenant),
        );
        assert_eq!(resolved.provider, "openai");
        assert_eq!(resolved.model, "gpt-4.1-mini");
        assert_eq!(resolved.source, ModelResolutionSource::Request);
    }

    #[test]
    fn llm_tenant_wins_when_workspace_has_no_override_metadata() {
        let mut ws = Workspace::new(Uuid::nil(), "ws", "ws");
        // Inherit-painted concrete fields without metadata must not win (LAW-123-8).
        ws.llm_provider = "ollama".into();
        ws.llm_model = "gemma4:latest".into();
        let mut tenant = Tenant::new("t", "t");
        tenant.default_llm_provider = "mistral".into();
        tenant.default_llm_model = "mistral-small-latest".into();

        let resolved = resolve_llm_choice(None, None, Some(&ws), Some(&tenant));
        assert_eq!(resolved.provider, "mistral");
        assert_eq!(resolved.model, "mistral-small-latest");
        assert_eq!(resolved.source, ModelResolutionSource::Tenant);
    }

    #[test]
    fn embedding_workspace_wins_over_tenant() {
        let mut ws = Workspace::new(Uuid::nil(), "ws", "ws");
        ws.embedding_provider = "ollama".into();
        ws.embedding_model = "embeddinggemma:latest".into();
        ws.embedding_dimension = 768;
        ws.metadata
            .insert("embedding_provider".into(), serde_json::json!("ollama"));
        ws.metadata.insert(
            "embedding_model".into(),
            serde_json::json!("embeddinggemma:latest"),
        );
        let mut tenant = Tenant::new("t", "t");
        tenant.default_embedding_provider = "openai".into();
        tenant.default_embedding_model = "text-embedding-3-small".into();
        tenant.default_embedding_dimension = 1536;

        let resolved = resolve_embedding_choice(None, None, None, Some(&ws), Some(&tenant));
        assert_eq!(resolved.provider, "ollama");
        assert_eq!(resolved.model, "embeddinggemma:latest");
        assert_eq!(resolved.dimension, 768);
        assert_eq!(resolved.source, ModelResolutionSource::Workspace);
    }

    #[test]
    fn vision_tenant_wins_when_workspace_vision_unset() {
        let mut ws = Workspace::new(Uuid::nil(), "ws", "ws");
        ws.vision_llm_provider = None;
        ws.vision_llm_model = None;
        let mut tenant = Tenant::new("t", "t");
        tenant.default_vision_llm_provider = Some("mistral".into());
        tenant.default_vision_llm_model = Some("mistral-small-latest".into());

        let resolved = resolve_vision_llm_choice(None, None, Some(&ws), Some(&tenant));
        assert_eq!(resolved.provider, "mistral");
        assert_eq!(resolved.model, "mistral-small-latest");
        assert_eq!(resolved.source, ModelResolutionSource::Tenant);
    }

    #[test]
    fn vision_upload_wins_over_workspace() {
        let mut ws = Workspace::new(Uuid::nil(), "ws", "ws");
        ws.vision_llm_provider = Some("ollama".into());
        ws.vision_llm_model = Some("gemma4:latest".into());

        let resolved =
            resolve_vision_llm_choice(Some("openai"), Some("gpt-4.1-nano"), Some(&ws), None);
        assert_eq!(resolved.provider, "openai");
        assert_eq!(resolved.model, "gpt-4.1-nano");
        assert_eq!(resolved.source, ModelResolutionSource::Request);
    }

    #[test]
    fn compiled_openai_vision_default_is_not_a_retired_model() {
        // gpt-4.1-nano is shut down on the OpenAI API on 2026-10-23.
        assert_eq!(compiled_vision_model_for("openai"), "gpt-5.6-luna");
    }
}
