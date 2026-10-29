use forgeplan_core::hints::{self, Hint};
use forgeplan_core::lifecycle;
use forgeplan_core::projection;

use crate::commands::common;

pub async fn run(id: &str, reason: &str, until: &str) -> anyhow::Result<()> {
    // `renew` reads one artifact and writes three things into its canonical
    // Markdown: a `## Renewal` section, `status: active`, and the new
    // `valid_until`. None of that needs the relation graph — the transition
    // check is a status comparison — so this is file-first on a migrated
    // workspace, no capture and no store.
    let (ws, _lock, store) = common::open_store_locked_tolerating_constellation().await?;

    let Some(store) = store else {
        return run_file_first(ws, _lock, id, reason, until).await;
    };

    // PROB-060 / SPEC-005 Phase 2.6 (CD-6) — accept slug or display id.
    let id = store
        .resolve_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact '{id}' not found\nFix: forgeplan list"))?;
    let id = id.as_str();

    // Sync file→LanceDB before lifecycle call (preserve user edits)
    if let Some(record) = store.get_record(id).await? {
        projection::sync_file_to_store(&store, &ws, &record).await?;
    }

    let result = lifecycle::renew(&store, id, reason, until).await?;

    // Re-render projection with the updated status AND the appended body.
    // #478 — `render_projection` is files-first and would drop the
    // `## Renewal` section. Safe to force: `sync_file_to_store` ran above.
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
        ("status", Some("stale"), Some("active")),
        "cli",
    )
    .await;

    println!("  Renewed {id} (stale → active)");
    if let Some(old) = &result.old_valid_until {
        println!("  Valid until: {old} → {}", result.new_valid_until);
    } else {
        println!("  Valid until: {}", result.new_valid_until);
    }

    // PRD-071: post-renew, score the artifact to confirm R_eff still holds
    // with refreshed evidence dating.
    // PROB-060 / SPEC-005 / ADR-012 (W1.B, CD-5) — slug pre-merge / display
    // id post-merge so the score command stays canonical.
    let ref_form = match store.get_record(id).await? {
        Some(rec) => forgeplan_core::artifact::frontmatter::refs_form_from_body(&rec.body, &rec.id),
        None => id.to_string(),
    };
    let next_hints: Vec<Hint> = vec![
        Hint::info("Renewed — re-score to confirm trust")
            .with_action(format!("forgeplan score {}", ref_form)),
    ];
    print!("{}", hints::render_next_action_line(&next_hints));

    Ok(())
}

/// File-first `renew` for a constellation-migrated workspace.
///
/// `renew_captured` reuses the store path's date validation, transition check
/// and reason sanitisation, then writes the body section, the status and the
/// new `valid_until` as three canonical frontmatter/body edits. The last of
/// those needed a file-first writer that did not exist before this cutover —
/// without it `renew` would have had to refuse, and ADR-005's stale-recovery
/// path would be unreachable on a migrated workspace.
async fn run_file_first(
    ws: std::path::PathBuf,
    lock: forgeplan_core::workspace::lock::WorkspaceLock,
    id: &str,
    reason: &str,
    until: &str,
) -> anyhow::Result<()> {
    let read = common::workspace_read(&ws).await?;
    let canonical = read
        .resolve_id(id)
        .ok_or_else(|| anyhow::anyhow!("Artifact '{id}' not found\nFix: forgeplan list"))?;

    // Scoped so the context's borrow of the lock guard ends before the
    // reconcile pass below — that pass re-acquires the same lock.
    let result = {
        let ctx = projection::MutationContext::new(&ws, &lock, None);
        forgeplan_core::lifecycle::canonical::renew_captured(&ctx, &read, &canonical, reason, until)
            .await?
    };

    common::log_change_field(
        &ws,
        None,
        &canonical,
        "update",
        ("status", Some("stale"), Some("active")),
        "cli",
    )
    .await;

    let ref_form = forgeplan_core::lifecycle::canonical::refs_form(&read, &canonical);

    common::note_constellation_owns_index();
    // Lock released before reconcile — see `common::reconcile_after_file_mutation`.
    drop(lock);
    let published = common::reconcile_after_file_mutation(&ws).await;

    println!("  Renewed {canonical} (stale → active)");
    if let Some(old) = &result.old_valid_until {
        println!("  Valid until: {old} → {}", result.new_valid_until);
    } else {
        println!("  Valid until: {}", result.new_valid_until);
    }

    let next_hints: Vec<Hint> = vec![
        Hint::info("Renewed — re-score to confirm trust")
            .with_action(format!("forgeplan score {}", ref_form)),
    ];
    print!("{}", hints::render_next_action_line(&next_hints));

    if !published {
        println!("  ! derived index not yet updated");
    }
    Ok(())
}
