//! Inner persist budget (SPEC-171). The worker wall clock stays the outer bound.

/// Default persist deadline. Clamped strictly under the task timeout.
pub const DEFAULT_PERSIST_TIMEOUT_SECS: u64 = 900;

/// `EDGEQUAKE_PERSIST_TIMEOUT_SECS`, scaled by page count, then clamped to
/// `30 .. effective_task_timeout - 60`.
pub fn persist_timeout_secs(page_count: usize, effective_task_timeout_secs: u64) -> u64 {
    let base = std::env::var("EDGEQUAKE_PERSIST_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_PERSIST_TIMEOUT_SECS);
    let pages = page_count.max(1) as u64;
    let scaled = base.saturating_add(pages.saturating_sub(2));
    let ceiling = effective_task_timeout_secs.saturating_sub(60).max(30);
    scaled.clamp(30, ceiling)
}

/// Transient persist failures leave the worker as `TaskError`. Everything else
/// stays on the existing storage-error path.
pub fn contention_task_error(message: String) -> Option<String> {
    let class = edgequake_tasks::classify_ingestion_failure(&message);
    if class == edgequake_tasks::IngestionFailureClass::PersistContention {
        Some(message)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_pipeline::{decide_description_merge, persist_phase_log, DescriptionMergePolicy};

    #[test]
    fn t171_21_persist_budget_clamps_under_task_timeout() {
        let prev = std::env::var("EDGEQUAKE_PERSIST_TIMEOUT_SECS").ok();
        std::env::set_var("EDGEQUAKE_PERSIST_TIMEOUT_SECS", "900");
        let budget = persist_timeout_secs(2, 120);
        assert!(budget < 120);
        assert_eq!(budget, 60);
        match prev {
            Some(value) => std::env::set_var("EDGEQUAKE_PERSIST_TIMEOUT_SECS", value),
            None => std::env::remove_var("EDGEQUAKE_PERSIST_TIMEOUT_SECS"),
        }
    }

    #[test]
    fn t171_22_page_count_scales_and_stays_under_the_task_timeout() {
        let prev = std::env::var("EDGEQUAKE_PERSIST_TIMEOUT_SECS").ok();
        std::env::set_var("EDGEQUAKE_PERSIST_TIMEOUT_SECS", "900");
        let small = persist_timeout_secs(2, 7200);
        let large = persist_timeout_secs(5_000, 7200);
        assert_eq!(small, 900);
        assert!(large > small);
        assert!(large < 7200);
        match prev {
            Some(value) => std::env::set_var("EDGEQUAKE_PERSIST_TIMEOUT_SECS", value),
            None => std::env::remove_var("EDGEQUAKE_PERSIST_TIMEOUT_SECS"),
        }
    }

    #[test]
    fn t171_23_two_short_fragments_do_not_need_a_summary_llm() {
        let policy = DescriptionMergePolicy::from_parts(true, 8, 1200, 0.85, 4096);
        let decision =
            decide_description_merge("Python in Nice", "Machine learning in France", &policy);
        assert!(matches!(
            decision,
            edgequake_pipeline::DescriptionMergeDecision::Resolved(_)
        ));
    }

    #[test]
    fn t171_24_phase_log_requires_both_ids() {
        assert!(!persist_phase_log("", "task", "merger"));
        assert!(!persist_phase_log("doc", "", "merger"));
        assert!(persist_phase_log("doc", "task", "merger"));
    }

    #[test]
    fn t171_25_contention_is_a_task_error_not_a_failed_ok() {
        let contention = contention_task_error(
            "Knowledge graph persist failed: sqlstate 55P03 lock_not_available".into(),
        );
        assert!(contention.is_some());
        let merge = contention_task_error("knowledge-graph merge error(s) during persist".into());
        assert!(merge.is_none());
    }
}
