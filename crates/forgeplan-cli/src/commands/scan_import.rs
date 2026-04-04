use anyhow::Result;
use console::style;

use forgeplan_core::scan::detect::DetectionTier;
use forgeplan_core::scan::import::{ImportStatus, ScanImportOptions, scan_and_import};

use crate::commands::common;

/// `forgeplan scan-import [--path <dir>] [--dry-run]`
pub async fn run(path: Option<&str>, dry_run: bool) -> Result<()> {
    let (ws, store) = common::open_store().await?;
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

    let result = scan_and_import(project_root, &store, &options).await?;

    // Print results
    if result.entries.is_empty() {
        println!("  No markdown documents found.");
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
            .map(|k| format!("{}", k.template_key().to_uppercase()))
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
    }

    println!();
    println!(
        "  Summary: {} imported, {} skipped, {} unknown, {} failed",
        style(result.imported).green().bold(),
        style(result.skipped).yellow(),
        style(result.unknown).dim(),
        if result.failed > 0 {
            style(result.failed).red().bold().to_string()
        } else {
            style(result.failed).dim().to_string()
        }
    );

    if dry_run && result.imported > 0 {
        println!("\n  Run without {} to import.", style("--dry-run").cyan());
    }

    Ok(())
}
