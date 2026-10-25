//! SPEC-175 W1/W2: server-side enforcement of a per-request model pair.
//!
//! The WebUI picker is a convenience. The server is the authority (LAW-175-13): a
//! request that names a provider/model pair must be in `eligible_models(usage, None)` for the
//! caller's workspace, or it is refused with 422 `model_not_eligible` /
//! `capability_mismatch`.
//!
//! Requests without a pair are untouched, so the workspace default still applies.

use edgequake_core::model_policy::{eligible_models, CatalogEntry, PolicyRow};
use edgequake_core::{Usage, Workspace};

use crate::error::{ApiError, ApiResult};
use crate::services::model_policy_catalog::load_catalog;
use crate::services::model_policy_rows::load_rows;
use crate::state::AppState;

/// Why a pair is or is not selectable. Shared by enforcement and the effective view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairVerdict {
    Eligible,
    CapabilityMismatch,
    NotEligible,
}

/// A locked usage ignores the offer. Every other usage is limited to it.
pub fn offer_filter<'a>(
    locked: bool,
    offered: &'a std::collections::BTreeSet<String>,
) -> Option<&'a std::collections::BTreeSet<String>> {
    if locked { None } else { Some(offered) }
}

/// Pure decision over already-loaded facts.
pub fn pair_verdict(
    usage: Usage,
    provider: &str,
    model: &str,
    catalog: &[CatalogEntry],
    ceiling: &[PolicyRow],
    narrowing: &[PolicyRow],
    offered: Option<&std::collections::BTreeSet<String>>,
) -> PairVerdict {
    let provider = provider.trim().to_lowercase();
    let model = model.trim();
    let is_pair = |entry: &&CatalogEntry| entry.provider == provider && entry.model == model;
    if eligible_models(usage, catalog, ceiling, narrowing, offered)
        .iter()
        .any(is_pair)
    {
        return PairVerdict::Eligible;
    }
    match catalog.iter().find(is_pair) {
        Some(entry) if !entry.facts.supports(usage) => PairVerdict::CapabilityMismatch,
        _ => PairVerdict::NotEligible,
    }
}

/// Map a verdict to the request outcome: `Ok`, or a 422 with the matching code.
pub fn check_pair(
    usage: Usage,
    provider: &str,
    model: &str,
    catalog: &[CatalogEntry],
    ceiling: &[PolicyRow],
    narrowing: &[PolicyRow],
    offered: Option<&std::collections::BTreeSet<String>>,
) -> ApiResult<()> {
    match pair_verdict(usage, provider, model, catalog, ceiling, narrowing, offered) {
        PairVerdict::Eligible => Ok(()),
        PairVerdict::CapabilityMismatch => Err(ApiError::unprocessable_coded(
            "capability_mismatch",
            format!("model does not support the {} usage", usage.as_str()),
        )),
        PairVerdict::NotEligible => Err(ApiError::unprocessable_coded(
            "model_not_eligible",
            "model is not selectable for this workspace",
        )),
    }
}

/// Enforce the pair for `usage` in `workspace`. A request with no pair passes. A pair
/// with no workspace scope fails closed.
pub async fn ensure_pair_eligible(
    state: &AppState,
    workspace: Option<&Workspace>,
    usage: Usage,
    provider: Option<&str>,
    model: Option<&str>,
) -> ApiResult<()> {
    let (Some(provider), Some(model)) = (provider, model) else {
        return Ok(());
    };
    let Some(workspace) = workspace else {
        return Err(ApiError::unprocessable_coded(
            "model_not_eligible",
            "a model override needs a workspace scope",
        ));
    };
    let ceiling = load_rows(state, workspace.tenant_id, None).await?;
    let narrowing = load_rows(state, workspace.tenant_id, Some(workspace.workspace_id)).await?;
    let mut catalog = load_catalog(state).await;
    crate::services::model_policy_catalog::append_connection_models(state, &mut catalog).await?;
    let entries: Vec<CatalogEntry> = catalog.into_iter().map(|item| item.entry).collect();
    let offer = crate::services::provider_offer_rows::load_workspace_offer(
        state,
        workspace.tenant_id,
        workspace.workspace_id,
    )
    .await?;
    check_pair(
        usage,
        provider,
        model,
        &entries,
        &ceiling,
        &narrowing,
        Some(&offer.slugs),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_core::model_policy::ModelFacts;

    fn chat(provider: &str, model: &str) -> CatalogEntry {
        CatalogEntry {
            provider: provider.into(),
            model: model.into(),
            facts: ModelFacts {
                chat: true,
                registered: true,
                ..ModelFacts::default()
            },
        }
    }

    fn code(result: ApiResult<()>) -> Option<&'static str> {
        result.err().map(|e| e.code())
    }

    fn row(provider: &str, model: &str, usage: Usage, enabled: bool) -> PolicyRow {
        PolicyRow {
            provider: provider.into(),
            model: model.into(),
            usage,
            enabled,
        }
    }

    #[test]
    fn selectable_pair_passes_and_provider_is_case_insensitive() {
        let cat = [chat("openai", "gpt-5-nano")];
        assert!(check_pair(Usage::Query, " OpenAI ", "gpt-5-nano", &cat, &[], &[], None).is_ok());
    }

    #[test]
    fn unknown_pair_is_model_not_eligible() {
        let cat = [chat("openai", "gpt-5-nano")];
        let got = check_pair(Usage::Query, "openai", "gpt-unknown", &cat, &[], &[], None);
        assert_eq!(code(got), Some("model_not_eligible"));
    }

    #[test]
    fn pair_outside_ceiling_is_model_not_eligible() {
        let cat = [chat("openai", "gpt-5-nano"), chat("ollama", "gemma3")];
        let ceiling = [row("openai", "gpt-5-nano", Usage::Query, true)];
        let got = check_pair(Usage::Query, "ollama", "gemma3", &cat, &ceiling, &[], None);
        assert_eq!(code(got), Some("model_not_eligible"));
    }

    #[test]
    fn pair_disabled_by_workspace_is_model_not_eligible() {
        let cat = [chat("openai", "gpt-5-nano")];
        let narrowing = [row("openai", "gpt-5-nano", Usage::Query, false)];
        let got = check_pair(
            Usage::Query,
            "openai",
            "gpt-5-nano",
            &cat,
            &[],
            &narrowing,
            None,
        );
        assert_eq!(code(got), Some("model_not_eligible"));
    }

    #[test]
    fn known_model_without_the_usage_capability_is_capability_mismatch() {
        let cat = [chat("openai", "gpt-5-nano")];
        // A chat model cannot serve embedding.
        let got = check_pair(
            Usage::Embedding,
            "openai",
            "gpt-5-nano",
            &cat,
            &[],
            &[],
            None,
        );
        assert_eq!(code(got), Some("capability_mismatch"));
    }

    #[test]
    fn discovery_only_model_is_model_not_eligible() {
        let mut entry = chat("ollama", "qwen-local");
        entry.facts.registered = false;
        let got = check_pair(
            Usage::Query,
            "ollama",
            "qwen-local",
            &[entry],
            &[],
            &[],
            None,
        );
        assert_eq!(code(got), Some("model_not_eligible"));
    }

    #[test]
    fn eligible_omits_slug_outside_the_offer() {
        let cat = [chat("openai", "gpt-5-nano"), chat("acme", "local")];
        let offered = std::collections::BTreeSet::from(["openai".to_string()]);
        let got = check_pair(
            Usage::Query,
            "acme",
            "local",
            &cat,
            &[],
            &[],
            Some(&offered),
        );
        assert_eq!(code(got), Some("model_not_eligible"));
        assert!(check_pair(
            Usage::Query,
            "openai",
            "gpt-5-nano",
            &cat,
            &[],
            &[],
            Some(&offered),
        )
        .is_ok());
    }

    #[test]
    fn lock_ignores_withdrawn_grant() {
        let cat = [chat("openai", "gpt-5-nano")];
        let offered = std::collections::BTreeSet::from(["acme".to_string()]);
        let locked = offer_filter(true, &offered);
        assert!(locked.is_none());
        assert!(check_pair(
            Usage::Extract,
            "openai",
            "gpt-5-nano",
            &cat,
            &[],
            &[],
            locked,
        )
        .is_ok());
        let unlocked = offer_filter(false, &offered);
        let got = check_pair(
            Usage::Extract,
            "openai",
            "gpt-5-nano",
            &cat,
            &[],
            &[],
            unlocked,
        );
        assert_eq!(code(got), Some("model_not_eligible"));
    }
}
