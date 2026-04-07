//! forgeplan tag / untag — manage artifact tags.

use crate::commands::common;
use anyhow::{Context, Result};
use forgeplan_core::projection;

/// Add tags to an artifact.
pub async fn run_add(id: &str, tags: &[String]) -> Result<()> {
    let (ws, store) = common::open_store().await?;

    // Verify artifact exists
    let _existing = store
        .get_record(id)
        .await
        .with_context(|| format!("Failed to load {}", id))?
        .ok_or_else(|| anyhow::anyhow!("Artifact not found: {}", id))?;

    if tags.is_empty() {
        anyhow::bail!("No tags provided. Usage: forgeplan tag <id> <tag>...");
    }

    store
        .add_tags(id, tags)
        .await
        .with_context(|| format!("Failed to add tags to {}", id))?;

    let updated = store
        .get_record(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact disappeared after update: {}", id))?;

    // ADR-003 files-first: project tags back to markdown frontmatter so they
    // survive a reindex (otherwise tags live only in LanceDB and are lost).
    let links = store.get_relations(id).await.unwrap_or_default();
    projection::render_projection_record(&ws, &updated, &links)
        .await
        .with_context(|| format!("Failed to project tags to markdown for {}", id))?;

    println!("  ✓ Added {} tag(s) to {}", tags.len(), id);
    println!(
        "  Current tags: {}",
        if updated.tags.is_empty() {
            "(none)".to_string()
        } else {
            updated.tags.join(", ")
        }
    );

    Ok(())
}

/// Remove tags from an artifact.
pub async fn run_remove(id: &str, tags: &[String]) -> Result<()> {
    let (ws, store) = common::open_store().await?;

    let _existing = store
        .get_record(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact not found: {}", id))?;

    if tags.is_empty() {
        anyhow::bail!("No tags provided. Usage: forgeplan untag <id> <tag>...");
    }

    store
        .remove_tags(id, tags)
        .await
        .with_context(|| format!("Failed to remove tags from {}", id))?;

    let updated = store
        .get_record(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact disappeared after update: {}", id))?;

    // ADR-003 files-first: project tag removal to markdown frontmatter.
    let links = store.get_relations(id).await.unwrap_or_default();
    projection::render_projection_record(&ws, &updated, &links)
        .await
        .with_context(|| format!("Failed to project tags to markdown for {}", id))?;

    println!("  ✓ Removed {} tag(s) from {}", tags.len(), id);
    println!(
        "  Current tags: {}",
        if updated.tags.is_empty() {
            "(none)".to_string()
        } else {
            updated.tags.join(", ")
        }
    );

    Ok(())
}
