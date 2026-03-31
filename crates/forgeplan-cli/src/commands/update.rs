use forgeplan_core::projection;

use crate::commands::common;

pub async fn run(
    id: &str,
    status: Option<&str>,
    title: Option<&str>,
    depth: Option<&str>,
    body: Option<&str>,
) -> anyhow::Result<()> {
    let (ws, store) = common::open_store().await?;

    // Verify artifact exists (keep original for old projection cleanup)
    let original = store
        .get_record(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact '{}' not found", id))?;

    if status.is_none() && title.is_none() && depth.is_none() && body.is_none() {
        anyhow::bail!("Nothing to update. Use --status, --title, --depth, or --body.");
    }

    // Block direct status change to "active" — must go through lifecycle gates
    if let Some(s) = status {
        if s.eq_ignore_ascii_case("active") {
            anyhow::bail!(
                "Direct status change to 'active' is not allowed.\n\
                 Use `forgeplan activate {}` to activate artifacts (enforces validation gates).",
                id
            );
        }
    }

    // Depth update
    if let Some(d) = depth {
        let _: forgeplan_core::artifact::types::Mode = d
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid depth '{}'. Valid: tactical, standard, deep, critical", d))?;
        store.update_depth(id, d).await?;
    }

    // Sync file→LanceDB BEFORE any mutations — capture user edits from the OLD file
    // (must happen before title change which removes the old projection file)
    projection::sync_file_to_store(&store, &ws, &original).await?;

    // Update metadata (status, title)
    if status.is_some() || title.is_some() {
        store.update_artifact(id, status, title).await?;
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

    // Re-render projection with synced data
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
