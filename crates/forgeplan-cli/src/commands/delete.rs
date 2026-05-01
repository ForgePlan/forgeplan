use forgeplan_core::hints::{self, Hint};
use forgeplan_core::projection;

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

    // PRD-073 file-first: helper removes the projection file FIRST, then
    // cascades relations and the LanceDB row. Failure mid-flow leaves the
    // workspace recoverable via reindex. Audit fix: announce relation
    // removal AFTER the helper succeeds so a mid-flow failure doesn't lie
    // to the user.
    projection::delete_artifact_with_projection(&ws, &store, id).await?;

    if relation_count > 0 {
        eprintln!("  Removed {} relation(s) involving {}", relation_count, id);
    }

    println!("  Deleted: {} \"{}\"", record.id, record.title);

    // Terminal action: deletion can't be undone, but the operator usually
    // wants to verify the workspace is consistent next.
    let hint_list =
        vec![Hint::info("Verify workspace integrity").with_action("forgeplan health".to_string())];
    print!("{}", hints::render_next_action_line(&hint_list));

    Ok(())
}
