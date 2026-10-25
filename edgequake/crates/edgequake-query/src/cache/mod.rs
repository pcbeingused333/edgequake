//! Query-side caches (SPEC-021 P-G9 / RC-14 · 064 · SPEC-103).
//!
//! - `CachingEmbeddingProvider`: memoizes `embed_one` **and** per-text batch `embed`.
//! - `QueryResultCache`: memoizes `context_only` retrieval contexts.
//! - `LlmResponseCache`: unified keywords+query durable cache (LR parity).

pub mod answer_cache;
pub mod embedding_cache;
pub mod llm_response_cache;
pub mod query_result_cache;

pub use answer_cache::{
    answer_cache_enabled_from_env, answer_cache_key, InMemoryAnswerCache, SharedAnswerCache,
};
pub use embedding_cache::CachingEmbeddingProvider;
pub use llm_response_cache::{
    answer_cache_key_for_model, answer_model_identity, hash_keyword_args, hash_query_answer,
    hash_query_prompt, hash_query_prompt_with_effort, keyword_cache_enabled_from_flags,
    llm_cache_storage_key, master_llm_cache_enabled, resolve_llm_cache_flags, KvLlmResponseCache,
    LlmCacheFlags, LlmCacheType, LlmResponseCache, MemoryLlmResponseCache, SharedLlmResponseCache,
    TieredLlmResponseCache,
};
pub use query_result_cache::QueryResultCache;

/// Port for invalidating cached `context_only` retrieval after ingestion (DIP).
pub trait QueryResultCacheInvalidator: Send + Sync {
    fn invalidate_query_result_cache(&self);

    /// Workspace-scoped invalidation — does not bust other workspaces' cache entries.
    fn invalidate_query_result_cache_for_workspace(&self, workspace_id: &str) {
        let _ = workspace_id;
        self.invalidate_query_result_cache();
    }
}
