//! SPEC-175 W1: the effective usage view for one workspace.
//!
//! Resolution lives in `edgequake_core::usage_layers`. This module reads the process
//! env once, loads policy rows and the catalog, then annotates each resolved usage
//! with eligibility. The UI never recomputes the ladder (LAW-175-1).

use edgequake_core::llm_roles::{
    env_extract_role_llm, env_keyword_role_llm, role_config_from_workspace, ResolvedRoleLlm,
};
use edgequake_core::model_policy::{tenant_policy_mode, CatalogEntry, PolicyMode};
use edgequake_core::usage_layers::{
    llm_role_for, resolve_effective_usages, EnvLayers, LayerSources,
};
use edgequake_core::usage_resolution::{LeafOrder, ModelPair, ResolutionLayer, ResolvedUsage};
use edgequake_core::{
    env::first_non_empty_env_var, ConfigPriorityMode, ServerLlmDefaults, Tenant, Workspace,
};

use crate::error::ApiResult;
use crate::handlers::effective_usage_types::EffectiveUsageDto;
use crate::services::model_policy_catalog::{load_catalog, CatalogItem};
use crate::services::model_policy_enforcement::{pair_verdict, PairVerdict};
use crate::services::model_policy_rows::load_rows;
use crate::state::AppState;

/// Result of [`effective_view`]: the tenant ceiling mode and one row per usage.
pub struct EffectiveView {
    pub policy_mode: PolicyMode,
    pub usages: Vec<EffectiveUsageDto>,
}

/// Resolve and annotate every effective usage for `workspace`.
pub async fn effective_view(
    state: &AppState,
    workspace: &Workspace,
    tenant: Option<&Tenant>,
    server: &ServerLlmDefaults,
    priority: ConfigPriorityMode,
) -> ApiResult<EffectiveView> {
    let env = env_layers_from_process();
    let src = LayerSources {
        workspace,
        tenant,
        server,
        env: &env,
        leaf_order: leaf_order(priority),
    };
    let resolved = resolve_effective_usages(&src);
    let ceiling = load_rows(state, workspace.tenant_id, None).await?;
    let narrowing = load_rows(state, workspace.tenant_id, Some(workspace.workspace_id)).await?;
    let catalog = load_catalog(state).await;
    let entries: Vec<CatalogEntry> = catalog.iter().map(|item| item.entry.clone()).collect();
    let usages = resolved
        .iter()
        .map(|r| {
            annotate(
                r,
                workspace,
                &Facts {
                    entries: &entries,
                    catalog: &catalog,
                    ceiling: &ceiling,
                    narrowing: &narrowing,
                },
            )
        })
        .collect();
    Ok(EffectiveView {
        policy_mode: tenant_policy_mode(&ceiling),
        usages,
    })
}

/// Loaded facts shared by every usage row.
struct Facts<'a> {
    entries: &'a [CatalogEntry],
    catalog: &'a [CatalogItem],
    ceiling: &'a [edgequake_core::model_policy::PolicyRow],
    narrowing: &'a [edgequake_core::model_policy::PolicyRow],
}

fn annotate(r: &ResolvedUsage, workspace: &Workspace, facts: &Facts<'_>) -> EffectiveUsageDto {
    let verdict = pair_verdict(
        r.usage,
        &r.provider,
        &r.model,
        facts.entries,
        facts.ceiling,
        facts.narrowing,
    );
    let (connection_id, reasoning_effort) = role_extras(r, workspace);
    EffectiveUsageDto {
        usage: r.usage.as_str().to_string(),
        provider: r.provider.clone(),
        provider_display_name: display_name(facts.catalog, &r.provider),
        model: r.model.clone(),
        connection_id,
        reasoning_effort,
        source: r.source.as_str().to_string(),
        locked: r.is_locked(),
        eligible: verdict == PairVerdict::Eligible,
        broken_reason: broken_reason(verdict).to_string(),
    }
}

fn broken_reason(verdict: PairVerdict) -> &'static str {
    match verdict {
        PairVerdict::Eligible => "none",
        PairVerdict::CapabilityMismatch => "capability",
        PairVerdict::NotEligible => "not_eligible",
    }
}

fn display_name(catalog: &[CatalogItem], provider: &str) -> Option<String> {
    let wanted = provider.trim().to_lowercase();
    catalog
        .iter()
        .find(|item| item.entry.provider == wanted)
        .map(|item| item.provider_display_name.clone())
}

/// Connection and reasoning effort are role-owned, so they only appear when the
/// winning layer is the workspace role for that usage.
fn role_extras(r: &ResolvedUsage, workspace: &Workspace) -> (Option<String>, Option<String>) {
    if r.source != ResolutionLayer::WorkspaceRole {
        return (None, None);
    }
    llm_role_for(r.usage)
        .and_then(|role| role_config_from_workspace(workspace, role))
        .map(|cfg| (cfg.connection_id, cfg.reasoning_effort))
        .unwrap_or((None, None))
}

fn leaf_order(priority: ConfigPriorityMode) -> LeafOrder {
    match priority {
        ConfigPriorityMode::ServerFirst => LeafOrder::ServerFirst,
        ConfigPriorityMode::EnvFirst => LeafOrder::EnvFirst,
    }
}

/// Read every operator pin and env bootstrap value once, at the edge.
pub fn env_layers_from_process() -> EnvLayers {
    EnvLayers {
        extract_lock: role_pair(env_extract_role_llm()),
        keyword_lock: role_pair(env_keyword_role_llm()),
        vlm_manuscript_lock: pair_from_env(
            &["EDGEQUAKE_VISION_PROVIDER_MANUSCRIPT"],
            &["EDGEQUAKE_VISION_MODEL_MANUSCRIPT"],
        ),
        bootstrap_chat: pair_from_env(
            &["EDGEQUAKE_DEFAULT_LLM_PROVIDER", "EDGEQUAKE_LLM_PROVIDER"],
            &["EDGEQUAKE_DEFAULT_LLM_MODEL", "EDGEQUAKE_LLM_MODEL"],
        ),
        bootstrap_embedding: pair_from_env(
            &[
                "EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER",
                "EDGEQUAKE_EMBEDDING_PROVIDER",
            ],
            &[
                "EDGEQUAKE_DEFAULT_EMBEDDING_MODEL",
                "EDGEQUAKE_EMBEDDING_MODEL",
            ],
        ),
        bootstrap_vision: pair_from_env(
            &["EDGEQUAKE_VISION_PROVIDER", "EDGEQUAKE_VISION_LLM_PROVIDER"],
            &["EDGEQUAKE_VISION_MODEL", "EDGEQUAKE_VISION_LLM_MODEL"],
        ),
    }
}

fn role_pair(role: Option<ResolvedRoleLlm>) -> ModelPair {
    role.map(|r| ModelPair::new(Some(&r.provider), Some(&r.model)))
        .unwrap_or_default()
}

fn pair_from_env(provider_keys: &[&str], model_keys: &[&str]) -> ModelPair {
    ModelPair::new(
        first_non_empty_env_var(provider_keys).as_deref(),
        first_non_empty_env_var(model_keys).as_deref(),
    )
}
