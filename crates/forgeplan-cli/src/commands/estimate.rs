use anyhow::Result;

use forgeplan_core::estimate::{calculator, confidence, display, extractor, scorer};
use forgeplan_core::estimate::types::{EstimateConfig, Grade};

use crate::commands::common;

pub async fn run(
    id: &str,
    grade: Option<&str>,
    my_grade: bool,
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

    // Score complexity (rule-based L0)
    let scored_items = scorer::score_items(&work_items);

    // Calculate confidence
    let fr_items: Vec<_> = work_items.iter().filter(|w| w.id.starts_with("FR-")).collect();
    let phase_items: Vec<_> = work_items.iter().filter(|w| w.id.starts_with("P")).collect();

    let (conf, conf_reasons) = confidence::score_confidence(
        !fr_items.is_empty(),
        fr_items.len(),
        !phase_items.is_empty(),
        phase_items.len(),
        false, // TODO: check linked Spec
        false, // TODO: check linked Evidence
    );

    // Build config (defaults for now, TODO: read from config.yaml)
    let config = EstimateConfig::default();

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
        // TODO: read from config grade profile + artifact domain
        Some(Grade::Senior)
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
