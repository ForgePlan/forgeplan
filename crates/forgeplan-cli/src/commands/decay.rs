use forgeplan_core::scoring::decay;

use crate::commands::common;

pub async fn run() -> anyhow::Result<()> {
    let store = common::store().await?;
    let entries = decay::decay_report(&store).await?;

    if entries.is_empty() {
        println!("No evidence decay detected. All linked evidence is current.");
        return Ok(());
    }

    println!(
        "Evidence Decay Report — {} artifact(s) affected:\n",
        entries.len()
    );

    for entry in &entries {
        let drop = entry.fresh_r_eff - entry.current_r_eff;
        println!(
            "  {} \"{}\"",
            entry.artifact_id, entry.artifact_title
        );
        println!(
            "    R_eff: {:.2} → {:.2} (drop: {:.2})",
            entry.fresh_r_eff, entry.current_r_eff, drop
        );

        for ev in &entry.expired_evidence {
            println!(
                "    ⚠ {} expired {} ({} days ago, score={:.1})",
                ev.id, ev.valid_until, ev.days_expired, ev.individual_score
            );
        }
        println!();
    }

    println!("Hint: Create a RefreshReport to re-evaluate stale evidence:");
    println!("  forgeplan new refresh \"Re-evaluate <artifact>\"");

    Ok(())
}
