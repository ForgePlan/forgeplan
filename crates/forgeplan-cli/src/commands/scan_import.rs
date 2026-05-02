use anyhow::Result;
use console::style;

use forgeplan_core::hints::{self, Hint};
use forgeplan_core::scan::detect::DetectionTier;
use forgeplan_core::scan::import::{ImportStatus, ScanImportOptions, scan_and_import_to_workspace};

use crate::commands::common;

/// `forgeplan scan-import [--path <dir>] [--dry-run]`
pub async fn run(path: Option<&str>, dry_run: bool) -> Result<()> {
    let (ws, _lock, store) = common::open_store_locked().await?;
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

    // PRD-058 FR-001: pass `ws` (the .forgeplan/ directory) so each
    // imported artifact gets a markdown projection written, making the
    // scan-import pipeline ADR-003-compliant. Without this, reindex
    // considers imported artifacts orphans and purges them.
    let result = scan_and_import_to_workspace(project_root, &ws, &store, &options).await?;

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
