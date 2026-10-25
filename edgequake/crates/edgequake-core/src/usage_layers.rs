//! SPEC-175 W1: typed sources that feed [`resolve_usage`].
//!
//! Pure. The API edge reads env, storage and the server snapshot once and passes
//! them in. This module maps those values onto the ladder for each usage, so the
//! effective view and the runtime resolver share one set of rules.
//!
//! The view uses the print page class. Manuscript pages apply the manuscript lock
//! at ingest, so `Vlm` here reports the print-class answer.

use crate::llm_roles::{role_config_from_workspace, LlmRole};
use crate::model_resolution::compiled_vision_model_for;
use crate::usage_resolution::{
    resolve_usage, LeafOrder, ModelPair, ResolvedUsage, Usage, UsageLayers, UsageLocks,
    CONFIGURABLE_USAGES,
};
use crate::{ServerLlmDefaults, Tenant, Workspace};

/// Operator pins and env bootstrap values, read once at the edge.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnvLayers {
    pub extract_lock: ModelPair,
    pub keyword_lock: ModelPair,
    pub vlm_manuscript_lock: ModelPair,
    pub bootstrap_chat: ModelPair,
    pub bootstrap_embedding: ModelPair,
    pub bootstrap_vision: ModelPair,
}

/// Everything the ladder reads for one workspace.
pub struct LayerSources<'a> {
    pub workspace: &'a Workspace,
    pub tenant: Option<&'a Tenant>,
    pub server: &'a ServerLlmDefaults,
    pub env: &'a EnvLayers,
    pub leaf_order: LeafOrder,
}

/// The three configuration families. Each has its own fields on every layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family {
    Chat,
    Embedding,
    Vision,
}

fn family(usage: Usage) -> Family {
    match usage {
        Usage::Embedding => Family::Embedding,
        Usage::Vlm => Family::Vision,
        Usage::Extract
        | Usage::Query
        | Usage::Summary
        | Usage::Keyword
        | Usage::Decision
        | Usage::Rerank => Family::Chat,
    }
}

/// Workspace LLM role that owns this usage, if any. `Decision` and `Embedding` have none.
pub fn llm_role_for(usage: Usage) -> Option<LlmRole> {
    match usage {
        Usage::Extract => Some(LlmRole::Extract),
        Usage::Query => Some(LlmRole::Query),
        Usage::Summary => Some(LlmRole::Summary),
        Usage::Vlm => Some(LlmRole::Vlm),
        Usage::Keyword => Some(LlmRole::Keyword),
        Usage::Embedding | Usage::Decision | Usage::Rerank => None,
    }
}

fn compiled(family: Family) -> ModelPair {
    let provider = "ollama";
    let model = match family {
        Family::Chat => Workspace::default_model_for_provider(provider),
        Family::Embedding => Workspace::default_embedding_model_for_provider(provider),
        Family::Vision => compiled_vision_model_for(provider),
    };
    ModelPair::new(Some(provider), Some(&model))
}

fn locks_for(usage: Usage, env: &EnvLayers) -> UsageLocks {
    let mut locks = UsageLocks::default();
    match usage {
        Usage::Extract => locks.extract = env.extract_lock.clone(),
        Usage::Keyword => locks.keyword = env.keyword_lock.clone(),
        Usage::Vlm => locks.vlm_manuscript = env.vlm_manuscript_lock.clone(),
        _ => {}
    }
    locks
}

fn workspace_role(usage: Usage, ws: &Workspace) -> ModelPair {
    llm_role_for(usage)
        .and_then(|role| role_config_from_workspace(ws, role))
        .map(|cfg| ModelPair::new(cfg.provider.as_deref(), cfg.model.as_deref()))
        .unwrap_or_default()
}

fn workspace_default(family: Family, ws: &Workspace) -> ModelPair {
    match family {
        Family::Chat => ModelPair::new(Some(&ws.llm_provider), Some(&ws.llm_model)),
        Family::Embedding => {
            ModelPair::new(Some(&ws.embedding_provider), Some(&ws.embedding_model))
        }
        Family::Vision => ModelPair::new(
            ws.vision_llm_provider.as_deref(),
            ws.vision_llm_model.as_deref(),
        ),
    }
}

fn tenant_default(family: Family, tenant: Option<&Tenant>) -> ModelPair {
    let Some(t) = tenant else {
        return ModelPair::default();
    };
    match family {
        Family::Chat => ModelPair::new(Some(&t.default_llm_provider), Some(&t.default_llm_model)),
        Family::Embedding => ModelPair::new(
            Some(&t.default_embedding_provider),
            Some(&t.default_embedding_model),
        ),
        Family::Vision => ModelPair::new(t.default_vision_llm_provider.as_deref(), None),
    }
}

fn server_default(family: Family, server: &ServerLlmDefaults) -> ModelPair {
    match family {
        Family::Chat => ModelPair::new(server.llm_provider.as_deref(), server.llm_model.as_deref()),
        Family::Embedding => ModelPair::new(
            server.embedding_provider.as_deref(),
            server.embedding_model.as_deref(),
        ),
        Family::Vision => ModelPair::new(
            server.vision_provider.as_deref(),
            server.vision_model.as_deref(),
        ),
    }
}

fn env_bootstrap(family: Family, env: &EnvLayers) -> ModelPair {
    match family {
        Family::Chat => env.bootstrap_chat.clone(),
        Family::Embedding => env.bootstrap_embedding.clone(),
        Family::Vision => env.bootstrap_vision.clone(),
    }
}

/// Build the ladder for one usage from the typed sources.
pub fn layers_for(usage: Usage, src: &LayerSources<'_>) -> UsageLayers {
    let family = family(usage);
    let mut layers = UsageLayers::new(compiled(family));
    layers.locks = locks_for(usage, src.env);
    layers.workspace_role = workspace_role(usage, src.workspace);
    layers.workspace_default = workspace_default(family, src.workspace);
    layers.tenant_default = tenant_default(family, src.tenant);
    layers.server_config = server_default(family, src.server);
    layers.env_bootstrap = env_bootstrap(family, src.env);
    layers.leaf_order = src.leaf_order;
    layers
}

/// Resolve every effective usage, in [`CONFIGURABLE_USAGES`] order.
pub fn resolve_effective_usages(src: &LayerSources<'_>) -> Vec<ResolvedUsage> {
    CONFIGURABLE_USAGES
        .iter()
        .map(|&usage| resolve_usage(usage, &layers_for(usage, src)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usage_resolution::ResolutionLayer;

    struct Fixture {
        ws: Workspace,
        tenant: Tenant,
        server: ServerLlmDefaults,
        env: EnvLayers,
    }

    fn blank() -> Fixture {
        let mut ws = Workspace::new(uuid::Uuid::new_v4(), "ws", "ws");
        ws.llm_provider.clear();
        ws.llm_model.clear();
        ws.embedding_provider.clear();
        ws.embedding_model.clear();
        ws.vision_llm_provider = None;
        ws.vision_llm_model = None;
        ws.metadata.clear();
        let mut tenant = Tenant::new("t", "t");
        tenant.default_llm_provider.clear();
        tenant.default_llm_model.clear();
        tenant.default_embedding_provider.clear();
        tenant.default_embedding_model.clear();
        tenant.default_vision_llm_provider = None;
        Fixture {
            ws,
            tenant,
            server: ServerLlmDefaults::default(),
            env: EnvLayers::default(),
        }
    }

    fn pair(provider: &str, model: &str) -> ModelPair {
        ModelPair::new(Some(provider), Some(model))
    }

    fn resolve(f: &Fixture, usage: Usage, leaf: LeafOrder) -> ResolvedUsage {
        let src = LayerSources {
            workspace: &f.ws,
            tenant: Some(&f.tenant),
            server: &f.server,
            env: &f.env,
            leaf_order: leaf,
        };
        resolve_usage(usage, &layers_for(usage, &src))
    }

    fn find(list: &[ResolvedUsage], usage: Usage) -> &ResolvedUsage {
        list.iter()
            .find(|r| r.usage == usage)
            .expect("usage present")
    }

    #[test]
    fn seven_effective_usages_exclude_rerank() {
        let f = blank();
        let src = LayerSources {
            workspace: &f.ws,
            tenant: Some(&f.tenant),
            server: &f.server,
            env: &f.env,
            leaf_order: LeafOrder::ServerFirst,
        };
        let all = resolve_effective_usages(&src);
        assert_eq!(all.len(), 7);
        assert!(all.iter().all(|r| r.usage != Usage::Rerank));
    }

    #[test]
    fn operator_lock_beats_workspace_role_for_extract() {
        let mut f = blank();
        f.ws.metadata.insert(
            "llm_roles".into(),
            serde_json::json!({ "extract": { "provider": "openai", "model": "gpt-5-nano" } }),
        );
        f.env.extract_lock = pair("mistral", "mistral-medium-latest");
        let r = resolve(&f, Usage::Extract, LeafOrder::ServerFirst);
        assert_eq!(r.source, ResolutionLayer::OperatorLock);
        assert_eq!(r.provider, "mistral");
        assert!(r.is_locked());
    }

    #[test]
    fn workspace_role_beats_workspace_default() {
        let mut f = blank();
        f.ws.llm_provider = "ollama".into();
        f.ws.llm_model = "gemma3:latest".into();
        f.ws.metadata.insert(
            "llm_roles".into(),
            serde_json::json!({ "query": { "provider": "openai", "model": "gpt-5-nano" } }),
        );
        let r = resolve(&f, Usage::Query, LeafOrder::ServerFirst);
        assert_eq!(r.source, ResolutionLayer::WorkspaceRole);
        assert_eq!(
            (r.provider.as_str(), r.model.as_str()),
            ("openai", "gpt-5-nano")
        );
    }

    #[test]
    fn workspace_default_used_when_no_role() {
        let mut f = blank();
        f.ws.llm_provider = "openai".into();
        f.ws.llm_model = "gpt-5-nano".into();
        let r = resolve(&f, Usage::Summary, LeafOrder::ServerFirst);
        assert_eq!(r.source, ResolutionLayer::WorkspaceDefault);
        assert_eq!(r.model, "gpt-5-nano");
    }

    #[test]
    fn embedding_reads_only_embedding_fields() {
        let mut f = blank();
        f.ws.llm_provider = "openai".into();
        f.ws.llm_model = "gpt-5-nano".into();
        f.ws.embedding_provider = "ollama".into();
        f.ws.embedding_model = "nomic-embed-text".into();
        let r = resolve(&f, Usage::Embedding, LeafOrder::ServerFirst);
        assert_eq!(
            (r.provider.as_str(), r.model.as_str()),
            ("ollama", "nomic-embed-text")
        );
        let q = resolve(&f, Usage::Query, LeafOrder::ServerFirst);
        assert_eq!(q.provider, "openai");
    }

    #[test]
    fn blank_workspace_falls_back_to_server_then_compiled() {
        let mut f = blank();
        f.server.llm_provider = Some("mistral".into());
        f.server.llm_model = Some("mistral-small-latest".into());
        let r = resolve(&f, Usage::Query, LeafOrder::ServerFirst);
        assert_eq!(r.source, ResolutionLayer::ServerConfig);
        assert_eq!(r.provider, "mistral");

        let bare = blank();
        let c = resolve(&bare, Usage::Query, LeafOrder::ServerFirst);
        assert_eq!(c.source, ResolutionLayer::CompiledDefault);
        assert_eq!(c.provider, "ollama");
        assert!(!c.model.is_empty());
    }

    #[test]
    fn leaf_order_decides_between_server_and_env() {
        let mut f = blank();
        f.server.llm_provider = Some("mistral".into());
        f.server.llm_model = Some("mistral-small-latest".into());
        f.env.bootstrap_chat = pair("openai", "gpt-5-nano");
        let server_first = resolve(&f, Usage::Query, LeafOrder::ServerFirst);
        assert_eq!(server_first.source, ResolutionLayer::ServerConfig);
        let env_first = resolve(&f, Usage::Query, LeafOrder::EnvFirst);
        assert_eq!(env_first.source, ResolutionLayer::EnvBootstrap);
        assert_eq!(env_first.provider, "openai");
    }

    #[test]
    fn vlm_reads_vision_fields_and_decision_reads_chat_fields() {
        let mut f = blank();
        f.ws.llm_provider = "openai".into();
        f.ws.llm_model = "gpt-5-nano".into();
        f.ws.vision_llm_provider = Some("ollama".into());
        f.ws.vision_llm_model = Some("gemma4:latest".into());
        let v = resolve(&f, Usage::Vlm, LeafOrder::ServerFirst);
        assert_eq!(
            (v.provider.as_str(), v.model.as_str()),
            ("ollama", "gemma4:latest")
        );
        let d = resolve(&f, Usage::Decision, LeafOrder::ServerFirst);
        assert_eq!(d.provider, "openai");
    }

    #[test]
    fn effective_list_contains_every_usage_once() {
        let f = blank();
        let src = LayerSources {
            workspace: &f.ws,
            tenant: Some(&f.tenant),
            server: &f.server,
            env: &f.env,
            leaf_order: LeafOrder::ServerFirst,
        };
        let all = resolve_effective_usages(&src);
        for usage in CONFIGURABLE_USAGES {
            assert_eq!(all.iter().filter(|r| r.usage == usage).count(), 1);
        }
        assert_eq!(find(&all, Usage::Extract).usage, Usage::Extract);
    }
}
