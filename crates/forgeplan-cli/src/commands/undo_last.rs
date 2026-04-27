//! `forgeplan undo-last` — reverse the most recent destructive op (PRD-055).
//!
//! CLI parity for the `forgeplan_undo_last` MCP tool. Scans
//! `.forgeplan/trash/` newest-first within the configured window and
//! restores the first non-consumed receipt found. If the window is
//! empty, reports "no candidate" rather than guessing.

use chrono::{DateTime, Duration, Utc};
use console::style;
use forgeplan_core::undo;

use crate::commands::common;

/// Run undo-last across the workspace. Mirrors `UndoLastParams`:
/// - `within_hours` clamped 1..=720 (default 24).
pub async fn run(within_hours: u32, json: bool) -> anyhow::Result<()> {
    let (ws, store) = common::open_store().await?;

    let within = within_hours.clamp(1, 720);
    let receipts = undo::list_receipts(&ws).await?;
    let threshold = Utc::now() - Duration::hours(within as i64);

    let receipt = receipts.into_iter().find(|r| {
        if r.consumed {
            return false;
        }
        match DateTime::parse_from_rfc3339(&r.ts) {
            Ok(ts) => ts.with_timezone(&Utc) >= threshold,
            Err(_) => false,
        }
    });

    let receipt = match receipt {
        Some(r) => r,
        None => {
            if json {
                let payload = serde_json::json!({
                    "ok": false,
                    "error": format!(
                        "No non-consumed destructive op in the last {within} hour(s)"
                    ),
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                eprintln!(
                    "{} No non-consumed destructive op in the last {} hour(s).",
                    style("Error:").red().bold(),
                    within
                );
                eprintln!(
                    "  Hint: expand the window with `--within-hours 720`, or inspect \
                     `forgeplan activity --tool forgeplan_delete,forgeplan_supersede,\
                     forgeplan_deprecate --since-hours 720`."
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
            "receipt_id": receipt.receipt_id,
            "relations_restored": report.relations_restored,
            "relations_skipped": report.relations_skipped,
            "projection_restored": report.projection_restored,
            "warnings": report.warnings,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    println!(
        "{} Reversed most recent {} of `{}` (receipt {}).",
        style("OK").green().bold(),
        op_str,
        report.artifact_id,
        receipt.receipt_id
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
    println!();
    println!(
        "  Hint: call `forgeplan undo-last` again to reverse the next non-consumed receipt, \
         or restore a specific ID with `forgeplan restore <id>`."
    );

    Ok(())
}
