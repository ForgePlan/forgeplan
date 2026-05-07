use anyhow::Result;
use console::style;

use forgeplan_core::estimate::types::ItemSource;
use forgeplan_core::estimate::{calculator, confidence, extractor, scorer};
use forgeplan_core::hints::{self, Hint};

use crate::commands::common;
use crate::commands::estimate::load_estimate_config;

/// Compare estimated hours with actual hours to calibrate estimation accuracy.
pub async fn run(artifact_id: &str, actual_hours: f64, grade: Option<&str>) -> Result<()> {
    // PRD-071 contract: error paths emit `Fix:` markers so agents have a
    // deterministic next action.
    if !actual_hours.is_finite() || actual_hours <= 0.0 {
        anyhow::bail!(
            "Actual hours must be a positive finite number (got: {})\n\
             Fix: forgeplan calibrate-estimate {} --actual-hours 8",
            actual_hours,
            artifact_id
        );
    }

    let store = common::store().await?;

    // PROB-060 / SPEC-005 Phase 2.6 (CD-6) — accept slug or display id.
    let artifact_id = store.resolve_id(artifact_id).await?.ok_or_else(|| {
        anyhow::anyhow!("Artifact '{artifact_id}' not found\nFix: forgeplan list")
    })?;
    let artifact_id = artifact_id.as_str();

    // Get the artifact
    let record = store.get_record(artifact_id).await?.ok_or_else(|| {
        anyhow::anyhow!("Artifact '{}' not found\nFix: forgeplan list", artifact_id)
    })?;

    // Re-run estimate pipeline (same as estimate command)
    let work_items = extractor::extract_work_items(&record.body);
    if work_items.is_empty() {
        anyhow::bail!(
            "No estimable items in {}. Cannot calibrate.\n\
             Fix: forgeplan estimate {}",
            artifact_id,
            artifact_id
        );
    }

    let scored_items = scorer::score_items(&work_items);

    let fr_items: Vec<_> = work_items
        .iter()
        .filter(|w| w.source == ItemSource::Fr)
        .collect();
    let phase_items: Vec<_> = work_items
        .iter()
        .filter(|w| w.source == ItemSource::Phase)
        .collect();

    let outgoing = store.get_relations(&record.id).await.unwrap_or_default();
    let incoming = store
        .get_incoming_relations(&record.id)
        .await
        .unwrap_or_default();

    let has_spec = outgoing
        .iter()
        .any(|(t, _)| t.to_uppercase().starts_with("SPEC-"))
        || incoming
            .iter()
            .any(|(s, _)| s.to_uppercase().starts_with("SPEC-"));
    let has_evidence = outgoing
        .iter()
        .any(|(t, _)| t.to_uppercase().starts_with("EVID-"))
        || incoming
            .iter()
            .any(|(s, _)| s.to_uppercase().starts_with("EVID-"));

    let (conf, conf_reasons) = confidence::score_confidence(
        !fr_items.is_empty(),
        fr_items.len(),
        !phase_items.is_empty(),
        phase_items.len(),
        has_spec,
        has_evidence,
    );

    let config = load_estimate_config();
    let hints = extractor::collect_hints(&record.body, work_items.len(), &record.kind);

    let result = calculator::calculate(
        &record.id,
        &record.title,
        &scored_items,
        &config,
        conf,
        conf_reasons,
        hints,
    );

    // Find estimated hours for requested grade
    let estimated = if let Some(grade_str) = grade {
        let grade_lower = grade_str.to_lowercase();
        result
            .totals
            .iter()
            .find(|(g, _)| format!("{:?}", g).to_lowercase().contains(&grade_lower))
            .map(|(_, v)| *v)
            .unwrap_or(result.total_score)
    } else {
        result.total_score
    };

    if estimated <= 0.0 {
        anyhow::bail!(
            "Estimate returned 0h. Fill FR/Phase sections first.\n\
             Fix: forgeplan estimate {}",
            artifact_id
        );
    }

    // Calculate ratio
    let ratio = actual_hours / estimated;
    let accuracy_pct = if ratio > 1.0 {
        (1.0 / ratio) * 100.0
    } else {
        ratio * 100.0
    };

    // Display
    println!();
    println!("{} — {}", style(artifact_id).bold(), record.title);
    println!("{}", "─".repeat(50));
    println!(
        "  Estimated:  {:.1}h (confidence {:.0}%)",
        estimated,
        result.confidence * 100.0
    );
    println!("  Actual:     {:.1}h", actual_hours);
    println!();

    let ratio_styled = if ratio > 1.3 {
        style(format!("{:.2}x", ratio)).red().bold()
    } else if ratio < 0.7 {
        style(format!("{:.2}x", ratio)).yellow().bold()
    } else {
        style(format!("{:.2}x", ratio)).green().bold()
    };
    println!("  Ratio:      {} (actual / estimated)", ratio_styled);
    println!("  Accuracy:   {:.0}%", accuracy_pct);
    println!();

    if ratio > 1.5 {
        println!(
            "  {} Estimates are {:.1}x optimistic for {} tasks. Add {:.1}x buffer.",
            style("!").yellow(),
            ratio,
            record.kind,
            ratio
        );
    } else if ratio > 1.1 {
        println!(
            "  {} Slightly optimistic ({:.0}% accuracy). Acceptable range.",
            style("i").dim(),
            accuracy_pct
        );
    } else if ratio < 0.5 {
        println!(
            "  {} Finished {:.1}x faster. Estimates may be too conservative.",
            style("*").green(),
            1.0 / ratio
        );
    } else if ratio < 0.7 {
        println!(
            "  {} Finished faster than expected ({:.0}% of estimate). Estimates may be conservative.",
            style("i").dim(),
            ratio * 100.0
        );
    } else {
        println!(
            "  {} Good calibration — within 30% of actual.",
            style("*").green()
        );
    }
    println!();

    let mut hint_list: Vec<Hint> = Vec::new();
    if ratio > 1.5 {
        hint_list.push(
            Hint::warning(format!(
                "Estimates {:.1}x optimistic — re-estimate with buffer",
                ratio
            ))
            .with_action(format!("forgeplan estimate {} --my-grade", artifact_id)),
        );
    } else if ratio < 0.5 {
        hint_list.push(
            Hint::info("Estimates conservative — re-grade")
                .with_action(format!("forgeplan estimate {} --my-grade", artifact_id)),
        );
    } else {
        hint_list.push(
            Hint::info("Calibration recorded — verify next artifact")
                .with_action(format!("forgeplan score {}", artifact_id)),
        );
    }
    print!("{}", hints::render_next_action_line(&hint_list));

    Ok(())
}
