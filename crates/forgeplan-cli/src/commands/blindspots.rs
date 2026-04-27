use forgeplan_core::health;
use forgeplan_core::hints::{self, Hint};

use crate::commands::common;

pub async fn run() -> anyhow::Result<()> {
    let store = common::store().await?;
    let report = health::health_report(&store).await?;

    if report.blind_spots.is_empty() && report.orphans.is_empty() {
        println!("No blind spots found. All decision artifacts have linked evidence.");
        println!();
        println!("Done.");
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

    // Build actionable hint targeting the first blind-spot artifact (real ID),
    // falling back to the first orphan if no blind spots are present.
    let target_id = report
        .blind_spots
        .first()
        .map(|s| s.id.clone())
        .or_else(|| report.orphans.first().cloned());

    let mut hint_list: Vec<Hint> = Vec::new();
    if let Some(id) = target_id {
        hint_list.push(
            Hint::warning(format!("Add evidence linking to {}", id))
                .with_action(format!(
                    "forgeplan new evidence \"Proof for {}\" && forgeplan link EVID-XXX {} --relation informs",
                    id, id
                )),
        );
    }

    println!();
    println!("Fix: Create evidence with `forgeplan new evidence \"Proof for <ID>\"`");
    println!("     Then link: `forgeplan link EVID-001 <ID> --relation informs`");
    println!();
    print!("{}", hints::render_next_action_line(&hint_list));

    Ok(())
}
