use forgeplan_core::artifact::sanitize::sanitize_for_hint;
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::journal;

use crate::commands::common;

pub async fn run(kind: Option<&str>, risk: bool, json: bool) -> anyhow::Result<()> {
    let (_ws, source, _read_guard) = common::open_read_source().await?;
    // Filters, ordering and the empty-journal output are unchanged; only the
    // read source differs. A capture failure propagates — an unreadable
    // workspace must not read as "no decision artifacts found".
    let entries = match &source {
        common::ReadSource::Legacy(store) => {
            journal::build_journal(store, &_read_guard, kind, risk).await?
        }
        common::ReadSource::Captured(read) => {
            journal::build_journal_captured(read, kind, risk).await?
        }
    };

    if json {
        // Mirrors the `forgeplan_journal` MCP wire shape: {entries, total}
        // plus the `_next_action` marker. At-risk uses the same threshold
        // the MCP handler does (stale evidence or r_eff < 0.5) — the text
        // renderer's own indicator scale is display-only.
        let at_risk_count = entries
            .iter()
            .filter(|e| e.has_stale_evidence || e.r_eff < 0.5)
            .count();
        let total = entries.len();
        let dtos: Vec<serde_json::Value> = entries
            .iter()
            .map(|e| {
                serde_json::json!({
                    "id": e.id, "title": e.title, "kind": e.kind,
                    "created_at": e.created_at, "r_eff": e.r_eff,
                    "evidence_count": e.evidence_count,
                    "has_stale_evidence": e.has_stale_evidence,
                })
            })
            .collect();
        let next_action = if total == 0 {
            "No decision-kind artifacts yet (adr, note, problem, solution). Create one: \
             `forgeplan_new kind=adr title=\"...\"`."
                .to_string()
        } else if at_risk_count > 0 {
            let first_risky = entries
                .iter()
                .find(|e| e.has_stale_evidence || e.r_eff < 0.5)
                .map(|e| sanitize_for_hint(&e.id));
            match first_risky {
                Some(id) => format!(
                    "{at_risk_count} at-risk of {total} entries. Score the worst: \
                     `forgeplan_score {id}`."
                ),
                None => format!("{at_risk_count} at-risk decision(s). Run `forgeplan_health`."),
            }
        } else {
            format!("{total} decision(s) documented, all healthy.")
        };
        let payload = serde_json::json!({
            "entries": dtos,
            "total": total,
            "_next_action": next_action,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

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

        // SEC-H1 (CWE-117 / CWE-150): titles are attacker-controllable via
        // frontmatter; sanitize before TTY emission to neutralise
        // ANSI/bidi/control bytes.
        println!(
            "  {}  {} [{}] \"{}\"",
            date,
            entry.id,
            entry.kind,
            sanitize_for_hint(&entry.title)
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
