use forgeplan_core::hints::{self, Hint};
use forgeplan_core::lifecycle;
use forgeplan_core::projection;

use crate::commands::common;

pub async fn run(id: &str, reason: &str) -> anyhow::Result<()> {
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

    // Auto-generate new ID: same prefix, next sequence
    // PRD-071 contract: missing-artifact error emits a `Fix:` marker so the
    // agent has a deterministic next step (list available artifacts).
    let record = store
        .get_record(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact not found: {id}\nFix: forgeplan list"))?;
    let new_id = store.next_id(&record.kind).await?;

    // PRD-071 contract: lifecycle gate failure (e.g. status=draft) emits
    // a `Fix:` marker — `validate` reveals what state is missing.
    let result = lifecycle::reopen(&store, id, reason, &new_id)
        .await
        .map_err(|e| anyhow::anyhow!("{}\nFix: forgeplan validate {}", e, id))?;

    // Render projections for both old (deprecated) and new (draft)
    if let Some(old_record) = store.get_record(&result.old_id).await? {
        let links = store
            .get_relations(&result.old_id)
            .await
            .unwrap_or_default();
        projection::render_projection(
            &ws,
            &old_record.id,
            &old_record.kind,
            &old_record.title,
            &old_record.status,
            &old_record.depth,
            old_record.author.as_deref(),
            old_record.parent_epic.as_deref(),
            old_record.valid_until.as_deref(),
            &old_record.body,
            &links,
        )
        .await?;
    }
    if let Some(new_record) = store.get_record(&result.new_id).await? {
        let links = store
            .get_relations(&result.new_id)
            .await
            .unwrap_or_default();
        projection::render_projection(
            &ws,
            &new_record.id,
            &new_record.kind,
            &new_record.title,
            &new_record.status,
            &new_record.depth,
            new_record.author.as_deref(),
            new_record.parent_epic.as_deref(),
            new_record.valid_until.as_deref(),
            &new_record.body,
            &links,
        )
        .await?;
    }

    common::log_change_field(
        &store,
        id,
        "update",
        "status",
        Some(&record.status),
        Some("deprecated"),
        "cli",
    )
    .await;
    common::log_change_field(
        &store,
        &new_id,
        "create",
        "status",
        None,
        Some("draft"),
        "cli",
    )
    .await;

    println!("  Reopened {id} → deprecated");
    println!(
        "  Created {} ({}, draft) with lineage from {id}",
        result.new_id, result.new_kind
    );
    println!("  Link: {} --based_on--> {id}", result.new_id);

    // PRD-071: primary action is to validate the freshly-minted draft so
    // the user can fill MUST sections and chain into activation.
    let next_hints: Vec<Hint> = vec![
        Hint::info(format!(
            "New draft {} created — fill MUST sections",
            result.new_id
        ))
        .with_action(format!("forgeplan validate {}", result.new_id)),
    ];
    print!("{}", hints::render_next_action_line(&next_hints));

    Ok(())
}
