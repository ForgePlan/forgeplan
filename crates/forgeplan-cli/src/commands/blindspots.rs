use forgeplan_core::artifact::sanitize::sanitize_for_hint;
use forgeplan_core::health;
use forgeplan_core::hints::{self, Hint};

use crate::commands::common;

pub async fn run(json: bool) -> anyhow::Result<()> {
    let (_ws, source, _read_guard) = common::open_read_source().await?;
    // Same blind-spot / orphan computation on both layouts — the captured
    // variant is the same shared computation over one capture, so a migrated
    // workspace answers instead of refusing on the retired store.
    let report = match &source {
        common::ReadSource::Legacy(store) => health::health_report(store, &_read_guard).await?,
        common::ReadSource::Captured(read) => health::health_report_captured(read).await?,
    };

    if json {
        // Mirrors the `forgeplan_blindspots` MCP wire shape.
        let blind_count = report.blind_spots.len();
        let orphan_count = report.orphans.len();
        let next_action = if blind_count == 0 && orphan_count == 0 {
            "No blind spots or orphans. Workflow healthy.".to_string()
        } else if let Some(b) = report.blind_spots.first() {
            let first = sanitize_for_hint(&b.id);
            format!(
                "{blind_count} blind spot(s). Score `{first}` first: `forgeplan_score {first}`."
            )
        } else if let Some(o) = report.orphans.first() {
            let first = sanitize_for_hint(o);
            format!("{orphan_count} orphan(s). Inspect `{first}`: `forgeplan_get {first}`.")
        } else {
            "No actionable items.".to_string()
        };
        let payload = serde_json::json!({
            "blind_spots": report.blind_spots.iter().map(|b| serde_json::json!({
                "id": b.id, "title": b.title, "issue": b.issue,
            })).collect::<Vec<_>>(),
            "orphans": report.orphans,
            "total_blind_spots": blind_count,
            "total_orphans": orphan_count,
            "_next_action": next_action,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

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
            // SEC-H1 (CWE-117 / CWE-150): titles and issue text are
            // attacker-controllable via frontmatter; sanitize before TTY
            // emission to neutralise ANSI/bidi/control bytes.
            println!("    {} \"{}\"", spot.id, sanitize_for_hint(&spot.title));
            println!("      → {}", sanitize_for_hint(&spot.issue));
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
