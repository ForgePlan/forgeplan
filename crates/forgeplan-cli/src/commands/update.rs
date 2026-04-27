use forgeplan_core::hints::{self, Hint};
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
    let original = store.get_record(id).await?.ok_or_else(|| {
        anyhow::anyhow!(
            "Artifact '{}' not found
Fix: forgeplan list",
            id
        )
    })?;

    if status.is_none() && title.is_none() && depth.is_none() && body.is_none() {
        // PRD-071: Error pairs with concrete Fix: command (use --status as the
        // most-common operation; agent picks the right flag from the example).
        anyhow::bail!(
            "Nothing to update. Use --status, --title, --depth, or --body.\nFix: forgeplan update {} --status draft",
            id
        );
    }

    // Block direct status change to "active" — must go through lifecycle gates.
    // PRD-071 contract: pair `Error:` with a structured `Fix:` line that the
    // agent can copy verbatim. anyhow renders the bail message after a literal
    // "Error: " prefix, so the embedded `\nFix: ...` appears on its own line.
    if let Some(s) = status
        && s.eq_ignore_ascii_case("active")
    {
        anyhow::bail!(
            "Direct status change to 'active' is not allowed.\nFix: forgeplan activate {}",
            id
        );
    }

    // Depth update
    if let Some(d) = depth {
        let _: forgeplan_core::artifact::types::Mode = d.parse().map_err(|_| {
            // PRD-071: pair Error with a concrete Fix command (default to
            // `standard` — the most-common reset value).
            anyhow::anyhow!(
                "Invalid depth '{}'. Valid: tactical, standard, deep, critical\nFix: forgeplan update {} --depth standard",
                d,
                id,
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
        let raw_content = if let Some(path) = b.strip_prefix('@') {
            tokio::fs::read_to_string(path)
                .await
                .map_err(|e| anyhow::anyhow!("Cannot read '{}': {}", path, e))?
        } else {
            b.to_string()
        };

        // Strip YAML frontmatter if present (when reading from @file.md)
        let body_content = if raw_content.starts_with("---") {
            use forgeplan_core::artifact::frontmatter;
            match frontmatter::parse_frontmatter(&raw_content) {
                Ok((_fm, body)) => body.to_string(),
                Err(_) => raw_content,
            }
        } else {
            raw_content
        };

        // Safety check: warn if new body is significantly shorter than existing
        // (likely shell escaping corruption — use --body @file for safe updates)
        // Re-read from store AFTER sync to get the latest body (sync may have updated it from file)
        let current = store.get_record(id).await?.unwrap_or(original.clone());
        let old_len = current.body.len();
        let new_len = body_content.len();
        if old_len > 100 && new_len < old_len / 3 {
            eprintln!(
                "  ⚠ Warning: new body ({} chars) is much shorter than existing ({} chars).",
                new_len, old_len
            );
            eprintln!(
                "  Tip: use --body @file.md for safe multi-line updates (avoids shell escaping issues)."
            );
        }

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

    // PRD-071: any update — re-validate to make sure the artifact still
    // satisfies its kind+depth rules.
    let next_hints: Vec<Hint> = vec![
        Hint::info("Updated — re-run validator").with_action(format!("forgeplan validate {}", id)),
    ];
    print!("{}", hints::render_next_action_line(&next_hints));

    Ok(())
}
