//! Load recent conversation history for chat completions (DRY / SOLID).
//!
//! Single responsibility: map persisted messages → engine `ConversationMessage`
//! **recent pool**. History *policy* (window + token budget + pair-safe) lives
//! only in `edgequake_query::conversation_context` (2026 AI eng SSOT).

use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use edgequake_core::conversation_service::ConversationService;
use edgequake_query::ConversationMessage;

/// Raw recent pool before policy (enough for token-budget trim of long turns).
///
/// Fetched with a backward index scan. One extra row covers the just-saved
/// user message, which this loader drops before the pool is trimmed.
const RECENT_HISTORY_POOL: usize = 64;

/// Load prior turns for a conversation, excluding the just-saved user message.
///
/// Returns the recent chronological pool (not yet token-trimmed). The engine
/// applies [`edgequake_query::conversation_context::apply_history_policy`].
pub async fn load_recent_conversation_history(
    conversation_service: &dyn ConversationService,
    conversation_id: Uuid,
    exclude_message_id: Uuid,
) -> ApiResult<Vec<ConversationMessage>> {
    let page = conversation_service
        .list_recent_messages(conversation_id, RECENT_HISTORY_POOL + 1)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to load conversation history: {e}")))?;

    let hidden = hidden_document_titles().await;
    let mut prior: Vec<ConversationMessage> = page
        .into_iter()
        .filter(|m| m.message_id != exclude_message_id)
        .filter(|m| !m.content.trim().is_empty())
        .map(|m| ConversationMessage {
            role: m.role.to_string(),
            content: m.content,
        })
        .filter(|message| history_keeps(&message.content, &hidden))
        .collect();

    // ASC list → keep recent tail only (First Principles: model needs recency).
    if prior.len() > RECENT_HISTORY_POOL {
        prior = prior.split_off(prior.len() - RECENT_HISTORY_POOL);
    }

    Ok(prior)
}

/// Drop a turn that quotes a document the caller cannot open.
pub fn history_keeps(content: &str, hidden_titles: &[String]) -> bool {
    hidden_titles
        .iter()
        .all(|title| title.is_empty() || !content.contains(title.as_str()))
}

#[allow(clippy::needless_return)] // `return` ends the cfg(postgres) block; the fallback follows it.
async fn hidden_document_titles() -> Vec<String> {
    let Some(session) = edgequake_storage::clearance_session::ClearanceSession::current() else {
        return Vec::new();
    };
    if session.predicate_sql().is_empty() {
        return Vec::new();
    }
    #[cfg(feature = "postgres")]
    {
        let Some(pool) = crate::services::relational_sidecar_store::sidecar_pool() else {
            return Vec::new();
        };
        let mut tx = match pool.begin().await {
            Ok(tx) => tx,
            Err(_) => return Vec::new(),
        };
        if edgequake_storage::adapters::postgres::rls::apply_clearance_session(&mut tx, &session)
            .await
            .is_err()
        {
            return Vec::new();
        }
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT COALESCE(title, '')
             FROM public.documents
             WHERE workspace_id = (SELECT public.current_workspace_id())
               AND NOT (
                 public.clearance_covers(classification_rank, compartment_ids)
                 OR (uploaded_by = (SELECT public.current_user_id()) AND classification_rank = 5)
               )",
        )
        .fetch_all(&mut *tx)
        .await
        .unwrap_or_default();
        let _ = tx.commit().await;
        return rows
            .into_iter()
            .map(|(title,)| title)
            .filter(|t| !t.is_empty())
            .collect();
    }
    #[cfg(not(feature = "postgres"))]
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn e2e_spec174_history_redaction() {
        let hidden = vec!["Atlas payroll".to_string()];
        assert!(!history_keeps("See Atlas payroll for the figure.", &hidden));
        assert!(history_keeps("The public note is fine.", &hidden));
    }

    #[test]
    fn recent_pool_is_larger_than_default_window() {
        const {
            assert!(RECENT_HISTORY_POOL >= 16);
            assert!(RECENT_HISTORY_POOL < 10_000);
        }
    }
}
