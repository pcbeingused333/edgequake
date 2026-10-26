//! SPEC-179 — the filter is the judge, including ingest.
//!
//! Pure decisions live next to `model_gate`. This file locks the callers and
//! the constructor boundary. Behaviour that needs Postgres is covered by the
//! existing offer tests plus `judge_ingest`, which no-ops without a pool and
//! refuses a pair the offer removed when a pool is present.

use std::path::Path;

#[test]
fn constructor_guard() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offenders = Vec::new();
    walk(&root, &mut offenders);
    assert!(
        offenders.is_empty(),
        "ProviderFactory choice calls must stay in boot, the gate, probe, or safety_limits:\n{}",
        offenders.join("\n")
    );
}

fn walk(dir: &Path, offenders: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, offenders);
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let text = path.to_string_lossy();
        if text.contains("state/provider_setup.rs")
            || text.contains("state/postgres.rs")
            || text.contains("state/memory.rs")
            || text.contains("services/model_gate.rs")
            || text.contains("providers/probe.rs")
            || text.contains("providers/connection_factory.rs")
            || text.contains("providers/connection_store.rs")
            || text.contains("safety_limits.rs")
        {
            continue;
        }
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        for (index, line) in source.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with("///") || trimmed.starts_with("//!")
            {
                continue;
            }
            if line.contains("ProviderFactory::from_env")
                || line.contains("ProviderFactory::create_llm_provider")
                || line.contains("ProviderFactory::create_embedding_provider")
            {
                offenders.push(format!("{}:{}", path.display(), index + 1));
            }
        }
    }
}

#[test]
fn ingest_calls_the_gate() {
    let source = include_str!("../src/processor/workspace_resolver.rs");
    assert!(source.contains("model_gate::judge_ingest"));
    let reprocess = include_str!("../src/handlers/documents/recovery/reprocess.rs");
    assert!(!reprocess.contains("unwrap_or(\"ollama\")"));
    assert!(reprocess.contains("judge_running_pair"));
    let parse = include_str!("../src/handlers/parse/service.rs");
    assert!(!parse.contains("ProviderFactory::create_llm_provider"));
}

#[test]
fn keyword_does_not_swap_on_an_ineligible_role() {
    let source = include_str!("../src/services/query_execution.rs");
    assert!(source.contains("judge_running_pair"));
    assert!(source.contains("credential_missing"));
}

#[test]
fn rebuild_pair_is_judged_before_write() {
    let embeddings = include_str!("../src/handlers/workspaces/bulk_ops/rebuild_embeddings.rs");
    let graph = include_str!("../src/handlers/workspaces/bulk_ops/rebuild_knowledge_graph.rs");
    assert!(embeddings.contains("ensure_workspace_choice"));
    assert!(graph.contains("ensure_workspace_choice"));
}

#[test]
fn vision_summary_query_and_embedding_call_the_gate() {
    let pdf = include_str!("../src/processor/pdf_processing.rs");
    assert!(pdf.contains("refuse_ineligible_usage"));
    let summary = include_str!("../src/services/summary_role.rs");
    assert!(summary.contains("credential_missing"));
    assert!(!summary.contains("Err(_) => fallback"));
    let query = include_str!("../src/services/query_context.rs");
    assert!(query.contains("judge_workspace_usage"));
    let embedding = include_str!("../src/handlers/query/workspace_resolve.rs");
    assert!(embedding.contains("Usage::Embedding"));
    let parse = include_str!("../src/handlers/parse/handler.rs");
    assert!(parse.contains("ensure_deployment_choice"));
    let image = include_str!("../src/services/multimodal_admission.rs");
    assert!(image.contains("judge_workspace_usage"));
}

#[test]
fn empty_offer_refuses_defaults() {
    let source = include_str!("../src/services/model_policy_enforcement.rs");
    assert!(source.contains("no provider is offered"));
}
