//! `forgeplan orphans` — issue #571 CLI surface for the brownfield orphans
//! report. Mirrors the `forgeplan_orphans` MCP tool: same records + relations
//! inputs, same `orphans_report` computation, identical JSON with `--json`.

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
    let relations = match &source {
        common::ReadSource::Legacy(store) => store.get_all_relations(&_read_guard).await?,
        common::ReadSource::Captured(read) => read.get_all_relations(),
    };

    let report = forgeplan_core::brownfield::orphans_report(records, &relations);

    if json {
        // Mirrors the `forgeplan_orphans` MCP wire shape: OrphansReport
        // serializes straight through, plus the `_next_action` marker.
        let total = report.uncovered_use_cases.len()
            + report.unverified_invariants.len()
            + report.orphan_terms.len()
            + report.untriangulated_hypotheses.len();
        let next_action = if total == 0 {
            "Done. — no brownfield orphans.".to_string()
        } else {
            format!(
                "Next: link {total} orphan(s) across 4 buckets — add scenarios, reference terms, or triangulate hypotheses."
            )
        };
        let mut payload = serde_json::to_value(&report)?;
        payload
            .as_object_mut()
            .expect("OrphansReport serializes to an object")
            .insert(
                "_next_action".to_string(),
                serde_json::Value::String(next_action),
            );
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    let buckets = [
        (
            "Uncovered use cases (no scenario links)",
            &report.uncovered_use_cases,
        ),
        (
            "Unverified invariants (no scenario links)",
            &report.unverified_invariants,
        ),
        (
            "Orphan glossary terms (unreferenced in UC/INV bodies)",
            &report.orphan_terms,
        ),
        (
            "Untriangulated hypotheses (< 2 evidence refs)",
            &report.untriangulated_hypotheses,
        ),
    ];
    let total: usize = buckets.iter().map(|(_, ids)| ids.len()).sum();

    if total == 0 {
        println!("No brownfield orphans found.");
        println!();
        println!("Done.");
        return Ok(());
    }

    println!();
    println!("Forgeplan Brownfield Orphans");
    println!("{}", "═".repeat(50));

    for (label, ids) in &buckets {
        if ids.is_empty() {
            continue;
        }
        println!();
        println!("  {label} ({}):", ids.len());
        for id in *ids {
            println!("    {id}");
        }
    }

    println!();
    println!(
        "Fix: add scenarios, reference glossary terms in UC/INV bodies, or triangulate hypotheses."
    );
    Ok(())
}
