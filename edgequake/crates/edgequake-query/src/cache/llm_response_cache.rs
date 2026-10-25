//! SPEC-103 — LightRAG-parity LLM response cache (keywords + query).
//!
//! Unified durable cache: L1 memory + L2 `public.llm_cache` via [`KVStorage`].
//! Keys: `{mode}:{cache_type}:{hash}-cache` (LR flattened shape + SPEC-091 suffix).

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use edgequake_storage::kv_keys;
use edgequake_storage::traits::KVStorage;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// Field separator for hash inputs (avoids LR delimiter-less collisions).
const HASH_SEP: u8 = 0x1e;

/// LLM response cache family (v1: query path only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmCacheType {
    Keywords,
    Query,
}

impl LlmCacheType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Keywords => "keywords",
            Self::Query => "query",
        }
    }
}

/// Resolved enable flags (LAW-C6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LlmCacheFlags {
    pub master: bool,
    pub keywords: bool,
    pub answer: bool,
}

/// Master `EDGEQUAKE_LLM_CACHE` (default **on**).
pub fn master_llm_cache_enabled() -> bool {
    !matches!(
        std::env::var("EDGEQUAKE_LLM_CACHE")
            .ok()
            .as_deref()
            .map(str::trim)
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("0") | Some("false") | Some("off") | Some("no")
    )
}

fn env_falsey(name: &str) -> bool {
    matches!(
        std::env::var(name)
            .ok()
            .as_deref()
            .map(str::trim)
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("0") | Some("false") | Some("off") | Some("no")
    )
}

fn env_truthy_set(name: &str) -> Option<bool> {
    let raw = std::env::var(name).ok()?;
    let v = raw.trim().to_ascii_lowercase();
    if v.is_empty() {
        return None;
    }
    Some(matches!(v.as_str(), "1" | "true" | "yes" | "on"))
}

/// LAW-C6: master + keyword/answer overrides.
pub fn resolve_llm_cache_flags() -> LlmCacheFlags {
    let master = master_llm_cache_enabled();
    let keywords = master && !env_falsey("EDGEQUAKE_KEYWORD_CACHE");
    let answer = match env_truthy_set("EDGEQUAKE_QUERY_ANSWER_CACHE") {
        Some(explicit) => master && explicit,
        None => master, // unset → follow master (product default ON)
    };
    LlmCacheFlags {
        master,
        keywords,
        answer,
    }
}

/// Answer cache enabled (SPEC-103 / LAW-C6). Replaces 064 default-off when master on.
pub fn answer_cache_enabled_from_env() -> bool {
    resolve_llm_cache_flags().answer
}

/// Keyword cache enabled under master switch.
pub fn keyword_cache_enabled_from_flags() -> bool {
    resolve_llm_cache_flags().keywords
}

/// Delimited SHA-256 of args (LAW-C3).
pub fn compute_args_hash(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for (i, p) in parts.iter().enumerate() {
        if i > 0 {
            hasher.update([HASH_SEP]);
        }
        hasher.update(p.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

/// Flattened LR-shaped id before SPEC-091 suffix.
pub fn flattened_cache_id(mode: &str, cache_type: LlmCacheType, hash: &str) -> String {
    format!("{mode}:{}:{hash}", cache_type.as_str())
}

/// Storage key: `{mode}:{cache_type}:{hash}-cache`.
pub fn llm_cache_storage_key(mode: &str, cache_type: LlmCacheType, hash: &str) -> String {
    kv_keys::llm_cache(&flattened_cache_id(mode, cache_type, hash))
}

pub fn hash_keyword_args(query: &str, mode: &str, model: &str, language: Option<&str>) -> String {
    compute_args_hash(&[query, mode, model, language.unwrap_or("")])
}

/// Query answer hash = full built RAG prompt (context-inclusive).
pub fn hash_query_prompt(prompt: &str) -> String {
    compute_args_hash(&[prompt])
}

/// SPEC-109: include effective reasoning effort when set so effort changes bust cache.
pub fn hash_query_prompt_with_effort(prompt: &str, reasoning_effort: Option<&str>) -> String {
    match reasoning_effort.map(str::trim).filter(|s| !s.is_empty()) {
        Some(effort) => compute_args_hash(&[prompt, effort]),
        None => hash_query_prompt(prompt),
    }
}

/// Legacy helper used by answer_cache module — SHA of prompt.
pub fn answer_cache_key(prompt: &str) -> String {
    llm_cache_storage_key("mix", LlmCacheType::Query, &hash_query_prompt(prompt))
}

/// Model identity an answer is produced under (`provider/model`).
pub fn answer_model_identity(llm: &dyn crate::LLMProvider) -> String {
    format!("{}/{}", llm.name(), llm.model())
}

/// SPEC-175 EC-175-27: the answer hash covers the built prompt, the model that writes the
/// answer, and the reasoning effort. A completion from model A is never served for model B.
pub fn hash_query_answer(
    prompt: &str,
    model_identity: &str,
    reasoning_effort: Option<&str>,
) -> String {
    let effort = reasoning_effort
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("");
    compute_args_hash(&[model_identity, effort, prompt])
}

/// Legacy L1 answer key that includes the model identity (SPEC-175 EC-175-27).
pub fn answer_cache_key_for_model(prompt: &str, model_identity: &str) -> String {
    llm_cache_storage_key(
        "mix",
        LlmCacheType::Query,
        &hash_query_answer(prompt, model_identity, None),
    )
}

/// Unified LLM response cache port (DIP).
#[async_trait]
pub trait LlmResponseCache: Send + Sync {
    async fn get_return(&self, key: &str) -> Option<String>;
    async fn set_return(
        &self,
        key: &str,
        cache_type: LlmCacheType,
        value: &str,
        original_prompt: Option<&str>,
    );
    fn clear_l1(&self);
}

fn envelope(cache_type: LlmCacheType, value: &str, original_prompt: Option<&str>) -> Value {
    let mut obj = json!({
        "return": value,
        "cache_type": cache_type.as_str(),
    });
    if let Some(p) = original_prompt {
        obj.as_object_mut()
            .unwrap()
            .insert("original_prompt".into(), json!(p));
    }
    obj
}

fn parse_return(v: &Value) -> Option<String> {
    v.get("return").and_then(|x| x.as_str()).map(str::to_string)
}

/// In-process L1 LRU+TTL.
pub struct MemoryLlmResponseCache {
    max_size: usize,
    ttl: Duration,
    cache: RwLock<HashMap<String, (String, Instant, Instant)>>,
}

impl MemoryLlmResponseCache {
    pub fn new(max_size: usize, ttl: Duration) -> Self {
        Self {
            max_size,
            ttl,
            cache: RwLock::new(HashMap::new()),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(1_000, Duration::from_secs(3600))
    }
}

#[async_trait]
impl LlmResponseCache for MemoryLlmResponseCache {
    async fn get_return(&self, key: &str) -> Option<String> {
        let now = Instant::now();
        let mut cache = self.cache.write().unwrap();
        if let Some((val, expires, accessed)) = cache.get_mut(key) {
            if *expires > now {
                *accessed = now;
                return Some(val.clone());
            }
            cache.remove(key);
        }
        None
    }

    async fn set_return(
        &self,
        key: &str,
        _cache_type: LlmCacheType,
        value: &str,
        _original_prompt: Option<&str>,
    ) {
        let now = Instant::now();
        let mut cache = self.cache.write().unwrap();
        cache.retain(|_, (_, exp, _)| *exp > now);
        while cache.len() >= self.max_size {
            let oldest = cache
                .iter()
                .min_by_key(|(_, (_, _, a))| *a)
                .map(|(k, _)| k.clone());
            match oldest {
                Some(k) => {
                    cache.remove(&k);
                }
                None => break,
            }
        }
        cache.insert(key.to_string(), (value.to_string(), now + self.ttl, now));
    }

    fn clear_l1(&self) {
        self.cache.write().unwrap().clear();
    }
}

/// L2 via KV / typed `public.llm_cache`.
pub struct KvLlmResponseCache {
    kv: Arc<dyn KVStorage>,
}

impl KvLlmResponseCache {
    pub fn new(kv: Arc<dyn KVStorage>) -> Self {
        Self { kv }
    }
}

#[async_trait]
impl LlmResponseCache for KvLlmResponseCache {
    async fn get_return(&self, key: &str) -> Option<String> {
        match self.kv.get_by_id(key).await {
            Ok(Some(v)) => parse_return(&v),
            Ok(None) => None,
            Err(e) => {
                tracing::warn!(error = %e, key = %key, "SPEC-103 L2 cache get failed");
                None
            }
        }
    }

    async fn set_return(
        &self,
        key: &str,
        cache_type: LlmCacheType,
        value: &str,
        original_prompt: Option<&str>,
    ) {
        let entry = envelope(cache_type, value, original_prompt);
        if let Err(e) = self.kv.upsert(&[(key.to_string(), entry)]).await {
            tracing::warn!(error = %e, key = %key, "SPEC-103 L2 cache set failed");
        }
    }

    fn clear_l1(&self) {}
}

/// L1 then L2 (LAW-C5).
pub struct TieredLlmResponseCache {
    l1: MemoryLlmResponseCache,
    l2: KvLlmResponseCache,
}

impl TieredLlmResponseCache {
    pub fn new(l1: MemoryLlmResponseCache, l2: KvLlmResponseCache) -> Self {
        Self { l1, l2 }
    }

    pub fn with_kv(kv: Arc<dyn KVStorage>) -> Self {
        Self::new(
            MemoryLlmResponseCache::with_defaults(),
            KvLlmResponseCache::new(kv),
        )
    }
}

#[async_trait]
impl LlmResponseCache for TieredLlmResponseCache {
    async fn get_return(&self, key: &str) -> Option<String> {
        if let Some(v) = self.l1.get_return(key).await {
            return Some(v);
        }
        if let Some(v) = self.l2.get_return(key).await {
            self.l1.set_return(key, LlmCacheType::Query, &v, None).await;
            return Some(v);
        }
        None
    }

    async fn set_return(
        &self,
        key: &str,
        cache_type: LlmCacheType,
        value: &str,
        original_prompt: Option<&str>,
    ) {
        self.l1
            .set_return(key, cache_type, value, original_prompt)
            .await;
        self.l2
            .set_return(key, cache_type, value, original_prompt)
            .await;
    }

    fn clear_l1(&self) {
        self.l1.clear_l1();
    }
}

pub type SharedLlmResponseCache = Arc<dyn LlmResponseCache>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answer_cache_key_changes_with_model() {
        // EC-175-27: the same prompt under two models must not share an answer.
        let a = hash_query_answer("ctx + question", "openai/gpt-5-nano", None);
        let b = hash_query_answer("ctx + question", "openai/gpt-5-mini", None);
        assert_ne!(a, b);
        assert_ne!(
            answer_cache_key_for_model("p", "openai/gpt-5-nano"),
            answer_cache_key_for_model("p", "openai/gpt-5-mini")
        );
        // Blank effort is the same as no effort; a real effort busts the key.
        assert_eq!(
            hash_query_answer("p", "m", Some("  ")),
            hash_query_answer("p", "m", None)
        );
        assert_ne!(
            hash_query_answer("p", "m", Some("low")),
            hash_query_answer("p", "m", None)
        );
    }

    #[test]
    fn key_format_and_hash_pins() {
        let h = hash_keyword_args("What is BRCA1?", "default", "mistral-small-latest", None);
        let key = llm_cache_storage_key("default", LlmCacheType::Keywords, &h);
        assert!(key.ends_with("-cache"), "{key}");
        assert!(key.contains(":keywords:"), "{key}");
        assert_ne!(
            hash_keyword_args("a", "default", "m1", None),
            hash_keyword_args("a", "default", "m2", None)
        );
        // Adjacent-field collision guard
        assert_ne!(
            compute_args_hash(&["ab", "c"]),
            compute_args_hash(&["a", "bc"])
        );
        let qk = answer_cache_key("full prompt with context");
        assert!(qk.contains(":query:"));
        assert!(qk.ends_with("-cache"));
    }

    #[test]
    fn hash_query_prompt_includes_effort_when_set() {
        let base = hash_query_prompt("prompt");
        assert_eq!(hash_query_prompt_with_effort("prompt", None), base);
        assert_eq!(hash_query_prompt_with_effort("prompt", Some("")), base);
        assert_ne!(
            hash_query_prompt_with_effort("prompt", Some("low")),
            hash_query_prompt_with_effort("prompt", Some("high"))
        );
        assert_ne!(hash_query_prompt_with_effort("prompt", Some("low")), base);
    }

    #[test]
    fn master_off_disables_answer_and_keywords() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::set_var("EDGEQUAKE_LLM_CACHE", "0");
        std::env::remove_var("EDGEQUAKE_QUERY_ANSWER_CACHE");
        std::env::remove_var("EDGEQUAKE_KEYWORD_CACHE");
        let f = resolve_llm_cache_flags();
        assert!(!f.master && !f.keywords && !f.answer);
        std::env::remove_var("EDGEQUAKE_LLM_CACHE");
    }

    #[test]
    fn master_on_answer_follows_unless_override() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::set_var("EDGEQUAKE_LLM_CACHE", "1");
        std::env::remove_var("EDGEQUAKE_QUERY_ANSWER_CACHE");
        assert!(resolve_llm_cache_flags().answer);
        std::env::set_var("EDGEQUAKE_QUERY_ANSWER_CACHE", "0");
        assert!(!resolve_llm_cache_flags().answer);
        std::env::remove_var("EDGEQUAKE_LLM_CACHE");
        std::env::remove_var("EDGEQUAKE_QUERY_ANSWER_CACHE");
    }

    #[tokio::test]
    async fn memory_round_trip() {
        let c = MemoryLlmResponseCache::with_defaults();
        let key = llm_cache_storage_key("mix", LlmCacheType::Query, &hash_query_prompt("p"));
        assert!(c.get_return(&key).await.is_none());
        c.set_return(&key, LlmCacheType::Query, "ans", Some("p"))
            .await;
        assert_eq!(c.get_return(&key).await.as_deref(), Some("ans"));
        c.clear_l1();
        assert!(c.get_return(&key).await.is_none());
    }

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
}
