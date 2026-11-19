//! `forgeplan contradictions` — issue #571 CLI surface for the cross-artifact
//! contradiction detector. Mirrors the `forgeplan_contradictions` MCP tool:
//! same `contradictions_v1_heuristic` computation, identical JSON with `--json`.

use forgeplan_core::artifact::sanitize::sanitize_for_hint;

use crate::commands::common;

pub async fn run(json: bool) -> anyhow::Result<()> {
    let (_ws, source, _read_guard) = common::open_read_source().await?;

    let legacy_records;
    let records: &[forgeplan_core::db::store::ArtifactRecord] = match &source {
        common::ReadSource::Legacy(store) => {
            legacy_records = store.list_records(&_read_guard, None).await?;
            legacy_records.as_slice()
        }
        common::ReadSource::Captured(read) => read.records(),
    };

    let report = forgeplan_core::brownfield::contradictions_v1_heuristic(records);

    if json {
        // Mirrors the `forgeplan_contradictions` MCP wire shape:
        // ContradictionsReport serializes straight through, plus
        // `_next_action`.
        let next_action = if report.contradictions.is_empty() {
            "Done. — no contradictions detected.".to_string()
        } else {
            format!(
                "Next: review {} contradiction(s) — `forgeplan_get <left>` then `forgeplan_get <right>`.",
                report.contradictions.len()
            )
        };
        let mut payload = serde_json::to_value(&report)?;
        payload
            .as_object_mut()
            .expect("ContradictionsReport serializes to an object")
            .insert(
                "_next_action".to_string(),
                serde_json::Value::String(next_action),
            );
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    if report.contradictions.is_empty() {
        println!("No contradictions detected.");
        println!();
        println!("Done.");
        return Ok(());
    }

    println!();
    println!("Forgeplan Contradictions");
    println!("{}", "═".repeat(50));
    println!();

    for c in &report.contradictions {
        // SEC-H1 (CWE-117 / CWE-150): ids and free text are attacker-
        // controllable via frontmatter; sanitize before TTY emission.
        println!(
            "  [{}] {} ↔ {} — {}",
            sanitize_for_hint(&c.severity),
            sanitize_for_hint(&c.left_artifact),
            sanitize_for_hint(&c.right_artifact),
            sanitize_for_hint(&c.kind_of_contradiction),
        );
        println!("      → {}", sanitize_for_hint(&c.suggested_resolution));
    }

    if !report.limitations.is_empty() {
        println!();
        println!("  Limitations:");
        for lim in &report.limitations {
            println!("    - {lim}");
        }
    }

    println!();
    println!("Fix: review each pair with `forgeplan get <left>` then `forgeplan get <right>`.");
    Ok(())
}
