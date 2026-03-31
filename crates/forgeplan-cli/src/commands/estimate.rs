use anyhow::Result;

use forgeplan_core::estimate::{calculator, confidence, display, extractor, scorer};
use forgeplan_core::estimate::types::{EstimateConfig, Grade, ItemSource};

use crate::commands::common;

pub async fn run(
    id: &str,
    grade: Option<&str>,
    my_grade: bool,
    llm_score: bool,
    json: bool,
) -> Result<()> {
    let store = common::store().await?;

    // Fetch artifact
    let record = store
        .get_record(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact '{}' not found", id))?;

    // Extract work items from artifact body
    let work_items = extractor::extract_work_items(&record.body);

    if work_items.is_empty() && !json {
        println!("  No FR or Phase items found in {}.", id);
        println!("  Add FR table to PRD or Phase checklist to RFC.");
        return Ok(());
    }

    // Score complexity: LLM L1 (opt-in) or rule-based L0 (default)
    let scored_items = if llm_score {
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

    // Calculate confidence
    let fr_items: Vec<_> = work_items.iter().filter(|w| w.source == ItemSource::Fr).collect();
    let phase_items: Vec<_> = work_items.iter().filter(|w| w.source == ItemSource::Phase).collect();

    let (conf, conf_reasons) = confidence::score_confidence(
        !fr_items.is_empty(),
        fr_items.len(),
        !phase_items.is_empty(),
        phase_items.len(),
        false, // TODO: check linked Spec
        false, // TODO: check linked Evidence
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
    );

    // Determine highlight grade
    let highlight_grade = if my_grade {
        let domain = infer_domain(&record.kind);
        let resolved = config.resolve_grade(&domain);
        if !json {
            eprintln!(
                "  Using grade: {} (domain: {}, from config grade_profile)",
                resolved, domain
            );
        }
        Some(resolved)
    } else if let Some(g) = grade {
        Some(g.parse::<Grade>().map_err(|e| anyhow::anyhow!("{}", e))?)
    } else {
        None
    };

    // Output
    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        print!("{}", display::format_table(&result, highlight_grade));
    }

    Ok(())
}

/// Load EstimateConfig from .forgeplan/config.yaml, falling back to defaults.
fn load_estimate_config() -> EstimateConfig {
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

/// Infer work domain from artifact kind for grade profile lookup.
fn infer_domain(kind: &str) -> String {
    match kind {
        "prd" | "epic" | "spec" => "backend".to_string(),
        "rfc" | "adr" => "backend".to_string(),
        _ => "default".to_string(),
    }
}
