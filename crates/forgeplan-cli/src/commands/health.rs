use std::env;

use forgeplan_core::db::store::LanceStore;
use forgeplan_core::health;
use forgeplan_core::workspace;

pub async fn run(compact: bool) -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let config = workspace::load_config(&ws)?;
    let store = LanceStore::open(&ws).await?;
    let report = health::health_report(&store).await?;

    if compact {
        // Compact mode for hooks/scripts
        println!("Project: {} | Artifacts: {} | Blind spots: {} | Stale: {} | At risk: {}",
            config.project_name,
            report.total,
            report.blind_spots.len(),
            report.stale_count,
            report.at_risk.len(),
        );
        if let Some(action) = report.next_actions.first() {
            println!("Next: {}", action);
        }
        return Ok(());
    }

    // Full dashboard
    println!();
    println!("Forgeplan Health — {}", config.project_name);
    println!("{}", "═".repeat(50));

    println!();
    println!("  Artifacts:  {} total", report.total);

    if !report.by_kind.is_empty() {
        println!();
        println!("  By kind:");
        for (kind, count) in &report.by_kind {
            println!("    {:<16} {}", kind, count);
        }
    }

    if !report.by_status.is_empty() {
        println!();
        println!("  By status:");
        for (status, count) in &report.by_status {
            let warning = if status == "draft" && *count == report.total && report.total > 0 {
                " ⚠ ALL DRAFT"
            } else {
                ""
            };
            println!("    {:<16} {}{}", status, count, warning);
        }
    }

    // At Risk
    if !report.at_risk.is_empty() {
        println!();
        println!("  ⚠ At Risk ({}):", report.at_risk.len());
        for item in &report.at_risk {
            println!("    {} \"{}\" — {}", item.id, item.title, item.reason);
        }
    }

    // Blind Spots
    if !report.blind_spots.is_empty() {
        println!();
        println!("  ● Blind Spots ({}):", report.blind_spots.len());
        for spot in &report.blind_spots {
            println!("    {} \"{}\" — {}", spot.id, spot.title, spot.issue);
        }
    }

    // Stale
    if report.stale_count > 0 {
        println!();
        println!("  ⏰ Stale: {} evidence expired", report.stale_count);
    }

    // Orphans
    if !report.orphans.is_empty() {
        println!();
        println!("  ○ Orphans ({}):", report.orphans.len());
        for id in &report.orphans {
            println!("    {} — no links", id);
        }
    }

    // Next Actions
    if !report.next_actions.is_empty() {
        println!();
        println!("  → Next actions:");
        for (i, action) in report.next_actions.iter().enumerate() {
            println!("    {}. {}", i + 1, action);
        }
    }

    println!();
    Ok(())
}
