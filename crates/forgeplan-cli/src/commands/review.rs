use console::style;
use forgeplan_core::lifecycle;

use crate::commands::common;

pub async fn run(id: &str) -> anyhow::Result<()> {
    let store = common::store().await?;
    let result = lifecycle::review(&store, id).await?;

    // Styled output instead of plain Display
    if result.can_activate {
        println!("  {}", style("Review PASSED").green().bold());
        println!("  {}", style("Ready to activate").green());
    } else {
        println!("  {}", style("Review FAILED").red().bold());
        println!("  {}", style("Fix MUST issues first").red());
    }

    if !result.must_findings.is_empty() {
        println!();
        for finding in &result.must_findings {
            println!("  {} [{}] {}", style("x").red().bold(), style("MUST").red().bold(), finding);
        }
    }

    if !result.should_findings.is_empty() {
        println!();
        for finding in &result.should_findings {
            println!("  {} [{}] {}", style("!").yellow(), style("SHOULD").yellow(), finding);
        }
    }

    if !result.warnings.is_empty() {
        println!();
        for warning in &result.warnings {
            println!("  {} {}", style("!").yellow().bold(), style(warning).yellow());
        }
    }

    Ok(())
}
