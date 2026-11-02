//! `forgeplan undo-last` — reverse the most recent destructive op (PRD-055).
//!
//! CLI parity for the `forgeplan_undo_last` MCP tool. Scans
//! `.forgeplan/trash/` newest-first within the configured window and
//! restores the first non-consumed receipt found. If the window is
//! empty, reports "no candidate" rather than guessing.

use chrono::{DateTime, Duration, Utc};
use console::style;
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::undo;

use crate::commands::common;

/// Run undo-last across the workspace. Mirrors `UndoLastParams`:
/// - `within_hours` clamped 1..=720 (default 24).
pub async fn run(within_hours: u32, json: bool) -> anyhow::Result<()> {
    let (ws, lock, store) = common::open_store_locked_tolerating_constellation().await?;

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
            // PRD-071: error path — primary fix is to widen the window to
            // the maximum (720h). Determinstic and copy-pasteable.
            let fix_hints: Vec<Hint> = vec![
                Hint::warning(format!(
                    "No non-consumed destructive op in the last {} hour(s)",
                    within
                ))
                .with_action("forgeplan undo-last --within-hours 720".to_string()),
            ];
            if json {
                let payload = serde_json::json!({
                    "ok": false,
                    "error": format!(
                        "No non-consumed destructive op in the last {within} hour(s)"
                    ),
                    "_next_action": hints::primary_action(&fix_hints),
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                eprintln!(
                    "{} No non-consumed destructive op in the last {} hour(s).",
                    style("Error:").red().bold(),
                    within
                );
                if let Some(fix) = hints::primary_action(&fix_hints) {
                    eprintln!("Fix: {}", fix);
                }
            }
            std::process::exit(1);
        }
    };
    undo::restore::preflight_restore_with_target(&ws, &lock, &receipt, None).await?;
    drop(store);
    let store = common::prepare_store_locked_checked(&ws, &lock, &[]).await?;
    undo::restore::preflight_restore_with_target(&ws, &lock, &receipt, None).await?;

    // Store-free sibling on a migrated workspace — same reasoning as
    // `restore`: receipts exist on both layouts, so the reverse path must too.
    common::ensure_ordinary_mutation_allowed(&ws)?;
    let report = match &store {
        Some(store) => undo::restore::apply_restore(&ws, &lock, store, &receipt).await
            .map_err(|e| common::describe_possible_legacy_partial_commit(true, e.into()))?,
        None => undo::restore::apply_restore_file_first(&ws, &lock, &receipt).await?,
    };
    let op_str = match report.op {
        undo::DestructiveOp::Delete => "delete",
        undo::DestructiveOp::Supersede => "supersede",
        undo::DestructiveOp::Deprecate => "deprecate",
    };

    // PRD-071: post-undo, verify the restored artifact's state.
    let next_hints: Vec<Hint> = vec![
        Hint::info("Reversed — verify state")
            .with_action(format!("forgeplan get {}", report.artifact_id)),
    ];

    let migrated = store.is_none();
    drop(store);
    drop(lock);
    let published = if migrated {
        common::note_constellation_owns_index();
        common::reconcile_after_file_mutation(&ws).await
    } else {
        common::drain_legacy_semantic_after_mutation(&ws).await?;
        true
    };

    if json {
        // Match restore: reconcile normally, expose its outcome in JSON,
        // and leave all status diagnostics on stderr.
        let (index_pending, index_pending_reason) = (
            !published,
            (!published).then(|| "derived index not yet updated".to_string()),
        );
        let payload = serde_json::json!({
            "ok": true,
            "restored": report.artifact_id,
            "op_reversed": op_str,
            "receipt_id": receipt.receipt_id,
            "relations_restored": report.relations_restored,
            "relations_skipped": report.relations_skipped,
            "projection_restored": report.projection_restored,
            "warnings": report.warnings,
            "index_pending": index_pending,
            "index_pending_reason": index_pending_reason,
            "_next_action": hints::primary_action(&next_hints),
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


    print!("{}", hints::render_next_action_line(&next_hints));

    Ok(())
}
