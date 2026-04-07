use std::collections::HashSet;
use std::time::Duration;

use notify::RecursiveMode;
use notify_debouncer_mini::{DebouncedEventKind, new_debouncer};

use crate::commands::common;

/// Watch .forgeplan/ markdown files and sync changes to LanceDB in real time.
pub async fn run() -> anyhow::Result<()> {
    let (ws, store) = common::open_store().await?;

    let (tx, rx) = std::sync::mpsc::channel();
    let mut debouncer = new_debouncer(Duration::from_millis(500), tx)?;

    // Watch all artifact directories
    let mut watched = 0usize;
    for dir_name in forgeplan_core::workspace::ARTIFACT_DIRS {
        let dir = ws.join(dir_name);
        if dir.exists() {
            debouncer
                .watcher()
                .watch(&dir, RecursiveMode::NonRecursive)?;
            watched += 1;
        }
    }

    if watched == 0 {
        anyhow::bail!("No artifact directories found in .forgeplan/. Run `forgeplan init` first.");
    }

    println!(
        "Watching .forgeplan/ for changes ({} dirs)... (Ctrl+C to stop)",
        watched
    );

    // Graceful shutdown on Ctrl+C
    let (shutdown_tx, shutdown_rx) = std::sync::mpsc::channel::<()>();
    ctrlc::set_handler(move || {
        let _ = shutdown_tx.send(());
    })?;

    loop {
        // Check for shutdown signal (non-blocking)
        if shutdown_rx.try_recv().is_ok() {
            println!("\nShutting down watcher.");
            break;
        }

        // Wait for file events with timeout so we can check shutdown
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(Ok(events)) => {
                // Deduplicate paths within a single debounce batch
                let mut seen = HashSet::new();
                for event in &events {
                    if event.kind != DebouncedEventKind::Any {
                        continue;
                    }
                    let path = &event.path;
                    if path.extension().is_none_or(|e| e != "md") {
                        continue;
                    }
                    if !seen.insert(path.clone()) {
                        continue;
                    }

                    // Determine if file was deleted or changed
                    if !path.exists() {
                        // File deleted — warn, don't remove from LanceDB
                        if let Some(id) = extract_id_from_filename(path) {
                            eprintln!(
                                "WARN: {}.md deleted — artifact still in LanceDB. Run `forgeplan delete {}` to remove.",
                                id, id
                            );
                        }
                        continue;
                    }

                    // File created or modified — sync to LanceDB
                    match sync_single_file(&store, &ws, path).await {
                        Ok(Some(id)) => {
                            println!("SYNC {} — body updated from file", id);
                        }
                        Ok(None) => {
                            // Body unchanged, no sync needed
                        }
                        Err(e) => {
                            eprintln!("ERROR syncing {}: {}", path.display(), e);
                        }
                    }
                }
            }
            Ok(Err(e)) => {
                eprintln!("Watch error: {e}");
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Normal timeout, continue loop to check shutdown
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                break;
            }
        }
    }

    Ok(())
}

/// Sync a single .md file to LanceDB. Returns Some(id) if synced, None if unchanged.
async fn sync_single_file(
    store: &forgeplan_core::db::store::LanceStore,
    workspace: &std::path::Path,
    path: &std::path::Path,
) -> anyhow::Result<Option<String>> {
    let content = tokio::fs::read_to_string(path).await?;
    let (fm, body) = forgeplan_core::artifact::frontmatter::parse_frontmatter(&content)?;

    let id = fm
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("No id in frontmatter: {}", path.display()))?
        .to_string();

    match store.get_record(&id).await? {
        Some(record) => {
            // Use projection::sync_file_to_store for proper comparison
            let synced =
                forgeplan_core::projection::sync_file_to_store(store, workspace, &record).await?;
            if synced { Ok(Some(id)) } else { Ok(None) }
        }
        None => {
            // New file not in LanceDB — create artifact
            let dir_name = path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("notes");
            let kind = fm
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or(dir_name.trim_end_matches('s'));
            let status = fm.get("status").and_then(|v| v.as_str()).unwrap_or("draft");
            let title = fm.get("title").and_then(|v| v.as_str()).unwrap_or(&id);
            let depth = fm
                .get("depth")
                .and_then(|v| v.as_str())
                .unwrap_or("standard");

            let artifact = forgeplan_core::db::store::NewArtifact {
                id: id.clone(),
                kind: kind.to_string(),
                status: status.to_lowercase(),
                title: title.to_string(),
                body: body.to_string(),
                depth: depth.to_string(),
                author: fm.get("author").and_then(|v| v.as_str()).map(String::from),
                parent_epic: fm
                    .get("parent_epic")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                valid_until: fm
                    .get("valid_until")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                tags: Vec::new(),
            };

            store.create_artifact(&artifact).await?;
            println!("NEW  {} — created from file", id);
            Ok(Some(id))
        }
    }
}

/// Extract artifact ID from filename like "PRD-001-auth-system.md" → "PRD-001"
fn extract_id_from_filename(path: &std::path::Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    // Pattern: PREFIX-NNN-rest-of-slug
    let parts: Vec<&str> = stem.splitn(3, '-').collect();
    if parts.len() >= 2 {
        Some(format!("{}-{}", parts[0], parts[1]))
    } else {
        None
    }
}
