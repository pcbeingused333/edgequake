//! SPEC-175 W2: adapts the deployment catalog (`/models` source) into the pure facts
//! that `edgequake_core::model_policy` decides on.
//!
//! The probe signal is `ModelResponse.available` (live discovery). Models with
//! `discovery_source = dynamic_api` have no static card and are never eligible.

use edgequake_core::model_policy::{CatalogEntry, ModelFacts};

use crate::error::ApiResult;
use crate::handlers::models_types::{EmbeddingModelItem, LlmModelItem, ModelResponse};
use crate::model_catalog::active_provider_names;
use crate::state::AppState;

/// One catalog model plus the display fields the member picker needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogItem {
    pub entry: CatalogEntry,
    pub display_name: String,
    pub provider_display_name: String,
    pub supports_vision: bool,
    pub supports_streaming: bool,
    /// Set for embedding cards. Chat cards leave this empty.
    pub embedding_dimension: Option<i32>,
}

/// Discovery-only models have no static card, so they are not eligible (EC-175-11).
fn registered(model: &ModelResponse) -> bool {
    model.discovery_source.as_deref() != Some("dynamic_api")
}

pub fn from_llm(item: &LlmModelItem) -> CatalogItem {
    let model = &item.model;
    let vision = model.capabilities.supports_vision;
    CatalogItem {
        entry: CatalogEntry {
            provider: item.provider.to_lowercase(),
            model: model.name.clone(),
            facts: ModelFacts {
                chat: true,
                vision,
                embedding: false,
                registered: registered(model),
                probe: model.available,
            },
        },
        display_name: model.display_name.clone(),
        provider_display_name: item.provider_display_name.clone(),
        supports_vision: vision,
        supports_streaming: model.capabilities.supports_streaming,
        embedding_dimension: None,
    }
}

pub fn from_embedding(item: &EmbeddingModelItem) -> CatalogItem {
    let model = &item.model;
    let dim = model.capabilities.embedding_dimension;
    CatalogItem {
        entry: CatalogEntry {
            provider: item.provider.to_lowercase(),
            model: model.name.clone(),
            facts: ModelFacts {
                chat: false,
                vision: false,
                embedding: true,
                registered: registered(model),
                probe: model.available,
            },
        },
        display_name: model.display_name.clone(),
        provider_display_name: item.provider_display_name.clone(),
        supports_vision: false,
        supports_streaming: false,
        embedding_dimension: (dim > 0).then_some(dim as i32),
    }
}

/// Every LLM and embedding model the deployment exposes, in the same order as `/models`.
pub async fn load_catalog(state: &AppState) -> Vec<CatalogItem> {
    let config = &*state.query.models_config;
    let allowed = active_provider_names();
    let llm = state
        .query
        .model_catalog
        .list_llm_models(config, &allowed)
        .await;
    let embedding = state
        .query
        .model_catalog
        .list_embedding_models(config, &allowed)
        .await;
    llm.iter()
        .map(from_llm)
        .chain(embedding.iter().map(from_embedding))
        .collect()
}

/// Models saved on a fleet connection join the catalog under that connection's slug.
/// A custom provider does not inherit another provider's cards.
#[cfg(feature = "postgres")]
pub async fn append_connection_models(
    state: &AppState,
    catalog: &mut Vec<CatalogItem>,
) -> ApiResult<()> {
    let Some(pool) = state.optional_pg_pool() else {
        return Ok(());
    };
    let rows: Vec<(String, String, bool, bool, bool, Option<i32>)> = sqlx::query_as(
        "SELECT c.slug, m.model, m.supports_chat, m.supports_vision, m.supports_embedding, \
                m.embedding_dimension \
         FROM connection_models m \
         JOIN provider_connections c ON c.id = m.connection_id \
         WHERE c.tenant_id IS NULL \
         ORDER BY c.slug, m.model",
    )
    .fetch_all(pool)
    .await
    .map_err(|err| crate::error::ApiError::Internal(err.to_string()))?;
    for (slug, model, chat, vision, embedding, dimension) in rows {
        let slug = slug.to_lowercase();
        if catalog
            .iter()
            .any(|item| item.entry.provider == slug && item.entry.model == model)
        {
            continue;
        }
        catalog.push(CatalogItem {
            display_name: model.clone(),
            provider_display_name: slug.clone(),
            supports_vision: vision,
            supports_streaming: chat,
            embedding_dimension: dimension,
            entry: CatalogEntry {
                provider: slug,
                model,
                facts: ModelFacts {
                    chat,
                    vision,
                    embedding,
                    registered: true,
                    probe: Some(true),
                },
            },
        });
    }
    Ok(())
}

#[cfg(test)]
mod registration_eligibility_tests {
    use std::collections::BTreeSet;

    use edgequake_core::model_policy::{eligible_models, CatalogEntry, ModelFacts};
    use edgequake_core::usage_resolution::Usage;

    fn entry(provider: &str, model: &str, registered: bool) -> CatalogEntry {
        CatalogEntry {
            provider: provider.to_string(),
            model: model.to_string(),
            facts: ModelFacts {
                chat: true,
                vision: false,
                embedding: false,
                registered,
                probe: Some(true),
            },
        }
    }

    /// Confirmed `connection_models` rows are registered and eligible.
    /// A discovery-only id (`registered = false`) stays off the choice list.
    #[test]
    fn confirmed_models_are_eligible_for_each_refreshable_provider() {
        let catalog = vec![
            entry("ollama", "gemma4:latest", true),
            entry("openai", "gpt-5-nano", true),
            entry("openrouter", "live-only", false),
        ];
        let offered = BTreeSet::from([
            "ollama".to_string(),
            "openai".to_string(),
            "openrouter".to_string(),
        ]);
        let eligible = eligible_models(Usage::Query, &catalog, &[], &[], Some(&offered));
        let pairs: Vec<_> = eligible
            .iter()
            .map(|item| (item.provider.as_str(), item.model.as_str()))
            .collect();
        assert!(pairs.contains(&("ollama", "gemma4:latest")));
        assert!(pairs.contains(&("openai", "gpt-5-nano")));
        assert!(!pairs.iter().any(|(_, model)| *model == "live-only"));
    }
}

#[cfg(not(feature = "postgres"))]
pub async fn append_connection_models(
    _state: &AppState,
    _catalog: &mut Vec<CatalogItem>,
) -> ApiResult<()> {
    Ok(())
}
