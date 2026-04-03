use std::env;

use forgeplan_core::coverage;
use forgeplan_core::db::store::LanceStore;
use forgeplan_core::workspace;

/// `forgeplan scan [--path <dir>]` — scan codebase for modules
pub async fn run_scan(path: Option<&str>) -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let project_root = match path {
        Some(p) => {
            let candidate = cwd.join(p);
            let canonical = candidate.canonicalize().map_err(|e| {
                anyhow::anyhow!("Scan path '{}' does not exist or is inaccessible: {}", p, e)
            })?;
            let canonical_root = cwd.canonicalize()?;
            if !canonical.starts_with(&canonical_root) {
                anyhow::bail!(
                    "Scan path '{}' is outside project root. Path traversal rejected.",
                    p
                );
            }
            canonical
        }
        None => cwd,
    };

    println!("  Scanning {}...", project_root.display());
    let modules = coverage::scan_modules(&project_root).await?;

    if modules.is_empty() {
        println!("  No source modules found.");
        return Ok(());
    }

    println!();
    println!("  {:40} {:>5} {:>7}", "Module", "Files", "Lines");
    println!("  {}", "-".repeat(55));
    for m in &modules {
        println!("  {:40} {:>5} {:>7}", m.path, m.file_count, m.line_count);
    }
    println!();
    let total_files: usize = modules.iter().map(|m| m.file_count).sum();
    let total_lines: usize = modules.iter().map(|m| m.line_count).sum();
    println!(
        "  {} modules, {} files, {} lines",
        modules.len(),
        total_files,
        total_lines
    );

    Ok(())
}

/// `forgeplan coverage` — show decision coverage per module
pub async fn run_coverage() -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let project_root = ws.parent().unwrap_or(&ws);
    let store = LanceStore::open(&ws).await?;

    println!("  Scanning codebase...");
    let mut modules = coverage::scan_modules(project_root).await?;
    let report = coverage::build_coverage(&mut modules, &store).await?;

    println!();
    println!(
        "  Decision Coverage: {:.0}% ({}/{} modules)",
        report.coverage_percent, report.covered_modules, report.total_modules
    );
    println!();

    // Show uncovered modules first (blind spots)
    let uncovered: Vec<_> = report
        .modules
        .iter()
        .filter(|m| m.decisions.is_empty())
        .collect();
    if !uncovered.is_empty() {
        println!("  \u{26a0} Uncovered modules (architectural blind spots):");
        for m in &uncovered {
            println!(
                "    {} ({} files, {} lines)",
                m.path, m.file_count, m.line_count
            );
        }
        println!();
    }

    // Show covered modules
    let covered: Vec<_> = report
        .modules
        .iter()
        .filter(|m| !m.decisions.is_empty())
        .collect();
    if !covered.is_empty() {
        println!("  \u{2713} Covered modules:");
        for m in &covered {
            println!("    {} \u{2190} {}", m.path, m.decisions.join(", "));
        }
    }

    Ok(())
}

/// `forgeplan coverage --backfill` — add "Affected Files" section to artifacts missing it
pub async fn run_backfill() -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let store = LanceStore::open(&ws).await?;
    let updated = coverage::backfill_affected_files(&store).await?;

    if updated.is_empty() {
        println!("  All active PRD/RFC/ADR already have 'Affected Files' section.");
    } else {
        println!("  Backfilled {} artifact(s):", updated.len());
        for id in &updated {
            println!("    + {id}");
        }
        println!();
        println!("  Edit each artifact to fill in actual file paths.");
    }

    Ok(())
}
