use anyhow::Context;
use forgeplan_core::artifact::sanitize::sanitize_for_hint;
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::projection;
use forgeplan_core::workspace::lock::WorkspaceLock;

use crate::commands::common;

pub async fn run(
    id: &str,
    status: Option<&str>,
    title: Option<&str>,
    depth: Option<&str>,
    body: Option<&str>,
) -> anyhow::Result<()> {
    // Wave 9 SEC-C1: validate new title BEFORE acquiring lock / touching DB.
    // Mirrors `forgeplan new` — rejects empty/whitespace, oversize, control
    // chars (incl. ANSI ESC), and bidi overrides (Trojan Source CWE-1007).
    // Pre-fix, `--title` accepted any bytes and only failed downstream when
    // YAML frontmatter rendering corrupted, leaving a half-mutated row.
    if let Some(t) = title {
        forgeplan_core::artifact::validate_title(t)?;
    }

    let (ws, _lock, store) = common::open_store_locked_checked(&[(id, true)]).await?;
    let Some(store) = store else {
        return run_file_first(ws, _lock, id, status, title, depth, body).await;
    };

    let resolution = common::resolve_against_workspace(&ws, id).await?;
    let (canonical_key, display_id) = common::require_found_identity(id, &resolution)?;
    let id = canonical_key.as_str();

    // The file-first path has always refused an empty update; the legacy
    // store path accepted it and rewrote the document for nothing. One
    // answer on both layouts: say what to pass instead.
    if status.is_none() && title.is_none() && depth.is_none() && body.is_none() {
        anyhow::bail!(
            "Nothing to update. Use --status, --title, --depth, or --body.\nFix: forgeplan update {} --status draft",
            id
        );
    }

    // Verify artifact exists (keep original for old projection cleanup)
    let original = store.get_record(&_lock, id).await?.ok_or_else(|| {
        anyhow::anyhow!(
            "Artifact '{}' not found
Fix: forgeplan list",
            id
        )
    })?;

    let body_content = prepare_body_update(body, &original.canonical_key).await?;

    // Block direct status change to "active" — must go through lifecycle gates.
    // PRD-071 contract: pair `Error:` with a structured `Fix:` line that the
    // agent can copy verbatim. anyhow renders the bail message after a literal
    // "Error: " prefix, so the embedded `\nFix: ...` appears on its own line.
    if let Some(s) = status
        && s.eq_ignore_ascii_case("active")
    {
        anyhow::bail!(
            "Direct status change to 'active' is not allowed.\nFix: forgeplan activate {}",
            id
        );
    }

    // ADR-020 audit BLOCKER: terminal statuses are score-relevant displacement
    // (a terminal evidence pack leaves the R_eff min), so they must go through
    // the lifecycle verbs — validated transition, successor (`supersede --by`)
    // or reason (`deprecate --reason`), journal entry. A raw metadata write
    // here was a one-call score-laundering vector: `update --status superseded`
    // cleared an active refutes pack with no successor, no edge, no gate.
    if let Some(s) = status
        && (s.eq_ignore_ascii_case("superseded") || s.eq_ignore_ascii_case("deprecated"))
    {
        anyhow::bail!(
            "Direct status change to '{}' is not allowed — displacement goes through the lifecycle.\nFix: forgeplan supersede {} --by <NEW-ID>\nOr: forgeplan deprecate {} --reason \"...\"",
            s.to_lowercase(),
            id,
            id
        );
    }

    // Validate depth string up-front so the inner helper doesn't see invalid input.
    if let Some(d) = depth {
        let _: forgeplan_core::artifact::types::Mode = d.parse().map_err(|_| {
            // PRD-071: pair Error with a concrete Fix command (default to
            // `standard` — the most-common reset value).
            anyhow::anyhow!(
                "Invalid depth '{}'. Valid: tactical, standard, deep, critical\nFix: forgeplan update {} --depth standard",
                d,
                id,
            )
        })?;
    }

    common::ensure_ordinary_mutation_allowed(&ws)?;
    let ctx = projection::MutationContext::new(&ws, &_lock, Some(&store));
    // Update metadata (status, title) FIRST so subsequent renders see the new title.
    if status.is_some() || title.is_some() {
        projection::update_metadata_with_projection(&ctx, id, status, title)
            .await
            .context("update failed; canonical files may be partially committed, semantic backlog may be pending")?;
    }

    // Depth update — renders against the (possibly new) title from DB.
    if let Some(d) = depth {
        projection::update_depth_with_projection(&ctx, id, d)
            .await
            .context("update failed; canonical files may be partially committed, semantic backlog may be pending")?;
    }

    // Update body
    if let Some(body_content) = body_content.as_deref() {
        // Safety check: warn if new body is significantly shorter than existing.
        // Re-read after the metadata mutation above may have synced file→DB.
        let current = store
            .get_record(&_lock, id)
            .await?
            .unwrap_or(original.clone());
        let old_len = current.body.len();
        let new_len = body_content.len();
        if old_len > 100 && new_len < old_len / 3 {
            eprintln!(
                "  ⚠ Warning: new body ({} chars) is much shorter than existing ({} chars).",
                new_len, old_len
            );
            eprintln!(
                "  Tip: use --body @file.md for safe multi-line updates (avoids shell escaping issues)."
            );
        }

        projection::update_body_with_projection(&ctx, id, body_content)
            .await
            .context("update failed; canonical files may be partially committed, semantic backlog may be pending")?;
    }

    // Log changes
    if let Some(s) = status {
        common::log_change_field(
            &ws,
            &_lock,
            Some(&store),
            (&canonical_key, &display_id),
            "update",
            ("status", Some(&original.status), Some(s)),
            "cli",
        )
        .await;
    }
    if let Some(t) = title {
        common::log_change_field(
            &ws,
            &_lock,
            Some(&store),
            (&canonical_key, &display_id),
            "update",
            ("title", Some(&original.title), Some(t)),
            "cli",
        )
        .await;
    }
    if body.is_some() {
        common::log_change_field(
            &ws,
            &_lock,
            Some(&store),
            (&canonical_key, &display_id),
            "update",
            ("body", None, None),
            "cli",
        )
        .await;
    }

    drop(store);
    drop(_lock);
    common::drain_legacy_semantic_after_mutation(&ws).await?;

    println!("  Updated: {}", display_id);
    if let Some(s) = status {
        println!("  Status:  {}", s);
    }
    if let Some(t) = title {
        // LOG-001 defence (Wave 9 audit): even though `validate_title` above
        // rejects control chars + bidi overrides, sanitise on the way out as
        // belt-and-braces — keeps the rendered line safe even if a future
        // refactor weakens the validator or the title travels through this
        // print path from a different (non-validated) source. Sanitiser
        // strips zero-width chars, ANSI, shell metas, and bidi.
        println!("  Title:   {}", sanitize_for_hint(t));
    }
    if body.is_some() {
        println!("  Body:    updated");
    }

    // PRD-071: any update — re-validate to make sure the artifact still
    // satisfies its kind+depth rules.
    let ref_form = canonical_key;
    let next_hints: Vec<Hint> = vec![
        Hint::info("Updated — re-run validator")
            .with_action(format!("forgeplan validate {}", ref_form)),
    ];
    print!("{}", hints::render_next_action_line(&next_hints));

    Ok(())
}

/// File-first `update` for constellation-migrated workspaces: no
/// `LanceStore` exists, so the mutation edits the markdown directly
/// (`update_metadata_file_first` / `update_body_file_first`) and one
/// reconcile pass publishes it. Audit events use auxiliary history; old
/// projection cleanup bookkeeping beyond the rename itself is skipped.
///
/// Same gates, ordering and output shape as the store path: resolve →
/// nothing-to-update → status guards → metadata → body → prints.
async fn run_file_first(
    ws: std::path::PathBuf,
    lock: WorkspaceLock,
    id: &str,
    status: Option<&str>,
    title: Option<&str>,
    depth: Option<&str>,
    body: Option<&str>,
) -> anyhow::Result<()> {
    let resolution = common::resolve_against_workspace(&ws, id).await?;
    let (canonical_key, _) = common::require_found_identity(id, &resolution)?;
    let artifact_path =
        forgeplan_core::artifact::documents::resolve_in_workspace(&ws, &canonical_key)
            .await?
            .map(|document| ws.join(document.relative_path))
            .ok_or_else(|| anyhow::anyhow!("Artifact '{id}' not found\nFix: forgeplan list"))?;
    let content = tokio::fs::read_to_string(&artifact_path).await?;
    let (fm, old_body) = forgeplan_core::artifact::frontmatter::parse_frontmatter(&content)?;
    // Canonical id from the file's own `id:` field — every downstream
    // message and the rename use it, regardless of input form (slug or
    // display id, same as `store.resolve_id` produced).
    let id_owned = fm
        .get("id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| id.to_string());
    let id = id_owned.as_str();
    let current_key =
        forgeplan_core::artifact::canonical_key::canonical_key_from_document(id, &content);
    let body_content = prepare_body_update(body, &current_key).await?;

    if status.is_none() && title.is_none() && depth.is_none() && body.is_none() {
        anyhow::bail!(
            "Nothing to update. Use --status, --title, --depth, or --body.\nFix: forgeplan update {} --status draft",
            id
        );
    }

    if let Some(s) = status
        && s.eq_ignore_ascii_case("active")
    {
        anyhow::bail!(
            "Direct status change to 'active' is not allowed.\nFix: forgeplan activate {}",
            id
        );
    }

    // Same lifecycle-laundering guard as the store path — terminal statuses
    // go through `supersede`/`deprecate` even on a migrated workspace.
    if let Some(s) = status
        && (s.eq_ignore_ascii_case("superseded") || s.eq_ignore_ascii_case("deprecated"))
    {
        anyhow::bail!(
            "Direct status change to '{}' is not allowed — displacement goes through the lifecycle.\nFix: forgeplan supersede {} --by <NEW-ID>\nOr: forgeplan deprecate {} --reason \"...\"",
            s.to_lowercase(),
            id,
            id
        );
    }

    if let Some(d) = depth {
        let _: forgeplan_core::artifact::types::Mode = d.parse().map_err(|_| {
            anyhow::anyhow!(
                "Invalid depth '{}'. Valid: tactical, standard, deep, critical\nFix: forgeplan update {} --depth standard",
                d,
                id,
            )
        })?;
    }

    // Metadata first (renames the file on --title), then body against the
    // possibly-renamed path — mirrors the store path's ordering.
    common::ensure_ordinary_mutation_allowed(&ws)?;
    let mut final_path = artifact_path.clone();
    if status.is_some() || title.is_some() || depth.is_some() {
        final_path = projection::update_metadata_file_first(&artifact_path, status, title, depth)
            .await
            .context(
                "canonical metadata update may be partially committed; semantic reconciliation and drain did not run",
            )?;
    }

    if let Some(body_content) = body_content.as_deref() {
        // Same short-body warning as the store path, measured on the file's
        // own body portion.
        let old_len = old_body.len();
        let new_len = body_content.len();
        if old_len > 100 && new_len < old_len / 3 {
            eprintln!(
                "  ⚠ Warning: new body ({} chars) is much shorter than existing ({} chars).",
                new_len, old_len
            );
            eprintln!(
                "  Tip: use --body @file.md for safe multi-line updates (avoids shell escaping issues)."
            );
        }

        projection::update_body_file_first(&final_path, body_content)
            .await
            .context(if status.is_some() || title.is_some() || depth.is_some() {
                "canonical metadata was committed before the body update failed; semantic reconciliation and drain did not run"
            } else {
                "canonical body update may be partially committed; semantic reconciliation and drain did not run"
            })?;
    }

    if let Some(s) = status {
        common::log_change_field(
            &ws,
            &lock,
            None,
            (&current_key, id),
            "update",
            ("status", fm.get("status").and_then(|v| v.as_str()), Some(s)),
            "cli",
        )
        .await;
    }
    if let Some(t) = title {
        common::log_change_field(
            &ws,
            &lock,
            None,
            (&current_key, id),
            "update",
            ("title", fm.get("title").and_then(|v| v.as_str()), Some(t)),
            "cli",
        )
        .await;
    }
    if body.is_some() {
        common::log_change_field(
            &ws,
            &lock,
            None,
            (&current_key, id),
            "update",
            ("body", None, None),
            "cli",
        )
        .await;
    }

    println!("  Updated: {}", id);
    if let Some(s) = status {
        println!("  Status:  {}", s);
    }
    if let Some(t) = title {
        println!("  Title:   {}", sanitize_for_hint(t));
    }
    if body.is_some() {
        println!("  Body:    updated");
    }

    let ref_form = canonical_key;

    common::note_constellation_owns_index();
    // Lock must drop before reconcile — see `common::reconcile_after_file_mutation`.
    drop(lock);
    match common::reconcile_after_file_mutation(&ws).await {
        common::ReconcileOutcome::Published => {
            if let Err(error) = drain_file_first_semantic_queue(&ws).await {
                if error.chain().any(|cause| {
                    cause
                        .downcast_ref::<forgeplan_core::constellation::ConstellationError>()
                        .is_some_and(|error| {
                            matches!(
                                error,
                                forgeplan_core::constellation::ConstellationError::IndexBusy { .. }
                            )
                        })
                }) {
                    eprintln!(
                        "  warning: semantic publication is pending because the constellation index is busy"
                    );
                    eprintln!("  index_pending: true — derived index not yet updated");
                } else {
                    return Err(error);
                }
            }
        }
        common::ReconcileOutcome::PendingIndexBusy => {}
        common::ReconcileOutcome::Failed(error) => {
            anyhow::bail!(
                "canonical update committed, but Constellation reconciliation failed: {error}; semantic backlog remains pending"
            );
        }
    }

    let next_hints: Vec<Hint> = vec![
        Hint::info("Updated — re-run validator")
            .with_action(format!("forgeplan validate {}", ref_form)),
    ];
    print!("{}", hints::render_next_action_line(&next_hints));

    Ok(())
}
async fn prepare_body_update(
    body: Option<&str>,
    current_key: &str,
) -> anyhow::Result<Option<String>> {
    let Some(body) = body else {
        return Ok(None);
    };
    let raw_content = if let Some(path) = body.strip_prefix('@') {
        tokio::fs::read_to_string(path)
            .await
            .map_err(|error| anyhow::anyhow!("Cannot read '{}': {}", path, error))?
    } else {
        body.to_string()
    };
    forgeplan_core::artifact::frontmatter::refuse_canonical_key_change(current_key, &raw_content)?;
    let content = if raw_content.starts_with("---") {
        match forgeplan_core::artifact::frontmatter::parse_frontmatter(&raw_content) {
            Ok((_frontmatter, body)) => body.to_string(),
            Err(_) => raw_content,
        }
    } else {
        raw_content
    };
    Ok(Some(content))
}

#[cfg(feature = "semantic-search")]
async fn drain_file_first_semantic_queue(ws: &std::path::Path) -> anyhow::Result<()> {
    use forgeplan_core::constellation::{facade, jobs, runtime};

    let config = forgeplan_core::workspace::load_config(ws)
        .context("canonical update committed; cannot read embedding configuration")?;
    let Some(embedding) = config.embedding.as_ref() else {
        return Ok(());
    };
    let model = embedding.model.as_str();
    let repo = forgeplan_core::embed::resolve_repo(model).ok_or_else(|| {
        anyhow::anyhow!("canonical update committed; unsupported embedding model `{model}`")
    })?;
    let resolved = jobs::ResolvedProfile::open(
        &forgeplan_core::embed::resolve_cache_dir(),
        repo,
        facade::ENGINE_VERSION,
        embedding.chunk_size,
    )
    .context("canonical update committed; configured embedding model is unavailable")?;
    let capability = facade::MaintainCapability::for_strict_maintenance(
        ws.canonicalize().unwrap_or_else(|_| ws.to_path_buf()),
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
    let control = runtime::DrainControl::never();

    let outcome = if let Some(supervisor) = runtime::MaintenanceSupervisor::registered(ws) {
        supervisor
            .drain_inference(&capability, model, &resolved, deadline, &control)
            .await
            .context("canonical update committed; supervisor semantic drain failed")?
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "canonical update committed; supervisor did not drain the semantic queue"
                )
            })?
    } else {
        let mut worker = runtime::InferenceWorker::spawn_resolved(model, &resolved)
            .context("canonical update committed; could not start semantic inference worker")?;
        let drained = facade::drain_embedding_queue(
            &capability,
            &resolved.profile,
            &mut worker,
            deadline,
            &control,
        )
        .await;
        let shutdown = worker.shutdown();
        let outcome =
            drained.context("canonical update committed; owner-specific semantic drain failed")?;
        match shutdown.context("canonical update committed; semantic worker did not shut down")? {
            runtime::ShutdownOutcome::Joined => outcome,
            runtime::ShutdownOutcome::Detached => {
                anyhow::bail!(
                    "canonical update committed; semantic worker detached before inference completed"
                )
            }
        }
    };

    let eligible = outcome.after.eligible_keys();
    let cannot_judge = outcome.after.cannot_judge();
    if outcome.failed > 0 || !eligible.is_empty() || !cannot_judge.is_empty() {
        anyhow::bail!(
            "canonical update committed; semantic backlog was not fully drained ({} failed, {} still eligible, {} cannot be judged)",
            outcome.failed,
            eligible.len(),
            cannot_judge.len(),
        );
    }
    Ok(())
}

#[cfg(not(feature = "semantic-search"))]
async fn drain_file_first_semantic_queue(ws: &std::path::Path) -> anyhow::Result<()> {
    let config = forgeplan_core::workspace::load_config(ws)
        .context("canonical update committed; cannot read embedding configuration")?;
    if config.embedding.is_some() {
        anyhow::bail!(
            "canonical update committed; semantic backlog remains pending because this binary was built without semantic-search"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn configured_but_unsupported_embedder_returns_error_without_mutating_workspace() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().join(".forgeplan");
        std::fs::create_dir(&workspace).unwrap();
        let config_path = workspace.join("config.yaml");
        let config = "version: 1\nproject_name: test\nembedding:\n  model: unsupported-model\n  chunk_size: 2000\n";
        std::fs::write(&config_path, config).unwrap();

        let error = drain_file_first_semantic_queue(&workspace)
            .await
            .expect_err("a configured embedder without a real implementation must not be reported as drained");

        assert!(
            format!("{error:#}").contains("unsupported embedding model")
                || format!("{error:#}").contains("built without semantic-search"),
            "unexpected drain error: {error:#}"
        );
        assert_eq!(std::fs::read_to_string(config_path).unwrap(), config);
        assert_eq!(
            std::fs::read_dir(&workspace).unwrap().count(),
            1,
            "the drain must not create index state when it cannot embed"
        );
    }
}
