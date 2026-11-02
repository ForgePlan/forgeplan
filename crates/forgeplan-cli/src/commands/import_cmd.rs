use forgeplan_core::artifact::frontmatter::Frontmatter;
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

fn import_preflight_error(error: anyhow::Error) -> anyhow::Error {
    use forgeplan_core::artifact::canonical_key::ResolveError;
    let resolve = error.downcast_ref::<ResolveError>().or_else(|| {
        match error.downcast_ref::<forgeplan_core::import_plan::ImportPlanError>() {
            Some(forgeplan_core::import_plan::ImportPlanError::Resolve(resolve)) => Some(resolve),
            _ => None,
        }
    });
    if let Some(resolve) = resolve {
        let (input, candidates) = match resolve {
            ResolveError::Ambiguous { input, candidates } => (input.as_str(), candidates.as_slice()),
            ResolveError::ConflictingCanonicalKey { key, candidates } => (key.as_str(), candidates.as_slice()),
            ResolveError::InvalidSelector { input } => (input.as_str(), &[][..]),
            ResolveError::IncompleteCapture { .. } => ("", &[][..]),
        };
        let outcome = forgeplan_mcp::convert::Resolution::from(Err(resolve.clone()));
        return common::refuse_resolution(input, &outcome, candidates);
    }
    error
}


pub async fn run(path: &str, force: bool) -> anyhow::Result<()> {
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
    let mut data: serde_json::Value = serde_json::from_str(&json).map_err(|e| {
        anyhow::anyhow!(
            "Invalid export JSON: {}\nFix: forgeplan export --output backup.json",
            e
        )
    })?;

    // Preserve CLI normalization and the active-stub gate before freezing the
    // batch. The planner validates every resulting artifact and relation.
    let artifacts = data["artifacts"].as_array_mut().ok_or_else(|| {
        anyhow::anyhow!("Missing 'artifacts' array in export file\nFix: forgeplan export --output backup.json")
    })?;
    let mut downgraded_flags = Vec::with_capacity(artifacts.len());
    for art in artifacts {
        let Some(object) = art.as_object_mut() else { downgraded_flags.push(false); continue };
        let id = object.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let kind = object.get("kind").and_then(|v| v.as_str()).unwrap_or("note").to_string();
        let kind = if kind.parse::<forgeplan_core::artifact::types::ArtifactKind>().is_err() {
            eprintln!("  Warning: unknown kind '{kind}' for {id}, defaulting to note");
            "note".to_string()
        } else { kind };
        let status = object.get("status").and_then(|v| v.as_str()).unwrap_or("draft").to_string();
        let status = if matches!(status.as_str(), "draft" | "active" | "superseded" | "deprecated") {
            status
        } else {
            eprintln!("  Warning: unknown status '{status}' for {id}, defaulting to draft");
            "draft".to_string()
        };
        let body = object.get("body").and_then(|v| v.as_str()).unwrap_or("");
        let title = object.get("title").and_then(|v| v.as_str()).unwrap_or("");
        let is_stub = status == "active" && check_stub_detailed(body, &build_frontmatter(&id, &kind, &status, title)).is_some();
        downgraded_flags.push(is_stub && !force);
        object.insert("kind".into(), kind.into());
        object.insert("status".into(), if is_stub && !force { "draft".into() } else { status.into() });
    }

    let (ws, lock) = common::open_workspace_locked().await?;
    let migrated = common::index_migrated(&ws);
    let before_store = if migrated { None } else { Some(forgeplan_core::db::store::LanceStore::open(&ws).await?) };
    let plan = forgeplan_core::import_plan::plan_import(&ws, &lock, before_store.as_ref(), &data, force).await.map_err(import_preflight_error)?;
    let downgraded = plan.actions().iter().zip(&downgraded_flags)
        .filter(|(item, flag)| **flag && !matches!(&item.action, forgeplan_core::import_plan::ImportAction::Skip { .. }))
        .count();
    let mut skipped = 0usize;
    let mut imported = 0usize;
    let links: Vec<_> = plan.relations().iter().chain(plan.embedded_relations())
        .cloned().collect::<std::collections::BTreeSet<_>>().into_iter().collect();
    let relations_attempted = links.len();
    let has_writes = plan.actions().iter().any(|item| !matches!(&item.action, forgeplan_core::import_plan::ImportAction::Skip { .. })) || relations_attempted != 0;
    if has_writes {
        common::ensure_ordinary_mutation_allowed(&ws)?;
    }
    let store = if migrated {
        None
    } else if has_writes {
        drop(before_store);
        Some(forgeplan_core::db::store::LanceStore::open_for_write(&ws, &lock).await
            .map_err(|e| e.context("import index preparation failed; legacy migration may be partial"))?)
    } else {
        before_store
    };
    plan.verify(&ws, store.as_ref()).await.map_err(import_preflight_error)?;
    let mut attempted = false;
    let batch: anyhow::Result<usize> = async {
        let ctx = projection::MutationContext::new(&ws, &lock, store.as_ref());
        for item in plan.actions() {
            match &item.action {
                forgeplan_core::import_plan::ImportAction::Skip { .. } => { skipped += 1; continue; }
                forgeplan_core::import_plan::ImportAction::Replace { .. } => {
                    attempted = true;
                    projection::delete_artifact_with_projection(&ctx, &item.artifact.canonical_key).await?;
                }
                forgeplan_core::import_plan::ImportAction::Create => {}
            }
            attempted = true;
            projection::import_artifact_with_projection(&ctx, &item.artifact).await?;
            imported += 1;
        }
        if links.is_empty() { return Ok(0); }
        attempted = true;
        Ok(projection::import_links_with_projection(&ctx, &links).await?)
    }.await;
    drop(plan);
    drop(store);
    drop(lock);
    let drain = if attempted && !migrated {
        common::drain_legacy_semantic_after_mutation(&ws).await
    } else {
        Ok(())
    };
    if attempted && migrated {
        common::note_constellation_owns_index();
        common::reconcile_after_file_mutation(&ws).await;
    }
    let relations_imported = match (batch, drain) {
        (Ok(count), Ok(())) => count,
        (Err(error), Ok(())) if attempted => return Err(error.context("import failed; canonical files may be partially committed")),
        (Err(error), Ok(())) => return Err(error),
        (Ok(_), Err(error)) => return Err(error),
        (Err(error), Err(drain_error)) => return Err(error.context(format!("canonical files may be partially committed; semantic backlog pending: {drain_error:#}"))),
    };

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
