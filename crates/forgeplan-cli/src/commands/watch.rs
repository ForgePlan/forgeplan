use std::collections::HashSet;
use std::time::Duration;

use forgeplan_core::projection;
use notify::RecursiveMode;
use notify_debouncer_mini::{DebouncedEventKind, new_debouncer};

use crate::commands::common;

/// Watch .forgeplan/ markdown files and sync changes to LanceDB in real time.
///
/// `constellation` additionally runs a bounded [`maintain_index`] pass after
/// every debounced batch: the legacy sync keeps existing commands reading,
/// while the corrected 0A/0B index picks up the same change atomically —
/// creates, updates, deletes and renames land in one published generation,
/// which is exactly what the per-file legacy path cannot express.
pub async fn run(constellation: bool) -> anyhow::Result<()> {
    // The legacy store and the constellation index cannot coexist as live
    // tables — bootstrap renames `artifacts.lance`/`relations.lance` aside so
    // two protocols never write the same logical index. On a migrated
    // workspace `LanceStore::open` fails with a bare ENOENT; detect that
    // state explicitly so the failure names the cause and `--constellation`
    // keeps watching through the corrected index alone.
    let (ws, store) = common::open_store_tolerating_constellation().await?;
    if store.is_none() {
        if !constellation {
            anyhow::bail!(
                "this workspace's index moved to the constellation protocol — run `forgeplan watch --constellation`"
            );
        }
        println!(
            "Legacy index retired (constellation owns .forgeplan/lance) — constellation maintenance only."
        );
    }

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
        "Watching .forgeplan/ for changes ({} dirs){}... (Ctrl+C to stop)",
        watched,
        if constellation {
            ", constellation index on"
        } else {
            ""
        }
    );

    // PRD-071: long-running surface — emit Wait: line so agents know this
    // is a foreground process that mutates state on file changes.
    println!("Wait: changes will sync to LanceDB as files change (Ctrl-C to stop)");

    // Graceful shutdown on Ctrl+C
    let (shutdown_tx, shutdown_rx) = std::sync::mpsc::channel::<()>();
    ctrlc::set_handler(move || {
        let _ = shutdown_tx.send(());
    })?;

    // Files whose names do not follow the `<id>-<slug>.md` convention can
    // never be addressed by the legacy store — it derives the path from
    // id+kind+title, so `ADR-002.md` is out-of-reach for it (the corrected
    // index handles them fine). Retry would error on every debounce batch
    // forever; report once and skip until the path is recreated.
    let mut unsyncable: HashSet<std::path::PathBuf> = HashSet::new();

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
                let mut batch_had_change = false;
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
                    batch_had_change = true;

                    // Determine if file was deleted or changed
                    if !path.exists() {
                        // File deleted — warn, don't remove from LanceDB
                        unsyncable.remove(path);
                        if let Some(id) = extract_id_from_filename(path) {
                            eprintln!(
                                "WARN: {}.md deleted — artifact still in LanceDB. Run `forgeplan delete {}` to remove.",
                                id, id
                            );
                        }
                        continue;
                    }

                    if unsyncable.contains(path) {
                        continue;
                    }

                    // File created or modified — sync to LanceDB. On a
                    // migrated workspace there is no legacy store to feed;
                    // the constellation pass below owns the index alone.
                    let Some(store) = store.as_ref() else {
                        batch_had_change = true;
                        continue;
                    };
                    match sync_single_file(store, &ws, path).await {
                        Ok(Some(id)) => {
                            println!("SYNC {} — body updated from file", id);
                        }
                        Ok(None) => {
                            // Body unchanged, no sync needed
                        }
                        Err(e) => {
                            if let Some(forgeplan_core::projection::MutationError::FileNotFound {
                                ..
                            }) = e.downcast_ref::<forgeplan_core::projection::MutationError>()
                            {
                                unsyncable.insert(path.clone());
                                eprintln!(
                                    "WARN {} — filename does not follow '<id>-<slug>.md'; the legacy index cannot address it (the constellation index still covers it). Skipping.",
                                    path.display()
                                );
                            } else {
                                eprintln!("ERROR syncing {}: {}", path.display(), e);
                            }
                        }
                    }
                }

                // One maintenance pass per debounced batch, never per file:
                // the pass itself captures the whole workspace, so batching
                // is what keeps N rapid saves from becoming N publishes.
                if constellation && batch_had_change {
                    let canonical = ws.canonicalize().unwrap_or_else(|_| ws.clone());
                    let capability =
                        forgeplan_core::constellation::facade::MaintainCapability::for_strict_maintenance(canonical);
                    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
                    match forgeplan_core::constellation::facade::maintain_index(
                        &capability,
                        forgeplan_core::constellation::capture::CaptureLimits::default(),
                        deadline,
                    )
                    .await
                    {
                        Ok((view, outcome)) => {
                            println!(
                                "INDEX {} — {} commit(s)",
                                view.manifest.generation_id,
                                outcome.commits.len()
                            );
                            // 0B: a configured model means the queue drains
                            // here too — normal operation needs no manual
                            // `forgeplan embed`. Unconfigured is a no-op.
                            crate::commands::index::drain_queue_if_configured(
                                &capability,
                                std::time::Instant::now() + std::time::Duration::from_secs(300),
                                "watch",
                            )
                            .await;
                        }
                        Err(e) => {
                            // A failed pass leaves the published generation
                            // untouched — the index degrades to stale, never
                            // to torn. Report and keep watching.
                            eprintln!("ERROR constellation maintenance: {e}");
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

            projection::sync_artifact_from_file(
                &projection::MutationContext::new(workspace, store),
                &artifact,
            )
            .await?;
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
