use std::env;

use forgeplan_core::db::store::LanceStore;
use forgeplan_core::projection;
use forgeplan_core::workspace;

pub async fn run(
    id: &str,
    status: Option<&str>,
    title: Option<&str>,
    depth: Option<&str>,
    body: Option<&str>,
) -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let store = LanceStore::open(&ws).await?;

    // Verify artifact exists (keep original for old projection cleanup)
    let original = store
        .get_record(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact '{}' not found", id))?;

    if status.is_none() && title.is_none() && depth.is_none() && body.is_none() {
        anyhow::bail!("Nothing to update. Use --status, --title, --depth, or --body.");
    }

    // Update metadata (status, title)
    if status.is_some() || title.is_some() {
        store.update_artifact(id, status, title).await?;
    }

    // Update depth (via update_artifact column)
    if let Some(d) = depth {
        // Validate depth
        let _: forgeplan_core::artifact::types::Mode = d
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid depth '{}'. Use: tactical, standard, deep", d))?;
        // LanceStore::update_artifact doesn't support depth yet — use update_body workaround
        // For now, we'll need to extend update_artifact. Skip depth for v1.
        eprintln!("  Note: depth update not yet supported in LanceStore. Use forgeplan new with correct depth.");
    }

    // Update body
    if let Some(b) = body {
        let body_content = if b.starts_with('@') {
            // Read from file
            let path = &b[1..];
            tokio::fs::read_to_string(path)
                .await
                .map_err(|e| anyhow::anyhow!("Cannot read '{}': {}", path, e))?
        } else {
            b.to_string()
        };
        store.update_body(id, &body_content).await?;
    }

    // Remove old projection file (title change → different slug → stale file)
    if title.is_some() {
        let _ = projection::remove_projection(&ws, id, &original.kind).await;
    }

    // Re-render projection
    let updated = store
        .get_record(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact '{}' disappeared after update", id))?;

    let links = store.get_relations(id).await.unwrap_or_default();
    projection::render_projection(
        &ws,
        &updated.id,
        &updated.kind,
        &updated.title,
        &updated.status,
        &updated.depth,
        updated.author.as_deref(),
        updated.parent_epic.as_deref(),
        updated.valid_until.as_deref(),
        &updated.body,
        &links,
    )
    .await?;

    println!("  Updated: {}", id);
    if let Some(s) = status {
        println!("  Status:  {}", s);
    }
    if let Some(t) = title {
        println!("  Title:   {}", t);
    }
    if body.is_some() {
        println!("  Body:    updated");
    }

    Ok(())
}
