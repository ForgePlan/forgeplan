//! `forgeplan restore <ID>` — recover a soft-deleted artifact (PRD-055).
//!
//! CLI parity for the `forgeplan_restore` MCP tool. Reads the most
//! recent non-consumed receipt for the given ID from
//! `.forgeplan/trash/`, and reverses the destructive op (delete /
//! supersede / deprecate). Refuses if the ID is now occupied by a
//! different artifact (manual resolution required).

use console::style;
use forgeplan_core::undo;

use crate::commands::common;

/// Run restore for a specific artifact ID.
pub async fn run(id: &str, json: bool) -> anyhow::Result<()> {
    let (ws, store) = common::open_store().await?;

    // Lazy TTL purge — best-effort, never fails the command. Mirrors
    // the MCP tool's defense against unbounded receipt accumulation.
    let ws_clone = ws.clone();
    tokio::spawn(async move {
        let _ = undo::purge_expired(&ws_clone, undo::DEFAULT_TTL_DAYS).await;
    });

    let receipt = match undo::find_latest_for(&ws, id).await? {
        Some(r) => r,
        None => {
            if json {
                let payload = serde_json::json!({
                    "ok": false,
                    "error": format!("No non-consumed receipt found for {id}"),
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                eprintln!(
                    "{} No non-consumed receipt found for `{}`.",
                    style("Error:").red().bold(),
                    id
                );
                eprintln!(
                    "  Hint: receipts older than {} days are purged. Inspect \
                     `.forgeplan/trash/` directly or run `forgeplan activity \
                     --tool forgeplan_delete,forgeplan_supersede,forgeplan_deprecate \
                     --since-hours 720` for recent destructive ops.",
                    undo::DEFAULT_TTL_DAYS
                );
            }
            std::process::exit(1);
        }
    };

    let report = undo::restore::apply_restore(&ws, &store, &receipt).await?;
    let op_str = match report.op {
        undo::DestructiveOp::Delete => "delete",
        undo::DestructiveOp::Supersede => "supersede",
        undo::DestructiveOp::Deprecate => "deprecate",
    };

    if json {
        let payload = serde_json::json!({
            "ok": true,
            "restored": report.artifact_id,
            "op_reversed": op_str,
            "relations_restored": report.relations_restored,
            "relations_skipped": report.relations_skipped,
            "projection_restored": report.projection_restored,
            "warnings": report.warnings,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    println!(
        "{} Restored `{}` (reversed {}).",
        style("OK").green().bold(),
        report.artifact_id,
        op_str
    );
    println!("  Relations restored: {}", report.relations_restored);
    if !report.relations_skipped.is_empty() {
        println!(
            "  Relations skipped (target missing): {}",
            report.relations_skipped.join(", ")
        );
    }
    if report.projection_restored {
        println!("  Markdown projection: restored on disk");
    }
    if !report.warnings.is_empty() {
        println!();
        println!("{}", style("Warnings:").yellow().bold());
        for w in &report.warnings {
            println!("  - {w}");
        }
    }

    Ok(())
}
