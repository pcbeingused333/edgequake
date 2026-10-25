//! SPEC-175 W2: adapts the deployment catalog (`/models` source) into the pure facts
//! that `edgequake_core::model_policy` decides on.
//!
//! The probe signal is `ModelResponse.available` (live discovery). Models with
//! `discovery_source = dynamic_api` have no static card and are never eligible.

use edgequake_core::model_policy::{CatalogEntry, ModelFacts};

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
    }
}

pub fn from_embedding(item: &EmbeddingModelItem) -> CatalogItem {
    let model = &item.model;
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
