use console::style;
use forgeplan_core::gaps::{self, GapSeverity};
use forgeplan_core::hints::{self, Hint};

use crate::commands::common;
use crate::ui;

pub async fn run() -> anyhow::Result<()> {
    let store = common::store().await?;
    let gaps = gaps::find_gaps(&store).await?;

    if gaps.is_empty() {
        println!("No pipeline gaps found. All artifacts comply with depth rules.");
        // PRD-071 contract: terminal — no actionable next step beyond keeping
        // the workspace healthy.
        println!("\nDone.");
        return Ok(());
    }

    ui::header("Forgeplan Gaps", "pipeline compliance");

    let must_count = gaps
        .iter()
        .filter(|g| g.severity == GapSeverity::Must)
        .count();
    let should_count = gaps
        .iter()
        .filter(|g| g.severity == GapSeverity::Should)
        .count();
    let could_count = gaps
        .iter()
        .filter(|g| g.severity == GapSeverity::Could)
        .count();

    println!();
    println!(
        "  {} {} MUST  {} SHOULD  {} COULD",
        style("Gaps:").bold(),
        ui::styled_count(must_count, must_count > 0),
        ui::styled_count(should_count, should_count > 0),
        ui::styled_count(could_count, false),
    );

    let mut current_severity = None;
    for gap in &gaps {
        if current_severity != Some(gap.severity) {
            current_severity = Some(gap.severity);
            println!();
            println!("  {}:", style(gap.severity.label()).bold());
        }

        let severity_marker = match gap.severity {
            GapSeverity::Must => style("!").red().bold().to_string(),
            GapSeverity::Should => style("~").yellow().to_string(),
            GapSeverity::Could => style("?").dim().to_string(),
        };

        println!(
            "    {} {} \"{}\"",
            severity_marker,
            style(&gap.artifact_id).yellow(),
            gap.artifact_title,
        );
        println!("      {}", style(&gap.message).dim());
    }

    println!();
    if must_count > 0 {
        println!(
            "  {} Fix MUST gaps first: create missing artifacts and link them.",
            style("->").red(),
        );
    }
    println!();

    // PRD-071 contract: the highest-severity gap drives the Next: hint. MUST
    // first, then SHOULD, then COULD. Reuse the gap's artifact_id so the
    // suggested action lands on a real id.
    let mut hints_vec: Vec<Hint> = Vec::new();
    let pick = gaps
        .iter()
        .find(|g| g.severity == GapSeverity::Must)
        .or_else(|| gaps.iter().find(|g| g.severity == GapSeverity::Should))
        .or_else(|| gaps.first());
    if let Some(gap) = pick {
        hints_vec.push(
            Hint::warning(format!("Fix gap on {}: {}", gap.artifact_id, gap.message))
                .with_action(format!("forgeplan get {}", gap.artifact_id)),
        );
    }
    print!("{}", hints::render_next_action_line(&hints_vec));

    Ok(())
}
