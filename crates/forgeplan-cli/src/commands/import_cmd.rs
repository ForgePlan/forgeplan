use forgeplan_core::artifact::canonical_key::canonical_key_from_document;
use forgeplan_core::artifact::frontmatter::Frontmatter;
use forgeplan_core::db::store::NewArtifact;
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::projection;
use forgeplan_core::validation::rules::check_stub_detailed;

use crate::commands::common;

/// Build a minimal Frontmatter BTreeMap from record fields for stub checking.
///
/// Note: import_cmd reads raw JSON (not `ArtifactRecord`), so we cannot reuse
/// `ArtifactRecord::frontmatter_map()` here. For the canonical builder, see
/// `forgeplan_core::db::store::ArtifactRecord::frontmatter_map`.
fn build_frontmatter(id: &str, kind: &str, status: &str, title: &str) -> Frontmatter {
    let mut fm = Frontmatter::new();
    fm.insert("id".to_string(), serde_yaml::Value::String(id.to_string()));
    fm.insert(
        "kind".to_string(),
        serde_yaml::Value::String(kind.to_string()),
    );
    fm.insert(
        "status".to_string(),
        serde_yaml::Value::String(status.to_string()),
    );
    fm.insert(
        "title".to_string(),
        serde_yaml::Value::String(title.to_string()),
    );
    fm
}

/// Build the record shape the import's "does it exist / what kind is it"
/// checks need, straight from a canonical projection file.
///
/// Only the fields those two checks read are populated (`kind` for HIGH-6,
/// `id` for the delete-then-create path). Everything else stays at its
/// `Default`, which is honest: this is a projection of one file's
/// frontmatter, not a store row, and inventing `r_eff_score` or timestamps
/// would claim a derivation that never happened. `body` keeps the whole
/// file so the HIGH-6 check and any later ref-form computation see the same
/// bytes the store would have held.
fn record_from_projection_file(
    id: &str,
    content: &str,
) -> forgeplan_core::db::store::ArtifactRecord {
    let (fm, _body) = forgeplan_core::artifact::frontmatter::parse_frontmatter(content)
        .unwrap_or_else(|_| (Frontmatter::new(), String::new()));
    let string = |key: &str| -> Option<String> {
        fm.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
    };
    forgeplan_core::db::store::ArtifactRecord {
        // ADR-030: the canonical key of a document is the slug its
        // frontmatter declares, and its display id when it declares none.
        canonical_key: canonical_key_from_document(
            &string("id").unwrap_or_else(|| id.to_string()),
            content,
        ),
        id: string("id").unwrap_or_else(|| id.to_string()),
        kind: string("kind").unwrap_or_default(),
        status: string("status").unwrap_or_default(),
        title: string("title").unwrap_or_default(),
        body: content.to_string(),
        depth: string("depth").unwrap_or_default(),
        author: string("author"),
        valid_until: string("valid_until"),
        tags: forgeplan_core::artifact::frontmatter::tags_from_frontmatter(&fm),
        ..Default::default()
    }
}

pub async fn run(path: &str, force: bool) -> anyhow::Result<()> {
    let (ws, lock, store) = common::open_store_locked_tolerating_constellation().await?;
    let cwd = std::env::current_dir()?;

    let full_path = if std::path::Path::new(path).is_absolute() {
        std::path::PathBuf::from(path)
    } else {
        cwd.join(path)
    };

    // PRD-071 contract: I/O and parse errors emit `Fix:` markers so agents
    // have a deterministic next action (re-run with a valid path / file).
    let file_size = std::fs::metadata(&full_path)
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to stat '{}': {}\nFix: forgeplan import <valid-path>",
                full_path.display(),
                e
            )
        })?
        .len();
    if file_size > 100 * 1024 * 1024 {
        anyhow::bail!(
            "Import file too large ({} MB). Max 100 MB.\n\
             Fix: forgeplan export --output <smaller-file>",
            file_size / 1024 / 1024
        );
    }

    let json = std::fs::read_to_string(&full_path).map_err(|e| {
        anyhow::anyhow!(
            "Failed to read '{}': {}\nFix: forgeplan import <valid-path>",
            full_path.display(),
            e
        )
    })?;
    let data: serde_json::Value = serde_json::from_str(&json).map_err(|e| {
        anyhow::anyhow!(
            "Invalid export JSON: {}\nFix: forgeplan export --output backup.json",
            e
        )
    })?;

    let artifacts = data["artifacts"].as_array().ok_or_else(|| {
        anyhow::anyhow!(
            "Missing 'artifacts' array in export file\n\
             Fix: forgeplan export --output backup.json"
        )
    })?;

    let mut imported = 0usize;
    let mut skipped = 0usize;
    let mut downgraded = 0usize;

    for art in artifacts {
        let raw_id = art["id"].as_str().unwrap_or_default();
        if raw_id.is_empty() {
            continue;
        }

        // PROB-060 / SPEC-005 Phase 2.6 (CD-6) — bulk resolve each id in
        // the payload. Hand-edited JSON exports may carry slugs instead of
        // display ids; resolve_id maps them onto the canonical DB form.
        // Resolver returning None just means the id is novel — fall back
        // to the raw input so the new artifact gets created as-is.
        //
        // On a migrated workspace the same question is asked of the
        // canonical markdown through the file-first resolver, and the
        // "does it already exist / what kind is it" answer comes from the
        // same file — never from re-opening the retired legacy store.
        let resolved_id: Option<String>;
        let existing: Option<forgeplan_core::db::store::ArtifactRecord> = match &store {
            Some(store) => {
                let canonical = store.resolve_id(raw_id).await?;
                let id_owned: String;
                let id: &str = if let Some(c) = canonical {
                    id_owned = c;
                    id_owned.as_str()
                } else {
                    raw_id
                };
                let existing = store.get_record(id).await?;
                resolved_id = Some(id.to_string());
                existing
            }
            None => match forgeplan_core::artifact::store::resolve_artifact_path(&ws, raw_id).await
            {
                Some(path) => {
                    let content = tokio::fs::read_to_string(&path).await?;
                    let (fm, _) =
                        forgeplan_core::artifact::frontmatter::parse_frontmatter(&content)?;
                    let canonical = fm
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or(raw_id)
                        .to_string();
                    resolved_id = Some(canonical);
                    Some(record_from_projection_file(raw_id, &content))
                }
                None => {
                    resolved_id = None;
                    None
                }
            },
        };
        let id: &str = resolved_id.as_deref().unwrap_or(raw_id);

        if existing.is_some() && !force {
            skipped += 1;
            continue;
        }

        // HIGH-6 (Round-1 audit, CWE-639 / data corruption): when the
        // resolver maps `prd-foo` (slug) onto an existing PRD-001 row,
        // a payload claiming `"kind": "rfc"` would otherwise delete the
        // PRD and create an `id="PRD-001", kind="rfc"` row — kind/id
        // incoherent. Refuse the import outright with an actionable
        // remediation hint. The check fires before the destructive
        // delete-then-create so the existing artifact stays intact.
        if let Some(existing_record) = &existing {
            let raw_kind_for_check = art["kind"].as_str().unwrap_or("");
            if !raw_kind_for_check.is_empty() && existing_record.kind != raw_kind_for_check {
                anyhow::bail!(
                    "Import would change kind of {} from {} to {}\n\
                     Fix: change `kind` в payload или use a different `id`",
                    id,
                    existing_record.kind,
                    raw_kind_for_check
                );
            }
        }

        if existing.is_some() {
            // PRD-073 audit H3: routed through helper so the markdown
            // projection is removed in lockstep with the LanceDB row.
            // Fixes the previous bypass where re-import via `--force`
            // left the OLD file on disk while the OLD row was deleted.
            // With a `None` sink the same helper removes the canonical file
            // and stops — the derived row retires on reconcile.
            projection::delete_artifact_with_projection(
                &projection::MutationContext::new(&ws, &lock, store.as_ref()),
                id,
            )
            .await?;
        }

        // Validate kind against known types
        let raw_kind = art["kind"].as_str().unwrap_or("note");
        let kind_str = if raw_kind
            .parse::<forgeplan_core::artifact::types::ArtifactKind>()
            .is_err()
        {
            eprintln!(
                "  Warning: unknown kind '{}' for {}, defaulting to note",
                raw_kind, id
            );
            "note"
        } else {
            raw_kind
        };
        let raw_status = art["status"].as_str().unwrap_or("draft");
        let status_str = if !matches!(raw_status, "draft" | "active" | "superseded" | "deprecated")
        {
            eprintln!(
                "  Warning: unknown status '{}' for {}, defaulting to draft",
                raw_status, id
            );
            "draft"
        } else {
            raw_status
        };

        let title = art["title"].as_str().unwrap_or("").to_string();
        let body = art["body"].as_str().unwrap_or("").to_string();

        // F3: Stub gate — prevent importing active artifacts with stub content.
        // Without --force, downgrade to draft to force reviewers through the
        // activate() gate instead of silently bypassing it.
        let final_status = if status_str == "active" {
            let fm = build_frontmatter(id, kind_str, status_str, &title);
            if let Some(report) = check_stub_detailed(&body, &fm) {
                if force {
                    eprintln!(
                        "  Warning: {} is a stub ({} markers) but --force bypasses gate, importing as active",
                        id, report.count
                    );
                    status_str.to_string()
                } else {
                    eprintln!(
                        "⚠ Importing {}: stub detected ({} markers). Downgrading to draft.",
                        id, report.count
                    );
                    downgraded += 1;
                    "draft".to_string()
                }
            } else {
                status_str.to_string()
            }
        } else {
            status_str.to_string()
        };

        let new_artifact = NewArtifact {
            // ADR-030: no minted key yet — the imported document declares no
            // slug, so its canonical key is its display id.
            canonical_key: id.to_string(),
            id: id.to_string(),
            kind: kind_str.to_string(),
            status: final_status,
            title,
            body,
            depth: art["depth"].as_str().unwrap_or("standard").to_string(),
            author: art["author"].as_str().map(String::from),
            parent_epic: art["parent_epic"].as_str().map(String::from),
            valid_until: art["valid_until"].as_str().map(String::from),
            tags: art["tags"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
        };

        // PRD-073 audit H3: helper writes the markdown projection FIRST,
        // then syncs the LanceDB row. Previous direct `store.create_artifact`
        // call left every imported artifact in DB-only state with no
        // markdown file backing — breaking ADR-003 invariant the moment
        // import completed.
        //
        // With a `None` sink the same helper writes the canonical projection
        // and stops; the whole bundle is published by the ONE reconcile pass
        // at the end, not one per artifact.
        projection::import_artifact_with_projection(
            &projection::MutationContext::new(&ws, &lock, store.as_ref()),
            &new_artifact,
        )
        .await?;
        imported += 1;
    }

    // Restore bundle edges idempotently. Complete canonical files already
    // contain their edges; duplicate additions must not be reported as failures
    // or trigger a legacy-cache render over the exact imported bytes.
    let link_triples: Vec<(String, String, String)> = data["relations"]
        .as_array()
        .map(|relations| {
            relations
                .iter()
                .filter_map(|rel| {
                    // The bundle names each endpoint twice: the display id a
                    // person reads and the canonical key that identifies the
                    // document exactly. The key is preferred; a bundle
                    // written before the key was added still imports.
                    let source = rel["source_key"]
                        .as_str()
                        .or(rel["source"].as_str())?;
                    let target = rel["target_key"]
                        .as_str()
                        .or(rel["target"].as_str())?;
                    let relation = rel["relation"].as_str().unwrap_or("informs");
                    if source.is_empty() || target.is_empty() {
                        return None;
                    }
                    Some((source.to_string(), target.to_string(), relation.to_string()))
                })
                .collect()
        })
        .unwrap_or_default();
    let relations_attempted = link_triples.len();
    let relations_imported = projection::import_links_with_projection(
        &projection::MutationContext::new(&ws, &lock, store.as_ref()),
        &link_triples,
    )
    .await
    .unwrap_or(0);

    if store.is_none() {
        common::note_constellation_owns_index();
        // Lock released before reconcile — it re-acquires the same lock per
        // storage commit, and a held guard would time the capture out.
        drop(lock);
        common::reconcile_after_file_mutation(&ws).await;
    }

    println!(
        "Imported {} artifacts ({} skipped, {} stubs downgraded to draft), {} of {} relations applied",
        imported, skipped, downgraded, relations_imported, relations_attempted
    );

    // Audit A10 (architect): half-failed imports were previously silent at
    // the CLI level. Surface the gap explicitly so the operator knows to
    // run `forgeplan health` immediately.
    if relations_imported < relations_attempted {
        eprintln!(
            "  ⚠ {} relation(s) failed to apply (likely missing artifacts). Run `forgeplan health` to verify.",
            relations_attempted - relations_imported
        );
    }

    // PRD-071 contract: after import, run a health check to surface drafts /
    // stubs / blind spots that came in.
    let hints_vec = vec![
        Hint::suggestion("Audit imported artifacts").with_action("forgeplan health".to_string()),
    ];
    print!("{}", hints::render_next_action_line(&hints_vec));

    Ok(())
}
