use forgeplan_core::lifecycle;
use forgeplan_core::projection;

use crate::commands::common;

pub async fn run(id: &str, reason: &str, until: &str) -> anyhow::Result<()> {
    let (ws, store) = common::open_store().await?;

    // Sync file→LanceDB before lifecycle call (preserve user edits)
    if let Some(record) = store.get_record(id).await? {
        projection::sync_file_to_store(&store, &ws, &record).await?;
    }

    let result = lifecycle::renew(&store, id, reason, until).await?;

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
        Some("stale"),
        Some("active"),
        "cli",
    )
    .await;

    println!("  Renewed {id} (stale → active)");
    if let Some(old) = &result.old_valid_until {
        println!("  Valid until: {old} → {}", result.new_valid_until);
    } else {
        println!("  Valid until: {}", result.new_valid_until);
    }

    Ok(())
}
