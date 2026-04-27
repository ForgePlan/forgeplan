use forgeplan_core::hints::{self, Hint};
use forgeplan_core::journal;

use crate::commands::common;

pub async fn run(kind: Option<&str>, risk: bool) -> anyhow::Result<()> {
    let store = common::store().await?;
    let entries = journal::build_journal(&store, kind, risk).await?;

    if entries.is_empty() {
        println!("No decision artifacts found.");
        if kind.is_some() {
            println!("Try without --kind filter.");
        }
        // PRD-071 contract: empty journal — no decision artifacts yet, suggest
        // creating one.
        let hints_vec = vec![
            Hint::suggestion("Shape your first decision artifact")
                .with_action("forgeplan new prd \"<title>\"".to_string()),
        ];
        print!("{}", hints::render_next_action_line(&hints_vec));
        return Ok(());
    }

    println!();
    println!(
        "Decision Journal{}",
        if risk { " (at-risk only)" } else { "" }
    );
    println!("{}", "═".repeat(50));
    println!();

    for entry in &entries {
        let date = entry
            .created_at
            .split('T')
            .next()
            .unwrap_or(&entry.created_at);
        let is_terminal = entry.status == "deprecated" || entry.status == "superseded";
        let risk_indicator = if entry.evidence_count == 0 && !is_terminal {
            " ⚠ NO EVIDENCE"
        } else if entry.has_stale_evidence {
            " ⏰ STALE"
        } else if entry.r_eff < 0.3 {
            " ⚠ AT RISK"
        } else {
            ""
        };

        println!(
            "  {}  {} [{}] \"{}\"",
            date, entry.id, entry.kind, entry.title
        );
        if entry.evidence_count > 0 {
            println!(
                "         R_eff: {:.2} | {} evidence{}",
                entry.r_eff, entry.evidence_count, risk_indicator
            );
        } else {
            println!("         {}", risk_indicator.trim());
        }
    }

    // Summary
    let no_evidence = entries
        .iter()
        .filter(|e| e.evidence_count == 0)
        .filter(|e| e.status != "deprecated" && e.status != "superseded")
        .count();
    if no_evidence > 0 {
        println!();
        println!("  ⚠ {} decision(s) without any evidence", no_evidence);
    }

    // PRD-071 contract: pick the most recent decision and suggest inspecting
    // it. journal::build_journal returns reverse-chronological so entries[0]
    // is the most recent.
    let mut hints_vec: Vec<Hint> = Vec::new();
    if let Some(first) = entries.first() {
        if first.evidence_count == 0 && first.status != "deprecated" && first.status != "superseded"
        {
            hints_vec.push(
                Hint::warning(format!("{} has no evidence — score it", first.id))
                    .with_action(format!("forgeplan score {}", first.id)),
            );
        } else {
            hints_vec.push(
                Hint::info(format!("Inspect most recent decision {}", first.id))
                    .with_action(format!("forgeplan get {}", first.id)),
            );
        }
    }

    println!();
    print!("{}", hints::render_next_action_line(&hints_vec));
    Ok(())
}
