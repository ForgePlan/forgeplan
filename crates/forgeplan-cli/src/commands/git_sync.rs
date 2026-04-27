use forgeplan_core::artifact::frontmatter;
use forgeplan_core::git;
use forgeplan_core::hints::{self, Hint};

use crate::commands::common;

/// `forgeplan git-sync [--since <ref>]` — sync artifact changes from git operations.
///
/// Detects .forgeplan/ files changed since a git ref (default: ORIG_HEAD from last pull/merge)
/// and syncs them into LanceDB with source=git_sync and commit_hash.
pub async fn run(since: Option<&str>) -> anyhow::Result<()> {
    let (ws, store) = common::open_store().await?;

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
        // PRD-071 contract: nothing to sync — terminal state.
        println!("\nDone.");
        return Ok(());
    }

    let mut synced = 0usize;
    let mut deleted = 0usize;
    let mut errors = 0usize;

    for file in &changed {
        let full_path = repo_root.join(&file.path);

        match file.status {
            'D' => {
                // File deleted in git — extract ID from filename
                if let Some(id) = extract_id_from_path(&file.path)
                    && store.get_record(&id).await?.is_some()
                {
                    store.delete_artifact(&id).await?;
                    let entry =
                        forgeplan_core::changelog::ChangeLogEntry::new(&id, "delete", "git_sync")
                            .with_commit(&commit_hash);
                    if let Err(e) = store.log_change(&entry).await {
                        eprintln!("  Warning: changelog write failed for {}: {}", id, e);
                    }
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

                let action = match store.get_record(&id).await? {
                    Some(record) => {
                        // Existing artifact — update body if changed
                        if record.body.trim() != body.trim() {
                            store.update_body(&id, &body).await?;
                            // Also sync frontmatter fields
                            if let Some(status) = fm.get("status").and_then(|v| v.as_str()) {
                                let status_lower = status.to_lowercase();
                                if record.status != status_lower {
                                    store
                                        .update_artifact(&id, Some(&status_lower), None)
                                        .await?;
                                }
                            }
                            "update"
                        } else {
                            continue; // No changes
                        }
                    }
                    None => {
                        // New artifact from git
                        let kind = fm.get("kind").and_then(|v| v.as_str()).unwrap_or("note");
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
                            tags: forgeplan_core::artifact::frontmatter::tags_from_frontmatter(&fm),
                        };

                        store.create_artifact(&artifact).await?;
                        "create"
                    }
                };

                // Restore links from frontmatter
                if let Some(links_val) = fm.get("links")
                    && let Some(links_arr) = links_val.as_sequence()
                {
                    let existing = store.get_relations(&id).await.unwrap_or_default();
                    for link in links_arr {
                        let target = link.get("target").and_then(|v| v.as_str());
                        let relation = link.get("relation").and_then(|v| v.as_str());
                        if let (Some(t), Some(r)) = (target, relation)
                            && !existing.iter().any(|(et, er)| et == t && er == r)
                        {
                            let _ = store.add_relation(&id, t, r).await;
                        }
                    }
                }

                let entry = forgeplan_core::changelog::ChangeLogEntry::new(&id, action, "git_sync")
                    .with_commit(&commit_hash);
                if let Err(e) = store.log_change(&entry).await {
                    eprintln!("  Warning: changelog write failed for {}: {}", id, e);
                }

                let status_char = if action == "create" { "NEW " } else { "SYNC" };
                println!(
                    "  {} {} (from git, commit {})",
                    status_char, id, commit_hash
                );
                synced += 1;
            }
            _ => {
                // R (rename), C (copy), etc. — skip silently.
                // Renames produce two-path output that our parser doesn't handle.
                // The renamed file will be picked up on next reindex.
                continue;
            }
        }
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
