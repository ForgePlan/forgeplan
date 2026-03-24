use std::env;

use forgeplan_core::db::store::LanceStore;
use forgeplan_core::drift;
use forgeplan_core::workspace;

pub async fn run() -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let store = LanceStore::open(&ws).await?;

    // workspace_root = parent of .forgeplan
    let workspace_root = ws.parent().unwrap_or(&ws);
    let reports = drift::check_drift(&store, workspace_root).await?;

    if reports.is_empty() {
        println!("  No active ADR/RFC with affected_files found.");
        println!("  Hint: Add '## Affected Files' section to ADR/RFC artifacts.");
        return Ok(());
    }

    let stale_count = reports.iter().filter(|r| r.is_stale).count();
    let total = reports.len();

    println!();
    for report in &reports {
        if report.is_stale {
            println!(
                "  \u{26a0} {} \"{}\" \u{2014} STALE",
                report.artifact_id, report.artifact_title
            );
            println!("    Created: {}", report.created_at);
            for file in &report.changed_files {
                println!("    Changed: {} ({})", file.path, file.last_modified);
            }
        } else {
            println!(
                "  \u{2713} {} \"{}\" \u{2014} up to date",
                report.artifact_id, report.artifact_title
            );
        }
        println!();
    }

    println!("  {} decisions checked, {} stale", total, stale_count);
    if stale_count > 0 {
        println!("  Action: Review stale decisions and update or supersede them.");
    }

    Ok(())
}
