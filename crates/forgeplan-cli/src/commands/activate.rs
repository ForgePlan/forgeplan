use forgeplan_core::lifecycle;
use forgeplan_core::projection;

use crate::commands::common;

pub async fn run(id: &str, force: bool) -> anyhow::Result<()> {
    let (ws, store) = common::open_store().await?;

    // Sync file→LanceDB before lifecycle call (preserve user edits)
    if let Some(record) = store.get_record(id).await? {
        projection::sync_file_to_store(&store, &ws, &record).await?;
    }

    let result = lifecycle::activate(&store, id, force).await?;

    // Re-render projection with updated status
    if let Some(record) = store.get_record(id).await? {
        let links = store.get_relations(id).await.unwrap_or_default();
        projection::render_projection(
            &ws, &record.id, &record.kind, &record.title, &record.status,
            &record.depth, record.author.as_deref(), record.parent_epic.as_deref(),
            record.valid_until.as_deref(), &record.body, &links,
        ).await?;
    }

    if result.forced {
        println!("  Activated {id} (draft → active)");
        println!(
            "  Warning: Activated with {} validation error{}",
            result.must_errors.len(),
            if result.must_errors.len() == 1 { "" } else { "s" }
        );
    } else {
        println!("  Activated {id} (draft → active)");
    }

    Ok(())
}
