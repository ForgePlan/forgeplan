use forgeplan_core::hints::{self, Hint};
use forgeplan_core::lifecycle;
use forgeplan_core::projection;

use crate::commands::common;

pub async fn run(id: &str, reason: &str) -> anyhow::Result<()> {
    // `deprecate` displaces a terminal evidence pack and then lists its
    // dependents — both need the relation graph, which on a migrated workspace
    // is the canonical Markdown. So the transition, the `## Deprecation`
    // section and the status flip are all file-first here; the store-backed
    // cached-score rescore has no counterpart and is not faked.
    let (ws, _lock, store) = common::open_store_locked_tolerating_constellation().await?;

    let Some(store) = store else {
        return run_file_first(ws, _lock, id, reason).await;
    };

    // PROB-060 / SPEC-005 Phase 1.5b — slug-aware lookup.
    let id = store
        .resolve_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact '{id}' not found\nFix: forgeplan list"))?;
    let id = id.as_str();

    let old_status = store
        .get_record(id)
        .await?
        .map(|r| r.status)
        .unwrap_or_else(|| "active".to_string());

    // Sync file→LanceDB before lifecycle call (preserve user edits)
    if let Some(record) = store.get_record(id).await? {
        projection::sync_file_to_store(&store, &ws, &record).await?;
    }

    // PRD-071 contract: error path emits a `Fix:` marker line so agents have
    // a deterministic next action — typically `validate` to surface why the
    // transition is rejected (e.g. draft→deprecated requires active first).
    let dependents = lifecycle::deprecate(&store, id, reason)
        .await
        .map_err(|e| anyhow::anyhow!("{}\nFix: forgeplan validate {}", e, id))?;

    // Re-render projection with the updated status AND the appended body.
    //
    // #478: this used `render_projection`, which is files-first — it discards
    // the body it is handed whenever the file already has one (RFC-004, so a
    // user's edits survive `link`/`tag`/`activate`). The status reached the
    // file because status lives in frontmatter; the `## Deprecation` section
    // did not. `lance/` is gitignored, and the next lifecycle command syncs
    // the section-less file body back over the DB, so the reason ended up
    // nowhere at all.
    //
    // Forcing is safe *here* because `sync_file_to_store` ran above: at this
    // point the DB body is the file body plus the section just appended.
    if let Some(record) = store.get_record(id).await? {
        let links = store.get_relations(id).await.unwrap_or_default();
        projection::render_projection_with_body(
            &ws,
            &record.id,
            &record.kind,
            &record.title,
            &record.status,
            &record.depth,
            record.author.as_deref(),
            record.parent_epic.as_deref(),
            record.valid_until.as_deref(),
            &record.body,
            &links,
        )
        .await?;
    }

    common::log_change_field(
        &ws,
        Some(&store),
        id,
        "update",
        ("status", Some(&old_status), Some("deprecated")),
        "cli",
    )
    .await;

    println!("  Deprecated {id}: {reason}");

    // ADR-020: a displaced evidence pack just left the R_eff min of the
    // artifacts it informs — refresh their cached scores now.
    for target in forgeplan_core::scoring::rescore_evidence_dependents(&store, id).await {
        // The dependent is named by its canonical key; the report names it
        // by the display id a reader can act on.
        let display = store
            .resolve_display_id(&target)
            .await
            .ok()
            .flatten()
            .unwrap_or(target);
        println!("  Rescored {display} (displaced evidence left its weakest-link min)");
    }

    if !dependents.is_empty() {
        println!("\nDependents affected:");
        for dep in &dependents {
            println!("  ! {dep} depends on deprecated {id}");
        }
    }

    // Hint: if there are dependents, point at the first one (real ID) so
    // the operator can supersede or refactor; otherwise the action is
    // terminal — surface health.
    let hint_list = if let Some(first_dep) = dependents.first() {
        vec![
            Hint::warning(format!("{} depends on deprecated {}", first_dep, id))
                .with_action(format!("forgeplan get {}", first_dep)),
        ]
    } else {
        vec![Hint::info("Verify workspace integrity").with_action("forgeplan health".to_string())]
    };
    print!("{}", hints::render_next_action_line(&hint_list));

    Ok(())
}

/// File-first `deprecate` for a constellation-migrated workspace.
///
/// #478 is structurally satisfied here rather than worked around: the reason is
/// appended to the canonical body by `update_body_with_projection`, which IS
/// the file write, so the `## Deprecation` section cannot be dropped by a
/// files-first renderer later. Dependents come from the same capture the
/// transition check read.
async fn run_file_first(
    ws: std::path::PathBuf,
    lock: forgeplan_core::workspace::lock::WorkspaceLock,
    id: &str,
    reason: &str,
) -> anyhow::Result<()> {
    let read = common::workspace_read(&ws).await?;
    let canonical = read
        .resolve_id(id)
        .ok_or_else(|| anyhow::anyhow!("Artifact '{id}' not found\nFix: forgeplan list"))?;
    let old_status = read
        .get_record(&canonical)
        .map(|r| r.status.as_str())
        .unwrap_or("active");

    // Scoped so the context's borrow of the lock guard ends before the
    // reconcile pass below — that pass re-acquires the same lock.
    let dependents = {
        let ctx = projection::MutationContext::new(&ws, &lock, None);
        forgeplan_core::lifecycle::canonical::deprecate_captured(&ctx, &read, &canonical, reason)
            .await
            .map_err(|e| anyhow::anyhow!("{}\nFix: forgeplan validate {}", e, canonical))?
    };

    common::log_change_field(
        &ws,
        None,
        &canonical,
        "update",
        ("status", Some(old_status), Some("deprecated")),
        "cli",
    )
    .await;

    common::note_constellation_owns_index();
    // Lock released before reconcile — see `common::reconcile_after_file_mutation`.
    drop(lock);
    let published = common::reconcile_after_file_mutation(&ws).await;

    println!("  Deprecated {canonical}: {reason}");

    if !dependents.is_empty() {
        println!("\nDependents affected:");
        for dep in &dependents {
            println!("  ! {dep} depends on deprecated {canonical}");
        }
    }

    let hint_list = if let Some(first_dep) = dependents.first() {
        vec![
            Hint::warning(format!("{first_dep} depends on deprecated {canonical}"))
                .with_action(format!("forgeplan get {first_dep}")),
        ]
    } else {
        vec![Hint::info("Verify workspace integrity").with_action("forgeplan health".to_string())]
    };
    print!("{}", hints::render_next_action_line(&hint_list));

    if !published {
        println!("  ! derived index not yet updated");
    }
    Ok(())
}
