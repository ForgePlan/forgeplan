use std::env;

use forgeplan_core::db::store::LanceStore;
use forgeplan_core::journal;
use forgeplan_core::workspace;

pub async fn run(kind: Option<&str>, risk: bool) -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let store = LanceStore::open(&ws).await?;
    let entries = journal::build_journal(&store, kind, risk).await?;

    if entries.is_empty() {
        println!("No decision artifacts found.");
        if kind.is_some() {
            println!("Try without --kind filter.");
        }
        return Ok(());
    }

    println!();
    println!("Decision Journal{}", if risk { " (at-risk only)" } else { "" });
    println!("{}", "═".repeat(50));
    println!();

    for entry in &entries {
        let date = entry.created_at.split('T').next().unwrap_or(&entry.created_at);
        let risk_indicator = if entry.evidence_count == 0 {
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
    let no_evidence = entries.iter().filter(|e| e.evidence_count == 0).count();
    if no_evidence > 0 {
        println!();
        println!(
            "  ⚠ {} decision(s) without any evidence",
            no_evidence
        );
    }

    println!();
    Ok(())
}
