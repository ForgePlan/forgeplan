//! `forgeplan restore <ID>` — recover a soft-deleted artifact (PRD-055).
//!
//! CLI parity for the `forgeplan_restore` MCP tool. Reads the most
//! recent non-consumed receipt for the given ID from
//! `.forgeplan/trash/`, and reverses the destructive op (delete /
//! supersede / deprecate). Refuses if the ID is now occupied by a
//! different artifact (manual resolution required).

use console::style;
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::undo;

use crate::commands::common;

/// Run restore for a specific artifact ID.
pub async fn run(id: &str, json: bool) -> anyhow::Result<()> {
    let (ws, lock, store) = common::open_store_locked_tolerating_constellation().await?;

    // Lazy TTL purge — best-effort, never fails the command. Mirrors
    // the MCP tool's defense against unbounded receipt accumulation.
    let ws_clone = ws.clone();
    tokio::spawn(async move {
        let _ = undo::purge_expired(&ws_clone, undo::DEFAULT_TTL_DAYS).await;
    });

    // PROB-060 Phase 2.5: resolve receipt by display id OR slug. The
    // artifact is no longer in the main store (soft-deleted), so we
    // cannot use `store.resolve_id` here — receipts carry their own
    // identity (snapshot.id is canonical display id; snapshot.slug is
    // optional canonical slug captured at delete time).
    let receipt = match undo::find_latest_for(&ws, id).await? {
        Some(r) => Some(r),
        None => undo::find_latest_for_slug(&ws, id).await?,
    };
    let receipt = match receipt {
        Some(r) => r,
        None => {
            // PRD-071 contract: error path — direct user to widest activity
            // window so they can inspect what destructive ops exist.
            let fix_hints: Vec<Hint> = vec![
                Hint::warning(format!("No non-consumed receipt for {id}")).with_action(
                    "forgeplan activity --tool forgeplan_delete,\
                         forgeplan_supersede,forgeplan_deprecate --since-hours 720"
                        .to_string(),
                ),
            ];

            if json {
                let payload = serde_json::json!({
                    "ok": false,
                    "error": format!("No non-consumed receipt found for {id}"),
                    "_next_action": hints::primary_action(&fix_hints),
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                eprintln!(
                    "{} No non-consumed receipt found for `{}`.",
                    style("Error:").red().bold(),
                    id
                );
                if let Some(fix) = hints::primary_action(&fix_hints) {
                    eprintln!("Fix: {}", fix);
                }
            }
            std::process::exit(1);
        }
    };

    // The restore itself is the store-free sibling on a migrated workspace:
    // the artifact comes back as canonical markdown, and the derived row is
    // published by the reconcile pass below. Receipts are written by `delete`
    // on BOTH layouts, so refusing here would make the recovery path `delete`
    // prints a lie.
    let report = match &store {
        Some(store) => undo::restore::apply_restore(&ws, store, &receipt).await?,
        None => undo::restore::apply_restore_file_first(&ws, &receipt).await?,
    };
    let op_str = match report.op {
        undo::DestructiveOp::Delete => "delete",
        undo::DestructiveOp::Supersede => "supersede",
        undo::DestructiveOp::Deprecate => "deprecate",
    };

    // PRD-071: post-restore the artifact is in its previous state — show
    // it so the agent can verify and decide what to do next.
    let next_hints: Vec<Hint> = vec![
        Hint::info("Restored — verify state")
            .with_action(format!("forgeplan get {}", report.artifact_id)),
    ];

    if json {
        // Reconciliation diagnostics use stderr; stdout remains one JSON value
        // carrying whether the derived index now agrees with the restore.
        let (index_pending, index_pending_reason) = if store.is_none() {
            common::note_constellation_owns_index();
            // Lock released before reconcile — it re-acquires the same lock
            // per storage commit, and a held guard would time the capture out.
            drop(lock);
            let published = common::reconcile_after_file_mutation(&ws).await;
            (
                !published,
                (!published).then(|| "derived index not yet updated".to_string()),
            )
        } else {
            (false, None)
        };
        let payload = serde_json::json!({
            "ok": true,
            "restored": report.artifact_id,
            "op_reversed": op_str,
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

    if store.is_none() {
        common::note_constellation_owns_index();
        drop(lock);
        common::reconcile_after_file_mutation(&ws).await;
    }

    print!("{}", hints::render_next_action_line(&next_hints));

    Ok(())
}
