use forgeplan_core::link;

use crate::commands::common;

pub async fn run(source_id: &str, target_id: &str, relation: &str) -> anyhow::Result<()> {
    // Normalize relation
    let relation = link::normalize_relation(relation)?;

    let (ws, store) = common::open_store().await?;

    // Verify source exists
    let source = store.get_artifact(source_id).await?;
    if source.is_none() {
        anyhow::bail!("Source artifact '{}' not found", source_id);
    }

    // Verify target exists (warning only)
    let target = store.get_artifact(target_id).await?;
    if target.is_none() {
        eprintln!(
            "Warning: Target artifact '{}' not found in workspace (creating link anyway)",
            target_id
        );
    }

    // Add relation in LanceDB
    store.add_relation(source_id, target_id, &relation).await?;

    // Sync file→LanceDB if user edited the file, then update projection
    if let Some(record) = store.get_record(source_id).await? {
        forgeplan_core::projection::sync_file_to_store(&store, &ws, &record).await?;
        // Re-read record after potential sync
        let record = store.get_record(source_id).await?.unwrap();
        let all_relations = store.get_relations(source_id).await?;
        let links: Vec<(String, String)> = all_relations;
        forgeplan_core::projection::render_projection(
            &ws, &record.id, &record.kind, &record.title, &record.status,
            &record.depth, record.author.as_deref(), record.parent_epic.as_deref(),
            record.valid_until.as_deref(), &record.body, &links,
        ).await?;
    }

    common::log_change_field(&store, source_id, "link", "relation", None, Some(&format!("{}:{}", target_id, relation)), "cli").await;

    println!("Linked: {} --{}--> {}", source_id, relation, target_id);
    Ok(())
}

pub async fn run_unlink(source_id: &str, target_id: &str, relation: &str) -> anyhow::Result<()> {
    let relation = link::normalize_relation(relation)?;
    let (ws, store) = common::open_store().await?;

    store.delete_relation(source_id, target_id, &relation).await?;

    // Sync file→LanceDB if user edited the file, then update projection
    if let Some(record) = store.get_record(source_id).await? {
        forgeplan_core::projection::sync_file_to_store(&store, &ws, &record).await?;
        let record = store.get_record(source_id).await?.unwrap();
        let all_relations = store.get_relations(source_id).await?;
        let links: Vec<(String, String)> = all_relations;
        forgeplan_core::projection::render_projection(
            &ws, &record.id, &record.kind, &record.title, &record.status,
            &record.depth, record.author.as_deref(), record.parent_epic.as_deref(),
            record.valid_until.as_deref(), &record.body, &links,
        ).await?;
    }

    common::log_change_field(&store, source_id, "unlink", "relation", Some(&format!("{}:{}", target_id, relation)), None, "cli").await;

    println!("Unlinked: {} --{}--> {}", source_id, relation, target_id);
    Ok(())
}
