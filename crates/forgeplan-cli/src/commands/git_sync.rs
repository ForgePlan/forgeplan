use std::process::Command;

use forgeplan_core::artifact::{
    canonical_key::canonical_key_from_document, documents::workspace_candidates, frontmatter,
};
use forgeplan_core::db::{
    auxiliary::AuxiliaryStore,
    store::{ArtifactRecord, LanceStore},
};
use forgeplan_core::git;
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::workspace::lock::WorkspaceLock;

use crate::commands::common;

struct GitDeletionPreimage {
    canonical_key: String,
    display_id: String,
    needs_delete: bool,
}

struct GitArtifactBeforeImage {
    canonical_key: String,
    display_id: String,
    kind: String,
    status: String,
    title: String,
    body: String,
    depth: String,
    author: Option<String>,
    parent_epic: Option<String>,
    valid_until: Option<String>,
    tags: Vec<String>,
}

fn resolve_git_commit(repo_root: &std::path::Path, reference: &str) -> anyhow::Result<String> {
    if reference.is_empty()
        || reference.starts_with('-')
        || reference.contains("..")
        || reference.bytes().any(|byte| byte.is_ascii_control())
    {
        anyhow::bail!("invalid Git revision");
    }
    let expression = format!("{reference}^{{commit}}");
    let output = Command::new("git")
        .args(["rev-parse", "--verify", "--end-of-options", &expression])
        .current_dir(repo_root)
        .output()?;
    if !output.status.success() {
        anyhow::bail!("cannot resolve Git before-image revision");
    }
    let mut lines = std::str::from_utf8(&output.stdout)?.lines();
    let commit = lines
        .next()
        .ok_or_else(|| anyhow::anyhow!("Git returned no commit object id"))?;
    if lines.next().is_some()
        || !matches!(commit.len(), 40 | 64)
        || !commit.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        anyhow::bail!("Git returned an invalid commit object id");
    }
    Ok(commit.to_owned())
}

fn read_git_before_image(
    repo_root: &std::path::Path,
    commit: &str,
    relative_path: &str,
) -> anyhow::Result<String> {
    let object = format!("{commit}:{relative_path}");
    let output = Command::new("git")
        .args(["cat-file", "blob", &object])
        .current_dir(repo_root)
        .output()?;
    if !output.status.success() {
        anyhow::bail!("cannot read Git before-image for {relative_path}");
    }
    String::from_utf8(output.stdout).map_err(|error| {
        anyhow::anyhow!("Git before-image for {relative_path} is not UTF-8: {error}")
    })
}
fn changed_files_between(
    repo_root: &std::path::Path,
    before_oid: &str,
    head_oid: &str,
) -> anyhow::Result<Vec<git::GitChangedFile>> {
    let output = Command::new("git")
        .args([
            "diff",
            "--name-status",
            "-z",
            "--no-renames",
            before_oid,
            head_oid,
            "--",
            ".forgeplan/",
        ])
        .current_dir(repo_root)
        .output()?;
    if !output.status.success() {
        anyhow::bail!(
            "git diff failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    if !output.stdout.is_empty() && output.stdout.last() != Some(&0) {
        anyhow::bail!("git diff output is truncated");
    }
    let mut tokens = output.stdout.split(|byte| *byte == 0);
    let mut files = Vec::new();
    while let Some(status) = tokens.next() {
        if status.is_empty() {
            break;
        }
        let path = std::str::from_utf8(
            tokens
                .next()
                .ok_or_else(|| anyhow::anyhow!("git diff path is missing"))?,
        )?;
        let relative = std::path::Path::new(path);
        if !path.starts_with(".forgeplan/")
            || relative.is_absolute()
            || relative
                .components()
                .any(|part| !matches!(part, std::path::Component::Normal(_)))
        {
            anyhow::bail!("git diff contains an invalid artifact path");
        }
        if path.ends_with(".md") {
            let status = match status {
                b"A" => 'A',
                b"M" => 'M',
                b"D" => 'D',
                _ => anyhow::bail!("git diff contains an unsupported artifact change status"),
            };
            files.push(git::GitChangedFile {
                path: path.to_owned(),
                status,
            });
        }
    }
    Ok(files)
}

fn git_artifact_before_image(path: &str, content: &str) -> anyhow::Result<GitArtifactBeforeImage> {
    let (frontmatter, body) = frontmatter::parse_frontmatter(content)?;
    let display_id = frontmatter
        .get("id")
        .and_then(|value| value.as_str())
        .ok_or_else(|| anyhow::anyhow!("Git before-image at {path} has no display id"))?
        .to_owned();
    let canonical_key = canonical_key_from_document(&display_id, content);
    let filename = path.rsplit('/').next().unwrap_or(path);
    let default_title = filename.strip_suffix(".md").unwrap_or(filename).to_owned();
    let kind = frontmatter
        .get("kind")
        .and_then(|value| value.as_str())
        .map(str::to_owned)
        .unwrap_or_else(|| {
            match path.split('/').nth(1).unwrap_or_default() {
                "prds" => "prd",
                "epics" => "epic",
                "specs" => "spec",
                "rfcs" => "rfc",
                "adrs" => "adr",
                "problems" => "problem",
                "solutions" => "solution",
                "evidence" => "evidence",
                "notes" => "note",
                "refresh" => "refresh",
                "memory" => "memory",
                "use_cases" => "use_case",
                "glossary" => "glossary",
                "invariants" => "invariant",
                "scenarios" => "scenario",
                "hypotheses" => "hypothesis",
                "domain_models" => "domain_model",
                _ => "note",
            }
            .to_owned()
        });
    let field = |name: &str| {
        frontmatter
            .get(name)
            .and_then(|value| value.as_str())
            .map(str::to_owned)
    };
    Ok(GitArtifactBeforeImage {
        canonical_key,
        display_id,
        kind,
        status: field("status")
            .unwrap_or_else(|| "draft".to_owned())
            .to_lowercase(),
        title: field("title").unwrap_or(default_title),
        body,
        depth: field("depth").unwrap_or_else(|| "standard".to_owned()),
        author: frontmatter::author_from_frontmatter(&frontmatter),
        parent_epic: field("parent_epic"),
        valid_until: field("valid_until"),
        tags: frontmatter::tags_from_frontmatter(&frontmatter),
    })
}

fn normalized_parent_epic(
    parent_epic: Option<&str>,
    candidates: &[forgeplan_core::artifact::canonical_key::ArtifactCandidate],
) -> anyhow::Result<Option<String>> {
    parent_epic
        .map(|parent| {
            if parent.contains('@') {
                return Ok(parent.to_owned());
            }
            match common::resolve_in_document_set(parent, candidates) {
                common::Resolution::Found(found) => Ok(found.identity.canonical_key),
                common::Resolution::NotFound => Ok(parent.to_owned()),
                outcome => Err(common::refuse_resolution(parent, &outcome, candidates)),
            }
        })
        .transpose()
}

fn verify_identity_v5_before_image(
    record: &ArtifactRecord,
    before: &GitArtifactBeforeImage,
    full_text: &str,
    candidates: &[forgeplan_core::artifact::canonical_key::ArtifactCandidate],
) -> anyhow::Result<()> {
    let parent_epic = normalized_parent_epic(before.parent_epic.as_deref(), candidates)?;
    if record.canonical_key != before.canonical_key
        || record.id != before.display_id
        || record.kind != before.kind
        || record.status != before.status
        || record.title != before.title
        || (record.body != before.body && record.body != full_text)
        || record.depth != before.depth
        || record.author != before.author
        || record.parent_epic != parent_epic
        || record.valid_until != before.valid_until
        || record.tags != before.tags
    {
        anyhow::bail!(
            "Git before-image for '{}' does not match the identity-v5 indexed before-image",
            before.canonical_key
        );
    }
    Ok(())
}

async fn verify_git_deletion_preimages(
    ws: &std::path::Path,
    repo_root: &std::path::Path,
    before_oid: &str,
    changed: &[git::GitChangedFile],
    captured_layout: bool,
    guard: &WorkspaceLock,
) -> anyhow::Result<std::collections::BTreeMap<String, GitDeletionPreimage>> {
    let deleted = changed
        .iter()
        .filter(|file| file.status == 'D')
        .collect::<Vec<_>>();
    if deleted.is_empty() {
        return Ok(std::collections::BTreeMap::new());
    }

    let candidates = workspace_candidates(ws).await?;
    let mut published = None;
    let mut legacy: Option<LanceStore> = None;
    let mut preimages = std::collections::BTreeMap::new();
    for file in deleted {
        let content = read_git_before_image(repo_root, before_oid, &file.path)?;
        let source_digest =
            forgeplan_core::artifact::identity_repair::sha256_hex(content.as_bytes());
        let before = git_artifact_before_image(&file.path, &content)?;
        let source_present =
            match common::resolve_in_document_set(&before.canonical_key, &candidates) {
                common::Resolution::Found(found)
                    if found.identity.canonical_key == before.canonical_key =>
                {
                    true
                }
                common::Resolution::NotFound => false,
                outcome => {
                    return Err(common::refuse_resolution(
                        &before.canonical_key,
                        &outcome,
                        &candidates,
                    ));
                }
            };
        let mut needs_delete = false;
        if !source_present {
            if captured_layout {
                if published.is_none() {
                    published = Some(super::reindex::published_history_records(ws).await?);
                }
                if let Some(published_record) = published
                    .as_ref()
                    .and_then(|records| records.get(&before.canonical_key))
                {
                    if published_record.display_id.as_deref() != Some(before.display_id.as_str())
                        || published_record.title != before.title
                        || published_record.kind.as_deref() != Some(before.kind.as_str())
                        || published_record.status.as_deref() != Some(before.status.as_str())
                        || published_record.body.as_deref() != Some(before.body.as_str())
                        || published_record.source_path.as_deref()
                            != file.path.strip_prefix(".forgeplan/")
                        || published_record.source_digest.as_deref() != Some(source_digest.as_str())
                    {
                        anyhow::bail!(
                            "Git before-image for '{}' does not match the published before-image",
                            before.canonical_key
                        );
                    }
                    needs_delete = true;
                }
            } else {
                if legacy.is_none() {
                    let store = LanceStore::open(ws).await?;
                    if !store.uses_identity_v5() {
                        anyhow::bail!(
                            "git-sync cannot verify a deletion against an index without canonical-key before-images"
                        );
                    }
                    legacy = Some(store);
                }
                if let Some(record) = legacy
                    .as_ref()
                    .expect("identity-v5 store was opened")
                    .get_record(guard, &before.canonical_key)
                    .await?
                {
                    verify_identity_v5_before_image(&record, &before, &content, &candidates)?;
                    needs_delete = true;
                }
            }
        }
        if preimages
            .values()
            .any(|preimage: &GitDeletionPreimage| preimage.canonical_key == before.canonical_key)
        {
            anyhow::bail!(
                "Git diff contains multiple deleted paths for canonical key '{}'",
                before.canonical_key
            );
        }
        if preimages
            .insert(
                file.path.clone(),
                GitDeletionPreimage {
                    canonical_key: before.canonical_key,
                    display_id: before.display_id,
                    needs_delete,
                },
            )
            .is_some()
        {
            anyhow::bail!("Git diff repeats a deleted source path");
        }
    }
    Ok(preimages)
}

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
        common::reconcile_and_drain(ws, "git-sync").await
    } else {
        Ok(())
    }
}

/// `forgeplan git-sync [--since <ref>]` — sync artifact changes from git operations.
///
/// Detects .forgeplan/ files changed since a git ref (default: ORIG_HEAD from last pull/merge)
/// and syncs them into LanceDB with source=git_sync and commit_hash.
pub async fn run(since: Option<&str>) -> anyhow::Result<()> {
    let (ws, initial_guard) = common::open_workspace_locked().await?;
    let captured_layout = common::index_migrated(&ws);
    let (guard, _audit_serial) = if captured_layout {
        drop(initial_guard);
        let audit_serial = common::acquire_managed_maintenance_audit_lock(&ws).await?;
        let (locked_ws, guard) = common::open_workspace_locked().await?;
        if locked_ws != ws || !common::index_migrated(&ws) {
            anyhow::bail!("workspace ownership changed during managed Git sync setup");
        }
        (guard, Some(audit_serial))
    } else {
        (initial_guard, None)
    };
    let repo_root = ws
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine repo root"))?;
    let since_ref = match since {
        Some(reference) => reference.to_owned(),
        None => git::orig_head(repo_root).ok_or_else(|| {
            anyhow::anyhow!("No ORIG_HEAD found. Use forgeplan git-sync --since <ref>.")
        })?,
    };
    let before_oid = resolve_git_commit(repo_root, &since_ref)?;
    let head_oid = resolve_git_commit(repo_root, "HEAD")?;
    let commit_hash = head_oid[..7].to_owned();
    let changed = changed_files_between(repo_root, &before_oid, &head_oid)?;
    if changed.is_empty() {
        drop(guard);
        reconcile_if_constellation_owned(&ws).await?;
        println!("No artifact changes since {since_ref}.");
        return Ok(());
    }
    for file in changed
        .iter()
        .filter(|file| matches!(file.status, 'A' | 'M'))
    {
        let pinned = read_git_before_image(repo_root, &head_oid, &file.path)?;
        let current = tokio::fs::read_to_string(repo_root.join(&file.path)).await?;
        if current != pinned {
            anyhow::bail!(
                "Git artifact '{}' differs from the pinned HEAD blob",
                file.path
            );
        }
        if file.status == 'M' {
            let old = read_git_before_image(repo_root, &before_oid, &file.path)?;
            let old_key = git_artifact_before_image(&file.path, &old)?.canonical_key;
            let new_key = git_artifact_before_image(&file.path, &pinned)?.canonical_key;
            if old_key != new_key {
                anyhow::bail!(
                    "Git modified '{}' from canonical key '{}' to '{}'; key replacement is not supported",
                    file.path,
                    old_key,
                    new_key
                );
            }
        }
    }
    let deletion_preimages = verify_git_deletion_preimages(
        &ws,
        repo_root,
        &before_oid,
        &changed,
        captured_layout,
        &guard,
    )
    .await?;
    let changed_paths: Vec<_> = changed
        .iter()
        .filter(|file| matches!(file.status, 'A' | 'M'))
        .map(|file| repo_root.join(&file.path))
        .collect();
    let reference_plan =
        super::reindex::preflight_batch_references(&ws, Some(&changed_paths)).await?;
    let mut selector_names = Vec::with_capacity(changed_paths.len());
    for path in &changed_paths {
        let content = tokio::fs::read_to_string(path).await?;
        let (fm, _) = frontmatter::parse_frontmatter(&content)?;
        let id = fm
            .get("id")
            .and_then(|value| value.as_str())
            .ok_or_else(|| anyhow::anyhow!("Git artifact {} has no display id", path.display()))?;
        selector_names.push(canonical_key_from_document(id, &content));
    }
    let selectors: Vec<_> = selector_names
        .iter()
        .map(|key| (key.as_str(), false))
        .collect();
    let before = common::preflight_selectors(&ws, &selectors).await?;
    let store = common::prepare_store_locked_checked(&ws, &guard, &[]).await?;
    if before != common::preflight_selectors(&ws, &selectors).await?
        || reference_plan
            != super::reindex::preflight_batch_references(&ws, Some(&changed_paths)).await?
    {
        anyhow::bail!("artifact identity or references changed during Git preparation");
    }

    let (synced, deleted) = if !captured_layout {
        let store = store.ok_or_else(|| {
            anyhow::anyhow!("workspace index owner changed during Git preparation")
        })?;
        // Historical rows may contain a complete canonical document. Normalize
        // only when that header proves the same identity, never arbitrary prose.
        let audit_body = |record: ArtifactRecord| {
            let mut body = record.body;
            if body.starts_with("---")
                && let Ok((fm, prose)) = frontmatter::parse_frontmatter(&body)
                && fm.get("id").and_then(|value| value.as_str()) == Some(record.id.as_str())
                && canonical_key_from_document(&record.id, &body) == record.canonical_key
            {
                body = prose;
            }
            (record.canonical_key, body)
        };
        let old_bodies = store
            .list_records(&guard, None)
            .await?
            .into_iter()
            .map(audit_body)
            .collect::<std::collections::BTreeMap<_, _>>();
        let changed_keys: std::collections::HashSet<_> = selector_names
            .iter()
            .map(String::as_str)
            .chain(
                deletion_preimages
                    .values()
                    .filter(|item| item.needs_delete)
                    .map(|item| item.canonical_key.as_str()),
            )
            .collect();
        drop(store);
        drop(guard);
        // One complete transaction handles A/M/D together. Row-by-row writes
        // cannot rebuild safely while another deleted row remains in the batch.
        let changes = LanceStore::reconcile_identity_v5_source(&ws).await?;
        let (audit_ws, audit_guard, audit_store) = common::open_store_locked().await?;
        if audit_ws != ws {
            anyhow::bail!("workspace changed before Git audit");
        }
        let new_bodies = audit_store
            .list_records(&audit_guard, None)
            .await?
            .into_iter()
            .map(audit_body)
            .collect::<std::collections::BTreeMap<_, _>>();
        let mut history = None;
        let mut synced = 0usize;
        let mut deleted = 0usize;
        for change in changes {
            if !changed_keys.contains(change.canonical_key.as_str()) {
                continue;
            }
            if change.action == "update" {
                let old = old_bodies
                    .get(&change.canonical_key)
                    .ok_or_else(|| anyhow::anyhow!("Git update has no indexed before-image"))?;
                let new = new_bodies
                    .get(&change.canonical_key)
                    .ok_or_else(|| anyhow::anyhow!("Git update has no published after-image"))?;
                if old.trim() == new.trim() {
                    continue; // Metadata-only changes do not invent body audit events.
                }
            }
            let entry = forgeplan_core::changelog::ChangeLogEntry::new(
                &change.display_id,
                change.action,
                "git_sync",
            )
            .with_canonical_key(&change.canonical_key)
            .with_commit(&commit_hash);
            log_git_change(&ws, &audit_guard, Some(&audit_store), &mut history, &entry).await;
            if change.action == "delete" {
                deleted += 1;
            } else {
                synced += 1;
            }
        }
        drop(history);
        drop(audit_store);
        drop(audit_guard);
        common::drain_legacy_semantic_after_mutation(&ws).await?;
        (synced, deleted)
    } else {
        drop(store);
        let published = super::reindex::published_history_records(&ws).await?;
        let current =
            forgeplan_core::constellation::facade::open_workspace_read_with_permit(&ws, &guard)
                .await?;
        if !current.captured().is_complete() {
            anyhow::bail!("git-sync requires a complete source capture");
        }
        let mut planned: Vec<(
            String,
            Option<forgeplan_core::changelog::ChangeLogEntry>,
            Option<String>,
        )> = Vec::new();
        for file in &changed {
            if !matches!(file.status, 'A' | 'M') {
                continue;
            }
            let content = read_git_before_image(repo_root, &head_oid, &file.path)?;
            let document = git_artifact_before_image(&file.path, &content)?;
            if current.get_record_by_key(&document.canonical_key).is_none() {
                anyhow::bail!("Git artifact is absent from the confirmed source capture");
            }
            let entry = match published.get(&document.canonical_key) {
                None => Some("create"),
                Some(old) => {
                    let old_body = old
                        .body
                        .as_deref()
                        .ok_or_else(|| anyhow::anyhow!("published Git artifact has no body"))?;
                    (old_body.trim() != document.body.trim()).then_some("update")
                }
            }
            .map(|action| {
                forgeplan_core::changelog::ChangeLogEntry::new(
                    &document.display_id,
                    action,
                    "git_sync",
                )
                .with_canonical_key(&document.canonical_key)
                .with_commit(&commit_hash)
            });
            let digest = forgeplan_core::artifact::identity_repair::sha256_hex(content.as_bytes());
            planned.push((document.canonical_key, entry, Some(digest)));
        }
        for deletion in deletion_preimages.values().filter(|item| item.needs_delete) {
            let entry = forgeplan_core::changelog::ChangeLogEntry::new(
                &deletion.display_id,
                "delete",
                "git_sync",
            )
            .with_canonical_key(&deletion.canonical_key)
            .with_commit(&commit_hash);
            planned.push((deletion.canonical_key.clone(), Some(entry), None));
        }
        drop(current);
        drop(guard);
        reconcile_if_constellation_owned(&ws).await?;
        let (audit_ws, audit_guard) = common::open_workspace_locked().await?;
        if audit_ws != ws {
            anyhow::bail!("workspace changed before Git audit");
        }
        let after = super::reindex::published_history_records(&ws).await?;
        // Verify the entire batch before appending any success event.
        for (key, _, expected_digest) in &planned {
            match (expected_digest, after.get(key)) {
                (None, None) => {}
                (Some(expected), Some(record))
                    if record.source_digest.as_deref() == Some(expected.as_str()) => {}
                _ => anyhow::bail!(
                    "Git reconciliation published a different source; no audit appended"
                ),
            }
        }
        let mut history = None;
        let mut synced = 0usize;
        let mut deleted = 0usize;
        for (_, entry, _) in planned {
            let Some(entry) = entry else {
                continue;
            };
            log_git_change(&ws, &audit_guard, None, &mut history, &entry).await;
            if entry.action == "delete" {
                deleted += 1;
            } else {
                synced += 1;
            }
        }
        (synced, deleted)
    };
    println!("Git sync complete: {synced} synced, {deleted} deleted (commit {commit_hash}).");
    let next =
        [Hint::suggestion("Audit synced artifacts").with_action("forgeplan health".to_string())];
    print!("{}", hints::render_next_action_line(&next));
    Ok(())
}

/// Preserve Git's already-built entry, including its commit hash, while
/// selecting exactly one sink. The caller owns the workspace lock.
async fn log_git_change(
    ws: &std::path::Path,
    guard: &forgeplan_core::workspace::lock::WorkspaceLock,
    store: Option<&LanceStore>,
    history: &mut Option<AuxiliaryStore>,
    entry: &forgeplan_core::changelog::ChangeLogEntry,
) {
    let result = match store {
        Some(store) => store.log_change(guard, entry).await,
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
