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

pub async fn run(path: &str, force: bool) -> anyhow::Result<()> {
    let (ws, _lock, store) = common::open_store_locked().await?;
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
        let canonical = store.resolve_id(raw_id).await?;
        let id_owned: String;
        let id: &str = if let Some(c) = canonical {
            id_owned = c;
            id_owned.as_str()
        } else {
            raw_id
        };

        let existing = store.get_record(id).await?;
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
            projection::delete_artifact_with_projection(
                &projection::MutationContext::new(&ws, &store),
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
        projection::create_artifact_with_projection(
            &projection::MutationContext::new(&ws, &store),
            &new_artifact,
        )
        .await?;
        imported += 1;
    }

    // PRD-073 H6 (audit follow-up): batch helper deduplicates pre-sync +
    // post-render per unique participant. For a 100-link bundle this is
    // ~2×U LanceDB+file ops vs the naive 6×N (audit measurement).
    let link_triples: Vec<(String, String, String)> = data["relations"]
        .as_array()
        .map(|relations| {
            relations
                .iter()
                .filter_map(|rel| {
                    let source = rel["source"].as_str()?;
                    let target = rel["target"].as_str()?;
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
    let relations_imported = projection::add_links_batch_with_projection(
        &projection::MutationContext::new(&ws, &store),
        &link_triples,
    )
    .await
    .unwrap_or(0);

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
