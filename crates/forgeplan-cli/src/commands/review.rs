use console::style;
use forgeplan_core::lifecycle;

use crate::commands::common;

pub async fn run(id: &str) -> anyhow::Result<()> {
    let store = common::store().await?;
    let result = lifecycle::review(&store, id).await?;

    // Styled output — distinguish MUST errors from gate warnings
    if result.can_activate {
        println!("  {}", style("Review PASSED").green().bold());
        println!("  {}", style("Ready to activate").green());
    } else if !result.must_findings.is_empty() {
        println!("  {}", style("Review FAILED").red().bold());
        println!("  {}", style("Fix MUST validation errors first").red());
    } else {
        // Gates failed but no MUST errors — methodology gates (evidence, body length)
        println!("  {}", style("Review BLOCKED").yellow().bold());
        println!(
            "  {}",
            style("Methodology gates not met (see warnings below)").yellow()
        );
    }

    if !result.must_findings.is_empty() {
        println!();
        for finding in &result.must_findings {
            println!(
                "  {} [{}] {}",
                style("x").red().bold(),
                style("MUST").red().bold(),
                finding
            );
        }
    }

    if !result.should_findings.is_empty() {
        println!();
        for finding in &result.should_findings {
            println!(
                "  {} [{}] {}",
                style("!").yellow(),
                style("SHOULD").yellow(),
                finding
            );
        }
    }

    if !result.warnings.is_empty() {
        println!();
        for warning in &result.warnings {
            println!(
                "  {} {}",
                style("!").yellow().bold(),
                style(warning).yellow()
            );
        }
    }

    // Contextual hints
    let has_evidence = !result
        .warnings
        .iter()
        .any(|w| w.contains("No evidence linked"));
    let is_stub = result.warnings.iter().any(|w| w.contains("Body too short"));
    let has_must_errors = !result.must_findings.is_empty();
    let kind = store
        .get_record(id)
        .await
        .ok()
        .flatten()
        .and_then(|r| {
            r.kind
                .parse::<forgeplan_core::artifact::types::ArtifactKind>()
                .ok()
        })
        .unwrap_or(forgeplan_core::artifact::types::ArtifactKind::Note);
    let review_hints =
        forgeplan_core::hints::review_hints(has_evidence, is_stub, has_must_errors, &kind);
    if !review_hints.is_empty() {
        print!("{}", forgeplan_core::hints::format_hints(&review_hints));
    }

    Ok(())
}
