use super::*;

fn chat() -> ModelFacts {
    ModelFacts {
        chat: true,
        registered: true,
        ..ModelFacts::default()
    }
}

fn entry(provider: &str, model: &str, facts: ModelFacts) -> CatalogEntry {
    CatalogEntry {
        provider: provider.into(),
        model: model.into(),
        facts,
    }
}

fn row(provider: &str, model: &str, usage: Usage, enabled: bool) -> PolicyRow {
    PolicyRow {
        provider: provider.into(),
        model: model.into(),
        usage,
        enabled,
    }
}

fn names(models: &[&CatalogEntry]) -> Vec<String> {
    models.iter().map(|m| m.model.clone()).collect()
}

fn catalog() -> Vec<CatalogEntry> {
    vec![
        entry("openai", "gpt-5-nano", chat()),
        entry("openai", "gpt-5-mini", chat()),
        entry("ollama", "gemma3", chat()),
        entry(
            "openai",
            "text-embedding-3-small",
            ModelFacts {
                embedding: true,
                registered: true,
                ..ModelFacts::default()
            },
        ),
    ]
}

#[test]
fn unconstrained_without_rows_returns_every_capable_registered_model() {
    let cat = catalog();
    let got = eligible_models(Usage::Query, &cat, &[], &[], None);
    assert_eq!(names(&got), ["gpt-5-nano", "gpt-5-mini", "gemma3"]);
    assert_eq!(policy_mode(&[], Usage::Query), PolicyMode::Unconstrained);
}

#[test]
fn ceiling_keeps_only_enabled_pairs_for_that_usage() {
    let cat = catalog();
    let ceiling = [row("openai", "gpt-5-nano", Usage::Query, true)];
    let got = eligible_models(Usage::Query, &cat, &ceiling, &[], None);
    assert_eq!(names(&got), ["gpt-5-nano"]);
    assert_eq!(policy_mode(&ceiling, Usage::Query), PolicyMode::Constrained);
}

#[test]
fn disabled_ceiling_row_excludes_its_pair() {
    let cat = catalog();
    let ceiling = [
        row("openai", "gpt-5-nano", Usage::Query, true),
        row("ollama", "gemma3", Usage::Query, false),
    ];
    let got = eligible_models(Usage::Query, &cat, &ceiling, &[], None);
    assert_eq!(names(&got), ["gpt-5-nano"]);
}

#[test]
fn ceiling_for_one_usage_does_not_constrain_another() {
    let cat = catalog();
    let ceiling = [row("openai", "gpt-5-nano", Usage::Query, true)];
    let extract = eligible_models(Usage::Extract, &cat, &ceiling, &[], None);
    assert_eq!(names(&extract), ["gpt-5-nano", "gpt-5-mini", "gemma3"]);
    assert_eq!(
        policy_mode(&ceiling, Usage::Extract),
        PolicyMode::Unconstrained
    );
}

#[test]
fn discovery_only_model_is_not_eligible() {
    let mut cat = catalog();
    cat.push(entry(
        "openai",
        "gpt-discovered-only",
        ModelFacts {
            chat: true,
            registered: false,
            ..ModelFacts::default()
        },
    ));
    let got = eligible_models(Usage::Query, &cat, &[], &[], None);
    assert!(!names(&got).contains(&"gpt-discovered-only".to_string()));
}

#[test]
fn failed_probe_removes_model_and_missing_probe_keeps_it() {
    let mut cat = catalog();
    cat[0].facts.probe = Some(false);
    cat[1].facts.probe = None;
    let got = eligible_models(Usage::Query, &cat, &[], &[], None);
    assert_eq!(names(&got), ["gpt-5-mini", "gemma3"]);
}

#[test]
fn capability_predicate_gates_each_usage() {
    let cat = catalog();
    let emb = eligible_models(Usage::Embedding, &cat, &[], &[], None);
    assert_eq!(names(&emb), ["text-embedding-3-small"]);
    let query = eligible_models(Usage::Query, &cat, &[], &[], None);
    assert!(!names(&query).contains(&"text-embedding-3-small".to_string()));
}

#[test]
fn vlm_requires_vision_and_chat() {
    let mut cat = catalog();
    cat.push(entry(
        "openai",
        "gpt-vision",
        ModelFacts {
            chat: true,
            vision: true,
            registered: true,
            ..ModelFacts::default()
        },
    ));
    let got = eligible_models(Usage::Vlm, &cat, &[], &[], None);
    assert_eq!(names(&got), ["gpt-vision"]);
}

#[test]
fn rerank_is_never_eligible_in_this_wave() {
    let cat = catalog();
    assert!(eligible_models(Usage::Rerank, &cat, &[], &[], None).is_empty());
}

#[test]
fn disabled_narrowing_row_removes_pair_and_others_stay() {
    let cat = catalog();
    let narrowing = [row("openai", "gpt-5-mini", Usage::Query, false)];
    let got = eligible_models(Usage::Query, &cat, &[], &narrowing, None);
    assert_eq!(names(&got), ["gpt-5-nano", "gemma3"]);
}

#[test]
fn enabled_narrowing_row_does_not_add_a_pair_the_ceiling_omitted() {
    let cat = catalog();
    let ceiling = [row("openai", "gpt-5-nano", Usage::Query, true)];
    let narrowing = [row("ollama", "gemma3", Usage::Query, true)];
    let got = eligible_models(Usage::Query, &cat, &ceiling, &narrowing, None);
    assert_eq!(names(&got), ["gpt-5-nano"]);
}

#[test]
fn widening_is_reported_when_workspace_enables_an_omitted_pair() {
    let ceiling = [row("openai", "gpt-5-nano", Usage::Query, true)];
    let narrowing = [
        row("openai", "gpt-5-nano", Usage::Query, true),
        row("ollama", "gemma3", Usage::Query, true),
    ];
    let widened = widening_rows(&ceiling, &narrowing);
    assert_eq!(widened.len(), 1);
    assert_eq!(widened[0].model, "gemma3");
}

#[test]
fn no_widening_when_ceiling_is_unconstrained_for_usage() {
    let narrowing = [row("ollama", "gemma3", Usage::Query, true)];
    assert!(widening_rows(&[], &narrowing).is_empty());
}

#[test]
fn disabled_narrowing_rows_never_widen() {
    let ceiling = [row("openai", "gpt-5-nano", Usage::Query, true)];
    let narrowing = [row("ollama", "gemma3", Usage::Query, false)];
    assert!(widening_rows(&ceiling, &narrowing).is_empty());
}

#[test]
fn usage_wire_names_round_trip() {
    for usage in [
        Usage::Extract,
        Usage::Query,
        Usage::Summary,
        Usage::Vlm,
        Usage::Keyword,
        Usage::Embedding,
        Usage::Decision,
        Usage::Rerank,
    ] {
        assert_eq!(Usage::from_wire(usage.as_str()), Some(usage));
    }
    assert_eq!(Usage::from_wire("chat"), None);
}

#[test]
fn ceiling_that_disables_every_model_for_a_usage_is_reported_empty() {
    let cat = catalog();
    let ceiling = [row("openai", "gpt-5-nano", Usage::Query, false)];
    let empty = usages_left_empty(&ceiling, &cat, &ceiling, &[], None);
    assert_eq!(empty, vec![Usage::Query]);
}

#[test]
fn ceiling_naming_only_a_missing_model_is_reported_empty() {
    let cat = catalog();
    let ceiling = [row("openai", "gpt-not-in-catalog", Usage::Query, true)];
    assert_eq!(
        usages_left_empty(&ceiling, &cat, &ceiling, &[], None),
        vec![Usage::Query]
    );
}

#[test]
fn usable_ceiling_is_not_reported_empty() {
    let cat = catalog();
    let ceiling = [row("ollama", "gemma3", Usage::Query, true)];
    assert!(usages_left_empty(&ceiling, &cat, &ceiling, &[], None).is_empty());
}

#[test]
fn narrowing_that_removes_the_only_model_is_reported_empty() {
    let cat = catalog();
    let ceiling = [row("ollama", "gemma3", Usage::Query, true)];
    let narrowing = [row("ollama", "gemma3", Usage::Query, false)];
    assert_eq!(
        usages_left_empty(&narrowing, &cat, &ceiling, &narrowing, None),
        vec![Usage::Query]
    );
}

#[test]
fn only_written_usages_are_checked() {
    let cat = catalog();
    // The tenant ceiling for extract is empty, but the write names only query.
    let written = [row("ollama", "gemma3", Usage::Query, true)];
    assert!(usages_left_empty(&written, &cat, &written, &[], None).is_empty());
}

#[test]
fn unoffered_static_card_absent_from_eligible() {
    let cat = catalog();
    let offered = std::collections::BTreeSet::from(["openai".to_string()]);
    let got = eligible_models(Usage::Query, &cat, &[], &[], Some(&offered));
    assert_eq!(names(&got), ["gpt-5-nano", "gpt-5-mini"]);
}

#[test]
fn unconstrained_eligible_omits_unoffered_card() {
    let cat = catalog();
    let offered = std::collections::BTreeSet::from(["ollama".to_string()]);
    assert_eq!(policy_mode(&[], Usage::Query), PolicyMode::Unconstrained);
    let got = eligible_models(Usage::Query, &cat, &[], &[], Some(&offered));
    assert_eq!(names(&got), ["gemma3"]);
}

#[test]
fn deployment_eligible_ignores_workspace_narrowing() {
    let cat = catalog();
    let narrowing = [row("openai", "gpt-5-nano", Usage::Query, false)];
    let workspace = eligible_models(Usage::Query, &cat, &[], &narrowing, None);
    let deployment = eligible_models(Usage::Query, &cat, &[], &[], None);
    assert!(!names(&workspace).contains(&"gpt-5-nano".to_string()));
    assert!(names(&deployment).contains(&"gpt-5-nano".to_string()));
}

#[test]
fn probe_failed_omitted_from_eligible() {
    let mut cat = catalog();
    cat[2].facts.probe = Some(false);
    let got = eligible_models(Usage::Query, &cat, &[], &[], None);
    assert!(!names(&got).contains(&"gemma3".to_string()));
}
