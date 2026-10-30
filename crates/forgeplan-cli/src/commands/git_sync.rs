use forgeplan_core::artifact::frontmatter;
use forgeplan_core::db::{auxiliary::AuxiliaryStore, store::LanceStore};
use forgeplan_core::git;
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::projection;

use crate::commands::common;

/// Run the reconcile pass only on a workspace the constellation protocol
/// already owns.
///
/// `git-sync` is an ordinary maintenance command that runs after `git pull`.
/// Calling [`common::reconcile_and_drain`] unconditionally would migrate a
/// legacy workspace as a side effect of a pull: it renames the legacy tables
/// to `*.legacy-backup.lance` and publishes a generation. That is a one-way
/// door the operator never asked for, and it cannot be undone from the CLI.
/// Migration stays where it was asked for by name — `forgeplan index`.
async fn reconcile_if_constellation_owned(ws: &std::path::Path) -> anyhow::Result<()> {
    if common::index_migrated(ws) {
        common::reconcile_and_drain(ws, "git-sync").await?;
    }
    Ok(())
}

/// `forgeplan git-sync [--since <ref>]` — sync artifact changes from git operations.
///
/// Detects .forgeplan/ files changed since a git ref (default: ORIG_HEAD from last pull/merge)
/// and syncs them into LanceDB with source=git_sync and commit_hash.
pub async fn run(since: Option<&str>) -> anyhow::Result<()> {
    let (ws, _lock, store) = common::open_store_locked_tolerating_constellation().await?;
    let captured_layout = store.is_none();

    // PRD-071 contract: errors emit `Fix:` markers so agents have a
    // deterministic next action.
    let repo_root = ws.parent().ok_or_else(|| {
        anyhow::anyhow!(
            "Cannot determine repo root from workspace\n\
             Fix: forgeplan health"
        )
    })?;

    // Determine the reference point
    let since_ref = if let Some(s) = since {
        s.to_string()
    } else {
        // Try ORIG_HEAD (set after git pull/merge/rebase)
        git::orig_head(repo_root).ok_or_else(|| {
            anyhow::anyhow!(
                "No ORIG_HEAD found (no recent git pull/merge).\n\
                 Fix: forgeplan git-sync --since HEAD~3"
            )
        })?
    };

    let commit_hash = git::head_commit_hash(repo_root).unwrap_or_else(|| "unknown".to_string());

    println!(
        "  Git sync: {}..HEAD (commit {})",
        since_ref.chars().take(10).collect::<String>(),
        commit_hash
    );

    // PRD-071 contract: git diff failures emit `Fix:` markers.
    let changed = git::changed_artifact_files(repo_root, &since_ref)
        .map_err(|e| anyhow::anyhow!("{}\nFix: forgeplan git-sync --since HEAD~3", e))?;

    if changed.is_empty() {
        println!(
            "  No .forgeplan/ files changed since {}.",
            since_ref.chars().take(10).collect::<String>()
        );
        // An empty diff can still leave pending vectors behind — but only on a
        // workspace the protocol already owns. Reconciling a legacy workspace
        // here would migrate it as a side effect of an ordinary `git pull`,
        // which is not what the operator asked for.
        drop(store);
        drop(_lock);
        reconcile_if_constellation_owned(&ws).await?;
        println!("\nDone.");
        return Ok(());
    }

    let published = if captured_layout {
        let published = super::reindex::published_history_records(&ws)
            .await
            .map_err(|error| {
                anyhow::anyhow!("git-sync: Constellation reconcile failed: {error}")
            })?;
        println!(
            "  {} file(s) changed — legacy index retired; constellation reconcile owns them:",
            changed.len()
        );
        for file in &changed {
            println!("    {} {}", file.status, file.path);
        }
        published
    } else {
        std::collections::BTreeMap::new()
    };
    let current = if captured_layout {
        Some(common::workspace_read(&ws).await.map_err(|error| {
            anyhow::anyhow!("git-sync: Constellation reconcile failed: {error}")
        })?)
    } else {
        None
    };

    let mut synced = 0usize;
    let mut deleted = 0usize;
    let mut errors = 0usize;

    // The source mutation/audit phase owns the guard returned above. Keep
    // the short-lived history handle inside it, then drop both store handles
    // before unlocking for reconciliation.
    let mut history = None;

    for file in &changed {
        let full_path = repo_root.join(&file.path);

        match file.status {
            'D' => {
                // File deleted in git — extract ID from filename
                if let Some(id) = extract_id_from_path(&file.path) {
                    let existed = match store.as_ref() {
                        Some(store) => match store.get_record(&id).await? {
                            Some(record) => {
                                super::reindex::orphan_reason(
                        &ws,
                        &id,
                        &record.kind,
                        &record.title,
                        &record.canonical_key,
                    )
                                    .await
                                    .is_some()
                            }
                            None => false,
                        },
                        None => {
                            let absent = match tokio::fs::symlink_metadata(&full_path).await {
                                Ok(_) => false,
                                Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
                                Err(error) => return Err(error.into()),
                            };
                            absent
                                && published.contains_key(&id)
                                && current
                                    .as_ref()
                                    .is_some_and(|read| read.get_record(&id).is_none())
                        }
                    };
                    if !existed {
                        continue;
                    }
                    if let Some(store) = store.as_ref() {
                        projection::delete_orphan_artifact(
                            &projection::MutationContext::new(&ws, &_lock, Some(store)),
                            &id,
                        )
                        .await?;
                    }
                    let entry =
                        forgeplan_core::changelog::ChangeLogEntry::new(&id, "delete", "git_sync")
                            .with_commit(&commit_hash);
                    log_git_change(&ws, store.as_ref(), &mut history, &entry).await;
                    println!("  DEL  {} (removed in git)", id);
                    deleted += 1;
                }
            }
            'A' | 'M' => {
                // File added or modified — sync to LanceDB
                if !full_path.exists() {
                    eprintln!("  SKIP {} — file not found on disk", file.path);
                    errors += 1;
                    continue;
                }

                let content = match tokio::fs::read_to_string(&full_path).await {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("  SKIP {} — read error: {}", file.path, e);
                        errors += 1;
                        continue;
                    }
                };

                let (fm, body) = match frontmatter::parse_frontmatter(&content) {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("  SKIP {} — parse error: {}", file.path, e);
                        errors += 1;
                        continue;
                    }
                };

                let id = match fm.get("id").and_then(|v| v.as_str()) {
                    Some(id) => id.to_string(),
                    None => {
                        eprintln!("  SKIP {} — no id in frontmatter", file.path);
                        errors += 1;
                        continue;
                    }
                };
                if let Some(current) = current.as_ref()
                    && current.get_record(&id).is_none()
                {
                    anyhow::bail!(
                        "git-sync: artifact {id} is absent from the confirmed source capture"
                    );
                }

                let record = match store.as_ref() {
                    Some(store) => store.get_record(&id).await?,
                    None => None,
                };
                let old_body = match record.as_ref() {
                    Some(record) => Some(record.body.as_str()),
                    None => match published.get(&id) {
                        Some(record) => Some(record.body.as_deref().ok_or_else(|| {
                            anyhow::anyhow!("published artifact {id} has no body")
                        })?),
                        None => None,
                    },
                };
                let action = match old_body {
                    Some(old_body) => {
                        // Use the established body-trim/no-op rule for either
                        // index owner; a fresh live capture is not a before-image.
                        if old_body.trim() != body.trim() {
                            if let (Some(store), Some(record)) = (store.as_ref(), record.as_ref()) {
                                let ctx =
                                    projection::MutationContext::new(&ws, &_lock, Some(store));
                                // A title edit may have moved the file without changing its ID.
                                let file_title = fm
                                    .get("title")
                                    .and_then(|value| value.as_str())
                                    .unwrap_or(&record.title);
                                let status = fm
                                    .get("status")
                                    .and_then(|value| value.as_str())
                                    .map(str::to_lowercase);
                                projection::sync_metadata_from_file(
                                    &ctx,
                                    &id,
                                    status
                                        .as_deref()
                                        .filter(|status| *status != record.status.as_str()),
                                    (file_title != record.title).then_some(file_title),
                                )
                                .await?;
                                projection::sync_body_from_file(
                                    &ctx,
                                    &id,
                                    &record.kind,
                                    file_title,
                                    &body,
                                )
                                .await?;
                            }
                            "update"
                        } else {
                            continue; // No changes
                        }
                    }
                    None => {
                        // New artifact from git
                        if let Some(store) = store.as_ref() {
                            let kind = fm.get("kind").and_then(|v| v.as_str()).unwrap_or("note");
                            let status =
                                fm.get("status").and_then(|v| v.as_str()).unwrap_or("draft");
                            let title = fm.get("title").and_then(|v| v.as_str()).unwrap_or(&id);
                            let depth = fm
                                .get("depth")
                                .and_then(|v| v.as_str())
                                .unwrap_or("standard");

                            let artifact = forgeplan_core::db::store::NewArtifact {
                                canonical_key: id.clone(),
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
                                tags: forgeplan_core::artifact::frontmatter::tags_from_frontmatter(
                                    &fm,
                                ),
                            };

                            projection::sync_artifact_from_file(
                                &projection::MutationContext::new(&ws, &_lock, Some(store)),
                                &artifact,
                            )
                            .await?;
                        }
                        "create"
                    }
                };

                // Restore links from frontmatter
                if let Some(store) = store.as_ref()
                    && let Some(links_val) = fm.get("links")
                    && let Some(links_arr) = links_val.as_sequence()
                {
                    let existing = store.get_relations(&id).await.unwrap_or_default();
                    for link in links_arr {
                        let target = link.get("target").and_then(|v| v.as_str());
                        let relation = link.get("relation").and_then(|v| v.as_str());
                        if let (Some(t), Some(r)) = (target, relation)
                            && !existing.iter().any(|(et, er)| et == t && er == r)
                        {
                            let _ = projection::sync_relation_from_file(
                                &projection::MutationContext::new(&ws, &_lock, Some(store)),
                                &id,
                                t,
                                r,
                            )
                            .await;
                        }
                    }
                }

                let entry = forgeplan_core::changelog::ChangeLogEntry::new(&id, action, "git_sync")
                    .with_commit(&commit_hash);
                log_git_change(&ws, store.as_ref(), &mut history, &entry).await;

                let status_char = if action == "create" { "NEW " } else { "SYNC" };
                println!(
                    "  {} {} (from git, commit {})",
                    status_char, id, commit_hash
                );
                synced += 1;
            }
            _ => {
                // --no-renames exposes identity moves as A/D paths. Only A/M/D
                // statuses are actionable; the reconciler owns the managed index.
                continue;
            }
        }
    }

    // One reconcile for the whole sync, not one per changed file. The capture
    // reads the entire workspace, so batching is what keeps N files in one
    // commit from becoming N publications. The workspace lock taken for the
    // per-file writes above is released first — `reconcile_and_drain`
    // re-acquires it per storage commit.
    //
    // Signatures:
    //   `MaintainCapability::for_strict_maintenance(impl Into<PathBuf>)`
    //     — facade.rs:90
    //   `maintain_index(&MaintainCapability, CaptureLimits, Instant)
    //     -> Result<(PublishedReadView, ReconcileOutcome), ConstellationError>`
    //     — facade.rs:577
    //
    // The capability takes the **canonical** workspace path: identity is
    // scheme-independent, and a non-canonical path would be a different
    // physical index and a different owner lock.
    drop(history);
    drop(store);
    drop(current);
    drop(_lock);
    // Legacy workspaces keep their legacy writer: an ordinary `git pull` must
    // not migrate the index as a side effect. Only a workspace the protocol
    // already owns gets a reconcile pass here.
    reconcile_if_constellation_owned(&ws).await?;
    if captured_layout {
        println!("\nDone.");
        return Ok(());
    }

    println!(
        "\nGit sync complete: {} synced, {} deleted, {} errors (commit {})",
        synced, deleted, errors, commit_hash
    );

    // PRD-071 contract: after a sync, run health to surface anything new.
    let hints_vec = vec![
        Hint::suggestion("Audit synced artifacts").with_action("forgeplan health".to_string()),
    ];
    print!("{}", hints::render_next_action_line(&hints_vec));

    Ok(())
}

/// Preserve Git's already-built entry, including its commit hash, while
/// selecting exactly one sink. The caller owns the workspace lock.
async fn log_git_change(
    ws: &std::path::Path,
    store: Option<&LanceStore>,
    history: &mut Option<AuxiliaryStore>,
    entry: &forgeplan_core::changelog::ChangeLogEntry,
) {
    let result = match store {
        Some(store) => store.log_change(entry).await,
        None => {
            if history.is_none() {
                match AuxiliaryStore::open_history_or_create(ws).await {
                    Ok(opened) => *history = Some(opened),
                    Err(error) => {
                        eprintln!(
                            "  Warning: changelog write failed for {}: {}",
                            entry.artifact_id, error
                        );
                        return;
                    }
                }
            }
            match history.as_ref() {
                Some(history) => history.log_change(entry).await,
                None => unreachable!("history is initialized above or the function returned"),
            }
        }
    };
    if let Err(error) = result {
        eprintln!(
            "  Warning: changelog write failed for {}: {}",
            entry.artifact_id, error
        );
    }
}

/// Extract artifact ID from a path like ".forgeplan/prds/PRD-001-auth-system.md"
fn extract_id_from_path(path: &str) -> Option<String> {
    let filename = path.rsplit('/').next()?;
    let stem = filename.strip_suffix(".md")?;
    // mem- IDs are full slugs (mem-llm-routing), not "PREFIX-NNN"
    if stem.starts_with("mem-") {
        return Some(stem.to_string());
    }
    // Standard IDs: PREFIX-NNN-slug → "PREFIX-NNN"
    let parts: Vec<&str> = stem.splitn(3, '-').collect();
    if parts.len() >= 2 {
        Some(format!("{}-{}", parts[0], parts[1]))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_id_from_path_basic() {
        assert_eq!(
            extract_id_from_path(".forgeplan/prds/PRD-001-auth-system.md"),
            Some("PRD-001".to_string())
        );
    }

    #[test]
    fn extract_id_from_path_evidence() {
        assert_eq!(
            extract_id_from_path(".forgeplan/evidence/EVID-037-e2e-verification.md"),
            Some("EVID-037".to_string())
        );
    }

    #[test]
    fn extract_id_from_path_no_slug() {
        // NOTE-001.md has no slug suffix — still extracts ID correctly
        assert_eq!(
            extract_id_from_path(".forgeplan/notes/NOTE-001.md"),
            Some("NOTE-001".to_string())
        );
    }

    #[test]
    fn extract_id_basic() {
        assert_eq!(
            extract_id_from_path("prds/PRD-001-foo.md"),
            Some("PRD-001".to_string())
        );
        assert_eq!(
            extract_id_from_path("RFC-004-bar.md"),
            Some("RFC-004".to_string())
        );
    }

    #[test]
    fn extract_id_memory_slug() {
        assert_eq!(
            extract_id_from_path(".forgeplan/memory/mem-llm-routing.md"),
            Some("mem-llm-routing".to_string())
        );
        assert_eq!(
            extract_id_from_path(".forgeplan/memory/mem-api-prefix-is-v1.md"),
            Some("mem-api-prefix-is-v1".to_string())
        );
    }
}
