use forgeplan_core::hints;
use forgeplan_core::link;

use crate::commands::common;

pub async fn run(
    source_id: &str,
    target_id: &str,
    relation: &str,
    replace: bool,
) -> anyhow::Result<()> {
    // Normalize relation
    let relation = link::normalize_relation(relation)?;

    let (ws, _lock, store) = common::open_store_locked().await?;

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

    // Verify target exists (warning only)
    if store.get_artifact(target_id).await?.is_none() {
        eprintln!(
            "Warning: Target artifact '{}' not found in workspace (creating link anyway)",
            target_id
        );
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

    // PRD-075 FR-009: hint string lives in core (forgeplan_core::hints) so
    // mutator call sites cannot drift independently.
    let hints_vec = vec![hints::reconcile_parents_hint()];
    print!("{}", hints::render_next_action_line(&hints_vec));
    Ok(())
}

pub async fn run_unlink(source_id: &str, target_id: &str, relation: &str) -> anyhow::Result<()> {
    let relation = link::normalize_relation(relation)?;
    let (ws, _lock, store) = common::open_store_locked().await?;

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
