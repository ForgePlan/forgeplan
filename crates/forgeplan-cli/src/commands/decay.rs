use forgeplan_core::artifact::sanitize::sanitize_for_hint;
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::scoring::decay;

use crate::commands::common;

pub async fn run(json: bool) -> anyhow::Result<()> {
    let (_ws, source, _read_guard) = common::open_read_source().await?;
    // Both-sided evidence accounting (incoming AND outgoing) is preserved by
    // the captured variant, so the R_eff numbers a migrated workspace prints
    // are the same ones the legacy path computed.
    let entries = match &source {
        common::ReadSource::Legacy(store) => decay::decay_report(store, &_read_guard).await?,
        common::ReadSource::Captured(read) => decay::decay_report_captured(read).await?,
    };

    if json {
        // Mirrors the `forgeplan_decay` MCP wire shape (DecayResponse).
        let total = entries.len();
        let dtos: Vec<serde_json::Value> = entries
            .iter()
            .map(|e| {
                serde_json::json!({
                    "artifact_id": e.artifact_id,
                    "artifact_title": e.artifact_title,
                    "current_r_eff": e.current_r_eff,
                    "fresh_r_eff": e.fresh_r_eff,
                    "r_eff_drop": e.fresh_r_eff - e.current_r_eff,
                    "expired_evidence": e.expired_evidence.iter().map(|ev| serde_json::json!({
                        "id": ev.id,
                        "valid_until": ev.valid_until,
                        "days_expired": ev.days_expired,
                        "score": ev.individual_score,
                    })).collect::<Vec<_>>(),
                })
            })
            .collect();
        let next_action = if total == 0 {
            "No decayed evidence — all evidence within valid_until window.".to_string()
        } else {
            let worst = dtos
                .iter()
                .max_by(|a, b| {
                    a["r_eff_drop"]
                        .as_f64()
                        .unwrap_or(0.0)
                        .partial_cmp(&b["r_eff_drop"].as_f64().unwrap_or(0.0))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|d| {
                    (
                        sanitize_for_hint(d["artifact_id"].as_str().unwrap_or("")),
                        d["r_eff_drop"].as_f64().unwrap_or(0.0),
                    )
                });
            match worst {
                Some((id, drop)) => format!(
                    "{total} artifact(s) with decayed evidence. Worst: `{id}` (R_eff drop \
                     {drop:.2}). Inspect: `forgeplan_get {id}`."
                ),
                None => format!("{total} artifact(s) decayed."),
            }
        };
        let payload = serde_json::json!({
            "entries": dtos,
            "total_affected": total,
            "_next_action": next_action,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

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
