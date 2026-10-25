//! SPEC-175 W1 — one pure resolver for every model usage.
//!
//! `resolve_usage` picks provider, model, and provenance for one [`Usage`].
//! It is pure: callers pass every layer as data (env values are read once at
//! the edge), so the ladder is testable without I/O.
//!
//! # Laws
//!
//! - **LAW-175-1**: one resolver; runtime and display call the same function.
//! - **LAW-175-2**: `operator_lock > request > workspace_role > workspace_default
//!   > tenant_default > server_config > env_bootstrap > compiled_default`.
//!   A blank layer is skipped. `EDGEQUAKE_CONFIG_PRIORITY` swaps only the two
//!   leaves (`server_config` and `env_bootstrap`).
//! - **LAW-175-4**: only `query` and `vlm` accept a request pair.
//! - **LAW-175-10**: pins are locks; a manuscript lock applies to manuscript pages only.

/// A model usage (LAW-175-3). `LlmRole` covers the five chat roles; the rest
/// are usages in the same resolver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Usage {
    Extract,
    Query,
    Summary,
    Vlm,
    Keyword,
    Embedding,
    Decision,
    Rerank,
}

/// Usages that carry a model row: every usage except `rerank` (a deployment fact, EC-175-09).
/// The single source for the catalog, the effective view and the layers.
pub const CONFIGURABLE_USAGES: [Usage; 7] = [
    Usage::Extract,
    Usage::Query,
    Usage::Summary,
    Usage::Vlm,
    Usage::Keyword,
    Usage::Embedding,
    Usage::Decision,
];

impl Usage {
    /// Stable wire name, matching the `usage` column in `model_catalog_policy`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Extract => "extract",
            Self::Query => "query",
            Self::Summary => "summary",
            Self::Vlm => "vlm",
            Self::Keyword => "keyword",
            Self::Embedding => "embedding",
            Self::Decision => "decision",
            Self::Rerank => "rerank",
        }
    }

    /// Parse a stored `usage` value. Unknown names are rejected, never defaulted.
    pub fn from_wire(name: &str) -> Option<Self> {
        match name {
            "extract" => Some(Self::Extract),
            "query" => Some(Self::Query),
            "summary" => Some(Self::Summary),
            "vlm" => Some(Self::Vlm),
            "keyword" => Some(Self::Keyword),
            "embedding" => Some(Self::Embedding),
            "decision" => Some(Self::Decision),
            "rerank" => Some(Self::Rerank),
            _ => None,
        }
    }

    /// Whether a per-request pair may override this usage (LAW-175-4).
    pub fn accepts_request(self) -> bool {
        matches!(self, Self::Query | Self::Vlm)
    }
}

/// Which layer produced the winning value (LAW-175-2 provenance).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionLayer {
    OperatorLock,
    Request,
    WorkspaceRole,
    WorkspaceDefault,
    TenantDefault,
    ServerConfig,
    EnvBootstrap,
    CompiledDefault,
}

impl ResolutionLayer {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OperatorLock => "operator_lock",
            Self::Request => "request",
            Self::WorkspaceRole => "workspace_role",
            Self::WorkspaceDefault => "workspace_default",
            Self::TenantDefault => "tenant_default",
            Self::ServerConfig => "server_config",
            Self::EnvBootstrap => "env_bootstrap",
            Self::CompiledDefault => "compiled_default",
        }
    }
}

/// Provider and model as data. Blank strings are stored as `None`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModelPair {
    pub provider: Option<String>,
    pub model: Option<String>,
}

impl ModelPair {
    pub fn new(provider: Option<&str>, model: Option<&str>) -> Self {
        Self {
            provider: clean(provider),
            model: clean(model),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.provider.is_none() && self.model.is_none()
    }
}

fn clean(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Class of the page being read. Only manuscript pages see the manuscript lock.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PageClass {
    #[default]
    Print,
    Manuscript,
}

/// `EDGEQUAKE_CONFIG_PRIORITY`: which leaf wins between server and env.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LeafOrder {
    #[default]
    ServerFirst,
    EnvFirst,
}

/// Operator pins read from the environment (LAW-175-10).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UsageLocks {
    pub extract: ModelPair,
    pub keyword: ModelPair,
    pub vlm_manuscript: ModelPair,
}

/// Every layer for one usage, already read by the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageLayers {
    pub locks: UsageLocks,
    pub page_class: PageClass,
    /// Per-request pair (query body or upload form). Ignored unless the usage accepts requests.
    pub request: ModelPair,
    /// `llm_roles.<usage>` of the workspace for this usage.
    pub workspace_role: ModelPair,
    pub workspace_default: ModelPair,
    pub tenant_default: ModelPair,
    pub server_config: ModelPair,
    pub env_bootstrap: ModelPair,
    pub leaf_order: LeafOrder,
    /// Last layer. Always present so the resolver never returns a blank pair.
    pub compiled_default: ModelPair,
}

impl UsageLayers {
    /// Layers with only the compiled default set; callers fill the rest.
    pub fn new(compiled_default: ModelPair) -> Self {
        Self {
            locks: UsageLocks::default(),
            page_class: PageClass::Print,
            request: ModelPair::default(),
            workspace_role: ModelPair::default(),
            workspace_default: ModelPair::default(),
            tenant_default: ModelPair::default(),
            server_config: ModelPair::default(),
            env_bootstrap: ModelPair::default(),
            leaf_order: LeafOrder::ServerFirst,
            compiled_default,
        }
    }
}

/// Output of [`resolve_usage`]: the pair that runs and the layer that named it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedUsage {
    pub usage: Usage,
    pub provider: String,
    pub model: String,
    pub source: ResolutionLayer,
}

impl ResolvedUsage {
    /// Locked means an operator pin decided the pair (the picker is disabled).
    pub fn is_locked(&self) -> bool {
        self.source == ResolutionLayer::OperatorLock
    }

    /// Identity for caches that must not cross models (EC-175-27).
    pub fn model_identity(&self) -> String {
        format!("{}/{}", self.provider, self.model)
    }
}

fn lock_for(usage: Usage, layers: &UsageLayers) -> Option<&ModelPair> {
    match usage {
        Usage::Extract => Some(&layers.locks.extract),
        Usage::Keyword => Some(&layers.locks.keyword),
        Usage::Vlm if layers.page_class == PageClass::Manuscript => {
            Some(&layers.locks.vlm_manuscript)
        }
        _ => None,
    }
}

/// Ordered candidate layers for one usage, highest precedence first.
fn ordered_layers(usage: Usage, layers: &UsageLayers) -> Vec<(ResolutionLayer, &ModelPair)> {
    let mut ordered = Vec::with_capacity(8);
    if let Some(lock) = lock_for(usage, layers) {
        ordered.push((ResolutionLayer::OperatorLock, lock));
    }
    if usage.accepts_request() {
        ordered.push((ResolutionLayer::Request, &layers.request));
    }
    ordered.push((ResolutionLayer::WorkspaceRole, &layers.workspace_role));
    ordered.push((ResolutionLayer::WorkspaceDefault, &layers.workspace_default));
    ordered.push((ResolutionLayer::TenantDefault, &layers.tenant_default));
    let (first, second) = match layers.leaf_order {
        LeafOrder::ServerFirst => (
            (ResolutionLayer::ServerConfig, &layers.server_config),
            (ResolutionLayer::EnvBootstrap, &layers.env_bootstrap),
        ),
        LeafOrder::EnvFirst => (
            (ResolutionLayer::EnvBootstrap, &layers.env_bootstrap),
            (ResolutionLayer::ServerConfig, &layers.server_config),
        ),
    };
    ordered.push(first);
    ordered.push(second);
    ordered.push((ResolutionLayer::CompiledDefault, &layers.compiled_default));
    ordered
}

/// Resolve one usage. The first non-blank layer wins; missing fields are
/// filled from lower layers in order, so provenance names the first layer.
pub fn resolve_usage(usage: Usage, layers: &UsageLayers) -> ResolvedUsage {
    let ordered = ordered_layers(usage, layers);
    let winner_index = ordered
        .iter()
        .position(|(_, pair)| !pair.is_empty())
        .unwrap_or(ordered.len() - 1);
    let (source, winner) = ordered[winner_index];

    let provider = first_present(
        std::iter::once(winner.provider.as_ref()).chain(
            ordered[winner_index + 1..]
                .iter()
                .map(|(_, p)| p.provider.as_ref()),
        ),
    );
    let model = first_present(
        std::iter::once(winner.model.as_ref()).chain(
            ordered[winner_index + 1..]
                .iter()
                .map(|(_, p)| p.model.as_ref()),
        ),
    );

    ResolvedUsage {
        usage,
        provider: provider.unwrap_or_default(),
        model: model.unwrap_or_default(),
        source,
    }
}

fn first_present<'a>(values: impl Iterator<Item = Option<&'a String>>) -> Option<String> {
    values.flatten().next().cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(provider: &str, model: &str) -> ModelPair {
        ModelPair::new(Some(provider), Some(model))
    }

    fn base() -> UsageLayers {
        UsageLayers::new(pair("ollama", "gemma3:latest"))
    }

    #[test]
    fn usage_lock_beats_workspace_role() {
        let mut layers = base();
        layers.locks.extract = pair("openai", "gpt-5-nano");
        layers.workspace_role = pair("ollama", "gemma3");
        let r = resolve_usage(Usage::Extract, &layers);
        assert_eq!(r.model, "gpt-5-nano");
        assert_eq!(r.source, ResolutionLayer::OperatorLock);
        assert!(r.is_locked());
    }

    #[test]
    fn usage_extract_pin_without_metadata() {
        let mut layers = base();
        layers.locks.extract = pair("openai", "gpt-5-nano");
        let r = resolve_usage(Usage::Extract, &layers);
        assert_eq!(r.source, ResolutionLayer::OperatorLock);
        assert_eq!(r.model_identity(), "openai/gpt-5-nano");

        // Unsetting the pin falls through to the next layer, not a cached pin.
        layers.locks.extract = ModelPair::default();
        let r = resolve_usage(Usage::Extract, &layers);
        assert_eq!(r.source, ResolutionLayer::CompiledDefault);
        assert!(!r.is_locked());
    }

    #[test]
    fn usage_embedding_env_is_bootstrap() {
        let mut layers = base();
        layers.env_bootstrap = pair("ollama", "embeddinggemma:latest");
        let r = resolve_usage(Usage::Embedding, &layers);
        assert_eq!(r.source, ResolutionLayer::EnvBootstrap);
        assert_eq!(r.model, "embeddinggemma:latest");

        // A workspace embedding key beats the env bootstrap leaf.
        layers.workspace_default = pair("openai", "text-embedding-3-small");
        let r = resolve_usage(Usage::Embedding, &layers);
        assert_eq!(r.source, ResolutionLayer::WorkspaceDefault);
        assert_eq!(r.model, "text-embedding-3-small");
    }

    #[test]
    fn usage_priority_swaps_leaf_only() {
        let mut layers = base();
        layers.env_bootstrap = pair("ollama", "env-model");
        layers.server_config = pair("openai", "server-model");
        // Default order: server_config beats env_bootstrap.
        assert_eq!(
            resolve_usage(Usage::Query, &layers).source,
            ResolutionLayer::ServerConfig
        );

        // EnvFirst: env beats server.
        layers.leaf_order = LeafOrder::EnvFirst;
        assert_eq!(
            resolve_usage(Usage::Query, &layers).source,
            ResolutionLayer::EnvBootstrap
        );

        // Under either order a tenant default still beats both leaves.
        layers.tenant_default = pair("mistral", "tenant-model");
        let r = resolve_usage(Usage::Query, &layers);
        assert_eq!(r.source, ResolutionLayer::TenantDefault);
        assert_eq!(r.model, "tenant-model");
    }

    #[test]
    fn usage_keyword_pin_is_lock() {
        let mut layers = base();
        layers.locks.keyword = pair("openai", "gpt-5-nano");
        layers.workspace_role = pair("ollama", "gemma3");
        let r = resolve_usage(Usage::Keyword, &layers);
        assert_eq!(r.source, ResolutionLayer::OperatorLock);
        assert!(r.is_locked());
        // The extract pin does not lock keyword.
        layers.locks.extract = pair("openai", "other");
        assert_eq!(resolve_usage(Usage::Keyword, &layers).model, "gpt-5-nano");
    }

    #[test]
    fn usage_manuscript_lock_scoped() {
        let mut layers = base();
        layers.locks.vlm_manuscript = pair("openai", "gpt-5.2");
        layers.workspace_role = pair("ollama", "llava");

        layers.page_class = PageClass::Manuscript;
        let r = resolve_usage(Usage::Vlm, &layers);
        assert_eq!(r.source, ResolutionLayer::OperatorLock);
        assert_eq!(r.model, "gpt-5.2");

        layers.page_class = PageClass::Print;
        let r = resolve_usage(Usage::Vlm, &layers);
        assert_eq!(r.source, ResolutionLayer::WorkspaceRole);
        assert_eq!(r.model, "llava");
    }

    #[test]
    fn usage_upload_vision_is_request() {
        let mut layers = base();
        layers.workspace_role = pair("ollama", "llava");
        layers.request = pair("openai", "gpt-5-mini");
        let r = resolve_usage(Usage::Vlm, &layers);
        assert_eq!(r.source, ResolutionLayer::Request);
        assert_eq!(r.model_identity(), "openai/gpt-5-mini");
    }

    #[test]
    fn usage_extract_ignores_request_pair() {
        let mut layers = base();
        layers.workspace_role = pair("ollama", "gemma3");
        layers.request = pair("openai", "gpt-5-nano");
        let r = resolve_usage(Usage::Extract, &layers);
        assert_eq!(r.source, ResolutionLayer::WorkspaceRole);
        assert_eq!(r.model, "gemma3");
        assert!(!Usage::Extract.accepts_request());
        assert!(!Usage::Summary.accepts_request());
    }

    #[test]
    fn query_request_beats_workspace_role_but_not_lock() {
        let mut layers = base();
        layers.workspace_role = pair("ollama", "gemma3");
        layers.request = pair("openai", "gpt-5-nano");
        assert_eq!(
            resolve_usage(Usage::Query, &layers).source,
            ResolutionLayer::Request
        );
    }

    #[test]
    fn answer_cache_key_includes_model() {
        let mut layers = base();
        layers.workspace_role = pair("openai", "gpt-5-nano");
        let a = resolve_usage(Usage::Query, &layers).model_identity();
        layers.workspace_role = pair("openai", "gpt-5-mini");
        let b = resolve_usage(Usage::Query, &layers).model_identity();
        assert_ne!(a, b, "a different model must change the cache identity");
        assert_eq!(a, "openai/gpt-5-nano");
    }

    #[test]
    fn missing_fields_fill_from_lower_layers() {
        let mut layers = base();
        // Winner names only a model; provider comes from the next layer.
        layers.workspace_role = ModelPair::new(None, Some("llava"));
        layers.tenant_default = pair("openai", "tenant-model");
        let r = resolve_usage(Usage::Vlm, &layers);
        assert_eq!(r.source, ResolutionLayer::WorkspaceRole);
        assert_eq!(r.provider, "openai");
        assert_eq!(r.model, "llava");
    }

    #[test]
    fn blank_layers_are_skipped() {
        let mut layers = base();
        layers.workspace_role = ModelPair::new(Some("  "), Some(""));
        let r = resolve_usage(Usage::Summary, &layers);
        assert_eq!(r.source, ResolutionLayer::CompiledDefault);
        assert_eq!(r.model, "gemma3:latest");
    }
}
