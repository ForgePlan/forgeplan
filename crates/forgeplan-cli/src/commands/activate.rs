use forgeplan_core::hints::{self, Hint};
use forgeplan_core::lifecycle;
use forgeplan_core::projection;

use crate::commands::common;

pub async fn run(id: &str, force: bool) -> anyhow::Result<()> {
    let (ws, _lock, store) = common::open_store_locked().await?;

    // Capture old status before transition
    let old_status = store
        .get_record(id)
        .await?
        .map(|r| r.status)
        .unwrap_or_else(|| "draft".to_string());

    // Sync file→LanceDB before lifecycle call (preserve user edits)
    if let Some(record) = store.get_record(id).await? {
        projection::sync_file_to_store(&store, &ws, &record).await?;
    }

    let result = lifecycle::activate(&store, id, force)
        .await
        .map_err(|e| anyhow::anyhow!("{}\nFix: forgeplan validate {}", e, id))?;

    // PRD-075 FR-003 (Round 8 audit MED-3): recompute the cached R_eff BEFORE
    // re-rendering the markdown so that if the process crashes between the
    // status flip and the file write, the on-disk source-of-truth still
    // reflects the freshly recomputed score (ADR-003 — markdown is the source
    // of truth and `scan-import` later re-imports it). Doing it AFTER render
    // would leave a stale-r_eff "active" markdown on disk on crash, which is
    // exactly the leak PROB-057 closes.
    common::sync_score_target_or_warn(&store, id).await;

    // Re-render projection with updated status (now picks up fresh R_eff via
    // the record reload).
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
        &store,
        id,
        "update",
        "status",
        Some(&old_status),
        Some("active"),
        "cli",
    )
    .await;

    if result.forced {
        println!("  Activated {id} ({old_status} → active)");
        println!(
            "  Warning: Activated with {} validation error{}",
            result.must_errors.len(),
            if result.must_errors.len() == 1 {
                ""
            } else {
                "s"
            }
        );
    } else {
        println!("  Activated {id} ({old_status} → active)");
    }

    // Hints: suggest evidence if not linked, then reconcile parents.
    let mut emitted: Vec<Hint> = Vec::new();
    if let Some(record) = store.get_record(id).await? {
        let rels = store.get_relations(id).await.unwrap_or_default();
        let incoming = store.get_incoming_relations(id).await.unwrap_or_default();
        let has_evidence = rels
            .iter()
            .chain(incoming.iter())
            .any(|(t, _)| t.to_uppercase().starts_with("EVID-"));
        let kind: forgeplan_core::artifact::types::ArtifactKind = record
            .kind
            .parse()
            .unwrap_or(forgeplan_core::artifact::types::ArtifactKind::Note);
        emitted = forgeplan_core::hints::activate_hints(id, true, has_evidence, &kind);
        if !emitted.is_empty() {
            print!("{}", forgeplan_core::hints::format_hints(&emitted));
        }
    }

    // PRD-075 FR-009: per-target rescore is already done; the next sensible
    // action is parent reconciliation (or whatever activate_hints surfaced
    // as more specific).
    let mut next_hints: Vec<Hint> = Vec::new();
    if let Some(action) = hints::primary_action(&emitted) {
        next_hints.push(Hint::info("activation hint").with_action(action));
    } else {
        next_hints.push(hints::reconcile_parents_hint());
    }
    print!("{}", hints::render_next_action_line(&next_hints));

    // Session: reset to Idle after successful activation
    common::advance_session(forgeplan_core::session::Phase::Idle, None);

    Ok(())
}
