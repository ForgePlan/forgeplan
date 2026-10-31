use forgeplan_core::hints::{self, Hint};
use forgeplan_core::lifecycle;
use forgeplan_core::projection;

use crate::commands::common;

pub async fn run(id: &str, by: &str) -> anyhow::Result<()> {
    // The successor edge and the dependent-warning list both come from the
    // artifact's relation graph. On a migrated workspace that graph is the
    // canonical Markdown, so the read is one capture and the writes go through
    // the file-first helpers — no retired table is opened.
    let (ws, _lock, store) = common::open_store_locked_tolerating_constellation().await?;

    let Some(store) = store else {
        return run_file_first(ws, _lock, id, by).await;
    };

    // Both references must name one document. Ambiguous aliases and
    // contested keys are refusals, and an absent successor is not a forward
    // reference on this command.
    let resolution = common::resolve_against_workspace(&ws, id).await?;
    let (id, _) = common::require_found_identity(id, &resolution)?;
    let resolution = common::resolve_against_workspace(&ws, by).await?;
    let (by, _) = common::require_found_identity(by, &resolution)?;
    let id = id.as_str();
    let by = by.as_str();

    let old_status = store
        .get_record(id)
        .await?
        .map(|r| r.status)
        .unwrap_or_else(|| "active".to_string());

    // Sync file→LanceDB before lifecycle call (preserve user edits)
    if let Some(record) = store.get_record(id).await? {
        projection::sync_file_to_store(&store, &ws, &record).await?;
    }

    // PRD-071 contract: error path emits a `Fix:` marker line so agents have a
    // deterministic next action (validate the source, or pick a different
    // replacement target).
    let result = lifecycle::supersede(&store, id, by)
        .await
        .map_err(|e| anyhow::anyhow!("{}\nFix: forgeplan validate {}", e, id))?;

    // Re-render projection with updated status
    if let Some(record) = store.get_record(id).await? {
        let links = store.get_relations(id).await.unwrap_or_default();
        projection::render_projection(
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
        ("status", Some(&old_status), Some("superseded")),
        "cli",
    )
    .await;

    println!("  Superseded {id} → {by}");

    // ADR-020: a displaced evidence pack just left the R_eff min of the
    // artifacts it informs — refresh their cached scores now so `get`/`list`
    // serve the recovered value without a manual `forgeplan score`.
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

    for w in &result.warnings {
        println!("  {w}");
    }

    if !result.dependents.is_empty() {
        println!("\nDependents to update:");
        for dep in &result.dependents {
            println!("  ! {dep} depends on superseded {id} → consider updating to {by}");
        }
    }

    // PRD-071: verify the resolved successor exists and is in good shape.
    // Use its slug pre-merge or display id post-merge for the `forgeplan get`
    // hint; the fallback is the canonical key already resolved above.
    let by_ref_form = match store.get_record(by).await? {
        Some(rec) => forgeplan_core::artifact::frontmatter::refs_form_from_body(&rec.body, &rec.id),
        None => by.to_string(),
    };
    let next_hints: Vec<Hint> = vec![
        Hint::info(format!("Superseded by {} — verify successor", by_ref_form))
            .with_action(format!("forgeplan get {}", by_ref_form)),
    ];
    print!("{}", hints::render_next_action_line(&next_hints));

    Ok(())
}

/// File-first `supersede` for a constellation-migrated workspace.
///
/// The transition check, the "replacement is itself retired" refusal, the
/// draft-replacement warning and the dependent list all live in
/// `lifecycle::canonical::supersede_captured`, computed from one capture; the
/// successor edge and the status flip land in the canonical Markdown through
/// the same projection helpers the legacy path uses.
async fn run_file_first(
    ws: std::path::PathBuf,
    lock: forgeplan_core::workspace::lock::WorkspaceLock,
    id: &str,
    by: &str,
) -> anyhow::Result<()> {
    let read = common::workspace_read(&ws).await?;
    let resolution = common::resolve_against_capture(&read, id);
    let (canonical, _) = common::require_found_identity(id, &resolution)?;
    let resolution = common::resolve_against_capture(&read, by);
    let (by_canonical, _) = common::require_found_identity(by, &resolution)?;
    let old_status = read
        .get_record(&canonical)
        .map(|r| r.status.as_str())
        .unwrap_or("active");

    // Scoped so the context's borrow of the lock guard ends before the
    // reconcile pass below — that pass re-acquires the same lock.
    let result = {
        let ctx = projection::MutationContext::new(&ws, &lock, None);
        forgeplan_core::lifecycle::canonical::supersede_captured(
            &ctx,
            &read,
            &canonical,
            &by_canonical,
        )
        .await
        .map_err(|e| anyhow::anyhow!("{}\nFix: forgeplan validate {}", e, canonical))?
    };

    common::log_change_field(
        &ws,
        None,
        &canonical,
        "update",
        ("status", Some(old_status), Some("superseded")),
        "cli",
    )
    .await;

    common::note_constellation_owns_index();
    // Lock released before reconcile — see `common::reconcile_after_file_mutation`.
    drop(lock);
    let published = common::reconcile_after_file_mutation(&ws).await;

    println!("  Superseded {canonical} → {by_canonical}");

    for w in &result.warnings {
        println!("  {w}");
    }

    if !result.dependents.is_empty() {
        println!("\nDependents to update:");
        for dep in &result.dependents {
            println!(
                "  ! {dep} depends on superseded {canonical} → consider updating to {by_canonical}"
            );
        }
    }

    // Use the resolved successor's slug pre-merge or display id post-merge.
    // If the captured record is unavailable, keep its canonical key as the
    // usable reference rather than reverting to an unchecked selector.
    let by_ref_form = read
        .get_record(&by_canonical)
        .map(|rec| forgeplan_core::artifact::frontmatter::refs_form_from_body(&rec.body, &rec.id))
        .unwrap_or_else(|| by_canonical.clone());
    let next_hints: Vec<Hint> = vec![
        Hint::info(format!("Superseded by {} — verify successor", by_ref_form))
            .with_action(format!("forgeplan get {}", by_ref_form)),
    ];
    print!("{}", hints::render_next_action_line(&next_hints));

    if !published {
        println!("  ! derived index not yet updated");
    }
    Ok(())
}
