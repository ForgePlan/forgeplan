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
                        store.update_body(&id, &body).await?;
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

                    store.create_artifact(&artifact).await?;
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
                            if let Err(e) = store.add_relation(&id, t, r).await {
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
            store.delete_artifact(&record.id).await?;
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
                if let Err(e) = store.delete_relation(source, target, relation).await {
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
    Ok(())
}
