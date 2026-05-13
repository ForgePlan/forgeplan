use forgeplan_core::artifact::sanitize::sanitize_for_hint;
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::scoring::decay;

use crate::commands::common;

pub async fn run() -> anyhow::Result<()> {
    let store = common::store().await?;
    let entries = decay::decay_report(&store).await?;

    if entries.is_empty() {
        println!("No evidence decay detected. All linked evidence is current.");
        let hint_list =
            vec![Hint::info("Run health check next").with_action("forgeplan health".to_string())];
        print!("{}", hints::render_next_action_line(&hint_list));
        return Ok(());
    }

    println!(
        "Evidence Decay Report — {} artifact(s) affected:\n",
        entries.len()
    );

    for entry in &entries {
        let drop = entry.fresh_r_eff - entry.current_r_eff;
        // SEC-H1 (CWE-117 / CWE-150): artifact titles are attacker-
        // controllable via frontmatter; sanitize before TTY emission to
        // neutralise ANSI/bidi/control bytes.
        println!(
            "  {} \"{}\"",
            entry.artifact_id,
            sanitize_for_hint(&entry.artifact_title)
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

    // Anchor the next-action on the first decayed artifact (real ID).
    let first = &entries[0];
    let hint_list = vec![
        Hint::warning(format!("Refresh {}", first.artifact_id)).with_action(format!(
            "forgeplan new refresh \"Re-evaluate {}\"",
            first.artifact_id
        )),
    ];
    print!("{}", hints::render_next_action_line(&hint_list));

    Ok(())
}
