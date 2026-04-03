use forgeplan_core::drift;

use crate::commands::common;

pub async fn run(json: bool) -> anyhow::Result<()> {
    let (ws, store) = common::open_store().await?;

    let workspace_root = ws
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Workspace path has no parent directory"))?;
    let reports = drift::check_drift(&store, workspace_root).await?;

    if json {
        let data: Vec<_> = reports.iter().map(|r| {
            serde_json::json!({
                "id": r.artifact_id, "title": r.artifact_title, "is_stale": r.is_stale,
                "created_at": r.created_at,
                "changed_files": r.changed_files.iter().map(|f| serde_json::json!({"path": f.path, "last_modified": f.last_modified})).collect::<Vec<_>>(),
            })
        }).collect();
        println!("{}", serde_json::to_string_pretty(&data)?);
        return Ok(());
    }

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
