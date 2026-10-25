//! SPEC-175 W1: wire shape for one resolved usage in `GET /config/effective`.
//!
//! Matches `ResolvedUsage` in `edgequake_webui/src/lib/api/edgequake/model-catalog.ts`.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// One usage as the server resolved it for a workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct EffectiveUsageDto {
    /// Wire name: `extract`, `query`, `summary`, `vlm`, `keyword`, `embedding`, `decision`.
    pub usage: String,
    pub provider: String,
    pub provider_display_name: Option<String>,
    pub model: String,
    /// `provider_connections.id` when the winning workspace role set one.
    pub connection_id: Option<String>,
    /// Reasoning effort when the winning workspace role set one.
    pub reasoning_effort: Option<String>,
    /// Winning layer: `operator_lock`, `workspace_role`, `workspace_default`, …
    pub source: String,
    pub locked: bool,
    /// The pair is in `eligible_models(usage, None)` for this workspace.
    pub eligible: bool,
    /// `none` when eligible; otherwise `capability` or `not_eligible`.
    pub broken_reason: String,
}
