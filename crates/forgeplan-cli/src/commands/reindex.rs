use std::collections::BTreeMap;

use forgeplan_core::artifact::canonical_key::canonical_key_from_document;
use forgeplan_core::projection;

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
        let key = if record.canonical_key.is_empty() {
            record.display_id.clone().unwrap_or_default()
        } else {
            record.canonical_key.clone()
        };
        records.insert(key, record);
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
        common::Resolution::Found(found) if found.identity.canonical_key == canonical_key => Ok(None),
        common::Resolution::Found(_) => anyhow::bail!(
            "cannot prove canonical key '{canonical_key}' absent: it resolves as another document's alias"
        ),
        common::Resolution::NotFound => Ok(Some("no canonical document found")),
        outcome => Err(common::refuse_resolution(canonical_key, &outcome, candidates)),
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
            common::Resolution::Found(found) if found.identity.canonical_key == candidate.canonical_key => {
                resolved.push(Some(found.identity.canonical_key));
            }
            outcome => return Err(common::refuse_resolution(&candidate.canonical_key, &outcome, &candidates)),
        }
        if paths.is_some_and(|paths| !paths.iter().any(|path| path == &ws.join(&candidate.relative_path))) {
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
                    let outcome = common::Resolution::InvalidSelector { input: target.to_string() };
                    return Err(common::refuse_resolution(target, &outcome, &candidates));
                }
                resolved.push(Some(target.to_string()));
            } else {
                let outcome: common::Resolution =
                    forgeplan_core::artifact::canonical_key::resolve_structural_target(target, &candidates).into();
                match outcome {
                    common::Resolution::Found(found) => resolved.push(Some(found.identity.canonical_key)),
                    common::Resolution::NotFound => resolved.push(Some(target.to_string())),
                    outcome => return Err(common::refuse_resolution(target, &outcome, &candidates)),
                }
            }
        }
    }
    Ok(resolved)
}


async fn audit_captured_reindex(ws: &std::path::Path, guard: &forgeplan_core::workspace::lock::WorkspaceLock) -> anyhow::Result<()> {
    let previous = published_history_records(ws).await?;
    let current = forgeplan_core::constellation::facade::open_workspace_read_with_permit(ws, guard).await?;
    if !current.captured().is_complete() {
        anyhow::bail!("reindex: incomplete capture; refusing orphan decisions");
    }
    let candidates = forgeplan_core::artifact::documents::capture_candidates(&current);
    for candidate in &candidates {
        orphan_reason(&candidates, &candidate.canonical_key)?;
    }
    // Every receipt names the document it is about twice: the display id a
    // person reads, and the canonical key that names exactly one document.
    let mut changes: Vec<(String, String, &str)> = Vec::new();
    for record in current.records() {
        let (_, body) = forgeplan_core::artifact::frontmatter::parse_frontmatter(&record.body)?;
        match previous.get(&record.canonical_key) {
            Some(old) => {
                let old_body = old.body.as_deref().ok_or_else(|| {
                    anyhow::anyhow!("published artifact {} has no body", record.id)
                })?;
                if old_body.trim() != body.trim() {
                    changes.push((record.id.clone(), record.canonical_key.clone(), "update"));
                }
            }
            None => changes.push((record.id.clone(), record.canonical_key.clone(), "create")),
        }
    }
    for (key, record) in &previous {
        if orphan_reason(&candidates, key)?.is_some() {
            changes.push((
                record.display_id.clone().unwrap_or_else(|| key.clone()),
                key.clone(),
                "delete",
            ));
        }
    }
    if changes.is_empty() {
        return Ok(());
    }
    let history = forgeplan_core::db::auxiliary::AuxiliaryStore::open_history_or_create(ws).await;
    for (id, canonical_key, action) in changes {
        let entry = forgeplan_core::changelog::ChangeLogEntry::new(&id, action, "reindex")
            .with_canonical_key(&canonical_key);
        let result = match &history {
            Ok(history) => history.log_change(&entry).await,
            Err(error) => {
                eprintln!("  Warning: changelog write failed for {id}: {error}");
                continue;
            }
        };
        if let Err(error) = result {
            eprintln!("  Warning: changelog write failed for {id}: {error}");
        }
    }
    Ok(())
}

/// Rebuild LanceDB index from .md files (files-first, RFC-004).
///
/// Three-phase pipeline:
///
/// **Phase 1** — walks all artifact directories, parses frontmatter + body
/// from each .md file, and upserts into LanceDB. Safety net when lazy sync
/// missed changes. Also restores typed relations from frontmatter `links:`
/// blocks (F8 fix).
///
/// **Phase 2** — trims LanceDB artifact rows whose `.md` file no longer
/// exists on disk (files = source of truth per ADR-003). Two reasons for
/// trim: `MissingFile` (kind valid but file deleted, checks for title-change
/// rename) and `CorruptKind` (parse-kind failure — v0.17.1 fix for
/// PROB-028 Layer 1, where corrupt rows previously escaped cleanup).
///
/// **Phase 3** — trims orphan relations from `relations.lance` whose source
/// or target artifact is no longer in `artifacts.lance` (v0.17.1 fix for
/// PROB-028 Layer 2). Before this fix, deleting an artifact did not cascade
/// to its relations, causing `forgeplan tree` to show `?` phantom rows via
/// relation graph traversal (NOTE-037/038/040 dogfood bug). Iterates all
/// relations, checks both source and target against post-Phase-2 surviving
/// artifact set, deletes orphan edges with explicit reason.
pub async fn run() -> anyhow::Result<()> {
    // PROB-027 fix: use LanceStore::init() instead of open() so that
    // reindex can rebuild from scratch when lance/ dir is missing.
    // init() creates lance/ + tables if they don't exist, then opens.
    let cwd = std::env::current_dir()?;
    let ws = forgeplan_core::workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;
    let _config = forgeplan_core::workspace::load_config(&ws)?;

    // The constellation protocol claims `lance/` as soon as ANY of its
    // markers exist — not only once CURRENT.json lands. A bootstrap
    // interrupted mid-flight leaves `constellation/PROTOCOL.json` (written
    // before the new tables) or the legacy tables already renamed aside as
    // `*.legacy-backup.lance`, with CURRENT.json never published. Routing
    // on CURRENT.json alone would let `LanceStore::init` recreate
    // `artifacts.lance` there — after which `backup_legacy_tables` can
    // never finish the rename (the backup destination already exists) and
    // the resurrected table shadows the real index forever. On any of
    // these layouts "reindex from markdown" is the reconcile pass
    // `forgeplan index` owns — route there. The check is still not
    // `index_migrated`: a workspace where a pre-fix reindex already
    // resurrected artifacts.lance must still route, never pile more
    // writes into the shadowed table.
    // Reindex rebuilds the derived index from markdown — a mutation, so it
    // runs under the workspace lock for its whole duration. The lock is
    // taken BEFORE the ownership question is answered and before any store
    // is opened: deciding first and opening second is exactly the race that
    // resurrects `artifacts.lance` on a workspace another process migrated
    // while this one waited.
    //
    // The lock is released again before the reconcile pass, which acquires
    // its own per storage commit — holding it across that call would time
    // the capture out.
    let _lock = forgeplan_core::workspace::acquire_workspace_lock(&ws).await?;

    // Re-read the markers UNDER the guard. Another process may have migrated
    // this workspace while we waited for the lock, and the answer above is
    // only true for the instant it was taken.
    let lance = ws.join("lance");
    let constellation_owned = lance
        .join(forgeplan_core::constellation::storage::CURRENT_FILE_NAME)
        .exists()
        || lance
            .join(forgeplan_core::constellation::storage::PROTOCOL_RELATIVE_PATH)
            .exists()
        || std::fs::read_dir(&lance).is_ok_and(|mut entries| {
            entries.any(|e| {
                e.is_ok_and(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .ends_with(".legacy-backup.lance")
                })
            })
        });
    let reference_plan = preflight_batch_references(&ws, None).await?;
    if constellation_owned {
        audit_captured_reindex(&ws, &_lock)
            .await
            .map_err(|error| anyhow::anyhow!("reindex: Constellation reconcile failed: {error}"))?;
        drop(_lock);
        println!(
            "Index is constellation-managed — `forgeplan index` owns this layout; \
             running the reconcile pass:"
        );
        return common::reconcile_and_drain(&ws, "reindex").await;
    }
    // Shared display aliases are valid, but conflicting keys must be refused
    // before opening the writer or changing any derived rows.
    let candidates = forgeplan_core::artifact::documents::workspace_candidates(&ws).await?;
    let selectors: Vec<_> = candidates.iter()
        .map(|candidate| (candidate.canonical_key.as_str(), true))
        .collect();
    common::preflight_selectors(&ws, &selectors).await?;
    let store = forgeplan_core::db::store::LanceStore::open_for_write(&ws, &_lock).await?;
    if reference_plan != preflight_batch_references(&ws, None).await? {
        anyhow::bail!("artifact references changed during write preparation");
    }

    println!("Reindexing from .forgeplan/ markdown files...\n");

    let mut synced = 0usize;
    let mut skipped = 0usize;
    let mut errors = 0usize;


    for dir_name in forgeplan_core::workspace::ARTIFACT_DIRS {
        let dir = ws.join(dir_name);
        if !dir.exists() {
            continue;
        }
        let mut read_dir = tokio::fs::read_dir(&dir).await.map_err(|e| {
            common::describe_possible_legacy_partial_commit(true, e.into())
        })?;
        while let Some(entry) = read_dir.next_entry().await.map_err(|e| {
            common::describe_possible_legacy_partial_commit(true, e.into())
        })? {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "md") {
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

            let (fm, body) =
                match forgeplan_core::artifact::frontmatter::parse_frontmatter(&content) {
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

            let canonical = canonical_key_from_document(&id, &content);

            // Check if artifact exists in LanceDB
            match store.get_record(&_lock, &canonical).await.map_err(|e| {
                common::describe_possible_legacy_partial_commit(true, e)
            })? {
                Some(record) => {
                    // Compare body — sync if different.
                    //
                    // PROB-028 fix: pass the **file's** parsed title (not the
                    // DB-stored `record.title`) to `sync_body_from_file` so
                    // its internal path computation matches the file we just
                    // read. Pre-fix, when a user edited frontmatter `title:`
                    // on disk без syncing back to DB, the helper computed
                    // `workspace/<kind>/<id>-<DB-slug>.md`, didn't find it,
                    // returned `FileNotFound`, и `?` aborted the entire
                    // reindex — preventing Phase 2 (orphan trim) и Phase 3
                    // (orphan relations) from running. Workspace orphans
                    // would then persist forever despite the trim logic
                    // existing.
                    let file_title = fm
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&record.title);
                    if record.body.trim() != body.trim() {
                        // PROB-028 resilience: log per-file errors and
                        // continue rather than `?`-aborting the loop. Phase 2/3
                        // orphan trim depends on Phase 1 completing for ALL
                        // files even when individual files fail to sync.
                        match projection::sync_body_from_file(
                            &projection::MutationContext::new(&ws, &_lock, Some(&store)),
                            &canonical,
                            &record.kind,
                            file_title,
                            &body,
                        )
                        .await
                        {
                            Ok(()) => {
                                common::log_change(&ws, &_lock, Some(&store), &id, "update", "reindex")
                                    .await;
                                println!("  SYNC {} — body updated from file", id);
                                synced += 1;
                            }
                            Err(e) => {
                                eprintln!("  WARN {} — sync failed: {}", id, e);
                                errors += 1;
                            }
                        }
                    } else {
                        skipped += 1;
                    }
                }
                None => {
                    // Artifact in file but not in LanceDB — create it
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
                        // ADR-030: the canonical key of a document read from
                        // disk is the slug its frontmatter declares, and its
                        // display id when it declares none.
                        canonical_key: canonical.clone(),
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

                    // PROB-028 resilience: same per-file error handling as
                    // the Some(record) branch above — don't let one bad file
                    // abort the entire reindex pass.
                    match projection::sync_artifact_from_file(
                        &projection::MutationContext::new(&ws, &_lock, Some(&store)),
                        &artifact,
                    )
                    .await
                    {
                        Ok(()) => {
                            common::log_change(&ws, &_lock, Some(&store), &id, "create", "reindex").await;
                            println!("  NEW  {} — created from file", id);
                            synced += 1;
                        }
                        Err(e) => {
                            eprintln!("  WARN {} — create failed: {}", id, e);
                            errors += 1;
                        }
                    }
                }
            }

            // Restore links from frontmatter (F8 fix)
            if let Some(links_val) = fm.get("links")
                && let Some(links_arr) = links_val.as_sequence()
            {
                let existing_relations = store.get_relations(&_lock, &canonical).await
                    .map_err(|e| common::describe_possible_legacy_partial_commit(true, e))?;
                for link in links_arr {
                    let target = link.get("target").and_then(|v| v.as_str());
                    let relation = link.get("relation").and_then(|v| v.as_str());
                    if let (Some(t), Some(r)) = (target, relation) {
                        // Skip if relation already exists
                        let already_exists =
                            existing_relations.iter().any(|(et, er)| et == t && er == r);
                        if !already_exists {
                            projection::sync_relation_from_file(
                                &projection::MutationContext::new(&ws, &_lock, Some(&store)),
                                &canonical,
                                t,
                                r,
                            )
                            .await.map_err(|e| common::describe_possible_legacy_partial_commit(true, e.into()))?;
                            println!("  LINK {} --{}--> {}", id, r, t);
                            synced += 1;
                        }
                    }
                }
            }
        }
    }


    // Phase 2 removes rows only when the complete source set proves the
    // canonical key is absent. A corrupt row kind alone does not prove that.
    let mut removed = 0usize;
    let all_records = store.list_records(&_lock, None).await.map_err(|e| {
        common::describe_possible_legacy_partial_commit(true, e)
    })?;
    // Decide every orphan before deleting any row. An ambiguous selector must
    // not leave Phase 2 half-complete.
    let orphans: Vec<_> = all_records.iter().map(|record| {
        orphan_reason(&candidates, &record.canonical_key)
            .map(|reason| (record, reason))
    }).collect::<anyhow::Result<_>>()?;
    for (record, reason) in orphans {
        if let Some(reason) = reason {
            projection::delete_orphan_artifact(
                &projection::MutationContext::new(&ws, &_lock, Some(&store)),
                &record.canonical_key,
            )
            .await
            .map_err(|e| common::describe_possible_legacy_partial_commit(true, e.into()))?;
            common::log_change(&ws, &_lock, Some(&store), &record.id, "delete", "reindex").await;
            println!("  DEL  {} — {}, removed from DB", record.id, reason);
            removed += 1;
        }
    }

    // Phase 3: Trim orphan relations — edges whose source or target artifact
    // no longer exists. Before fix, orphan relations survived forever and
    // caused phantom rows in `forgeplan tree` rendering (NOTE-037/038/040 bug
    // from v0.17.0 dogfood audit). PRD-044 FR-001 extended scope.
    let mut orphan_relations = 0usize;
    let all_relations = store.get_all_relations(&_lock).await.map_err(|e| {
        common::describe_possible_legacy_partial_commit(true, e)
    })?;
    if !all_relations.is_empty() {
        // Re-fetch fresh artifact ID set after Phase 2 trims so cascade works.
        //
        // A relation names its endpoints by their canonical key (ADR-030),
        // while a row written before the identity work still holds a display
        // id. Both name a live document, so both survive the trim — only an
        // endpoint that names no document at all is an orphan. Comparing the
        // endpoints against the display ids alone would declare every edge of
        // a keyed index an orphan and delete the graph with it.
        let mut surviving_ids: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        for record in store.list_records(&_lock, None).await.map_err(|e| {
            common::describe_possible_legacy_partial_commit(true, e)
        })? {
            surviving_ids.insert(record.canonical_key);
            surviving_ids.insert(record.id);
        }
        for (source, target, relation) in &all_relations {
            let source_exists = surviving_ids.contains(source);
            // Constellation stage 4: a `KEY@qualifier` target names an
            // artifact in a *foreign* workspace — it never appears in the
            // local surviving set and is not an orphan. The edge's truth
            // is the source file's `links:` block, so the trim applies
            // only when the source itself is gone.
            let target_exists = surviving_ids.contains(target)
                || forgeplan_core::constellation::federation::parse_qualified_ref(target)
                    .1
                    .is_some();
            if !source_exists || !target_exists {
                if let Err(e) = projection::delete_orphan_relation(
                    &projection::MutationContext::new(&ws, &_lock, Some(&store)),
                    source,
                    target,
                    relation,
                )
                .await
                {
                    eprintln!(
                        "  WARN orphan relation {source} --{relation}--> {target} delete failed: {e}"
                    );
                    errors += 1;
                } else {
                    let why = match (source_exists, target_exists) {
                        (false, false) => "both source and target missing",
                        (false, true) => "source missing",
                        (true, false) => "target missing",
                        (true, true) => unreachable!(),
                    };
                    println!("  DEL  {source} --{relation}--> {target} — orphan relation ({why})");
                    orphan_relations += 1;
                }
            }
        }
    }

    drop(store);
    drop(_lock);
    common::drain_legacy_semantic_after_mutation(&ws).await?;

    println!(
        "\nReindex complete: {} synced, {} unchanged, {} removed, {} orphan relations, {} errors.",
        synced, skipped, removed, orphan_relations, errors
    );

    // PRD-071 hint contract: reindex is a maintenance op — point user to
    // health for next-action surfacing (or to scan-import for new docs).
    let mut next_hints: Vec<forgeplan_core::hints::Hint> = Vec::new();
    if errors > 0 {
        next_hints.push(
            forgeplan_core::hints::Hint::warning(format!(
                "{} parse/sync errors during reindex",
                errors
            ))
            .with_action("forgeplan health"),
        );
    } else {
        next_hints.push(
            forgeplan_core::hints::Hint::info("Reindex finished").with_action("forgeplan health"),
        );
    }
    print!(
        "{}",
        forgeplan_core::hints::render_next_action_line(&next_hints)
    );

    Ok(())
}
