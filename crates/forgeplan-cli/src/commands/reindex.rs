use forgeplan_core::projection;

use crate::commands::common;

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
    let store = forgeplan_core::db::store::LanceStore::init(&ws).await?;

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

            // Check if artifact exists in LanceDB
            match store.get_record(&id).await? {
                Some(record) => {
                    // Compare body — sync if different
                    if record.body.trim() != body.trim() {
                        projection::sync_body_from_file(
                            &ws,
                            &store,
                            &id,
                            &record.kind,
                            &record.title,
                            &body,
                        )
                        .await?;
                        common::log_change(&store, &id, "update", "reindex").await;
                        println!("  SYNC {} — body updated from file", id);
                        synced += 1;
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

                    projection::sync_artifact_from_file(&ws, &store, &artifact).await?;
                    common::log_change(&store, &id, "create", "reindex").await;
                    println!("  NEW  {} — created from file", id);
                    synced += 1;
                }
            }

            // Restore links from frontmatter (F8 fix)
            if let Some(links_val) = fm.get("links")
                && let Some(links_arr) = links_val.as_sequence()
            {
                let existing_relations = store.get_relations(&id).await.unwrap_or_default();
                for link in links_arr {
                    let target = link.get("target").and_then(|v| v.as_str());
                    let relation = link.get("relation").and_then(|v| v.as_str());
                    if let (Some(t), Some(r)) = (target, relation) {
                        // Skip if relation already exists
                        let already_exists =
                            existing_relations.iter().any(|(et, er)| et == t && er == r);
                        if !already_exists {
                            if let Err(e) =
                                projection::sync_relation_from_file(&store, &id, t, r).await
                            {
                                eprintln!("  WARN {} — link to {} failed: {}", id, t, e);
                            } else {
                                println!("  LINK {} --{}--> {}", id, r, t);
                                synced += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    // Phase 2: Remove DB records whose .md file no longer exists (files-first cleanup)
    //
    // PRD-044 fix: previously `parse::<ArtifactKind>()` returning Err caused
    // `continue`, which let rows with corrupt/empty kind escape trim forever
    // (the NOTE-037/NOTE-040 phantom bug observed in v0.17.0 dogfood audit).
    // Now we treat unparseable kind as a definite orphan: no valid kind =
    // no valid directory = no possible file = trim it.
    let mut removed = 0usize;
    let all_records = store.list_records(None).await?;
    for record in &all_records {
        enum OrphanReason {
            CorruptKind,
            MissingFile,
        }

        let orphan_reason: Option<OrphanReason> =
            match record
                .kind
                .parse::<forgeplan_core::artifact::types::ArtifactKind>()
            {
                Err(_) => Some(OrphanReason::CorruptKind),
                Ok(kind) => {
                    let dir = ws.join(kind.dir_name());
                    let slug = forgeplan_core::artifact::types::slugify(&record.title);
                    let filename = format!("{}-{}.md", record.id, slug);
                    let filepath = dir.join(&filename);

                    if filepath.exists() {
                        None
                    } else {
                        // Double-check: maybe file exists with different slug (title changed)
                        let mut found = false;
                        if dir.exists()
                            && let Ok(mut rd) = tokio::fs::read_dir(&dir).await
                        {
                            while let Ok(Some(entry)) = rd.next_entry().await {
                                let name = entry.file_name().to_string_lossy().to_string();
                                let id_prefix = format!("{}-", record.id.to_uppercase());
                                if name.to_uppercase().starts_with(&id_prefix)
                                    && name.ends_with(".md")
                                {
                                    found = true;
                                    break;
                                }
                            }
                        }
                        if found {
                            None
                        } else {
                            Some(OrphanReason::MissingFile)
                        }
                    }
                }
            };

        if let Some(reason) = orphan_reason {
            projection::delete_orphan_artifact(&store, &record.id).await?;
            common::log_change(&store, &record.id, "delete", "reindex").await;
            let reason_label = match reason {
                OrphanReason::CorruptKind => "corrupt kind field",
                OrphanReason::MissingFile => "no .md file found",
            };
            println!("  DEL  {} — {}, removed from DB", record.id, reason_label);
            removed += 1;
        }
    }

    // Phase 3: Trim orphan relations — edges whose source or target artifact
    // no longer exists. Before fix, orphan relations survived forever and
    // caused phantom rows in `forgeplan tree` rendering (NOTE-037/038/040 bug
    // from v0.17.0 dogfood audit). PRD-044 FR-001 extended scope.
    let mut orphan_relations = 0usize;
    let all_relations = store.get_all_relations().await?;
    if !all_relations.is_empty() {
        // Re-fetch fresh artifact ID set after Phase 2 trims so cascade works
        let surviving_ids: std::collections::HashSet<String> = store
            .list_records(None)
            .await?
            .into_iter()
            .map(|r| r.id)
            .collect();
        for (source, target, relation) in &all_relations {
            let source_exists = surviving_ids.contains(source);
            let target_exists = surviving_ids.contains(target);
            if !source_exists || !target_exists {
                if let Err(e) =
                    projection::delete_orphan_relation(&store, source, target, relation).await
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
