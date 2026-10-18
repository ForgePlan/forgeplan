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

    let (ws, _lock, store) = common::open_store_locked_tolerating_constellation().await?;
    let Some(store) = store else {
        return run_file_first(ws, _lock, id, status, title, depth, body).await;
    };

    // PROB-060 / SPEC-005 Phase 2.6 (CD-6) — accept slug or display id.
    // Resolve once at the top so every downstream operation (lifecycle,
    // projection, log_change, hint rendering) sees the canonical DB id
    // regardless of which form the user passed.
    let id = store
        .resolve_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact '{id}' not found\nFix: forgeplan list"))?;
    let id = id.as_str();

    // Verify artifact exists (keep original for old projection cleanup)
    let original = store.get_record(id).await?.ok_or_else(|| {
        anyhow::anyhow!(
            "Artifact '{}' not found
Fix: forgeplan list",
            id
        )
    })?;

    if status.is_none() && title.is_none() && depth.is_none() && body.is_none() {
        // PRD-071: Error pairs with concrete Fix: command (use --status as the
        // most-common operation; agent picks the right flag from the example).
        anyhow::bail!(
            "Nothing to update. Use --status, --title, --depth, or --body.\nFix: forgeplan update {} --status draft",
            id
        );
    }

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

    // PRD-073 audit fix: order is metadata FIRST (which writes any title
    // change into LanceDB), THEN depth/body (each renders against the new
    // title → new slug). The previous order ran depth before metadata, so
    // the depth helper rendered to the OLD slug — defeating the OLD-file
    // cleanup and leaving both filenames on disk. Old-slug cleanup happens
    // AT THE END via `remove_projection_at` with the original title so we
    // pin the exact path and don't risk prefix collisions.

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

    let ctx = projection::MutationContext::new(&ws, &store);
    // Update metadata (status, title) FIRST so subsequent renders see the new title.
    if status.is_some() || title.is_some() {
        projection::update_metadata_with_projection(&ctx, id, status, title).await?;
    }

    // Depth update — renders against the (possibly new) title from DB.
    if let Some(d) = depth {
        projection::update_depth_with_projection(&ctx, id, d).await?;
    }

    // Update body
    if let Some(b) = body {
        let raw_content = if let Some(path) = b.strip_prefix('@') {
            tokio::fs::read_to_string(path)
                .await
                .map_err(|e| anyhow::anyhow!("Cannot read '{}': {}", path, e))?
        } else {
            b.to_string()
        };

        // Strip YAML frontmatter if present (when reading from @file.md)
        let body_content = if raw_content.starts_with("---") {
            use forgeplan_core::artifact::frontmatter;
            match frontmatter::parse_frontmatter(&raw_content) {
                Ok((_fm, body)) => body.to_string(),
                Err(_) => raw_content,
            }
        } else {
            raw_content
        };

        // Safety check: warn if new body is significantly shorter than existing.
        // Re-read after the metadata mutation above may have synced file→DB.
        let current = store.get_record(id).await?.unwrap_or(original.clone());
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

        projection::update_body_with_projection(&ctx, id, &body_content).await?;
    }

    // PRD-073 audit M1 fix: clean up OLD slug AFTER the new file is in place
    // (so there's no orphan window) and use exact-path removal so we don't
    // accidentally clobber a sibling artifact whose ID is a prefix of this one.
    // The rename-aware helper additionally skips removal when the new title
    // slugifies to the same filename (case/punctuation-only edit) — otherwise
    // it deletes the file update just wrote (silent, reindex-unrecoverable).
    if let Some(new_title) = title {
        let _ = projection::remove_stale_projection_after_rename(
            &ws,
            id,
            &original.kind,
            &original.title,
            new_title,
        )
        .await;
    }

    // Log changes
    if let Some(s) = status {
        common::log_change_field(
            &store,
            id,
            "update",
            "status",
            Some(&original.status),
            Some(s),
            "cli",
        )
        .await;
    }
    if let Some(t) = title {
        common::log_change_field(
            &store,
            id,
            "update",
            "title",
            Some(&original.title),
            Some(t),
            "cli",
        )
        .await;
    }
    if body.is_some() {
        common::log_change_field(&store, id, "update", "body", None, None, "cli").await;
    }

    println!("  Updated: {}", id);
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
    // PROB-060 / SPEC-005 / ADR-012 (W1.B, CD-5) — slug pre-merge / display
    // id post-merge so the re-validation command stays canonical for
    // commit `Refs:` lines.
    let updated_record = store.get_record(id).await?;
    let ref_form = match &updated_record {
        Some(r) => forgeplan_core::artifact::frontmatter::refs_form_from_body(&r.body, &r.id),
        None => id.to_string(),
    };
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
/// reconcile pass publishes it. Store-backed extras (changelog row,
/// old-projection cleanup bookkeeping beyond the rename itself) are
/// skipped — the note names the owner instead of faking them.
///
/// Same gates, ordering and output shape as the store path: resolve →
/// nothing-to-update → status guards → metadata+rename → body → prints.
async fn run_file_first(
    ws: std::path::PathBuf,
    lock: WorkspaceLock,
    id: &str,
    status: Option<&str>,
    title: Option<&str>,
    depth: Option<&str>,
    body: Option<&str>,
) -> anyhow::Result<()> {
    use forgeplan_core::artifact::store::resolve_artifact_path;

    let artifact_path = resolve_artifact_path(&ws, id)
        .await
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
    let mut final_path = artifact_path.clone();
    if status.is_some() || title.is_some() || depth.is_some() {
        final_path =
            projection::update_metadata_file_first(&artifact_path, status, title, depth).await?;
    }

    if let Some(b) = body {
        let raw_content = if let Some(path) = b.strip_prefix('@') {
            tokio::fs::read_to_string(path)
                .await
                .map_err(|e| anyhow::anyhow!("Cannot read '{}': {}", path, e))?
        } else {
            b.to_string()
        };

        // Strip YAML frontmatter if present (when reading from @file.md)
        let body_content = if raw_content.starts_with("---") {
            use forgeplan_core::artifact::frontmatter;
            match frontmatter::parse_frontmatter(&raw_content) {
                Ok((_fm, body)) => body.to_string(),
                Err(_) => raw_content,
            }
        } else {
            raw_content
        };

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

        projection::update_body_file_first(&final_path, &body_content).await?;
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

    // Ref form for the validate hint: slug pre-merge, display id
    // post-merge — read from the mutated file's body portion, the file-first
    // equivalent of `refs_form_from_body(&record.body, ..)`.
    let ref_form = match tokio::fs::read_to_string(&final_path).await {
        Ok(content) => {
            let body_portion = forgeplan_core::artifact::frontmatter::parse_frontmatter(&content)
                .map(|(_, b)| b)
                .unwrap_or_default();
            forgeplan_core::artifact::frontmatter::refs_form_from_body(&body_portion, id)
        }
        Err(_) => id.to_string(),
    };

    common::note_constellation_owns_index();
    // Lock must drop before reconcile — see `common::reconcile_after_file_mutation`.
    drop(lock);
    common::reconcile_after_file_mutation(&ws).await;

    let next_hints: Vec<Hint> = vec![
        Hint::info("Updated — re-run validator")
            .with_action(format!("forgeplan validate {}", ref_form)),
    ];
    print!("{}", hints::render_next_action_line(&next_hints));

    Ok(())
}
