use forgeplan_core::hints;
use forgeplan_core::link;
use forgeplan_core::workspace::lock::WorkspaceLock;

use crate::commands::common;

pub async fn run(
    source_id: &str,
    target_id: &str,
    relation: &str,
    replace: bool,
    auto_activate_complete: bool,
) -> anyhow::Result<()> {
    // Normalize relation
    let relation = link::normalize_relation(relation)?;

    let (ws, _lock, store) = common::open_store_locked_tolerating_constellation().await?;
    let Some(store) = store else {
        return run_file_first(ws, _lock, source_id, target_id, &relation, replace).await;
    };

    // PROB-060 / SPEC-005 Phase 1.5b — accept slug or display id for both ends.
    // Source must resolve (we error otherwise); target may not exist yet
    // (cross-PR forward-reference is allowed) so we only resolve if possible.
    let source_id_owned = store.resolve_id(source_id).await?.ok_or_else(|| {
        anyhow::anyhow!("Source artifact '{source_id}' not found\nFix: forgeplan list")
    })?;
    let source_id = source_id_owned.as_str();

    let target_id_owned = store
        .resolve_id(target_id)
        .await?
        .unwrap_or_else(|| target_id.to_string());
    let target_id = target_id_owned.as_str();

    // Verify source exists (sanity check after resolve — should always pass).
    if store.get_artifact(source_id).await?.is_none() {
        anyhow::bail!(
            "Source artifact '{}' not found
Fix: forgeplan list",
            source_id
        );
    }

    // Verify target exists (warning only). A qualified `KEY@qualifier`
    // target is foreign by definition — "not in this workspace" is the
    // contract, not a warning — so instead we check the qualifier against
    // the registered constellation sources and warn when it names none
    // (a typo there reads as a deliberate forward reference otherwise).
    // Warn-only either way: forward refs stay legal. Registry problems
    // never block the write — a missing registry means "no constellation".
    // The `is_ok` guard keeps a *malformed* qualified ref on the plain
    // local-miss path: its validation error downstream is the real answer.
    match forgeplan_core::constellation::federation::parse_qualified_ref(target_id) {
        (_, Some(qualifier))
            if forgeplan_core::db::store::validate_link_target(target_id).is_ok() =>
        {
            match crate::commands::constellation::load_registry().map(|(_, registry)| registry) {
                Ok(registry) => {
                    match forgeplan_core::constellation::registry::current_sources(&registry) {
                        Some(specs) => {
                            if forgeplan_core::constellation::federation::specs_matching(
                                &specs, qualifier,
                            )
                            .is_empty()
                            {
                                eprintln!(
                                    "Warning: qualifier @{qualifier} matches no registered source \
                                     (creating link anyway — forward references are legal)\n\
                                     Fix: forgeplan constellation sources"
                                );
                            }
                        }
                        None => {
                            eprintln!(
                                "Warning: no active constellation to check qualifier @{qualifier} \
                                 against (creating link anyway)\n\
                                 Fix: forgeplan constellation list"
                            );
                        }
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Warning: could not check qualifier @{qualifier} against the \
                         constellation registry ({e}) — creating link anyway"
                    );
                }
            }
        }
        _ => {
            if store.get_artifact(target_id).await?.is_none() {
                eprintln!(
                    "Warning: Target artifact '{}' not found in workspace (creating link anyway)",
                    target_id
                );
            }
        }
    }

    // Issue #286: `--replace` selects the upsert helper, which collapses
    // any pre-existing edge between (source, target) onto the requested
    // relation. Without the flag we use the strict additive helper that
    // rejects duplicates and parallel-different-relation edges.
    let ctx = forgeplan_core::projection::MutationContext::new(&ws, &store);
    let outcome = if replace {
        Some(
            forgeplan_core::projection::replace_link_with_projection(
                &ctx, source_id, target_id, &relation,
            )
            .await?,
        )
    } else {
        forgeplan_core::projection::add_link_with_projection(&ctx, source_id, target_id, &relation)
            .await?;
        None
    };

    common::log_change_field(
        &store,
        source_id,
        "link",
        "relation",
        None,
        Some(&format!("{}:{}", target_id, relation)),
        "cli",
    )
    .await;

    // Surface the outcome verbatim when `--replace` was used so operators
    // can see whether the call was a no-op, a swap, or a creation. Without
    // the flag we keep the original one-liner for back-compat.
    match outcome {
        Some(forgeplan_core::projection::LinkUpsertOutcome::Replaced { old_relation }) => {
            println!(
                "Replaced: {} --{}--> {} (was: --{}-->)",
                source_id, relation, target_id, old_relation
            );
        }
        Some(forgeplan_core::projection::LinkUpsertOutcome::Unchanged) => {
            println!(
                "Unchanged: {} --{}--> {} already exists",
                source_id, relation, target_id
            );
        }
        Some(forgeplan_core::projection::LinkUpsertOutcome::Created) | None => {
            println!("Linked: {} --{}--> {}", source_id, relation, target_id);
        }
    }

    // PRD-075 FR-001: sync recompute closes stale-cache window. Failure is
    // non-fatal (mutation succeeded) — the wrapper surfaces a Fix: marker so
    // agents/operators see the score-all recovery path.
    common::sync_score_target_or_warn(&store, source_id).await;

    // Issue #288 Part A — CLI parity for auto-activate. Same predicate as
    // the MCP `forgeplan_link` tool. Failure is non-fatal: the link itself
    // already committed; we warn and let the user retry forgeplan_activate
    // manually.
    if auto_activate_complete
        && let Ok(Some(src_record)) = store.get_record(source_id).await
        && src_record.kind.eq_ignore_ascii_case("evidence")
        && src_record.status.eq_ignore_ascii_case("draft")
        && forgeplan_core::scoring::evidence::is_evidence_complete(&src_record.body)
    {
        match forgeplan_core::lifecycle::activate(&store, source_id, false).await {
            Ok(_) => {
                if let Err(e) =
                    forgeplan_core::projection::render_after_mutation(&ws, &store, source_id).await
                {
                    eprintln!("Warning: auto-activate post-render for {source_id} failed: {e}");
                }
                println!("Auto-activated: {source_id} (draft → active)");
            }
            Err(e) => {
                eprintln!(
                    "Warning: auto-activate of {source_id} failed: {e}\nFix: forgeplan activate {source_id}"
                );
            }
        }
    }

    // PRD-075 FR-009: hint string lives in core (forgeplan_core::hints) so
    // mutator call sites cannot drift independently.
    let hints_vec = vec![hints::reconcile_parents_hint()];
    print!("{}", hints::render_next_action_line(&hints_vec));
    Ok(())
}

/// File-first `link` for constellation-migrated workspaces: no `LanceStore`
/// exists, so the edge is written into the source file's `links:` frontmatter
/// and one reconcile pass publishes it to `constellation_*_v1`. Store-backed
/// extras (changelog row, R_eff recompute, auto-activate, projection
/// re-render) are skipped — the note names the owner instead of faking them.
///
/// Semantics preserved from the store path: slug/display-id source
/// resolution (via the file scan), the qualified `KEY@qualifier` registry
/// check, forward-reference warning, `--replace` upsert outcomes, verbatim
/// target storage, normalized relations.
async fn run_file_first(
    ws: std::path::PathBuf,
    lock: WorkspaceLock,
    source_id: &str,
    target_id: &str,
    relation: &str,
    replace: bool,
) -> anyhow::Result<()> {
    use forgeplan_core::artifact::store::resolve_artifact_path;

    // Boundary discipline mirrors the store path: a malformed source just
    // fails resolution below (same "not found" refusal), and the target is
    // validated inside `add_link_verbatim` / `replace_link` — the file-first
    // helpers apply `validate_link_target` exactly where the projection
    // helpers do.
    let source_path = resolve_artifact_path(&ws, source_id).await.ok_or_else(|| {
        anyhow::anyhow!("Source artifact '{source_id}' not found\nFix: forgeplan list")
    })?;
    let (source_fm, _) = forgeplan_core::artifact::frontmatter::parse_frontmatter(
        &tokio::fs::read_to_string(&source_path).await?,
    )?;
    let source_id_owned = source_fm
        .get("id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| source_id.to_string());
    let source_id = source_id_owned.as_str();

    // Target: qualified refs go through the same registry check as the
    // store path; local targets resolve to their canonical fm id, and a
    // miss stays a legal forward reference with the same warning.
    let target_id_owned: String;
    match forgeplan_core::constellation::federation::parse_qualified_ref(target_id) {
        // Same `is_ok` guard as the store path: a *malformed* qualified ref
        // drops to the plain local-miss arm, where the core helper's
        // `validate_link_target` produces the real answer downstream.
        (_, Some(qualifier))
            if forgeplan_core::db::store::validate_link_target(target_id).is_ok() =>
        {
            match crate::commands::constellation::load_registry().map(|(_, registry)| registry) {
                Ok(registry) => {
                    match forgeplan_core::constellation::registry::current_sources(&registry) {
                        Some(specs) => {
                            if forgeplan_core::constellation::federation::specs_matching(
                                &specs, qualifier,
                            )
                            .is_empty()
                            {
                                eprintln!(
                                    "Warning: qualifier @{qualifier} matches no registered source \
                                     (creating link anyway — forward references are legal)\n\
                                     Fix: forgeplan constellation sources"
                                );
                            }
                        }
                        None => {
                            eprintln!(
                                "Warning: no active constellation to check qualifier @{qualifier} \
                                 against (creating link anyway)\n\
                                 Fix: forgeplan constellation list"
                            );
                        }
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Warning: could not check qualifier @{qualifier} against the \
                         constellation registry ({e}) — creating link anyway"
                    );
                }
            }
            target_id_owned = target_id.to_string();
        }
        _ => match resolve_artifact_path(&ws, target_id).await {
            Some(target_path) => {
                let (target_fm, _) = forgeplan_core::artifact::frontmatter::parse_frontmatter(
                    &tokio::fs::read_to_string(&target_path).await?,
                )?;
                target_id_owned = target_fm
                    .get("id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| target_id.to_string());
            }
            None => {
                eprintln!(
                    "Warning: Target artifact '{}' not found in workspace (creating link anyway)",
                    target_id
                );
                target_id_owned = target_id.to_string();
            }
        },
    }
    let target_id = target_id_owned.as_str();

    let outcome = if replace {
        Some(link::replace_link(&source_path, target_id, relation).await?)
    } else {
        link::add_link_verbatim(&source_path, target_id, relation).await?;
        None
    };

    match &outcome {
        Some(forgeplan_core::projection::LinkUpsertOutcome::Replaced { old_relation }) => {
            println!(
                "Replaced: {} --{}--> {} (was: --{}-->)",
                source_id, relation, target_id, old_relation
            );
        }
        Some(forgeplan_core::projection::LinkUpsertOutcome::Unchanged) => {
            println!(
                "Unchanged: {} --{}--> {} already exists",
                source_id, relation, target_id
            );
        }
        Some(forgeplan_core::projection::LinkUpsertOutcome::Created) | None => {
            println!("Linked: {} --{}--> {}", source_id, relation, target_id);
        }
    }

    common::note_constellation_owns_index();
    // Release the workspace lock BEFORE reconcile — the pass re-acquires it
    // per storage commit and would deadlock on a held guard (30 s timeout).
    // Skip the pass entirely on Unchanged: no bytes moved, nothing to publish.
    drop(lock);
    if outcome != Some(forgeplan_core::projection::LinkUpsertOutcome::Unchanged) {
        common::reconcile_after_file_mutation(&ws).await;
    }

    let hints_vec = vec![hints::reconcile_parents_hint()];
    print!("{}", hints::render_next_action_line(&hints_vec));
    Ok(())
}

pub async fn run_unlink(source_id: &str, target_id: &str, relation: &str) -> anyhow::Result<()> {
    let relation = link::normalize_relation(relation)?;
    let (ws, _lock, store) = common::open_store_locked_tolerating_constellation().await?;
    let Some(store) = store else {
        return run_unlink_file_first(ws, _lock, source_id, target_id, &relation).await;
    };

    // Check relation exists before deleting.
    // Use get_all_relations for resilient lookup (works even if source artifact is deleted).
    let all_rels = store.get_all_relations().await?;
    let found = all_rels.iter().any(|(s, t, r)| {
        s.eq_ignore_ascii_case(source_id) && t.eq_ignore_ascii_case(target_id) && r == &relation
    });
    if !found {
        anyhow::bail!(
            "Relation '{}' from {} to {} not found",
            relation,
            source_id,
            target_id
        );
    }

    // PRD-073 FR-005: bidirectional render via helper.
    forgeplan_core::projection::delete_link_with_projection(
        &forgeplan_core::projection::MutationContext::new(&ws, &store),
        source_id,
        target_id,
        &relation,
    )
    .await?;

    common::log_change_field(
        &store,
        source_id,
        "unlink",
        "relation",
        Some(&format!("{}:{}", target_id, relation)),
        None,
        "cli",
    )
    .await;

    println!("Unlinked: {} --{}--> {}", source_id, relation, target_id);

    // PRD-075 FR-002: sync recompute after unlink mirrors the link path.
    common::sync_score_target_or_warn(&store, source_id).await;

    let hints_vec = vec![hints::reconcile_parents_hint()];
    print!("{}", hints::render_next_action_line(&hints_vec));
    Ok(())
}

/// File-first `unlink` for constellation-migrated workspaces: the edge is
/// dropped from the source file's `links:` frontmatter and one reconcile
/// pass removes it from `constellation_*_v1`. Mirrors `run_file_first` —
/// same resolution, same "relation not found" refusal, same reconcile.
async fn run_unlink_file_first(
    ws: std::path::PathBuf,
    lock: WorkspaceLock,
    source_id: &str,
    target_id: &str,
    relation: &str,
) -> anyhow::Result<()> {
    use forgeplan_core::artifact::store::resolve_artifact_path;

    // No validators here on purpose: the store path runs none either — a
    // malformed ref simply fails the edge lookup and lands on the same
    // "relation not found" refusal below.
    //
    // The store path reports "relation not found" for ANY miss — including
    // a source that doesn't exist — because it matches the edge in
    // `get_all_relations` first. Keep the same refusal here.
    let Some(source_path) = resolve_artifact_path(&ws, source_id).await else {
        anyhow::bail!(
            "Relation '{}' from {} to {} not found",
            relation,
            source_id,
            target_id
        );
    };

    let removed = link::remove_link(&source_path, target_id, relation).await?;
    if removed == 0 {
        anyhow::bail!(
            "Relation '{}' from {} to {} not found",
            relation,
            source_id,
            target_id
        );
    }

    println!("Unlinked: {} --{}--> {}", source_id, relation, target_id);

    common::note_constellation_owns_index();
    // Lock released before reconcile — see run_file_first.
    drop(lock);
    common::reconcile_after_file_mutation(&ws).await;

    let hints_vec = vec![hints::reconcile_parents_hint()];
    print!("{}", hints::render_next_action_line(&hints_vec));
    Ok(())
}
