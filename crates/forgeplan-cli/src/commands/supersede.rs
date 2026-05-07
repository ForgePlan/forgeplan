use forgeplan_core::hints::{self, Hint};
use forgeplan_core::lifecycle;
use forgeplan_core::projection;

use crate::commands::common;

pub async fn run(id: &str, by: &str) -> anyhow::Result<()> {
    let (ws, _lock, store) = common::open_store_locked().await?;

    // PROB-060 / SPEC-005 Phase 2.6 (CD-6) — accept slug or display id.
    // The source must resolve (it's the artifact being marked superseded);
    // the `--by` target may not exist yet (forward-reference cross-PR), so
    // we fall back to the raw input mirroring `link` semantics.
    let id = store
        .resolve_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact '{id}' not found\nFix: forgeplan list"))?;
    let id = id.as_str();
    let by_canonical = store.resolve_id(by).await?;
    let by = by_canonical.as_deref().unwrap_or(by);

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
        &store,
        id,
        "update",
        "status",
        Some(&old_status),
        Some("superseded"),
        "cli",
    )
    .await;

    println!("  Superseded {id} → {by}");

    for w in &result.warnings {
        println!("  {w}");
    }

    if !result.dependents.is_empty() {
        println!("\nDependents to update:");
        for dep in &result.dependents {
            println!("  ! {dep} depends on superseded {id} → consider updating to {by}");
        }
    }

    // PRD-071: verify the new successor exists and is in good shape.
    // PROB-060 / SPEC-005 / ADR-012 (W1.B, CD-5) — slug pre-merge / display
    // id post-merge for the successor `forgeplan get` hint. If the target
    // can't be loaded (forward-reference cross-PR), the raw input is the
    // safest fallback the agent can re-run.
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
