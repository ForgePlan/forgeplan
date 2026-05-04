use forgeplan_core::hints::{self, Hint};
use forgeplan_core::projection;
use forgeplan_core::undo;

use crate::commands::common;

pub async fn run(id: &str, yes: bool) -> anyhow::Result<()> {
    let (ws, _lock, store) = common::open_store_locked().await?;

    let record = store.get_record(id).await?.ok_or_else(|| {
        anyhow::anyhow!(
            "Artifact '{}' not found
Fix: forgeplan list",
            id
        )
    })?;

    // Check for dependents (other artifacts linking TO this one)
    let all_relations = store.get_all_relations().await?;
    let dependents: Vec<_> = all_relations
        .iter()
        .filter(|(_, target, _)| target.eq_ignore_ascii_case(id))
        .collect();

    if !dependents.is_empty() {
        eprintln!("  WARNING: {} has {} dependent(s):", id, dependents.len());
        for (source, _, rel) in &dependents {
            eprintln!("    {} --{}--> {}", source, rel, id);
        }
        if !yes {
            anyhow::bail!(
                "{} has {} dependent(s). Use --yes to confirm deletion despite dependents.",
                id,
                dependents.len()
            );
        }
        eprintln!("  Proceeding with --yes despite dependents.");
    }

    if !yes {
        anyhow::bail!(
            "About to delete {} \"{}\". This cannot be undone. Use --yes to confirm.",
            record.id,
            record.title
        );
    }

    // Cascade: delete all relations involving this artifact.
    // Count from already-fetched data to avoid double table scan.
    let relation_count = all_relations
        .iter()
        .filter(|(s, t, _)| s.eq_ignore_ascii_case(id) || t.eq_ignore_ascii_case(id))
        .count();

    // PRD-055 + audit follow-up 2026-05-01: capture soft-delete receipt
    // BEFORE mutating store. Brings CLI into parity with MCP
    // `forgeplan_delete` which has had soft-delete since release.
    // Receipt holds full snapshot + relations so `forgeplan undo-last`
    // and `forgeplan restore <id>` recover the artifact end-to-end.
    let receipt_id = undo::soft_delete_capture(
        &ws,
        &store,
        &record,
        undo::DestructiveOp::Delete,
        None,
        None,
    )
    .await?;

    // PRD-073 file-first: helper removes the projection file (likely no-op
    // because soft_delete_capture already moved it to trash) then cascades
    // relations and the LanceDB row. Failure mid-flow is recoverable via
    // `forgeplan restore <id>` from the receipt above.
    projection::delete_artifact_with_projection(&ws, &store, id).await?;

    if relation_count > 0 {
        eprintln!("  Removed {} relation(s) involving {}", relation_count, id);
    }

    println!(
        "  Deleted: {} \"{}\" (receipt {receipt_id})",
        record.id, record.title
    );

    // Soft-deleted: surface the recovery path.
    let hint_list = vec![
        Hint::info(format!(
            "Recoverable via `forgeplan restore {id}` (within 30 days)"
        ))
        .with_action(format!("forgeplan restore {id}")),
    ];
    print!("{}", hints::render_next_action_line(&hint_list));

    Ok(())
}
