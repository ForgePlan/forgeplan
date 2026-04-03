use forgeplan_core::health;

use crate::commands::common;

pub async fn run() -> anyhow::Result<()> {
    let store = common::store().await?;
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
        println!(
            "  Decisions without evidence ({}):",
            report.blind_spots.len()
        );
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
