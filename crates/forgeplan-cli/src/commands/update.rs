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
    if let Some(s) = status
        && s.eq_ignore_ascii_case("active")
    {
        anyhow::bail!(
            "Direct status change to 'active' is not allowed.\n\
             Use `forgeplan activate {}` to activate artifacts (enforces validation gates).",
            id
        );
    }

    // Depth update
    if let Some(d) = depth {
        let _: forgeplan_core::artifact::types::Mode = d.parse().map_err(|_| {
            anyhow::anyhow!(
                "Invalid depth '{}'. Valid: tactical, standard, deep, critical",
                d
            )
        })?;
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
    let body_updated = if let Some(b) = body {
        let body_content = if let Some(path) = b.strip_prefix('@') {
            tokio::fs::read_to_string(path)
                .await
                .map_err(|e| anyhow::anyhow!("Cannot read '{}': {}", path, e))?
        } else {
            b.to_string()
        };
        store.update_body(id, &body_content).await?;
        true
    } else {
        false
    };

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

    if body_updated {
        // Body was explicitly set via CLI — use render_projection_with_body
        // to override file-on-disk (files-first normally reads from file).
        projection::render_projection_with_body(
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
    } else {
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
    }

    // Log changes
    if let Some(s) = status {
        common::log_change_field(
            &store,
            id,
            "update",
            "status",
            Some(&original.status),
            Some(s),
            "cli",
        )
        .await;
    }
    if let Some(t) = title {
        common::log_change_field(
            &store,
            id,
            "update",
            "title",
            Some(&original.title),
            Some(t),
            "cli",
        )
        .await;
    }
    if body.is_some() {
        common::log_change_field(&store, id, "update", "body", None, None, "cli").await;
    }

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
