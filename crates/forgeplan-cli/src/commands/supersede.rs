use forgeplan_core::lifecycle;
use forgeplan_core::projection;

use crate::commands::common;

pub async fn run(id: &str, by: &str) -> anyhow::Result<()> {
    let (ws, store) = common::open_store().await?;

    let old_status = store
        .get_record(id)
        .await?
        .map(|r| r.status)
        .unwrap_or_else(|| "active".to_string());

    // Sync file→LanceDB before lifecycle call (preserve user edits)
    if let Some(record) = store.get_record(id).await? {
        projection::sync_file_to_store(&store, &ws, &record).await?;
    }

    let result = lifecycle::supersede(&store, id, by).await?;

    // Re-render projection with updated status
    if let Some(record) = store.get_record(id).await? {
        let links = store.get_relations(id).await.unwrap_or_default();
        projection::render_projection(
            &ws,
            &record.id,
            &record.kind,
            &record.title,
            &record.status,
            &record.depth,
            record.author.as_deref(),
            record.parent_epic.as_deref(),
            record.valid_until.as_deref(),
            &record.body,
            &links,
        )
        .await?;
    }

    common::log_change_field(
        &store,
        id,
        "update",
        "status",
        Some(&old_status),
        Some("superseded"),
        "cli",
    )
    .await;

    println!("  Superseded {id} → {by}");

    for w in &result.warnings {
        println!("  {w}");
    }

    if !result.dependents.is_empty() {
        println!("\nDependents to update:");
        for dep in &result.dependents {
            println!("  ! {dep} depends on superseded {id} → consider updating to {by}");
        }
    }

    Ok(())
}
