use forgeplan_core::hints::{self, Hint};
use forgeplan_core::lifecycle;
use forgeplan_core::projection;

use crate::commands::common;

pub async fn run(id: &str, reason: &str, until: &str) -> anyhow::Result<()> {
    let (ws, _lock, store) = common::open_store_locked().await?;

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
        Some("stale"),
        Some("active"),
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
