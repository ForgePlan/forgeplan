use anyhow::Result;

use forgeplan_core::estimate::types::{EstimateConfig, Grade, ItemSource};
use forgeplan_core::estimate::{
    calculator, confidence, display, domain, extractor, overrides, scorer,
};
use forgeplan_core::hints::{self, Hint};

use crate::commands::common;

pub async fn run(
    id: &str,
    grade: Option<&str>,
    my_grade: bool,
    llm_score: bool,
    complexity_overrides: Option<&str>,
    json: bool,
) -> Result<()> {
    let store = common::store().await?;

    // PROB-060 / SPEC-005 Phase 2.6 (CD-6) — accept slug or display id.
    let id = store
        .resolve_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact '{id}' not found\nFix: forgeplan list"))?;
    let id = id.as_str();

    // Fetch artifact
    let record = store.get_record(id).await?.ok_or_else(|| {
        anyhow::anyhow!(
            "Artifact '{}' not found
Fix: forgeplan list",
            id
        )
    })?;

    // PROB-060 / SPEC-005 / ADR-012 (W1.B, CD-5) — slug pre-merge / display
    // id post-merge so every hint emitted from this command stays canonical
    // for commit `Refs:` propagation.
    let ref_form =
        forgeplan_core::artifact::frontmatter::refs_form_from_body(&record.body, &record.id);

    // Extract work items from artifact body
    let work_items = extractor::extract_work_items(&record.body);

    // Collect hints about artifact quality
    let hints = extractor::collect_hints(&record.body, work_items.len(), &record.kind);

    if work_items.is_empty() {
        // Empty estimate: actionable next-step is to fill FR/Phase items.
        let next_hints = vec![
            Hint::warning("No estimable items — fill FR/Phase sections")
                .with_action(format!("forgeplan get {}", ref_form)),
        ];

        if json {
            // In JSON mode, return empty result with hints
            let result = forgeplan_core::estimate::types::EstimateResult {
                artifact_id: record.id.clone(),
                artifact_title: record.title.clone(),
                items: vec![],
                totals: std::collections::HashMap::new(),
                total_score: 0.0,
                confidence: 0.0,
                confidence_reasons: vec![],
                hints,
            };
            // Wrap with `_next_action` envelope for contract compliance.
            let inner = serde_json::to_value(&result)?;
            let payload = serde_json::json!({
                "result": inner,
                "_next_action": hints::primary_action(&next_hints),
                "hints": next_hints,
            });
            println!("{}", serde_json::to_string_pretty(&payload)?);
        } else {
            println!("  No estimable items found in {}.", id);
            for hint in &hints {
                let prefix = match hint.level {
                    forgeplan_core::estimate::types::HintLevel::Warning => "!",
                    forgeplan_core::estimate::types::HintLevel::Info => "i",
                    forgeplan_core::estimate::types::HintLevel::Suggestion => "*",
                };
                println!("  {} {}", prefix, hint.message);
                if let Some(ref action) = hint.action {
                    println!("    -> {}", action);
                }
            }
            print!("{}", hints::render_next_action_line(&next_hints));
        }
        return Ok(());
    }

    // Score complexity: LLM L1 (opt-in) or rule-based L0 (default)
    let mut scored_items = if llm_score {
        let llm_config = common::require_llm_config()?;
        if !json {
            eprintln!(
                "  Using LLM scorer ({}/{}). Fallback to rules if LLM fails.",
                llm_config.provider, llm_config.model
            );
        }
        scorer::score_items_with_llm(&work_items, &llm_config).await
    } else {
        scorer::score_items(&work_items)
    };

    // Apply manual complexity overrides (highest priority per ADR-004)
    if let Some(overrides_str) = complexity_overrides {
        let map = overrides::parse_complexity_overrides(overrides_str)?;
        overrides::apply_overrides(&mut scored_items, &map);
        if !json {
            eprintln!("  Applied {} manual complexity override(s)", map.len());
        }
    }

    // Calculate confidence
    let fr_items: Vec<_> = work_items
        .iter()
        .filter(|w| w.source == ItemSource::Fr)
        .collect();
    let phase_items: Vec<_> = work_items
        .iter()
        .filter(|w| w.source == ItemSource::Phase)
        .collect();

    // Check linked Spec and Evidence for confidence boost
    // Outgoing: this artifact → target (e.g., PRD → SPEC)
    let outgoing = store.get_relations(&record.id).await.unwrap_or_default();
    // Incoming: source → this artifact (e.g., EVID → PRD)
    let incoming = store
        .get_incoming_relations(&record.id)
        .await
        .unwrap_or_default();

    let has_spec = outgoing
        .iter()
        .any(|(target, _)| target.to_uppercase().starts_with("SPEC-"))
        || incoming
            .iter()
            .any(|(source, _)| source.to_uppercase().starts_with("SPEC-"));
    let has_evidence = outgoing
        .iter()
        .any(|(target, _)| target.to_uppercase().starts_with("EVID-"))
        || incoming
            .iter()
            .any(|(source, _)| source.to_uppercase().starts_with("EVID-"));

    let (conf, conf_reasons) = confidence::score_confidence(
        !fr_items.is_empty(),
        fr_items.len(),
        !phase_items.is_empty(),
        phase_items.len(),
        has_spec,
        has_evidence,
    );

    // Build config from .forgeplan/config.yaml or defaults
    let config = load_estimate_config();

    // Calculate hours
    let result = calculator::calculate(
        &record.id,
        &record.title,
        &scored_items,
        &config,
        conf,
        conf_reasons,
        hints,
    );

    // Determine highlight grade
    let highlight_grade = if my_grade {
        let d = if llm_score {
            // LLM-assisted domain inference when --llm-score is active
            let llm_config = common::config().ok().and_then(|c| c.llm);
            infer_domain_with_llm(&record.title, &record.body, llm_config.as_ref()).await
        } else {
            domain::infer_domain(&record.title, &record.body)
        };
        let resolved = config.resolve_grade(&d);
        if !json {
            eprintln!(
                "  Using grade: {} (domain: {}, from config grade_profile)",
                resolved, d
            );
        }
        Some(resolved)
    } else if let Some(g) = grade {
        Some(g.parse::<Grade>().map_err(|e| anyhow::anyhow!("{}", e))?)
    } else {
        None
    };

    // Suggest calibrating the estimate after delivery — this is the
    // canonical follow-up workflow (estimate → work → calibrate-estimate).
    // PROB-060 (W1.B, CD-5) — emit ref_form so the calibrate command stays
    // canonical (slug pre-merge / display id post-merge).
    let next_hints = vec![Hint::info("Calibrate after delivery").with_action(format!(
        "forgeplan calibrate-estimate {} --actual-hours <N>",
        ref_form
    ))];

    // Output
    if json {
        let inner = serde_json::to_value(&result)?;
        let payload = serde_json::json!({
            "result": inner,
            "_next_action": hints::primary_action(&next_hints),
            "hints": next_hints,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        print!("{}", display::format_table(&result, highlight_grade));
        print!("{}", hints::render_next_action_line(&next_hints));
    }

    Ok(())
}

/// Load EstimateConfig from .forgeplan/config.yaml, falling back to defaults.
pub fn load_estimate_config() -> EstimateConfig {
    match common::config() {
        Ok(cfg) => {
            if let Some(ref yaml) = cfg.estimate {
                EstimateConfig::from_yaml(yaml)
            } else {
                EstimateConfig::default()
            }
        }
        Err(_) => EstimateConfig::default(),
    }
}

/// LLM-assisted domain inference. Falls back to keyword-based if LLM unavailable.
async fn infer_domain_with_llm(
    title: &str,
    body: &str,
    llm_config: Option<&forgeplan_core::config::types::LlmConfig>,
) -> String {
    // Try LLM classification (frontmatter checked first inside domain::infer_domain fallback)
    if let Some(config) = llm_config {
        let snippet: String = body.chars().take(300).collect();
        let prompt = format!(
            "Classify this artifact into exactly ONE domain. Reply with ONLY the domain name, nothing else.\n\
             Domains: backend, frontend, devops, ai_ml, default\n\n\
             Title: {}\nBody: {}",
            title, snippet
        );
        let client = forgeplan_core::llm::LlmClient::new(config.clone());
        if let Ok(response) = client
            .generate(
                &prompt,
                Some("You are a domain classifier. Reply with exactly one word."),
            )
            .await
        {
            let d = response.trim().to_lowercase().replace(' ', "_");
            let valid = ["backend", "frontend", "devops", "ai_ml"];
            if valid.contains(&d.as_str()) {
                return d;
            }
        }
    }

    // Fallback to keyword-based (includes frontmatter check)
    domain::infer_domain(title, body)
}
