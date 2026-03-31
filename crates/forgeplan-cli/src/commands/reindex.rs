use crate::commands::common;

/// Rebuild LanceDB index from .md files (files-first, RFC-004).
///
/// Walks all artifact directories, parses frontmatter + body from each .md file,
/// and upserts into LanceDB. Safety net when lazy sync missed changes.
pub async fn run() -> anyhow::Result<()> {
    let (ws, store) = common::open_store().await?;

    println!("Reindexing from .forgeplan/ markdown files...\n");

    let mut synced = 0usize;
    let mut skipped = 0usize;
    let mut errors = 0usize;

    for dir_name in forgeplan_core::workspace::ARTIFACT_DIRS {
        let dir = ws.join(dir_name);
        if !dir.exists() {
            continue;
        }
        let mut read_dir = tokio::fs::read_dir(&dir).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(true, |e| e != "md") {
                continue;
            }

            let content = match tokio::fs::read_to_string(&path).await {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("  SKIP {}: read error: {}", path.display(), e);
                    errors += 1;
                    continue;
                }
            };

            let (fm, body) = match forgeplan_core::artifact::frontmatter::parse_frontmatter(&content) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("  SKIP {}: parse error: {}", path.display(), e);
                    errors += 1;
                    continue;
                }
            };

            let id = match fm.get("id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => {
                    eprintln!("  SKIP {}: no id in frontmatter", path.display());
                    errors += 1;
                    continue;
                }
            };

            // Check if artifact exists in LanceDB
            match store.get_record(&id).await? {
                Some(record) => {
                    // Compare body — sync if different
                    if record.body.trim() != body.trim() {
                        store.update_body(&id, &body).await?;
                        println!("  SYNC {} — body updated from file", id);
                        synced += 1;
                    } else {
                        skipped += 1;
                    }
                }
                None => {
                    // Artifact in file but not in LanceDB — create it
                    let kind = fm.get("kind").and_then(|v| v.as_str()).unwrap_or(dir_name.trim_end_matches('s'));
                    let status = fm.get("status").and_then(|v| v.as_str()).unwrap_or("draft");
                    let title = fm.get("title").and_then(|v| v.as_str()).unwrap_or(&id);
                    let depth = fm.get("depth").and_then(|v| v.as_str()).unwrap_or("standard");

                    let artifact = forgeplan_core::db::store::NewArtifact {
                        id: id.clone(),
                        kind: kind.to_string(),
                        status: status.to_lowercase(),
                        title: title.to_string(),
                        body: body.to_string(),
                        depth: depth.to_string(),
                        author: fm.get("author").and_then(|v| v.as_str()).map(String::from),
                        parent_epic: fm.get("parent_epic").and_then(|v| v.as_str()).map(String::from),
                        valid_until: fm.get("valid_until").and_then(|v| v.as_str()).map(String::from),
                    };

                    store.create_artifact(&artifact).await?;
                    println!("  NEW  {} — created from file", id);
                    synced += 1;
                }
            }
        }
    }

    println!("\nReindex complete: {} synced, {} unchanged, {} errors.", synced, skipped, errors);
    Ok(())
}
