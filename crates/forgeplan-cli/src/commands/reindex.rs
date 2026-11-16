use std::collections::BTreeMap;

use crate::commands::common;

/// The audit before-image is the published projection, never a second fresh
/// source capture: both captures would already contain the edited bytes.
pub(super) async fn published_history_records(
    ws: &std::path::Path,
) -> anyhow::Result<BTreeMap<String, forgeplan_core::constellation::facade::PinnedArtifact>> {
    use forgeplan_core::constellation::{facade, storage};

    let index_root = ws.join("lance");
    let Some(descriptor) = storage::read_current(&index_root)? else {
        if matches!(
            storage::read_marker(&index_root)?,
            storage::IndexProtocolState::Active(_)
        ) {
            anyhow::bail!("active constellation index has no published pointer");
        }
        // An interrupted first bootstrap has no published rows yet.
        return Ok(BTreeMap::new());
    };
    let mut records = BTreeMap::new();
    // One native scan at this exact descriptor/version, not N pinned gets
    // whose implementations each materialize the entire artifacts table.
    for record in facade::read_pinned_artifacts(&index_root, &descriptor).await? {
        // The map is keyed by the canonical key — the identity. A display id
        // is an alias, and two documents may share one: a map keyed by the
        // alias would let the second of them hide the first, and the audit
        // would report a change that never happened.
        if record.canonical_key.is_empty() {
            anyhow::bail!(
                "published artifact has no canonical key; refusing an unkeyed before-image"
            );
        }
        let key = record.canonical_key.clone();
        if records.insert(key.clone(), record).is_some() {
            anyhow::bail!("published index repeats canonical key '{key}'");
        }
    }
    if storage::read_current(&index_root)?.as_ref() != Some(&descriptor) {
        anyhow::bail!("published generation changed while reading history before-image");
    }
    Ok(records)
}

/// Decide absence from one complete, parsed document set, never from filenames.
pub(super) fn orphan_reason(
    candidates: &[forgeplan_core::artifact::canonical_key::ArtifactCandidate],
    canonical_key: &str,
) -> anyhow::Result<Option<&'static str>> {
    match common::resolve_in_document_set(canonical_key, candidates) {
        common::Resolution::Found(found) if found.identity.canonical_key == canonical_key => {
            Ok(None)
        }
        common::Resolution::Found(_) => anyhow::bail!(
            "cannot prove canonical key '{canonical_key}' absent: it resolves as another document's alias"
        ),
        common::Resolution::NotFound => Ok(Some("no canonical document found")),
        outcome => Err(common::refuse_resolution(
            canonical_key,
            &outcome,
            candidates,
        )),
    }
}
/// Resolve all local structural references against the complete source set,
/// including documents which have not yet been inserted into the index.
pub(super) async fn preflight_batch_references(
    ws: &std::path::Path,
    paths: Option<&[std::path::PathBuf]>,
) -> anyhow::Result<Vec<Option<String>>> {
    let candidates = forgeplan_core::artifact::documents::workspace_candidates(ws).await?;
    let mut resolved = Vec::new();
    for candidate in &candidates {
        match common::resolve_in_document_set(&candidate.canonical_key, &candidates) {
            common::Resolution::Found(found)
                if found.identity.canonical_key == candidate.canonical_key =>
            {
                resolved.push(Some(found.identity.canonical_key));
            }
            outcome => {
                return Err(common::refuse_resolution(
                    &candidate.canonical_key,
                    &outcome,
                    &candidates,
                ));
            }
        }
        if paths.is_some_and(|paths| {
            !paths
                .iter()
                .any(|path| path == &ws.join(&candidate.relative_path))
        }) {
            continue;
        }
        let content = tokio::fs::read_to_string(ws.join(&candidate.relative_path)).await?;
        let (fm, _) = forgeplan_core::artifact::frontmatter::parse_frontmatter(&content)?;
        let mut targets = Vec::new();
        if let Some(parent) = fm.get("parent_epic").and_then(|v| v.as_str()) {
            targets.push(parent);
        }
        if let Some(links) = fm.get("links").and_then(|v| v.as_sequence()) {
            for link in links {
                if let Some(target) = link.get("target").and_then(|v| v.as_str()) {
                    targets.push(target);
                }
            }
        }
        for target in targets {
            if target.contains('@') {
                if forgeplan_core::db::store::validate_link_target(target).is_err() {
                    let outcome = common::Resolution::InvalidSelector {
                        input: target.to_string(),
                    };
                    return Err(common::refuse_resolution(target, &outcome, &candidates));
                }
                resolved.push(Some(target.to_string()));
            } else {
                let outcome: common::Resolution =
                    forgeplan_core::artifact::canonical_key::resolve_structural_target(
                        target,
                        &candidates,
                    )
                    .into();
                match outcome {
                    common::Resolution::Found(found) => {
                        resolved.push(Some(found.identity.canonical_key))
                    }
                    common::Resolution::NotFound => resolved.push(Some(target.to_string())),
                    outcome => {
                        return Err(common::refuse_resolution(target, &outcome, &candidates));
                    }
                }
            }
        }
    }
    Ok(resolved)
}

async fn audit_captured_reindex(
    ws: &std::path::Path,
    guard: &forgeplan_core::workspace::lock::WorkspaceLock,
) -> anyhow::Result<BTreeMap<String, forgeplan_core::constellation::facade::PinnedArtifact>> {
    let previous = published_history_records(ws).await?;
    let current =
        forgeplan_core::constellation::facade::open_workspace_read_with_permit(ws, guard).await?;
    if !current.captured().is_complete() {
        anyhow::bail!("reindex: incomplete capture; refusing orphan decisions");
    }
    let candidates = forgeplan_core::artifact::documents::capture_candidates(&current);
    for candidate in &candidates {
        orphan_reason(&candidates, &candidate.canonical_key)?;
    }
    Ok(previous)
}

fn published_history_changes(
    previous: &BTreeMap<String, forgeplan_core::constellation::facade::PinnedArtifact>,
    current: &BTreeMap<String, forgeplan_core::constellation::facade::PinnedArtifact>,
) -> anyhow::Result<Vec<(String, String, &'static str)>> {
    let mut changes = Vec::new();
    for (key, record) in current {
        let id = record
            .display_id
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("published artifact {key} has no display id"))?;
        if record.body.is_none() {
            anyhow::bail!("published artifact {id} has no prose body");
        }
        match previous.get(key) {
            None => changes.push((id.to_string(), key.clone(), "create")),
            Some(old) => {
                let old_id = old
                    .display_id
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("published artifact {key} has no display id"))?;
                let old_body = old.body.as_deref().ok_or_else(|| {
                    anyhow::anyhow!("published artifact {old_id} has no prose body")
                })?;
                if old_id != id
                    || old.title != record.title
                    || old.kind != record.kind
                    || old.status != record.status
                    || old_body.trim() != record.body.as_deref().unwrap_or_default().trim()
                {
                    changes.push((id.to_string(), key.clone(), "update"));
                }
            }
        }
    }
    for (key, record) in previous {
        if !current.contains_key(key) {
            let id = record
                .display_id
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("published artifact {key} has no display id"))?;
            changes.push((id.to_string(), key.clone(), "delete"));
        }
    }
    Ok(changes)
}

/// Reconcile the complete canonical Markdown set using its current index protocol.
///
/// Incomplete or ambiguous source sets cannot authorize deletion. Constellation
/// publishes through its owner; identity-v5 uses its fixed-table transaction.
/// Canonical-key history is appended only after successful publication.
pub async fn run() -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let ws = forgeplan_core::workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;
    let _config = forgeplan_core::workspace::load_config(&ws)?;

    // Decide ownership under the workspace lock. The shared probe includes
    // interrupted bootstrap, damaged markers, managed datasets and backups;
    // none of those layouts may be reopened through the legacy writer.
    // Release this guard before reconciliation acquires its publication lock.
    let initial_lock = forgeplan_core::workspace::acquire_workspace_lock(&ws).await?;
    let constellation_owned = common::index_migrated(&ws);
    if constellation_owned {
        drop(initial_lock);
        let _audit_serial = common::acquire_managed_maintenance_audit_lock(&ws).await?;
        let guard = forgeplan_core::workspace::acquire_workspace_lock(&ws).await?;
        if !common::index_migrated(&ws) {
            anyhow::bail!("workspace ownership changed during managed reindex setup");
        }
        preflight_batch_references(&ws, None).await?;
        let previous = audit_captured_reindex(&ws, &guard)
            .await
            .map_err(|error| anyhow::anyhow!("reindex: Constellation reconcile failed: {error}"))?;
        drop(guard);
        println!(
            "Index is constellation-managed — `forgeplan index` owns this layout; \
             running the reconcile pass:"
        );
        common::reconcile_and_drain(&ws, "reindex").await?;
        let guard = forgeplan_core::workspace::acquire_workspace_lock(&ws).await?;
        let current = published_history_records(&ws).await?;
        for (id, canonical_key, action) in published_history_changes(&previous, &current)? {
            common::log_change(&ws, &guard, None, &canonical_key, &id, action, "reindex").await;
        }
        return Ok(());
    }
    let _lock = initial_lock;
    let reference_plan = preflight_batch_references(&ws, None).await?;
    // Shared display aliases are valid, but conflicting keys must be refused
    // before opening the writer or changing any derived rows.
    let candidates = forgeplan_core::artifact::documents::workspace_candidates(&ws).await?;
    let selectors: Vec<_> = candidates
        .iter()
        .map(|candidate| (candidate.canonical_key.as_str(), true))
        .collect();
    common::preflight_selectors(&ws, &selectors).await?;
    let store = forgeplan_core::db::store::LanceStore::open_for_write(&ws, &_lock).await?;
    if reference_plan != preflight_batch_references(&ws, None).await? {
        anyhow::bail!("artifact references changed during write preparation");
    }
    // open_for_write admits only a verified identity-v5 writer. Do not retain
    // an unreachable legacy row-by-row mutation path beside its transaction.
    drop(store);
    drop(_lock);
    let changes = forgeplan_core::db::store::LanceStore::reconcile_identity_v5_source(&ws).await?;
    let guard = forgeplan_core::workspace::acquire_workspace_lock(&ws).await?;
    let store = forgeplan_core::db::store::LanceStore::open_for_write(&ws, &guard).await?;
    for change in changes {
        common::log_change(
            &ws,
            &guard,
            Some(&store),
            &change.canonical_key,
            &change.display_id,
            change.action,
            "reindex",
        )
        .await;
    }
    drop(store);
    drop(guard);
    forgeplan_core::db::store::LanceStore::drain_identity_semantic_backlog(&ws).await?;
    println!("Identity-v5 index reconciled from canonical Markdown.");

    Ok(())
}
