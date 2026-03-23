use std::env;

use console::style;
use forgeplan_core::db::store::LanceStore;
use forgeplan_core::lifecycle;
use forgeplan_core::workspace;

pub async fn run(id: &str) -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let store = LanceStore::open(&ws).await?;
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
