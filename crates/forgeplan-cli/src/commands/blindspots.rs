use std::env;

use forgeplan_core::db::store::LanceStore;
use forgeplan_core::health;
use forgeplan_core::workspace;

pub async fn run() -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let store = LanceStore::open(&ws).await?;
    let report = health::health_report(&store).await?;

    if report.blind_spots.is_empty() && report.orphans.is_empty() {
        println!("No blind spots found. All decision artifacts have linked evidence.");
        return Ok(());
    }

    println!();
    println!("Forgeplan Blind Spots");
    println!("{}", "═".repeat(50));

    if !report.blind_spots.is_empty() {
        println!();
        println!("  Decisions without evidence ({}):", report.blind_spots.len());
        for spot in &report.blind_spots {
            println!("    {} \"{}\"", spot.id, spot.title);
            println!("      → {}", spot.issue);
        }
    }

    if !report.orphans.is_empty() {
        println!();
        println!("  Orphan artifacts ({}):", report.orphans.len());
        for id in &report.orphans {
            println!("    {} — no incoming or outgoing links", id);
        }
    }

    println!();
    println!("Fix: Create evidence with `forgeplan new evidence \"Proof for <ID>\"`");
    println!("     Then link: `forgeplan link EVID-001 <ID> --relation informs`");
    println!();

    Ok(())
}
