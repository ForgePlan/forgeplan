use forgeplan_core::drift;
use forgeplan_core::hints::{self, Hint};

use crate::commands::common;

pub async fn run(json: bool) -> anyhow::Result<()> {
    let (ws, store) = common::open_store().await?;

    let workspace_root = ws
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Workspace path has no parent directory"))?;
    let reports = drift::check_drift(&store, workspace_root).await?;

    let stale_count = reports.iter().filter(|r| r.is_stale).count();
    let total = reports.len();

    // Anchor next-action on the first stale ADR/RFC (real ID); fall back
    // to the backfill workflow when there's nothing to drift-check.
    let mut hint_list: Vec<Hint> = Vec::new();
    let first_stale = reports.iter().find(|r| r.is_stale);
    if let Some(r) = first_stale {
        hint_list.push(
            Hint::warning(format!("Refresh stale {}", r.artifact_id)).with_action(format!(
                "forgeplan new refresh \"Re-evaluate {}\"",
                r.artifact_id
            )),
        );
    } else if reports.is_empty() {
        hint_list.push(
            Hint::info("Backfill 'Affected Files' so drift can be checked")
                .with_action("forgeplan coverage --backfill".to_string()),
        );
    } else {
        hint_list.push(
            Hint::info("All decisions current — verify health")
                .with_action("forgeplan health".to_string()),
        );
    }

    if json {
        let data: Vec<_> = reports.iter().map(|r| {
            serde_json::json!({
                "id": r.artifact_id, "title": r.artifact_title, "is_stale": r.is_stale,
                "created_at": r.created_at,
                "changed_files": r.changed_files.iter().map(|f| serde_json::json!({"path": f.path, "last_modified": f.last_modified})).collect::<Vec<_>>(),
            })
        }).collect();
        let payload = serde_json::json!({
            "reports": data,
            "_next_action": hints::primary_action(&hint_list),
            "hints": hint_list,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    if reports.is_empty() {
        println!("  No active ADR/RFC with affected_files found.");
        println!("  Hint: Add '## Affected Files' section to ADR/RFC artifacts.");
        print!("{}", hints::render_next_action_line(&hint_list));
        return Ok(());
    }

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

    print!("{}", hints::render_next_action_line(&hint_list));

    Ok(())
}
