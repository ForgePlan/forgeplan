use anyhow::Result;
use console::style;

use forgeplan_core::hints::{self, Hint};
use forgeplan_core::scan::detect::DetectionTier;
use forgeplan_core::scan::import::{ImportStatus, ScanImportOptions, scan_and_import_to_workspace};

use crate::commands::common;

/// `forgeplan scan-import [--path <dir>] [--dry-run]`
pub async fn run(path: Option<&str>, dry_run: bool) -> Result<()> {
    // A preview acquires NOTHING. Taking the workspace lock would create
    // `.forgeplan/.lock`, and opening the store would run its schema
    // migrations — so `--dry-run` on a legacy layout would perform exactly
    // the mutations it promises not to perform, and a read-only workspace
    // would fail on a lock it never needed.
    let (ws, lock, store) = if dry_run {
        (common::find_workspace_path()?, None, None)
    } else {
        // Lock + OPTIONAL legacy store, decided before either is opened: on a
        // constellation-managed workspace the store is `None` and the Markdown
        // write is the whole mutation. `open_store_locked` used to refuse here.
        let (ws, lock, store) = common::open_store_locked_tolerating_constellation().await?;
        (ws, Some(lock), store)
    };
    let migrated = store.is_none() && !dry_run;
    let project_root = ws
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine project root"))?;

    let options = ScanImportOptions {
        dry_run,
        custom_path: path.map(|s| s.to_string()),
    };

    if dry_run {
        println!(
            "  {} Dry-run mode — no changes will be made\n",
            style("⊘").dim()
        );
    }

    // PRD-058 FR-001 + S1-R2: the canonical Markdown is written FIRST for
    // every imported document, and the legacy projection is taken afterwards
    // only where one still exists. No source file is ever moved or deleted.
    let result = scan_and_import_to_workspace(
        project_root,
        &ws,
        lock.as_ref(),
        store.as_ref(),
        &options,
    )
    .await?;
    drop(lock);

    // ONE reconcile for the whole successful batch, after the lock is
    // released — the reconcile re-acquires it per storage commit, so holding
    // it here would turn the pass into a lock timeout. Only a migrated
    // workspace needs it: on a legacy layout the batch just took its rows.
    if !dry_run && migrated && result.imported > 0 {
        if result.failed > 0 {
            // A partial batch is not a complete one: say so instead of
            // printing a clean "reconciled" line over a half-finished import.
            eprintln!(
                "  note: {} document(s) failed — the reconcile below publishes only the \
                 {} that landed; re-run after fixing them.",
                result.failed, result.imported
            );
        }
        common::reconcile_after_file_mutation(&ws).await;
    }

    // Print results
    if result.entries.is_empty() {
        println!("  No markdown documents found.");
        // PRD-071: empty scan — direct user to create artifacts.
        let next_hints: Vec<Hint> = vec![
            Hint::info("No documents to import")
                .with_action("forgeplan new prd \"<title>\"".to_string()),
        ];
        print!("{}", hints::render_next_action_line(&next_hints));
        return Ok(());
    }

    println!(
        "  Found {} document(s):\n",
        style(result.total_found).bold()
    );

    for entry in &result.entries {
        let status_icon = match &entry.status {
            ImportStatus::Imported => style("+").green().to_string(),
            ImportStatus::Skipped => style("~").yellow().to_string(),
            ImportStatus::Unknown => style("?").dim().to_string(),
            ImportStatus::Failed(_) => style("✗").red().to_string(),
        };

        let kind_str = entry
            .detected_kind
            .as_ref()
            .map(|k| k.template_key().to_uppercase().to_string())
            .unwrap_or_else(|| "???".to_string());

        let tier_str = entry
            .detection_tier
            .as_ref()
            .map(|t| match t {
                DetectionTier::Frontmatter => "fm",
                DetectionTier::Filename => "fn",
                DetectionTier::Content => "ct",
            })
            .unwrap_or("-");

        let id_str = entry.artifact_id.as_deref().unwrap_or("-");

        let status_note = match &entry.status {
            ImportStatus::Skipped => " (exists)".to_string(),
            ImportStatus::Failed(msg) => format!(" ({})", msg),
            _ => String::new(),
        };

        println!(
            "  {} {:5} [{:2}] {:10} {}{}",
            status_icon,
            kind_str,
            tier_str,
            id_str,
            entry.relative_path,
            style(status_note).dim()
        );

        // R2 audit rust-pro HIGH: surface per-entry warnings (unknown
        // status mapping, projection write failure). PRD-058 R-2
        // fail-loud: the core emits; CLI must display.
        for w in &entry.warnings {
            println!("    {} {}", style("⚠").yellow(), style(w).yellow().dim());
        }
    }

    // Aggregate warning count for the summary line.
    let warnings_total: usize = result.entries.iter().map(|e| e.warnings.len()).sum();

    println!();
    println!(
        "  Summary: {} imported, {} skipped, {} unknown, {} failed{}",
        style(result.imported).green().bold(),
        style(result.skipped).yellow(),
        style(result.unknown).dim(),
        if result.failed > 0 {
            style(result.failed).red().bold().to_string()
        } else {
            style(result.failed).dim().to_string()
        },
        if warnings_total > 0 {
            format!(", {} warning(s)", style(warnings_total).yellow().bold())
        } else {
            String::new()
        }
    );

    // File-level id-collision warning (PRD-008 dup class): two+ files → one id.
    // The resolver then returns desynced status/body (status from one file,
    // body from another), so surface this loudly for cleanup.
    if !result.duplicate_ids.is_empty() {
        println!();
        println!(
            "  {} {} duplicate-id collision(s) — multiple files resolve to one artifact id:",
            style("⚠").red().bold(),
            style(result.duplicate_ids.len()).red().bold()
        );
        for grp in &result.duplicate_ids {
            println!("    {}", style(&grp.id).red().bold());
            for p in &grp.paths {
                println!("      {} {}", style("·").dim(), p);
            }
        }
        println!(
            "    {} keep ONE file; delete the stale duplicate .md so the resolver \
             stops mixing status/body across files.",
            style("Fix:").yellow().bold()
        );
    }

    if dry_run && result.imported > 0 {
        println!("\n  Run without {} to import.", style("--dry-run").cyan());
    }

    // PRD-071 contract: emit primary next-action.
    // - dry-run with imports → re-run without --dry-run
    // - imports happened → run health to surface integrity issues
    // - only skipped/failed → reindex to refresh DB state
    let next_hints: Vec<Hint> = if dry_run && result.imported > 0 {
        let cmd = match path {
            Some(p) => format!("forgeplan scan-import --path {}", p),
            None => "forgeplan scan-import".to_string(),
        };
        vec![
            Hint::info(format!("{} document(s) ready to import", result.imported)).with_action(cmd),
        ]
    } else if result.imported > 0 {
        vec![
            Hint::info("Import complete — verify integrity")
                .with_action("forgeplan health".to_string()),
        ]
    } else {
        vec![
            Hint::info("Nothing imported — refresh index")
                .with_action("forgeplan reindex".to_string()),
        ]
    };
    print!("{}", hints::render_next_action_line(&next_hints));

    Ok(())
}
