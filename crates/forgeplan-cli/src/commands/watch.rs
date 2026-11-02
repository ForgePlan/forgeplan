use std::collections::HashSet;
use std::time::Duration;

use forgeplan_core::artifact::canonical_key::canonical_key_from_document;
use forgeplan_core::projection;
use notify::RecursiveMode;
use notify_debouncer_mini::{DebouncedEventKind, new_debouncer};

use crate::commands::common;

/// Watch .forgeplan/ markdown files through the workspace's index owner.
///
/// A migrated workspace automatically uses the Constellation maintenance
/// host. `--constellation` also selects that host on a legacy workspace and
/// performs the protocol handoff; it never keeps a legacy writer beside it.
///
/// # Two ways a change is noticed, and why both are there
///
/// The host signals its supervisor through a filesystem event, and the signal
/// coalesces: a burst of atomic saves is one pass. Events are only a *hint* —
/// an overflow, a lost notification or a sleep/resume can hide one — so the
/// host also re-captures on its own control-scan clock, which is what bounds
/// how long a missed event can hide a change.
pub async fn run(constellation: bool) -> anyhow::Result<()> {
    // Decide under the workspace lock, before opening any legacy tables.
    // Damaged protocol files are errors, not permission to resume legacy
    // migrations. The maintenance host takes its own locks after this guard
    // is released.
    let (ws, lock) = common::open_workspace_locked().await?;
    let index_root = ws.join("lance");
    forgeplan_core::constellation::storage::read_marker(&index_root)?;
    forgeplan_core::constellation::storage::read_current(&index_root)?;
    let migrated = common::index_migrated(&ws);
    let constellation = constellation || migrated;
    let legacy = !constellation;
    drop(lock);

    if migrated {
        println!(
            "Legacy index retired (constellation owns .forgeplan/lance) — constellation maintenance only."
        );
    }

    let (tx, rx) = std::sync::mpsc::channel();
    // The runtime's own debounce bound, not an arbitrary one: `DEBOUNCE_MS` is
    // the constant the maintenance protocol defines for folding a burst of
    // atomic saves into one capture.
    let mut debouncer = new_debouncer(
        Duration::from_millis(forgeplan_core::constellation::runtime::DEBOUNCE_MS),
        tx,
    )?;

    // The constellation index is kept current by a maintenance **host**, not by
    // this loop. The host owns the recursive watcher, the coalesced dirty bit,
    // the 30-second control scan and the registered supervisor; driving them
    // from here as well is how the two halves came to disagree about debounce
    // and about what a pass reports.
    //
    // The owner is selected by the existing flag or by the migrated layout.
    // A default legacy watch keeps its per-file projection and registers no
    // maintenance runtime.
    let host = if constellation {
        // This foreground watch explicitly maintains the selected workspace.
        // The host receives that capability rather than minting one itself.
        let capability =
            forgeplan_core::constellation::facade::MaintainCapability::for_strict_maintenance(
                ws.canonicalize()?,
            );
        Some(
            forgeplan_core::constellation::runtime::MaintenanceHost::start(
                capability,
                forgeplan_core::constellation::runtime::ReportSink::Foreground,
            )
            .await?
            .drive(),
        )
    } else {
        None
    };

    // Watch all artifact directories — for the **legacy** per-file projection
    // only. The host has its own watcher for the constellation index; this one
    // exists because `sync_single_file` needs a path per event.
    let mut watched: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    watch_artifact_dirs(&ws, debouncer.watcher(), &mut watched)?;

    if watched.is_empty() {
        anyhow::bail!("No artifact directories found in .forgeplan/. Run `forgeplan init` first.");
    }

    println!(
        "Watching .forgeplan/ for changes ({} dirs){}... (Ctrl+C to stop)",
        watched.len(),
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
                let mut synced_ids = Vec::new();
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

                    // The host owns every change on the migrated layout,
                    // including deletes and renames. Legacy-only diagnostics
                    // and syncs must not run beside it.
                    if !legacy {
                        continue;
                    };
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

                    // File created or modified in the legacy index.
                    match sync_single_file(&ws, path).await {
                        Ok(Some(id)) => synced_ids.push(id),
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

                // One signal per debounced batch, never per file. The
                // supervisor coalesces: N rapid saves of one artifact are one
                // signalled pass, and the pass itself captures the whole
                // workspace.
                if batch_had_change {
                    if legacy {
                        common::drain_legacy_semantic_after_mutation(&ws).await?;
                        for id in &synced_ids {
                            println!("SYNC {} — body updated from file", id);
                        }
                    }
                    // Through the registry, not through a handle, so this is
                    // the same call any other in-process writer would make —
                    // and its `false` is the honest report that nobody is
                    // listening, which the full-scan clock below covers.
                    forgeplan_core::constellation::runtime::mark_dirty(&ws);
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

        // Every tick, whether or not an event arrived, a new artifact
        // directory may have been created. The host widens its own watch set on
        // its own clock; this one only needs it for the legacy per-file
        // projection, which is told about a path at a time.
        watch_artifact_dirs(&ws, debouncer.watcher(), &mut watched)?;
    }

    // Stop the host: admission closes, the watcher goes, and a native call
    // still in flight is detached rather than waited on.
    if let Some(runner) = host {
        runner.stop().await;
    }

    Ok(())
}

/// Watch every artifact directory that exists, and add one for any that
/// appears later.
///
/// A watcher that only enumerates at startup never sees a directory created
/// after it — `forgeplan init` in a second worktree, or an artifact kind
/// added by a newer build — and the files written into it would sit
/// unindexed until a restart. The set is what makes this incremental: an
/// already-watched directory is not re-watched.
fn watch_artifact_dirs(
    workspace: &std::path::Path,
    watcher: &mut dyn notify::Watcher,
    watched: &mut std::collections::BTreeSet<String>,
) -> anyhow::Result<()> {
    // Recursive, for the same reason the maintenance host's watcher is: an
    // artifact file below a grouping directory (`prds/PRD-074/…`) is a real
    // layout, and a non-recursive watch would only ever see the directory.
    for dir_name in forgeplan_core::workspace::ARTIFACT_DIRS {
        if watched.contains(*dir_name) {
            continue;
        }
        let dir = workspace.join(dir_name);
        if dir.is_dir() {
            watcher.watch(&dir, RecursiveMode::Recursive)?;
            watched.insert((*dir_name).to_string());
        }
    }
    Ok(())
}

/// Sync a single .md file to LanceDB. Returns Some(id) if synced, None if unchanged.
async fn sync_single_file(
    workspace: &std::path::Path,
    path: &std::path::Path,
) -> anyhow::Result<Option<String>> {
    let guard = forgeplan_core::workspace::acquire_workspace_lock(workspace).await?;
    let content = tokio::fs::read_to_string(path).await?;
    let (fm, body) = forgeplan_core::artifact::frontmatter::parse_frontmatter(&content)?;

    let id = fm
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("No id in frontmatter: {}", path.display()))?
        .to_string();

    let canonical = canonical_key_from_document(&id, &content);
    let candidates = forgeplan_core::artifact::documents::workspace_candidates(workspace).await?;
    match common::resolve_in_document_set(&canonical, &candidates) {
        common::Resolution::Found(found)
            if workspace.join(&found.relative_path) == path
                && found.identity.canonical_key == canonical => {}
        common::Resolution::Found(_) => anyhow::bail!("watch: file no longer names the selected canonical artifact"),
        outcome => return Err(common::refuse_resolution(&canonical, &outcome, &candidates)),
    }
    let store = common::prepare_store_locked_checked(workspace, &guard, &[])
        .await?
        .ok_or_else(|| anyhow::anyhow!("watch: index ownership changed; restart watch"))?;

    match store.get_record(&guard, &canonical).await? {
        Some(record) => {
            // Use projection::sync_file_to_store for proper comparison
            let synced =
                forgeplan_core::projection::sync_file_to_store(&guard, &store, workspace, &record).await?;
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
                // ADR-030: the canonical key of a document read from disk is
                // the slug its frontmatter declares, and its display id when
                // it declares none.
                canonical_key: canonical,
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

            // The watcher writes derived rows, so this sync runs under the workspace
            // lock. Acquired here, per file, and released when this function
            // returns — the watcher loop must never hold it across a debounce
            // sleep, or every other mutator would block for the full timeout.
            projection::sync_artifact_from_file(
                &projection::MutationContext::new(workspace, &guard, Some(&store)),
                &artifact,
            )
            .await?;
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
