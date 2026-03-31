use std::collections::HashMap;

use anyhow::Result;

use forgeplan_core::estimate::{calculator, confidence, display, extractor, scorer};
use forgeplan_core::estimate::types::{Complexity, EstimateConfig, Grade, ItemSource, ScoredItem};

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
    if let Some(overrides) = complexity_overrides {
        let map = parse_complexity_overrides(overrides)?;
        apply_overrides(&mut scored_items, &map);
        if !json {
            eprintln!("  Applied {} manual complexity override(s)", map.len());
        }
    }

    // Calculate confidence
    let fr_items: Vec<_> = work_items.iter().filter(|w| w.source == ItemSource::Fr).collect();
    let phase_items: Vec<_> = work_items.iter().filter(|w| w.source == ItemSource::Phase).collect();

    // Check linked Spec and Evidence for confidence boost
    let relations = store.get_relations(&record.id).await.unwrap_or_default();
    let incoming = store.get_incoming_relations(&record.id).await.unwrap_or_default();
    let all_rels: Vec<_> = relations.iter().chain(incoming.iter()).collect();

    let has_spec = all_rels.iter().any(|(target, _)| target.to_uppercase().starts_with("SPEC-"));
    let has_evidence = all_rels.iter().any(|(target, _)| target.to_uppercase().starts_with("EVID-"));

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

/// Parse "FR-001=5,FR-002=3" into HashMap<String, Complexity>.
fn parse_complexity_overrides(input: &str) -> Result<HashMap<String, Complexity>> {
    let mut map = HashMap::new();
    for pair in input.split(',') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        let parts: Vec<&str> = pair.splitn(2, '=').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid complexity override '{}'. Format: FR-001=5", pair);
        }
        let id = parts[0].trim().to_string();
        let value: u32 = parts[1].trim().parse()
            .map_err(|_| anyhow::anyhow!("Invalid number '{}' in complexity override", parts[1].trim()))?;
        let complexity = Complexity::from_value(value)
            .ok_or_else(|| anyhow::anyhow!(
                "Invalid Fibonacci value {}. Valid: 1, 2, 3, 5, 8, 13", value
            ))?;
        map.insert(id, complexity);
    }
    Ok(map)
}

/// Apply manual overrides to scored items — override has highest priority.
fn apply_overrides(items: &mut [ScoredItem], overrides: &HashMap<String, Complexity>) {
    for item in items.iter_mut() {
        if let Some(complexity) = overrides.get(&item.id) {
            item.complexity = *complexity;
        }
    }
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
